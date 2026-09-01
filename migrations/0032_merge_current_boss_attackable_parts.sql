-- current_boss_attackable_parts exists only to flag which of
-- current_boss_parts's 8 rows (one per body part) are currently
-- attackable/targetable -- it shares the exact same part_name domain as
-- current_boss_parts, so it's a column on that table, not a separate one.
ALTER TABLE current_boss_parts ADD COLUMN is_attackable BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE current_boss_parts
SET is_attackable = TRUE
WHERE part_name IN (SELECT part_name FROM current_boss_attackable_parts);

DROP TABLE current_boss_attackable_parts;

-- boss_data/attackable_parts were the original raw JSONB columns, superseded
-- when current_boss was normalized into current_boss (its own real columns)
-- + current_boss_parts. Confirmed unread and unwritten by any query in the
-- codebase, same as player_stats.stats before it was dropped.
ALTER TABLE current_boss DROP COLUMN boss_data;
ALTER TABLE current_boss DROP COLUMN attackable_parts;
