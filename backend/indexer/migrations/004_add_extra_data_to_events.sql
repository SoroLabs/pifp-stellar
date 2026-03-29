-- Migration: 004_add_extra_data_to_events
-- Purpose: Store additional event fields (like token address, proof hash) in a flexible text column.

ALTER TABLE events ADD COLUMN extra_data TEXT;
