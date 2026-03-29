# PIFP Backend Indexer

The backend indexer is a long-running Rust daemon that monitors the Soroban network for Proof-of-Impact Funding Protocol (PIFP) events. It decodes these on-chain events, stores them in a local SQLite database, and exposes them via a REST API. Additionally, it provides an admin Quorum mechanism for oracle voting.

## 🚀 Quickstart

Ensure you have Rust and Cargo installed, along with SQLite.

1. **Navigate to the indexer directory:**
   ```bash
   cd backend/indexer
   ```

2. **Configure your environment:**
   Create a `.env` file in this directory with at minimum your deployed contract ID:
   ```env
   CONTRACT_ID=<YOUR_STRKEY_CONTRACT_ADDRESS>
   # or multi-deployment:
   # CONTRACT_IDS=<ADDR_1>,<ADDR_2>,<ADDR_3>
   ```

3. **Run the indexer & API server:**
   ```bash
   cargo run
   ```

The application will automatically perform database migrations (creating `pifp_events.db` by default) and start streaming events. The REST API will be available at `http://0.0.0.0:3001` (by default).

## ⚙️ Configuration

The indexer reads configuration from environment variables. You can supply these via a `.env` file or export them directly in your shell.

| Variable | Description | Default Value | Required |
|----------|-------------|---------------|----------|
| `CONTRACT_ID` | Single PIFP contract address (legacy compatibility if `CONTRACT_IDS` is unset). | *None* | Conditionally |
| `CONTRACT_IDS` | Comma-separated PIFP contract addresses for multi-deployment indexing. | *None* | Conditionally |
| `RPC_URL` | Soroban/Horizon RPC endpoint. | `https://soroban-testnet.stellar.org` | No |
| `DATABASE_URL` | Path or connection string for the SQLite database. | `sqlite:./pifp_events.db` | No |
| `API_PORT` | Port for the Axum REST API server. | `3001` | No |
| `POLL_INTERVAL_SECS` | How often (in seconds) to poll the RPC for events. | `5` | No |
| `EVENTS_PER_PAGE` | Maximum number of events to fetch per RPC pagination slice. | `100` | No |
| `START_LEDGER` | The ledger to start syncing from if no cursor is saved in the DB yet. | `0` | No |
| `BACKFILL_START_LEDGER` | Optional explicit ledger sequence to begin backfill from (overrides persisted cursor start). | *None* | No |
| `BACKFILL_CURSOR` | Optional explicit RPC cursor for deterministic resume/backfill from a known paging token. | *None* | No |
| `REDIS_URL` | Optional Redis URL for API response caching. If unset, caching is disabled. | *None* | No |
| `CACHE_TTL_TOP_PROJECTS_SECS` | TTL for cached `GET /projects/top` responses. | `30` | No |
| `CACHE_TTL_ACTIVE_PROJECTS_COUNT_SECS` | TTL for cached `GET /projects/active/count` response. | `15` | No |

## 🏗️ Architecture & Database

This project leverages the following primary stack:
- **Web Framework:** `axum` + `tokio`
- **Database ORM/Querying:** `sqlx` (SQLite)
- **HTTP Client:** `reqwest`

### Database Schema / Migrations

When you run the indexer, `sqlx` migrations automatically run to ensure the schema is up to date. The schema consists of:

- **`events`**: Stores all decoded `PifpEvent` instances. 
  - Tracks `event_type` (e.g., `project_created`, `project_funded`), `project_id`, `actor`, `amount`, `ledger`, `tx_hash`, and standard metadata.
- **`indexer_cursor`**: A crucial, single-row table storing the last processed ledger cursor. This allows the indexer to gracefully resume exactly where it left off following a crash or restart.
- **`quorum_settings`**: Global settings table for defining oracle voting threshold requirements.
- **`oracle_votes`**: Stores proof-hash votes cast by specific oracles for specific projects.

### Event Listening Logic

