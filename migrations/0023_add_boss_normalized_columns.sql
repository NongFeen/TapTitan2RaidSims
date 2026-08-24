-- Phase 3: normalize current_boss.boss_data / attackable_parts JSONB into
-- real columns + child tables. Same pattern as 0022 for player_stats: add
-- nullable columns, backfill, lock down NOT NULL/CHECK, then relax the old
-- JSONB columns to nullable (stale legacy copy, stops being written).

ALTER TABLE current_boss
    ADD COLUMN boss_name boss_name,
    ADD COLUMN global_raid_modifier global_raid_modifier,
    ADD COLUMN global_raid_modifier_amount DOUBLE PRECISION,
    ADD COLUMN curse_type curse_type,
    ADD COLUMN curse_damage_per_curse DOUBLE PRECISION,
    ADD COLUMN recommend_1_to_2_part_patterns_only BOOLEAN,
    ADD COLUMN damage_results JSONB;

UPDATE current_boss SET
    boss_name = (boss_data->>'boss_name')::boss_name,
    global_raid_modifier = (boss_data->>'global_raid_modifier')::global_raid_modifier,
    global_raid_modifier_amount = (boss_data->>'global_raid_modifier_amount')::double precision,
    curse_type = (boss_data->>'curse_type')::curse_type,
    curse_damage_per_curse = (boss_data->>'curse_damage_per_curse')::double precision,
    recommend_1_to_2_part_patterns_only = (boss_data->>'recommend_1_to_2_part_patterns_only')::boolean,
    damage_results = COALESCE(boss_data->'damage_results', '[]'::jsonb);

ALTER TABLE current_boss
    ALTER COLUMN boss_name SET NOT NULL,
    ALTER COLUMN global_raid_modifier SET NOT NULL,
    ALTER COLUMN global_raid_modifier SET DEFAULT 'None',
    ALTER COLUMN curse_type SET NOT NULL,
    ALTER COLUMN curse_type SET DEFAULT 'None',
    ALTER COLUMN curse_damage_per_curse SET NOT NULL,
    ALTER COLUMN curse_damage_per_curse SET DEFAULT 0.06,
    ALTER COLUMN recommend_1_to_2_part_patterns_only SET NOT NULL,
    ALTER COLUMN recommend_1_to_2_part_patterns_only SET DEFAULT FALSE,
    ALTER COLUMN damage_results SET NOT NULL,
    ALTER COLUMN damage_results SET DEFAULT '[]';

-- boss_data/attackable_parts become a stale legacy copy from here on --
-- dropped entirely in a later contract-phase migration.
ALTER TABLE current_boss
    ALTER COLUMN boss_data DROP NOT NULL,
    ALTER COLUMN attackable_parts DROP NOT NULL;

CREATE TABLE current_boss_parts (
    part_name boss_part_name PRIMARY KEY,
    part_state part_state NOT NULL,
    max_armor BIGINT NOT NULL CHECK (max_armor >= 0),
    max_health BIGINT NOT NULL CHECK (max_health >= 0),
    current_armor BIGINT NOT NULL CHECK (current_armor >= 0),
    current_health BIGINT NOT NULL CHECK (current_health >= 0),
    radioactivity_afflicted_seconds DOUBLE PRECISION NOT NULL DEFAULT 0
);

INSERT INTO current_boss_parts (part_name, part_state, max_armor, max_health, current_armor, current_health, radioactivity_afflicted_seconds)
SELECT
    (part->>'part_name')::boss_part_name,
    (part->>'part_state')::part_state,
    (part->>'max_armor')::bigint,
    (part->>'max_health')::bigint,
    (part->>'current_armor')::bigint,
    (part->>'current_health')::bigint,
    COALESCE((part->>'radioactivity_afflicted_seconds')::double precision, 0)
FROM current_boss cb,
     LATERAL (VALUES
        (cb.boss_data->'head'), (cb.boss_data->'torso'),
        (cb.boss_data->'left_shoulder'), (cb.boss_data->'right_shoulder'),
        (cb.boss_data->'left_hand'), (cb.boss_data->'right_hand'),
        (cb.boss_data->'left_leg'), (cb.boss_data->'right_leg')
     ) AS parts(part);

CREATE TABLE current_boss_attackable_parts (
    part_name boss_part_name PRIMARY KEY
);

INSERT INTO current_boss_attackable_parts (part_name)
SELECT (value)::boss_part_name
FROM current_boss, jsonb_array_elements_text(attackable_parts);
