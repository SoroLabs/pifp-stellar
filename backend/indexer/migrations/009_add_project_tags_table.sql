-- Migration: 006_add_project_tags_table
-- Purpose: Support many-to-many project tagging for search and discovery.

-- Up
CREATE TABLE IF NOT EXISTS project_tags (
    id         INTEGER  PRIMARY KEY AUTOINCREMENT,
    project_id TEXT     NOT NULL REFERENCES projects (project_id) ON DELETE CASCADE,
    tag_name   TEXT     NOT NULL,
    created_at INTEGER  NOT NULL DEFAULT (strftime('%s', 'now')),
    UNIQUE (project_id, tag_name)
);

CREATE INDEX IF NOT EXISTS idx_project_tags_tag_name   ON project_tags (tag_name);
CREATE INDEX IF NOT EXISTS idx_project_tags_project_id ON project_tags (project_id);

-- Down
-- DROP INDEX IF EXISTS idx_project_tags_project_id;
-- DROP INDEX IF EXISTS idx_project_tags_tag_name;
-- DROP TABLE IF EXISTS project_tags;
