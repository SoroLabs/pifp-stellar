-- Migration: 003_create_projects_table
-- Purpose: Store the current state of each project for fast REST API queries.

CREATE TABLE IF NOT EXISTS projects (
    project_id    TEXT    PRIMARY KEY,
    creator       TEXT    NOT NULL,
    status        TEXT    NOT NULL DEFAULT 'Funding',
    goal          TEXT    NOT NULL,
    primary_token TEXT    NOT NULL,
    created_ledger INTEGER NOT NULL,
    created_at    INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_projects_status  ON projects (status);
CREATE INDEX IF NOT EXISTS idx_projects_creator ON projects (creator);
