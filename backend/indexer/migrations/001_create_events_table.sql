-- Migration: 001_create_events_table
-- Creates the events table and the cursor table for resume support

CREATE TABLE IF NOT EXISTS events (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type    TEXT    NOT NULL,
    project_id    TEXT,
    actor         TEXT,
    amount        TEXT,
    ledger        INTEGER NOT NULL,
    timestamp     INTEGER NOT NULL,
    contract_id   TEXT    NOT NULL,
    tx_hash       TEXT,
    created_at    INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_events_project_id ON events (project_id);
CREATE INDEX IF NOT EXISTS idx_events_event_type ON events (event_type);
CREATE INDEX IF NOT EXISTS idx_events_ledger      ON events (ledger);

-- Cursor table: stores the last seen ledger/cursor so the indexer
-- can resume from where it stopped instead of re-scanning from genesis.
CREATE TABLE IF NOT EXISTS indexer_cursor (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    last_ledger INTEGER NOT NULL DEFAULT 0,
    last_cursor TEXT
);

-- Seed with row id=1 so we can do a simple UPDATE later
INSERT OR IGNORE INTO indexer_cursor (id, last_ledger) VALUES (1, 0);
