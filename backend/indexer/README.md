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
| `CONTRACT_ID` | The Strkey address of the PIFP smart contract. | *None* | **Yes** |
| `RPC_URL` | Soroban/Horizon RPC endpoint. | `https://soroban-testnet.stellar.org` | No |
| `DATABASE_URL` | Path or connection string for the SQLite database. | `sqlite:./pifp_events.db` | No |
| `API_PORT` | Port for the Axum REST API server. | `3001` | No |
| `POLL_INTERVAL_SECS` | How often (in seconds) to poll the RPC for events. | `5` | No |
| `EVENTS_PER_PAGE` | Maximum number of events to fetch per RPC pagination slice. | `100` | No |
| `START_LEDGER` | The ledger to start syncing from if no cursor is saved in the DB yet. | `0` | No |

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
2. A continuous, infinite loop requests `getEvents` from the configured Soroban RPC, filtering aggressively on the `CONTRACT_ID`.
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

**Quorum / Oracle Endpoints:**
- `POST /admin/quorum` : Update the global quorum threshold (expects a `{ threshold: u32 }` JSON payload).
- `POST /projects/:id/vote` : Submit an oracle vote (expects a `{ oracle: string, proof_hash: string }` JSON payload).
- `GET /projects/:id/quorum` : Returns the active vote count vs existing threshold for the given project.

## 🔧 Troubleshooting

- **`Indexer poll error: ...` loop:** 
  You most likely have an invalid `CONTRACT_ID` or an inaccessible `RPC_URL`. Validate your variables.
- **Missing older events:**
  If the indexer was started midway through the ledger lifecycle while the database cursor `last_ledger` was `0`, your `START_LEDGER` was likely too high or left as `0` meaning the `getEvents` RPC limit natively truncated older history. Update `START_LEDGER` to the exact ledger the contract was deployed on and wipe your database (`rm ./pifp_events.db`) to force a fresh sequential scan.
- **Database Locked Errors:**
  Normally handled gracefully by `sqlx`, but if another external tool (e.g., a DB browser) locks `pifp_events.db`, the indexer may crash or log warnings. Ensure SQLite WAL mode is unhindered or avoid long-running manual write queries.
