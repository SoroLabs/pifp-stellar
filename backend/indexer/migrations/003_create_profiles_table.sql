-- Migration: Create user profiles table for off-chain identity metadata

CREATE TABLE IF NOT EXISTS profiles (
    address     TEXT PRIMARY KEY,
    nickname    TEXT,
    bio         TEXT,
    avatar_url  TEXT,
    updated_at  INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);
