-- `stats` was the original raw PlayerRaidData JSONB blob, superseded when
-- player_stats was normalized into real columns (and player_cards) by an
-- earlier migration. It has been nullable and unwritten ever since -- the
-- only remaining reader (simulation_debug.rs) was already broken for any
-- player synced after that point, and is being fixed to use the normalized
-- columns via player_stats_repo::load instead of this column.
ALTER TABLE player_stats DROP COLUMN stats;
