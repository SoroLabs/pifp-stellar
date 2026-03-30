//! Database layer — migrations, queries, and cursor management.

use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tracing::info;

use crate::errors::Result;
use crate::events::{EventRecord, PifpEvent};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GlobalStats {
    pub total_projects: i64,
    pub total_tvl: String,
    pub total_donors: i64,
    pub completed_projects: i64,
    pub failed_projects: i64,
    pub updated_at: i64,
}

/// Establish a SQLite connection pool and run pending migrations.
pub async fn init_pool(database_url: &str) -> Result<SqlitePool> {
    // Make sure the file is created if it doesn't exist yet.
    let url = if database_url.starts_with("sqlite:") {
        database_url.to_string()
    } else {
        format!("sqlite:{database_url}")
    };

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    info!("Database migrations applied successfully");
    Ok(pool)
}

// ─────────────────────────────────────────────────────────
// Cursor helpers
// ─────────────────────────────────────────────────────────

/// Read the last-seen ledger from the cursor row.
/// Returns `0` when no cursor has been persisted yet.
pub async fn get_last_ledger(pool: &SqlitePool) -> Result<i64> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT last_ledger FROM indexer_cursor WHERE id = 1")
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(v,)| v).unwrap_or(0))
}

/// Persist the last-seen ledger (and optionally a pagination cursor string).
pub async fn save_cursor(
    pool: &SqlitePool,
    last_ledger: i64,
    last_cursor: Option<&str>,
) -> Result<()> {
    sqlx::query("UPDATE indexer_cursor SET last_ledger = ?1, last_cursor = ?2 WHERE id = 1")
        .bind(last_ledger)
        .bind(last_cursor)
        .execute(pool)
        .await?;
    Ok(())
}

/// Read back the raw cursor string (used to resume pagination mid-ledger).
pub async fn get_cursor_string(pool: &SqlitePool) -> Result<Option<String>> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT last_cursor FROM indexer_cursor WHERE id = 1")
            .fetch_optional(pool)
            .await?;
    Ok(row.and_then(|(v,)| v))
}

// ─────────────────────────────────────────────────────────
// Event writes
// ─────────────────────────────────────────────────────────

/// Persist a batch of decoded events.  Events that share the same
/// `(ledger, tx_hash, event_type, project_id)` tuple are silently ignored
/// to make the indexer idempotent.
#[allow(dead_code)]
pub async fn insert_events(pool: &SqlitePool, events: &[PifpEvent]) -> Result<usize> {
    Ok(insert_events_with_new(pool, events).await?.len())
}

/// Persist a batch and return only events that were newly inserted.
pub async fn insert_events_with_new(
    pool: &SqlitePool,
    events: &[PifpEvent],
) -> Result<Vec<PifpEvent>> {
    let mut inserted_events = Vec::new();
    for ev in events {
        let rows_affected = sqlx::query(
            r#"
            INSERT OR IGNORE INTO events
                (event_type, project_id, actor, amount, ledger, timestamp, contract_id, tx_hash, extra_data)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
        )
        .bind(&ev.event_type)
        .bind(&ev.project_id)
        .bind(&ev.actor)
        .bind(&ev.amount)
        .bind(ev.ledger)
        .bind(ev.timestamp)
        .bind(&ev.contract_id)
        .bind(&ev.tx_hash)
        .bind(&ev.extra_data)
        .execute(pool)
        .await?
        .rows_affected();

        if rows_affected > 0 {
            inserted_events.push(ev.clone());
            // Update Project Registry for life-cycle tracking
            if let Some(id) = &ev.project_id {
                match ev.event_type.as_str() {
                    "project_created" => {
                        sqlx::query(
                            r#"
                            INSERT OR IGNORE INTO projects (project_id, creator, goal, primary_token, created_ledger)
                            VALUES (?1, ?2, ?3, ?4, ?5)
                            "#,
                        )
                        .bind(id)
                        .bind(ev.actor.as_deref().unwrap_or("unknown"))
                        .bind(ev.amount.as_deref().unwrap_or("0"))
                        .bind(ev.extra_data.as_deref().unwrap_or("unknown"))
                        .bind(ev.ledger)
                        .execute(pool)
                        .await?;
                    }
                    "project_active" => {
                        sqlx::query("UPDATE projects SET status = 'Active' WHERE project_id = ?1")
                            .bind(id)
                            .execute(pool)
                            .await?;
                    }
                    "project_verified" => {
                        sqlx::query(
                            "UPDATE projects SET status = 'Completed' WHERE project_id = ?1",
                        )
                        .bind(id)
                        .execute(pool)
                        .await?;
                    }
                    "project_expired" => {
                        sqlx::query("UPDATE projects SET status = 'Expired' WHERE project_id = ?1")
                            .bind(id)
                            .execute(pool)
                            .await?;
                    }
                    _ => {}
                }
            }

            // Always update global stats incrementally
            update_global_stats(pool, ev).await?;
        }
    }
    Ok(inserted_events)
}

