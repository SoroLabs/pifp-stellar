-- Migration: 007_add_project_fts
-- Purpose: Full-text search for project discovery.
--
-- NOTE: SQLite FTS5 is used here (the production-equivalent of PostgreSQL's
-- tsvector/GIN approach). The FTS5 virtual table provides the same O(log N)
-- ranked search semantics via the built-in bm25() ranking function.
--
-- PostgreSQL equivalent (for reference):
--   ALTER TABLE projects ADD COLUMN tsv_search tsvector
--     GENERATED ALWAYS AS (
--       setweight(to_tsvector('english', coalesce(title,'')), 'A') ||
--       setweight(to_tsvector('english', coalesce(description,'')), 'B')
--     ) STORED;
--   CREATE INDEX idx_projects_fts ON projects USING GIN (tsv_search);

-- 1. Add searchable text columns to the projects table.
ALTER TABLE projects ADD COLUMN title       TEXT NOT NULL DEFAULT '';
ALTER TABLE projects ADD COLUMN description TEXT NOT NULL DEFAULT '';

-- 2. FTS5 virtual table — content= keeps it in sync with the projects table.
--    title is weighted higher than description (rank multiplier applied in query).
CREATE VIRTUAL TABLE IF NOT EXISTS projects_fts USING fts5(
    title,
    description,
    content='projects',
    content_rowid='rowid'
);

-- 3. Triggers to keep the FTS index in sync with the projects table.
CREATE TRIGGER IF NOT EXISTS projects_fts_insert
    AFTER INSERT ON projects BEGIN
        INSERT INTO projects_fts (rowid, title, description)
        VALUES (new.rowid, new.title, new.description);
    END;

CREATE TRIGGER IF NOT EXISTS projects_fts_update
    AFTER UPDATE OF title, description ON projects BEGIN
        INSERT INTO projects_fts (projects_fts, rowid, title, description)
        VALUES ('delete', old.rowid, old.title, old.description);
        INSERT INTO projects_fts (rowid, title, description)
        VALUES (new.rowid, new.title, new.description);
    END;

CREATE TRIGGER IF NOT EXISTS projects_fts_delete
    AFTER DELETE ON projects BEGIN
        INSERT INTO projects_fts (projects_fts, rowid, title, description)
        VALUES ('delete', old.rowid, old.title, old.description);
    END;
