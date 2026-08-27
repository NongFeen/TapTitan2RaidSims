-- player_code duplicated player_id in every row that ever existed (0 NULLs
-- across all rows to date). Going forward, handle_attack simply skips
-- logging an attack (and its component breakdown) entirely when the
-- attacking player isn't in `players` yet, instead of falling back to a raw
-- text code -- so player_id can be made NOT NULL and player_code dropped.
ALTER TABLE raid_attack_logs DROP CONSTRAINT raid_attack_logs_raid_id_player_code_attack_datetime_key;
DROP INDEX raid_attack_logs_player_time_idx;
ALTER TABLE raid_attack_logs DROP CONSTRAINT raid_attack_logs_player_id_fkey;

ALTER TABLE raid_attack_logs DROP COLUMN player_code;
ALTER TABLE raid_attack_logs ALTER COLUMN player_id SET NOT NULL;

ALTER TABLE raid_attack_logs
    ADD CONSTRAINT raid_attack_logs_player_id_fkey FOREIGN KEY (player_id) REFERENCES players(player_id) ON DELETE CASCADE;
ALTER TABLE raid_attack_logs
    ADD CONSTRAINT raid_attack_logs_raid_id_player_id_attack_datetime_key UNIQUE (raid_id, player_id, attack_datetime);
CREATE INDEX raid_attack_logs_player_time_idx ON raid_attack_logs (player_id, attack_datetime DESC);