The background indexing logic lives in `src/indexer.rs` and `src/rpc.rs`.
1. The indexer loads the last known cursor from the `indexer_cursor` table. If the database is empty, it falls back to `START_LEDGER`.
2. A continuous, infinite loop requests `getEvents` from the configured Soroban RPC using contract + topic filters (`created`, `funded`, `active`, `verified`, `expired`, `cancelled`, `released`, `refunded`, and admin topics).
3. The raw payloads are passed to `decode_events()`, which extracts the base64 XDR payload and maps the primary operational topics (e.g., `created`, `funded`, `verified`) into the structured `EventKind` enum (`src/events.rs`).
4. Extracted events are batch-inserted into the `events` table.
5. `indexer_cursor` is updated to reflect the successful batch persistence.
6. The process sleeps for `POLL_INTERVAL_SECS` and repeats.

## 📡 REST API Overview

In parallel to the background indexing daemon, an Axum web server runs to expose this data. 

**Public Query Endpoints:**
- `GET /health` : Returns server operational status and application version.
- `GET /events` : List all indexed events across the entire protocol.
- `GET /projects/:id/events` : Query historical events specifically generated for `project_id`.
- `GET /projects/top?limit=10` : Top funded projects ranked by indexed `project_funded` events (cached when Redis is configured).
- `GET /projects/active/count` : Current active projects count inferred from latest status events (`project_active`, `project_verified`, `project_expired`, `project_cancelled`) (cached when Redis is configured).

**Quorum / Oracle Endpoints:**
- `POST /admin/quorum` : Update the global quorum threshold (expects a `{ threshold: u32 }` JSON payload).
- `POST /projects/:id/vote` : Submit an oracle vote (expects a `{ oracle: string, proof_hash: string }` JSON payload).
- `GET /projects/:id/quorum` : Returns the active vote count vs existing threshold for the given project.

**Webhook Endpoints:**
- `POST /webhooks` : Register webhook URL + event subscriptions + secret (`{ "url": "...", "secret": "...", "event_types": ["project_active"] }`).
- `GET /webhooks` : List registered webhook subscriptions.

## 🧠 Caching Behavior

When `REDIS_URL` is provided, the API stores frequent response payloads in Redis:
- `GET /projects/top` uses a versioned key based on `limit` + cache version.
- `GET /projects/active/count` uses a versioned key based on cache version.

On each successful indexing batch that inserts new events, the indexer bumps a global cache version key in Redis. This invalidates previous cached entries without key scans.

If Redis is unavailable (startup or runtime), requests automatically fall back to SQLite queries and continue serving responses.

## 🔔 Webhook Dispatch

The indexer can dispatch signed webhook notifications whenever newly indexed events match subscriptions:
- Registry tables: `webhooks`, `webhook_subscriptions`, `webhook_deliveries`
- Signature header: `X-PIFP-Signature` with format `sha256=<hex_hmac>`
- HMAC algorithm: SHA-256 over raw JSON payload using the webhook secret
- Retry policy: exponential backoff (`500ms`, `1s`, `2s`, `4s`) for transient failures

## 🔧 Troubleshooting

- **`Indexer poll error: ...` loop:** 
  You most likely have invalid `CONTRACT_ID`/`CONTRACT_IDS` values or an inaccessible `RPC_URL`. Validate your variables.
- **Missing older events:**
  If the indexer was started midway through the ledger lifecycle while the database cursor `last_ledger` was `0`, your `START_LEDGER` was likely too high or left as `0` meaning the `getEvents` RPC limit natively truncated older history. Update `START_LEDGER` to the exact deployment ledger, or use `BACKFILL_START_LEDGER`/`BACKFILL_CURSOR` for a deterministic replay point.
- **Database Locked Errors:**
  Normally handled gracefully by `sqlx`, but if another external tool (e.g., a DB browser) locks `pifp_events.db`, the indexer may crash or log warnings. Ensure SQLite WAL mode is unhindered or avoid long-running manual write queries.
