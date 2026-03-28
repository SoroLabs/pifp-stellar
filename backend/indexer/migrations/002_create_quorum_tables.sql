-- Migration: Create Quorum Tables

-- Table for global quorum settings
CREATE TABLE IF NOT EXISTS quorum_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1), -- Only one row for global settings
    threshold INTEGER NOT NULL DEFAULT 1
);

-- Insert default threshold
INSERT OR IGNORE INTO quorum_settings (id, threshold) VALUES (1, 1);

-- Table for tracking oracle votes for projects
CREATE TABLE IF NOT EXISTS oracle_votes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL,
    oracle_address TEXT NOT NULL,
    proof_hash TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(project_id, oracle_address) -- Ensure idempotence per oracle per project
);

-- Index for counting votes per project and proof hash
CREATE INDEX IF NOT EXISTS idx_oracle_votes_project_hash ON oracle_votes(project_id, proof_hash);
