-- Migration: 005_create_project_stats_table
-- Purpose: Store pre-calculated global statistics for the protocol dashboard.

-- Global aggregator table (single row)
CREATE TABLE IF NOT EXISTS project_stats (
    id                 INTEGER PRIMARY KEY CHECK (id = 1),
    total_projects     INTEGER NOT NULL DEFAULT 0,
    total_tvl          TEXT    NOT NULL DEFAULT '0',
    total_donors       INTEGER NOT NULL DEFAULT 0,
    completed_projects INTEGER NOT NULL DEFAULT 0,
    failed_projects    INTEGER NOT NULL DEFAULT 0,
    updated_at         INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Seed global stats row
INSERT OR IGNORE INTO project_stats (id, total_projects, total_tvl, total_donors, completed_projects, failed_projects)
VALUES (1, 0, '0', 0, 0, 0);

-- Supplemental table to track unique donors accurately across projects
CREATE TABLE IF NOT EXISTS unique_donors (
    address    TEXT PRIMARY KEY,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_unique_donors_created ON unique_donors (created_at);
