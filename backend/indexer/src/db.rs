//! Database layer — migrations, queries, and cursor management.

use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tracing::info;

use crate::errors::Result;
use crate::events::{EventRecord, PifpEvent};

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
pub async fn insert_events(pool: &SqlitePool, events: &[PifpEvent]) -> Result<usize> {
    let mut count = 0usize;
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

        // Update Project Registry for life-cycle tracking
        if let Some(id) = &ev.project_id {
            match ev.event_type.as_str() {
                "project_created" => {
                    sqlx::query(
                        r#"
                        INSERT OR IGNORE INTO projects (project_id, creator, goal, primary_token, created_ledger)
                        VALUES (?1, ?2, ?3, ?4, ?5)
                        "#
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
                    sqlx::query("UPDATE projects SET status = 'Completed' WHERE project_id = ?1")
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

        count += rows_affected as usize;
    }
    Ok(count)
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
            "CREATE TABLE quorum_settings (id INTEGER PRIMARY KEY CHECK (id = 1), threshold INTEGER NOT NULL DEFAULT 1);",
        ).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO quorum_settings (id, threshold) VALUES (1, 1);")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE oracle_votes (id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT NOT NULL, oracle_address TEXT NOT NULL, proof_hash TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP, UNIQUE(project_id, oracle_address));",
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
}
