-- Decade Anniversary equipment set (+100 Raid Base Damage). Existing players
-- default to not having it; their next stats refresh fills in the real value.
ALTER TABLE player_stats
    ADD COLUMN IF NOT EXISTS raid_set_decade_anniversary BOOLEAN NOT NULL DEFAULT FALSE;