/// Incrementally update the global project statistics based on the observed event.
pub async fn update_global_stats(pool: &SqlitePool, event: &PifpEvent) -> Result<()> {
    let mut tx = pool.begin().await?;

    match event.event_type.as_str() {
        "project_created" => {
            sqlx::query("UPDATE project_stats SET total_projects = total_projects + 1, updated_at = strftime('%s', 'now') WHERE id = 1")
                .execute(&mut *tx)
                .await?;
        }
        "project_funded" => {
            // Update TVL
            if let Some(amt_str) = &event.amount {
                // We use big-integer addition in SQLite if possible, but SQLite doesn't support i128 natively.
                // For this implementation, we'll load, add in Rust, and save back.
                let current_tvl_str: (String,) =
                    sqlx::query_as("SELECT total_tvl FROM project_stats WHERE id = 1")
                        .fetch_one(&mut *tx)
                        .await?;

                let current_tvl = current_tvl_str.0.parse::<i128>().unwrap_or(0);
                let added_amt = amt_str.parse::<i128>().unwrap_or(0);
                let new_tvl = (current_tvl + added_amt).to_string();

                sqlx::query("UPDATE project_stats SET total_tvl = ?1, updated_at = strftime('%s', 'now') WHERE id = 1")
                    .bind(new_tvl)
                    .execute(&mut *tx)
                    .await?;
            }

            // Update unique donor count
            if let Some(actor) = &event.actor {
                let res = sqlx::query("INSERT OR IGNORE INTO unique_donors (address) VALUES (?1)")
                    .bind(actor)
                    .execute(&mut *tx)
                    .await?;

                if res.rows_affected() > 0 {
                    sqlx::query("UPDATE project_stats SET total_donors = total_donors + 1, updated_at = strftime('%s', 'now') WHERE id = 1")
                        .execute(&mut *tx)
                        .await?;
                }
            }
        }
        "project_verified" => {
            sqlx::query("UPDATE project_stats SET completed_projects = completed_projects + 1, updated_at = strftime('%s', 'now') WHERE id = 1")
                .execute(&mut *tx)
                .await?;
        }
        "project_expired" | "project_cancelled" => {
            sqlx::query("UPDATE project_stats SET failed_projects = failed_projects + 1, updated_at = strftime('%s', 'now') WHERE id = 1")
                .execute(&mut *tx)
                .await?;
        }
        _ => {}
    }

    tx.commit().await?;
    Ok(())
}

/// Fetch the pre-calculated global statistics.
pub async fn get_global_stats(pool: &SqlitePool) -> Result<GlobalStats> {
    let stats = sqlx::query_as::<_, GlobalStats>("SELECT * FROM project_stats WHERE id = 1")
        .fetch_one(pool)
        .await?;
    Ok(stats)
}

