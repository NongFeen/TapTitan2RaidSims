UPDATE players SET player_id = id::TEXT WHERE player_id IS NULL;
ALTER TABLE players ALTER COLUMN player_id SET NOT NULL;
