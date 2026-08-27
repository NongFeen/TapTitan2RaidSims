-- Phase A: add TEXT player_id columns to every table that FKs to players(id),
-- backfilled via join while players.id still exists.
ALTER TABLE simulation_jobs ADD COLUMN player_id_new TEXT;
UPDATE simulation_jobs sj SET player_id_new = p.player_id FROM players p WHERE p.id = sj.player_id;
ALTER TABLE simulation_jobs ALTER COLUMN player_id_new SET NOT NULL;

ALTER TABLE player_stats ADD COLUMN player_id_new TEXT;
UPDATE player_stats ps SET player_id_new = p.player_id FROM players p WHERE p.id = ps.player_id;
ALTER TABLE player_stats ALTER COLUMN player_id_new SET NOT NULL;

ALTER TABLE player_cards ADD COLUMN player_id_new TEXT;
UPDATE player_cards pc SET player_id_new = p.player_id FROM players p WHERE p.id = pc.player_id;
ALTER TABLE player_cards ALTER COLUMN player_id_new SET NOT NULL;

ALTER TABLE raid_attack_logs ADD COLUMN player_id_new TEXT;
UPDATE raid_attack_logs ral SET player_id_new = p.player_id FROM players p WHERE p.id = ral.player_id;
-- stays nullable: original raid_attack_logs.player_id is nullable (ON DELETE SET NULL)

-- Phase B: drop the old FK constraints (and the PKs that live on the same
-- UUID columns about to be dropped).
ALTER TABLE simulation_jobs DROP CONSTRAINT simulation_jobs_player_id_fkey;

ALTER TABLE player_stats DROP CONSTRAINT player_stats_player_id_fkey;
ALTER TABLE player_stats DROP CONSTRAINT player_stats_pkey;

ALTER TABLE player_cards DROP CONSTRAINT player_cards_player_id_fkey;
ALTER TABLE player_cards DROP CONSTRAINT player_cards_pkey;

ALTER TABLE raid_attack_logs DROP CONSTRAINT raid_attack_logs_player_id_fkey;

-- Phase C: drop the old UUID columns, rename the TEXT columns into place.
ALTER TABLE simulation_jobs DROP COLUMN player_id;
ALTER TABLE simulation_jobs RENAME COLUMN player_id_new TO player_id;

ALTER TABLE player_stats DROP COLUMN player_id;
ALTER TABLE player_stats RENAME COLUMN player_id_new TO player_id;

ALTER TABLE player_cards DROP COLUMN player_id;
ALTER TABLE player_cards RENAME COLUMN player_id_new TO player_id;

ALTER TABLE raid_attack_logs DROP COLUMN player_id;
ALTER TABLE raid_attack_logs RENAME COLUMN player_id_new TO player_id;

-- Phase D: flip players' own primary key from id to player_id.
ALTER TABLE players DROP CONSTRAINT players_pkey;
ALTER TABLE players DROP CONSTRAINT players_external_id_key;
ALTER TABLE players DROP COLUMN id;
ALTER TABLE players ADD CONSTRAINT players_pkey PRIMARY KEY (player_id);

-- Phase E: re-add FKs/PKs against players(player_id), recreate the lost index.
ALTER TABLE simulation_jobs
    ADD CONSTRAINT simulation_jobs_player_id_fkey FOREIGN KEY (player_id) REFERENCES players(player_id) ON DELETE CASCADE;
CREATE INDEX simulation_jobs_player_idx ON simulation_jobs (player_id, created_at DESC);

ALTER TABLE player_stats
    ADD CONSTRAINT player_stats_pkey PRIMARY KEY (player_id),
    ADD CONSTRAINT player_stats_player_id_fkey FOREIGN KEY (player_id) REFERENCES players(player_id) ON DELETE CASCADE;

ALTER TABLE player_cards
    ADD CONSTRAINT player_cards_pkey PRIMARY KEY (player_id, card_id),
    ADD CONSTRAINT player_cards_player_id_fkey FOREIGN KEY (player_id) REFERENCES players(player_id) ON DELETE CASCADE;

ALTER TABLE raid_attack_logs
    ADD CONSTRAINT raid_attack_logs_player_id_fkey FOREIGN KEY (player_id) REFERENCES players(player_id) ON DELETE SET NULL;