// ─────────────────────────────────────────────────────────
// Webhooks
// ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WebhookTarget {
    pub webhook_id: i64,
    pub url: String,
    pub secret: String,
    pub event_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookRegistration {
    pub id: i64,
    pub url: String,
    pub event_types: Vec<String>,
    pub enabled: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWebhookRegistration {
    pub url: String,
    pub secret: String,
    pub event_types: Vec<String>,
}

pub async fn create_webhook(
    pool: &SqlitePool,
    input: &NewWebhookRegistration,
) -> Result<WebhookRegistration> {
    let mut tx = pool.begin().await?;
    let enabled = true;
    let insert_res = sqlx::query("INSERT INTO webhooks (url, secret, enabled) VALUES (?1, ?2, 1)")
        .bind(&input.url)
        .bind(&input.secret)
        .execute(&mut *tx)
        .await?;
    let webhook_id = insert_res.last_insert_rowid();

    let mut event_types: Vec<String> = input
        .event_types
        .iter()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect();
    if event_types.is_empty() {
        event_types.push("*".to_string());
    }

    for event_type in &event_types {
        sqlx::query(
            "INSERT OR IGNORE INTO webhook_subscriptions (webhook_id, event_type) VALUES (?1, ?2)",
        )
        .bind(webhook_id)
        .bind(event_type)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let created_at: (i64,) = sqlx::query_as("SELECT created_at FROM webhooks WHERE id = ?1")
        .bind(webhook_id)
        .fetch_one(pool)
        .await?;

    Ok(WebhookRegistration {
        id: webhook_id,
        url: input.url.clone(),
        event_types,
        enabled,
        created_at: created_at.0,
    })
}

pub async fn list_webhooks(pool: &SqlitePool) -> Result<Vec<WebhookRegistration>> {
    let rows = sqlx::query_as::<_, (i64, String, i32, i64, String)>(
        r#"
        SELECT w.id, w.url, w.enabled, w.created_at, s.event_type
        FROM webhooks w
        JOIN webhook_subscriptions s ON s.webhook_id = w.id
        ORDER BY w.id ASC, s.event_type ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut out: Vec<WebhookRegistration> = Vec::new();
    for (id, url, enabled, created_at, event_type) in rows {
        match out.last_mut() {
            Some(last) if last.id == id => last.event_types.push(event_type),
            _ => out.push(WebhookRegistration {
                id,
                url,
                event_types: vec![event_type],
                enabled: enabled != 0,
                created_at,
            }),
        }
    }
    Ok(out)
}

pub async fn get_webhooks_for_event(
    pool: &SqlitePool,
    event_type: &str,
) -> Result<Vec<WebhookTarget>> {
    let rows = sqlx::query_as::<_, WebhookTarget>(
        r#"
        SELECT w.id as webhook_id, w.url, w.secret, s.event_type
        FROM webhooks w
        JOIN webhook_subscriptions s ON s.webhook_id = w.id
        WHERE w.enabled = 1
          AND (s.event_type = ?1 OR s.event_type = '*')
        "#,
    )
    .bind(event_type)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[allow(clippy::too_many_arguments)]
pub async fn log_webhook_delivery_attempt(
    pool: &SqlitePool,
    webhook_id: i64,
    event_type: &str,
    payload: &str,
    status_code: Option<i32>,
    success: bool,
    attempt: i32,
    error: Option<&str>,
    latency_ms: i64,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO webhook_deliveries
            (webhook_id, event_type, payload, status_code, success, attempt, error, latency_ms)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
    )
    .bind(webhook_id)
    .bind(event_type)
    .bind(payload)
    .bind(status_code)
    .bind(if success { 1 } else { 0 })
    .bind(attempt)
    .bind(error)
    .bind(latency_ms)
    .execute(pool)
    .await?;
    Ok(())
}

/// Count delivery attempts for a webhook. Useful for tests and diagnostics.
#[cfg(test)]
pub async fn count_webhook_deliveries(pool: &SqlitePool, webhook_id: i64) -> Result<i64> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM webhook_deliveries WHERE webhook_id = ?1")
            .bind(webhook_id)
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}

// ─────────────────────────────────────────────────────────
// Event reads
// ─────────────────────────────────────────────────────────

/// Fetch all events for a given project, ordered by ledger ascending.
#[allow(dead_code)]
pub async fn get_events_for_project(
    pool: &SqlitePool,
    project_id: &str,
) -> Result<Vec<EventRecord>> {
    let rows = sqlx::query_as::<_, EventRecord>(
        r#"
        SELECT id, event_type, project_id, actor, amount, ledger, timestamp,
               contract_id, tx_hash, extra_data, created_at
        FROM   events
        WHERE  project_id = ?1
        ORDER  BY ledger ASC, id ASC
        "#,
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Fetch all events, ordered by ledger ascending.
pub async fn get_all_events(pool: &SqlitePool) -> Result<Vec<EventRecord>> {
    let rows = sqlx::query_as::<_, EventRecord>(
        r#"
        SELECT id, event_type, project_id, actor, amount, ledger, timestamp,
               contract_id, tx_hash, extra_data, created_at
        FROM   events
        ORDER  BY ledger ASC, id ASC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TopProject {
    pub project_id: String,
    pub total_funded: i64,
    pub donation_events: i64,
}

/// Return top projects ranked by total funded amount from indexed funding events.
pub async fn get_top_projects(pool: &SqlitePool, limit: u32) -> Result<Vec<TopProject>> {
    let capped_limit = limit.clamp(1, 100) as i64;
    let rows = sqlx::query_as::<_, TopProject>(
        r#"
        SELECT
            project_id,
            COALESCE(SUM(CAST(amount AS INTEGER)), 0) AS total_funded,
            COUNT(*) AS donation_events
        FROM events
        WHERE event_type = 'project_funded'
          AND project_id IS NOT NULL
        GROUP BY project_id
        ORDER BY total_funded DESC, donation_events DESC, project_id ASC
        LIMIT ?1
        "#,
    )
    .bind(capped_limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Return the current number of active projects inferred from latest status events.
pub async fn get_active_projects_count(pool: &SqlitePool) -> Result<i64> {
    let row: (i64,) = sqlx::query_as(
        r#"
        WITH status_events AS (
            SELECT project_id, event_type, ledger, id
            FROM events
            WHERE project_id IS NOT NULL
              AND event_type IN (
                  'project_active',
                  'project_verified',
                  'project_expired',
                  'project_cancelled'
              )
        ),
        ranked AS (
            SELECT
                project_id,
                event_type,
                ROW_NUMBER() OVER (
                    PARTITION BY project_id
                    ORDER BY ledger DESC, id DESC
                ) AS rn
            FROM status_events
        )
        SELECT COUNT(*) FROM ranked
        WHERE rn = 1 AND event_type = 'project_active'
        "#,
    )
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

// ─────────────────────────────────────────────────────────
// Project Registry Reads
// ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ProjectRecord {
    pub project_id: String,
    pub creator: String,
    pub status: String,
    pub goal: String,
    pub primary_token: String,
    pub created_ledger: i64,
    pub created_at: i64,
    pub title: String,
    pub description: String,
}

/// List projects with filtering and pagination.
pub async fn list_projects(
    pool: &SqlitePool,
    status: Option<String>,
    creator: Option<String>,
    limit: i64,
    offset: i64,
) -> Result<Vec<ProjectRecord>> {
    let mut query = "SELECT * FROM projects WHERE 1=1".to_string();
    if status.is_some() {
        query.push_str(" AND status = ?");
    }
    if creator.is_some() {
        query.push_str(" AND creator = ?");
    }
    query.push_str(" ORDER BY created_ledger DESC LIMIT ? OFFSET ?");

    let mut q = sqlx::query_as::<_, ProjectRecord>(&query);
    if let Some(s) = status {
        q = q.bind(s);
    }
    if let Some(c) = creator {
        q = q.bind(c);
    }
    q = q.bind(limit).bind(offset);

    let rows = q.fetch_all(pool).await?;
    Ok(rows)
}

/// Full-text search over project title and description.
///
/// Uses SQLite FTS5 with bm25() ranking (title weighted 10x over description).
/// An empty or blank query returns an empty result set rather than an error.
///
/// PostgreSQL equivalent would use `websearch_to_tsquery` + `ts_rank`.
pub async fn search_projects(
    pool: &SqlitePool,
    query: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<ProjectRecord>> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(vec![]);
    }

    // Sanitize: strip FTS5 special chars that would cause a parse error.
    let safe_q = q.replace(['"', '\'', '*', '^', '(', ')', '-'], " ");
    let safe_q = safe_q.trim();
    if safe_q.is_empty() {
        return Ok(vec![]);
    }

    // Build a prefix-match query: each whitespace-separated token becomes "token*"
    let fts_query = safe_q
        .split_whitespace()
        .map(|t| format!("{t}*"))
        .collect::<Vec<_>>()
        .join(" ");

    // bm25 weights: column 0 (title) = -10.0, column 1 (description) = -1.0
    // Lower bm25 score = better match; ORDER BY ASC puts best matches first.
    let rows = sqlx::query_as::<_, ProjectRecord>(
        r#"
        SELECT p.project_id, p.creator, p.status, p.goal, p.primary_token,
               p.created_ledger, p.created_at, p.title, p.description
        FROM   projects_fts f
        JOIN   projects p ON p.rowid = f.rowid
        WHERE  projects_fts MATCH ?1
        ORDER  BY bm25(projects_fts, 10.0, 1.0) ASC
        LIMIT  ?2 OFFSET ?3
        "#,
    )
    .bind(&fts_query)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Fetch project history with pagination.
pub async fn get_project_history(
    pool: &SqlitePool,
    project_id: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<EventRecord>> {
    let rows = sqlx::query_as::<_, EventRecord>(
        r#"
        SELECT id, event_type, project_id, actor, amount, ledger, timestamp,
               contract_id, tx_hash, extra_data, created_at
        FROM   events
        WHERE  project_id = ?1
        ORDER  BY ledger DESC, id DESC
        LIMIT  ?2 OFFSET ?3
        "#,
    )
    .bind(project_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ─────────────────────────────────────────────────────────
// Quorum management
// ─────────────────────────────────────────────────────────

/// Get the global quorum threshold for proof verification.
pub async fn get_quorum_threshold(pool: &SqlitePool) -> Result<u32> {
    let row: Option<(i32,)> = sqlx::query_as("SELECT threshold FROM quorum_settings WHERE id = 1")
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(v,)| v as u32).unwrap_or(1))
}

/// Update the global quorum threshold.
pub async fn set_quorum_threshold(pool: &SqlitePool, threshold: u32) -> Result<()> {
    sqlx::query("UPDATE quorum_settings SET threshold = ?1 WHERE id = 1")
        .bind(threshold as i32)
        .execute(pool)
        .await?;
    Ok(())
}

/// Record an oracle vote for a specific project and proof hash.
pub async fn record_vote(
    pool: &SqlitePool,
    project_id: &str,
    oracle: &str,
    hash: &str,
) -> Result<bool> {
    let res = sqlx::query(
        "INSERT OR IGNORE INTO oracle_votes (project_id, oracle_address, proof_hash) VALUES (?1, ?2, ?3)",
    )
    .bind(project_id)
    .bind(oracle)
    .bind(hash)
    .execute(pool)
    .await?;

    Ok(res.rows_affected() > 0)
}

#[derive(Serialize)]
pub struct QuorumStatus {
    pub project_id: String,
    pub threshold: u32,
    pub votes: Vec<VoteInfo>,
    pub consensus_reached: bool,
}

#[derive(Serialize)]
pub struct VoteInfo {
    pub proof_hash: String,
    pub count: u32,
}

/// Fetch the current quorum status for a project.
pub async fn get_quorum_status(pool: &SqlitePool, project_id: &str) -> Result<QuorumStatus> {
    let threshold = get_quorum_threshold(pool).await?;

    // Query to count matching votes per hash for the given project
    let votes = sqlx::query_as::<_, (String, i32)>(
        "SELECT proof_hash, COUNT(*) as count FROM oracle_votes WHERE project_id = ?1 GROUP BY proof_hash",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    let vote_info: Vec<VoteInfo> = votes
        .into_iter()
        .map(|(proof_hash, count)| VoteInfo {
            proof_hash,
            count: count as u32,
        })
        .collect();

    let consensus_reached = vote_info.iter().any(|v| v.count >= threshold);

    Ok(QuorumStatus {
        project_id: project_id.to_string(),
        threshold,
        votes: vote_info,
        consensus_reached,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // Run migrations manually from the migrations folder
        // For simplicity in unit tests, we can just run the specific DDL
        sqlx::query(
            "CREATE TABLE events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                project_id TEXT,
                actor TEXT,
                amount TEXT,
                ledger INTEGER NOT NULL,
                timestamp INTEGER NOT NULL,
                contract_id TEXT NOT NULL,
                tx_hash TEXT,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("CREATE INDEX idx_events_project_id ON events (project_id);")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("CREATE INDEX idx_events_event_type ON events (event_type);")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE webhooks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                secret TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TABLE webhook_subscriptions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                webhook_id INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                UNIQUE(webhook_id, event_type)
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TABLE webhook_deliveries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                webhook_id INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                status_code INTEGER,
                success INTEGER NOT NULL DEFAULT 0,
                attempt INTEGER NOT NULL,
                error TEXT,
                latency_ms INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            );",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE quorum_settings (id INTEGER PRIMARY KEY CHECK (id = 1), threshold INTEGER NOT NULL DEFAULT 1);",
        ).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO quorum_settings (id, threshold) VALUES (1, 1);")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE oracle_votes (id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT NOT NULL, oracle_address TEXT NOT NULL, proof_hash TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP, UNIQUE(project_id, oracle_address));",
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE project_stats (id INTEGER PRIMARY KEY CHECK (id = 1), total_projects INTEGER NOT NULL DEFAULT 0, total_tvl TEXT NOT NULL DEFAULT '0', total_donors INTEGER NOT NULL DEFAULT 0, completed_projects INTEGER NOT NULL DEFAULT 0, failed_projects INTEGER NOT NULL DEFAULT 0, updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')));",
        ).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO project_stats (id, total_projects, total_tvl, total_donors, completed_projects, failed_projects) VALUES (1, 0, '0', 0, 0, 0);").execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE unique_donors (address TEXT PRIMARY KEY, created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')));",
        ).execute(&pool).await.unwrap();

        // FTS5 virtual table + triggers (mirrors migration 007)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS projects (
                project_id TEXT PRIMARY KEY,
                creator TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'Funding',
                goal TEXT NOT NULL DEFAULT '0',
                primary_token TEXT NOT NULL DEFAULT '',
                created_ledger INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                title TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL DEFAULT ''
            );",
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE VIRTUAL TABLE IF NOT EXISTS projects_fts USING fts5(
                title, description, content='projects', content_rowid='rowid'
            );",
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TRIGGER IF NOT EXISTS projects_fts_insert
                AFTER INSERT ON projects BEGIN
                    INSERT INTO projects_fts (rowid, title, description)
                    VALUES (new.rowid, new.title, new.description);
                END;",
        ).execute(&pool).await.unwrap();

        pool
    }

    #[tokio::test]
    async fn test_quorum_threshold() {
        let pool = setup_test_db().await;

        // Default should be 1
        assert_eq!(get_quorum_threshold(&pool).await.unwrap(), 1);

        // Update to 3
        set_quorum_threshold(&pool, 3).await.unwrap();
        assert_eq!(get_quorum_threshold(&pool).await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_get_top_projects() {
        let pool = setup_test_db().await;
        sqlx::query("INSERT INTO events (event_type, project_id, amount, ledger, timestamp, contract_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
            .bind("project_funded")
            .bind("1")
            .bind("100")
            .bind(1i64)
            .bind(1i64)
            .bind("c")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO events (event_type, project_id, amount, ledger, timestamp, contract_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
            .bind("project_funded")
            .bind("2")
            .bind("300")
            .bind(2i64)
            .bind(2i64)
            .bind("c")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO events (event_type, project_id, amount, ledger, timestamp, contract_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
            .bind("project_funded")
            .bind("1")
            .bind("50")
            .bind(3i64)
            .bind(3i64)
            .bind("c")
            .execute(&pool)
            .await
            .unwrap();

        let top = get_top_projects(&pool, 10).await.unwrap();
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].project_id, "2");
        assert_eq!(top[0].total_funded, 300);
        assert_eq!(top[1].project_id, "1");
        assert_eq!(top[1].total_funded, 150);
    }

    #[tokio::test]
    async fn test_get_active_projects_count() {
        let pool = setup_test_db().await;
        sqlx::query("INSERT INTO events (event_type, project_id, ledger, timestamp, contract_id) VALUES (?1, ?2, ?3, ?4, ?5)")
            .bind("project_active")
            .bind("1")
            .bind(10i64)
            .bind(10i64)
            .bind("c")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO events (event_type, project_id, ledger, timestamp, contract_id) VALUES (?1, ?2, ?3, ?4, ?5)")
            .bind("project_active")
            .bind("2")
            .bind(10i64)
            .bind(10i64)
            .bind("c")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO events (event_type, project_id, ledger, timestamp, contract_id) VALUES (?1, ?2, ?3, ?4, ?5)")
            .bind("project_verified")
            .bind("2")
            .bind(11i64)
            .bind(11i64)
            .bind("c")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO events (event_type, project_id, ledger, timestamp, contract_id) VALUES (?1, ?2, ?3, ?4, ?5)")
            .bind("project_cancelled")
            .bind("3")
            .bind(12i64)
            .bind(12i64)
            .bind("c")
            .execute(&pool)
            .await
            .unwrap();

        let count = get_active_projects_count(&pool).await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_create_and_list_webhooks() {
        let pool = setup_test_db().await;
        let created = create_webhook(
            &pool,
            &NewWebhookRegistration {
                url: "https://example.com/hook".to_string(),
                secret: "s3cr3t".to_string(),
                event_types: vec!["project_active".to_string(), "project_verified".to_string()],
            },
        )
        .await
        .unwrap();
        assert!(created.id > 0);
        let all = list_webhooks(&pool).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].event_types.len(), 2);
    }

    #[tokio::test]
    async fn test_get_webhooks_for_event_with_wildcard() {
        let pool = setup_test_db().await;
        let specific = create_webhook(
            &pool,
            &NewWebhookRegistration {
                url: "https://example.com/active".to_string(),
                secret: "a".to_string(),
                event_types: vec!["project_active".to_string()],
            },
        )
        .await
        .unwrap();
        let wildcard = create_webhook(
            &pool,
            &NewWebhookRegistration {
                url: "https://example.com/all".to_string(),
                secret: "b".to_string(),
                event_types: vec!["*".to_string()],
            },
        )
        .await
        .unwrap();

        let targets = get_webhooks_for_event(&pool, "project_active")
            .await
            .unwrap();
        assert_eq!(targets.len(), 2);
        let ids: Vec<i64> = targets.iter().map(|t| t.webhook_id).collect();
        assert!(ids.contains(&specific.id));
        assert!(ids.contains(&wildcard.id));
    }

    #[tokio::test]
    async fn test_log_webhook_delivery_attempt() {
        let pool = setup_test_db().await;
        let created = create_webhook(
            &pool,
            &NewWebhookRegistration {
                url: "https://example.com/hook".to_string(),
                secret: "x".to_string(),
                event_types: vec!["*".to_string()],
            },
        )
        .await
        .unwrap();

        log_webhook_delivery_attempt(
            &pool,
            created.id,
            "project_active",
            "{\"sample\":true}",
            Some(500),
            false,
            1,
            Some("server error"),
            23,
        )
        .await
        .unwrap();
        assert_eq!(
            count_webhook_deliveries(&pool, created.id).await.unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn test_voting_and_consensus() {
        let pool = setup_test_db().await;
        let project_id = "proj_123";
        let hash_a = "hash_aaaa";
        let hash_b = "hash_bbbb";

        set_quorum_threshold(&pool, 2).await.unwrap();

        // Oracle 1 votes for hash A
        let accepted = record_vote(&pool, project_id, "oracle_1", hash_a)
            .await
            .unwrap();
        assert!(accepted);

        let status = get_quorum_status(&pool, project_id).await.unwrap();
        assert_eq!(status.threshold, 2);
        assert_eq!(status.votes.len(), 1);
        assert_eq!(status.votes[0].count, 1);
        assert!(!status.consensus_reached);

        // Oracle 1 votes again (duplicate)
        let accepted = record_vote(&pool, project_id, "oracle_1", hash_a)
            .await
            .unwrap();
        assert!(!accepted);

        // Oracle 2 votes for hash B (different hash)
        record_vote(&pool, project_id, "oracle_2", hash_b)
            .await
            .unwrap();
        let status = get_quorum_status(&pool, project_id).await.unwrap();
        assert_eq!(status.votes.len(), 2);
        assert!(!status.consensus_reached);

        // Oracle 3 votes for hash A -> Consensus reached
        record_vote(&pool, project_id, "oracle_3", hash_a)
            .await
            .unwrap();
        let status = get_quorum_status(&pool, project_id).await.unwrap();
        assert!(status.consensus_reached);
        assert_eq!(
            status
                .votes
                .iter()
                .find(|v| v.proof_hash == hash_a)
                .unwrap()
                .count,
            2
        );
    }

    #[tokio::test]
    async fn test_global_statistics_aggregation() {
        let pool = setup_test_db().await;

        // 1. Initial state
        let stats = get_global_stats(&pool).await.unwrap();
        assert_eq!(stats.total_projects, 0);
        assert_eq!(stats.total_tvl, "0");

        // 2. Project Created
        let ev_created = PifpEvent {
            event_type: "project_created".to_string(),
            project_id: Some("1".to_string()),
            actor: Some("creator_1".to_string()),
            amount: Some("1000".to_string()),
            ledger: 100,
            timestamp: 1000,
            contract_id: "contract_1".to_string(),
            tx_hash: Some("tx_1".to_string()),
            extra_data: None,
        };
        update_global_stats(&pool, &ev_created).await.unwrap();
        let stats = get_global_stats(&pool).await.unwrap();
        assert_eq!(stats.total_projects, 1);

        // 3. Project Funded
        let ev_funded = PifpEvent {
            event_type: "project_funded".to_string(),
            project_id: Some("1".to_string()),
            actor: Some("donor_1".to_string()),
            amount: Some("500".to_string()),
            ledger: 101,
            timestamp: 1010,
            contract_id: "contract_1".to_string(),
            tx_hash: Some("tx_2".to_string()),
            extra_data: None,
        };
        update_global_stats(&pool, &ev_funded).await.unwrap();
        let stats = get_global_stats(&pool).await.unwrap();
        assert_eq!(stats.total_tvl, "500");
        assert_eq!(stats.total_donors, 1);

        // 4. Project Funded by same donor (should not increment donor count)
        let ev_funded_2 = PifpEvent {
            event_type: "project_funded".to_string(),
            project_id: Some("1".to_string()),
            actor: Some("donor_1".to_string()),
            amount: Some("300".to_string()),
            ledger: 102,
            timestamp: 1020,
            contract_id: "contract_1".to_string(),
            tx_hash: Some("tx_3".to_string()),
            extra_data: None,
        };
        update_global_stats(&pool, &ev_funded_2).await.unwrap();
        let stats = get_global_stats(&pool).await.unwrap();
        assert_eq!(stats.total_tvl, "800");
        assert_eq!(stats.total_donors, 1);

        // 5. Project Funded by new donor
        let ev_funded_3 = PifpEvent {
            event_type: "project_funded".to_string(),
            project_id: Some("1".to_string()),
            actor: Some("donor_2".to_string()),
            amount: Some("200".to_string()),
            ledger: 103,
            timestamp: 1030,
            contract_id: "contract_1".to_string(),
            tx_hash: Some("tx_4".to_string()),
            extra_data: None,
        };
        update_global_stats(&pool, &ev_funded_3).await.unwrap();
        let stats = get_global_stats(&pool).await.unwrap();
        assert_eq!(stats.total_tvl, "1000");
        assert_eq!(stats.total_donors, 2);

        // 6. Project Verified
        let ev_verified = PifpEvent {
            event_type: "project_verified".to_string(),
            project_id: Some("1".to_string()),
            actor: Some("oracle_1".to_string()),
            amount: None,
            ledger: 104,
            timestamp: 1040,
            contract_id: "contract_1".to_string(),
            tx_hash: Some("tx_5".to_string()),
            extra_data: None,
        };
        update_global_stats(&pool, &ev_verified).await.unwrap();
        let stats = get_global_stats(&pool).await.unwrap();
        assert_eq!(stats.completed_projects, 1);
    }

    // ── FTS search tests ──────────────────────────────────────────────

    async fn insert_project(pool: &SqlitePool, id: &str, title: &str, description: &str) {
        sqlx::query(
            "INSERT INTO projects (project_id, creator, goal, primary_token, created_ledger, title, description)
             VALUES (?1, 'creator', '1000', 'token', 1, ?2, ?3)",
        )
        .bind(id)
        .bind(title)
        .bind(description)
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_search_returns_matching_project() {
        let pool = setup_test_db().await;
        insert_project(&pool, "1", "Solar Power Initiative", "Bringing solar energy to rural areas").await;
        insert_project(&pool, "2", "Wind Turbine Project", "Harnessing wind energy for clean power").await;

        let results = search_projects(&pool, "Solar", 20, 0).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].project_id, "1");
    }

    #[tokio::test]
    async fn test_search_empty_query_returns_empty() {
        let pool = setup_test_db().await;
        insert_project(&pool, "1", "Solar Power Initiative", "Solar energy project").await;

        let results = search_projects(&pool, "", 20, 0).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_no_match_returns_empty() {
        let pool = setup_test_db().await;
        insert_project(&pool, "1", "Solar Power Initiative", "Solar energy project").await;
        insert_project(&pool, "2", "Wind Turbine Project", "Wind energy project").await;

        let results = search_projects(&pool, "blockchain", 20, 0).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_title_ranked_above_description() {
        let pool = setup_test_db().await;
        // "solar" only in description
        insert_project(&pool, "1", "Wind Project", "Uses solar panels as backup").await;
        // "solar" in title — should rank higher
        insert_project(&pool, "2", "Solar Power Initiative", "Clean energy for all").await;

        let results = search_projects(&pool, "solar", 20, 0).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].project_id, "2"); // title match ranks first
    }
}
