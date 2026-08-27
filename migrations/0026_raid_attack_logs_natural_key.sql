-- raw_payload was the full raw socket payload per attack -- everything
-- needed from it is already extracted into structured columns, and it was
-- the single largest contributor to this table's per-row size.
ALTER TABLE raid_attack_logs DROP COLUMN raw_payload;

-- Consolidate raid_attack_logs' identity onto its natural key
-- (raid_id, player_id, attack_datetime) -- already enforced today as a
-- separate UNIQUE constraint alongside the generated `id` surrogate, so this
-- drops one of the two redundant indexes rather than adding one. Matches the
-- in-game-id-only approach already used for the players table.
ALTER TABLE raid_attack_components DROP CONSTRAINT raid_attack_components_attack_log_id_fkey;
ALTER TABLE raid_attack_components DROP CONSTRAINT raid_attack_components_pkey;

ALTER TABLE raid_attack_components ADD COLUMN raid_id BIGINT;
ALTER TABLE raid_attack_components ADD COLUMN player_id TEXT;
ALTER TABLE raid_attack_components ADD COLUMN attack_datetime TIMESTAMPTZ;

UPDATE raid_attack_components c
SET raid_id = l.raid_id, player_id = l.player_id, attack_datetime = l.attack_datetime
FROM raid_attack_logs l
WHERE l.id = c.attack_log_id;

ALTER TABLE raid_attack_components ALTER COLUMN raid_id SET NOT NULL;
ALTER TABLE raid_attack_components ALTER COLUMN player_id SET NOT NULL;
ALTER TABLE raid_attack_components ALTER COLUMN attack_datetime SET NOT NULL;
ALTER TABLE raid_attack_components DROP COLUMN attack_log_id;

ALTER TABLE raid_attack_logs DROP CONSTRAINT raid_attack_logs_raid_id_player_id_attack_datetime_key;
ALTER TABLE raid_attack_logs DROP CONSTRAINT raid_attack_logs_pkey;
ALTER TABLE raid_attack_logs DROP COLUMN id;
ALTER TABLE raid_attack_logs ADD PRIMARY KEY (raid_id, player_id, attack_datetime);

ALTER TABLE raid_attack_components ADD PRIMARY KEY (raid_id, player_id, attack_datetime, position);
ALTER TABLE raid_attack_components
    ADD CONSTRAINT raid_attack_components_attack_log_fkey
    FOREIGN KEY (raid_id, player_id, attack_datetime)
    REFERENCES raid_attack_logs (raid_id, player_id, attack_datetime)
    ON DELETE CASCADE;
