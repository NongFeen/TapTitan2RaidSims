-- Phase 2: normalize player_stats.stats JSONB into real columns + a
-- player_cards child table. See plan for the EAV-vs-flat-columns decision --
-- flat columns were chosen since all 38 research fields (x2 tracks) are
-- always fully populated and never filtered by individual key at the SQL
-- level.
--
-- `stats` JSONB stays present (nullable) as a legacy/audit copy until a
-- later contract-phase migration drops it, once the app has been verified
-- to run correctly against the normalized columns.

-- research_kind/research_stat_key were created in 0020 for the EAV design
-- that was ultimately rejected in favor of flat columns -- nothing
-- references them yet, safe to drop.
DROP TYPE research_kind;
DROP TYPE research_stat_key;

ALTER TABLE player_stats
    ADD COLUMN player_raid_level INTEGER,
    ADD COLUMN player_raid_base_damage INTEGER,
    ADD COLUMN title REAL,

    ADD COLUMN raid_set_jade_anniversary BOOLEAN,
    ADD COLUMN raid_set_jukk_juggernaut BOOLEAN,
    ADD COLUMN raid_set_airforce_ace BOOLEAN,
    ADD COLUMN raid_set_dancer_venom BOOLEAN,
    ADD COLUMN raid_set_rose_anniversary BOOLEAN,

    ADD COLUMN titan_soul_head_mult REAL,
    ADD COLUMN titan_soul_torso_mult REAL,
    ADD COLUMN titan_soul_limbs_mult REAL,
    ADD COLUMN titan_soul_armor_mult REAL,
    ADD COLUMN titan_soul_body_mult REAL,
    ADD COLUMN titan_soul_lojak_mult REAL,
    ADD COLUMN titan_soul_takedar_mult REAL,
    ADD COLUMN titan_soul_jukk_mult REAL,
    ADD COLUMN titan_soul_sterl_mult REAL,
    ADD COLUMN titan_soul_mohaca_mult REAL,
    ADD COLUMN titan_soul_terro_mult REAL,
    ADD COLUMN titan_soul_klonk_mult REAL,
    ADD COLUMN titan_soul_priker_mult REAL,

    ADD COLUMN card_base_damage INTEGER,
    ADD COLUMN card_head_damage INTEGER,
    ADD COLUMN card_torso_damage INTEGER,
    ADD COLUMN card_limbs_damage INTEGER,
    ADD COLUMN card_armor_damage INTEGER,
    ADD COLUMN card_head_armor_damage INTEGER,
    ADD COLUMN card_torso_armor_damage INTEGER,
    ADD COLUMN card_limbs_armor_damage INTEGER,
    ADD COLUMN card_body_damage INTEGER,
    ADD COLUMN card_head_body_damage INTEGER,
    ADD COLUMN card_torso_body_damage INTEGER,
    ADD COLUMN card_limbs_body_damage INTEGER,
    ADD COLUMN card_lojak_damage INTEGER,
    ADD COLUMN card_takedar_damage INTEGER,
    ADD COLUMN card_jukk_damage INTEGER,
    ADD COLUMN card_sterl_damage INTEGER,
    ADD COLUMN card_mohaca_damage INTEGER,
    ADD COLUMN card_terro_damage INTEGER,
    ADD COLUMN card_klonk_damage INTEGER,
    ADD COLUMN card_priker_damage INTEGER,
    ADD COLUMN card_base_burst_damage INTEGER,
    ADD COLUMN card_burst_lojak_damage INTEGER,
    ADD COLUMN card_burst_takedar_damage INTEGER,
    ADD COLUMN card_burst_jukk_damage INTEGER,
    ADD COLUMN card_burst_sterl_damage INTEGER,
    ADD COLUMN card_burst_mohaca_damage INTEGER,
    ADD COLUMN card_burst_terro_damage INTEGER,
    ADD COLUMN card_burst_klonk_damage INTEGER,
    ADD COLUMN card_burst_priker_damage INTEGER,
    ADD COLUMN card_base_affliction_damage INTEGER,
    ADD COLUMN card_affliction_lojak_damage INTEGER,
    ADD COLUMN card_affliction_takedar_damage INTEGER,
    ADD COLUMN card_affliction_jukk_damage INTEGER,
    ADD COLUMN card_affliction_sterl_damage INTEGER,
    ADD COLUMN card_affliction_mohaca_damage INTEGER,
    ADD COLUMN card_affliction_terro_damage INTEGER,
    ADD COLUMN card_affliction_klonk_damage INTEGER,
    ADD COLUMN card_affliction_priker_damage INTEGER,

    ADD COLUMN gem_base_damage INTEGER,
    ADD COLUMN gem_head_damage INTEGER,
    ADD COLUMN gem_torso_damage INTEGER,
    ADD COLUMN gem_limbs_damage INTEGER,
    ADD COLUMN gem_armor_damage INTEGER,
    ADD COLUMN gem_head_armor_damage INTEGER,
    ADD COLUMN gem_torso_armor_damage INTEGER,
    ADD COLUMN gem_limbs_armor_damage INTEGER,
    ADD COLUMN gem_body_damage INTEGER,
    ADD COLUMN gem_head_body_damage INTEGER,
    ADD COLUMN gem_torso_body_damage INTEGER,
    ADD COLUMN gem_limbs_body_damage INTEGER,
    ADD COLUMN gem_lojak_damage INTEGER,
    ADD COLUMN gem_takedar_damage INTEGER,
    ADD COLUMN gem_jukk_damage INTEGER,
    ADD COLUMN gem_sterl_damage INTEGER,
    ADD COLUMN gem_mohaca_damage INTEGER,
    ADD COLUMN gem_terro_damage INTEGER,
    ADD COLUMN gem_klonk_damage INTEGER,
    ADD COLUMN gem_priker_damage INTEGER,
    ADD COLUMN gem_base_burst_damage INTEGER,
    ADD COLUMN gem_burst_lojak_damage INTEGER,
    ADD COLUMN gem_burst_takedar_damage INTEGER,
    ADD COLUMN gem_burst_jukk_damage INTEGER,
    ADD COLUMN gem_burst_sterl_damage INTEGER,
    ADD COLUMN gem_burst_mohaca_damage INTEGER,
    ADD COLUMN gem_burst_terro_damage INTEGER,
    ADD COLUMN gem_burst_klonk_damage INTEGER,
    ADD COLUMN gem_burst_priker_damage INTEGER,
    ADD COLUMN gem_base_affliction_damage INTEGER,
    ADD COLUMN gem_affliction_lojak_damage INTEGER,
    ADD COLUMN gem_affliction_takedar_damage INTEGER,
    ADD COLUMN gem_affliction_jukk_damage INTEGER,
    ADD COLUMN gem_affliction_sterl_damage INTEGER,
    ADD COLUMN gem_affliction_mohaca_damage INTEGER,
    ADD COLUMN gem_affliction_terro_damage INTEGER,
    ADD COLUMN gem_affliction_klonk_damage INTEGER,
    ADD COLUMN gem_affliction_priker_damage INTEGER;

UPDATE player_stats SET
    player_raid_level = (stats->>'player_raid_level')::INTEGER,
    player_raid_base_damage = (stats->>'player_raid_base_damage')::INTEGER,
    title = (stats->>'title')::REAL,

    raid_set_jade_anniversary = (stats->'raid_set'->>'jade_anniversary')::BOOLEAN,
    raid_set_jukk_juggernaut = (stats->'raid_set'->>'jukk_juggernaut')::BOOLEAN,
    raid_set_airforce_ace = (stats->'raid_set'->>'airforce_ace')::BOOLEAN,
    raid_set_dancer_venom = (stats->'raid_set'->>'dancer_venom')::BOOLEAN,
    raid_set_rose_anniversary = (stats->'raid_set'->>'rose_anniversary')::BOOLEAN,

    titan_soul_head_mult = (stats->'titan_soul_research'->>'head_mult')::REAL,
    titan_soul_torso_mult = (stats->'titan_soul_research'->>'torso_mult')::REAL,
    titan_soul_limbs_mult = (stats->'titan_soul_research'->>'limbs_mult')::REAL,
    titan_soul_armor_mult = (stats->'titan_soul_research'->>'armor_mult')::REAL,
    titan_soul_body_mult = (stats->'titan_soul_research'->>'body_mult')::REAL,
    titan_soul_lojak_mult = (stats->'titan_soul_research'->>'lojak_mult')::REAL,
    titan_soul_takedar_mult = (stats->'titan_soul_research'->>'takedar_mult')::REAL,
    titan_soul_jukk_mult = (stats->'titan_soul_research'->>'jukk_mult')::REAL,
    titan_soul_sterl_mult = (stats->'titan_soul_research'->>'sterl_mult')::REAL,
    titan_soul_mohaca_mult = (stats->'titan_soul_research'->>'mohaca_mult')::REAL,
    titan_soul_terro_mult = (stats->'titan_soul_research'->>'terro_mult')::REAL,
    titan_soul_klonk_mult = (stats->'titan_soul_research'->>'klonk_mult')::REAL,
    titan_soul_priker_mult = (stats->'titan_soul_research'->>'priker_mult')::REAL,

    card_base_damage = (stats->'raid_card_research'->>'base_damage')::INTEGER,
    card_head_damage = (stats->'raid_card_research'->>'head_damage')::INTEGER,
    card_torso_damage = (stats->'raid_card_research'->>'torso_damage')::INTEGER,
    card_limbs_damage = (stats->'raid_card_research'->>'limbs_damage')::INTEGER,
    card_armor_damage = (stats->'raid_card_research'->>'armor_damage')::INTEGER,
    card_head_armor_damage = (stats->'raid_card_research'->>'head_armor_damage')::INTEGER,
    card_torso_armor_damage = (stats->'raid_card_research'->>'torso_armor_damage')::INTEGER,
    card_limbs_armor_damage = (stats->'raid_card_research'->>'limbs_armor_damage')::INTEGER,
    card_body_damage = (stats->'raid_card_research'->>'body_damage')::INTEGER,
    card_head_body_damage = (stats->'raid_card_research'->>'head_body_damage')::INTEGER,
    card_torso_body_damage = (stats->'raid_card_research'->>'torso_body_damage')::INTEGER,
    card_limbs_body_damage = (stats->'raid_card_research'->>'limbs_body_damage')::INTEGER,
    card_lojak_damage = (stats->'raid_card_research'->>'lojak_damage')::INTEGER,
    card_takedar_damage = (stats->'raid_card_research'->>'takedar_damage')::INTEGER,
    card_jukk_damage = (stats->'raid_card_research'->>'jukk_damage')::INTEGER,
    card_sterl_damage = (stats->'raid_card_research'->>'sterl_damage')::INTEGER,
    card_mohaca_damage = (stats->'raid_card_research'->>'mohaca_damage')::INTEGER,
    card_terro_damage = (stats->'raid_card_research'->>'terro_damage')::INTEGER,
    card_klonk_damage = (stats->'raid_card_research'->>'klonk_damage')::INTEGER,
    card_priker_damage = (stats->'raid_card_research'->>'priker_damage')::INTEGER,
    card_base_burst_damage = (stats->'raid_card_research'->>'base_burst_damage')::INTEGER,
    card_burst_lojak_damage = (stats->'raid_card_research'->>'burst_lojak_damage')::INTEGER,
    card_burst_takedar_damage = (stats->'raid_card_research'->>'burst_takedar_damage')::INTEGER,
    card_burst_jukk_damage = (stats->'raid_card_research'->>'burst_jukk_damage')::INTEGER,
    card_burst_sterl_damage = (stats->'raid_card_research'->>'burst_sterl_damage')::INTEGER,
    card_burst_mohaca_damage = (stats->'raid_card_research'->>'burst_mohaca_damage')::INTEGER,
    card_burst_terro_damage = (stats->'raid_card_research'->>'burst_terro_damage')::INTEGER,
    card_burst_klonk_damage = (stats->'raid_card_research'->>'burst_klonk_damage')::INTEGER,
    card_burst_priker_damage = (stats->'raid_card_research'->>'burst_priker_damage')::INTEGER,
    card_base_affliction_damage = (stats->'raid_card_research'->>'base_affliction_damage')::INTEGER,
    card_affliction_lojak_damage = (stats->'raid_card_research'->>'affliction_lojak_damage')::INTEGER,
    card_affliction_takedar_damage = (stats->'raid_card_research'->>'affliction_takedar_damage')::INTEGER,
    card_affliction_jukk_damage = (stats->'raid_card_research'->>'affliction_jukk_damage')::INTEGER,
    card_affliction_sterl_damage = (stats->'raid_card_research'->>'affliction_sterl_damage')::INTEGER,
    card_affliction_mohaca_damage = (stats->'raid_card_research'->>'affliction_mohaca_damage')::INTEGER,
    card_affliction_terro_damage = (stats->'raid_card_research'->>'affliction_terro_damage')::INTEGER,
    card_affliction_klonk_damage = (stats->'raid_card_research'->>'affliction_klonk_damage')::INTEGER,
    card_affliction_priker_damage = (stats->'raid_card_research'->>'affliction_priker_damage')::INTEGER,

    gem_base_damage = (stats->'gem_stone_research'->>'base_damage')::INTEGER,
    gem_head_damage = (stats->'gem_stone_research'->>'head_damage')::INTEGER,
    gem_torso_damage = (stats->'gem_stone_research'->>'torso_damage')::INTEGER,
    gem_limbs_damage = (stats->'gem_stone_research'->>'limbs_damage')::INTEGER,
    gem_armor_damage = (stats->'gem_stone_research'->>'armor_damage')::INTEGER,
    gem_head_armor_damage = (stats->'gem_stone_research'->>'head_armor_damage')::INTEGER,
    gem_torso_armor_damage = (stats->'gem_stone_research'->>'torso_armor_damage')::INTEGER,
    gem_limbs_armor_damage = (stats->'gem_stone_research'->>'limbs_armor_damage')::INTEGER,
    gem_body_damage = (stats->'gem_stone_research'->>'body_damage')::INTEGER,
    gem_head_body_damage = (stats->'gem_stone_research'->>'head_body_damage')::INTEGER,
    gem_torso_body_damage = (stats->'gem_stone_research'->>'torso_body_damage')::INTEGER,
    gem_limbs_body_damage = (stats->'gem_stone_research'->>'limbs_body_damage')::INTEGER,
    gem_lojak_damage = (stats->'gem_stone_research'->>'lojak_damage')::INTEGER,
    gem_takedar_damage = (stats->'gem_stone_research'->>'takedar_damage')::INTEGER,
    gem_jukk_damage = (stats->'gem_stone_research'->>'jukk_damage')::INTEGER,
    gem_sterl_damage = (stats->'gem_stone_research'->>'sterl_damage')::INTEGER,
    gem_mohaca_damage = (stats->'gem_stone_research'->>'mohaca_damage')::INTEGER,
    gem_terro_damage = (stats->'gem_stone_research'->>'terro_damage')::INTEGER,
    gem_klonk_damage = (stats->'gem_stone_research'->>'klonk_damage')::INTEGER,
    gem_priker_damage = (stats->'gem_stone_research'->>'priker_damage')::INTEGER,
    gem_base_burst_damage = (stats->'gem_stone_research'->>'base_burst_damage')::INTEGER,
    gem_burst_lojak_damage = (stats->'gem_stone_research'->>'burst_lojak_damage')::INTEGER,
    gem_burst_takedar_damage = (stats->'gem_stone_research'->>'burst_takedar_damage')::INTEGER,
    gem_burst_jukk_damage = (stats->'gem_stone_research'->>'burst_jukk_damage')::INTEGER,
    gem_burst_sterl_damage = (stats->'gem_stone_research'->>'burst_sterl_damage')::INTEGER,
    gem_burst_mohaca_damage = (stats->'gem_stone_research'->>'burst_mohaca_damage')::INTEGER,
    gem_burst_terro_damage = (stats->'gem_stone_research'->>'burst_terro_damage')::INTEGER,
    gem_burst_klonk_damage = (stats->'gem_stone_research'->>'burst_klonk_damage')::INTEGER,
    gem_burst_priker_damage = (stats->'gem_stone_research'->>'burst_priker_damage')::INTEGER,
    gem_base_affliction_damage = (stats->'gem_stone_research'->>'base_affliction_damage')::INTEGER,
    gem_affliction_lojak_damage = (stats->'gem_stone_research'->>'affliction_lojak_damage')::INTEGER,
    gem_affliction_takedar_damage = (stats->'gem_stone_research'->>'affliction_takedar_damage')::INTEGER,
    gem_affliction_jukk_damage = (stats->'gem_stone_research'->>'affliction_jukk_damage')::INTEGER,
    gem_affliction_sterl_damage = (stats->'gem_stone_research'->>'affliction_sterl_damage')::INTEGER,
    gem_affliction_mohaca_damage = (stats->'gem_stone_research'->>'affliction_mohaca_damage')::INTEGER,
    gem_affliction_terro_damage = (stats->'gem_stone_research'->>'affliction_terro_damage')::INTEGER,
    gem_affliction_klonk_damage = (stats->'gem_stone_research'->>'affliction_klonk_damage')::INTEGER,
    gem_affliction_priker_damage = (stats->'gem_stone_research'->>'affliction_priker_damage')::INTEGER;

ALTER TABLE player_stats
    ALTER COLUMN player_raid_level SET NOT NULL,
    ALTER COLUMN player_raid_base_damage SET NOT NULL,
    ALTER COLUMN title SET NOT NULL,
    ALTER COLUMN raid_set_jade_anniversary SET NOT NULL,
    ALTER COLUMN raid_set_jukk_juggernaut SET NOT NULL,
    ALTER COLUMN raid_set_airforce_ace SET NOT NULL,
    ALTER COLUMN raid_set_dancer_venom SET NOT NULL,
    ALTER COLUMN raid_set_rose_anniversary SET NOT NULL,
    ALTER COLUMN titan_soul_head_mult SET NOT NULL,
    ALTER COLUMN titan_soul_torso_mult SET NOT NULL,
    ALTER COLUMN titan_soul_limbs_mult SET NOT NULL,
    ALTER COLUMN titan_soul_armor_mult SET NOT NULL,
    ALTER COLUMN titan_soul_body_mult SET NOT NULL,
    ALTER COLUMN titan_soul_lojak_mult SET NOT NULL,
    ALTER COLUMN titan_soul_takedar_mult SET NOT NULL,
    ALTER COLUMN titan_soul_jukk_mult SET NOT NULL,
    ALTER COLUMN titan_soul_sterl_mult SET NOT NULL,
    ALTER COLUMN titan_soul_mohaca_mult SET NOT NULL,
    ALTER COLUMN titan_soul_terro_mult SET NOT NULL,
    ALTER COLUMN titan_soul_klonk_mult SET NOT NULL,
    ALTER COLUMN titan_soul_priker_mult SET NOT NULL,
    ALTER COLUMN card_base_damage SET NOT NULL,
    ALTER COLUMN card_head_damage SET NOT NULL,
    ALTER COLUMN card_torso_damage SET NOT NULL,
    ALTER COLUMN card_limbs_damage SET NOT NULL,
    ALTER COLUMN card_armor_damage SET NOT NULL,
    ALTER COLUMN card_head_armor_damage SET NOT NULL,
    ALTER COLUMN card_torso_armor_damage SET NOT NULL,
    ALTER COLUMN card_limbs_armor_damage SET NOT NULL,
    ALTER COLUMN card_body_damage SET NOT NULL,
    ALTER COLUMN card_head_body_damage SET NOT NULL,
    ALTER COLUMN card_torso_body_damage SET NOT NULL,
    ALTER COLUMN card_limbs_body_damage SET NOT NULL,
    ALTER COLUMN card_lojak_damage SET NOT NULL,
    ALTER COLUMN card_takedar_damage SET NOT NULL,
    ALTER COLUMN card_jukk_damage SET NOT NULL,
    ALTER COLUMN card_sterl_damage SET NOT NULL,
    ALTER COLUMN card_mohaca_damage SET NOT NULL,
    ALTER COLUMN card_terro_damage SET NOT NULL,
    ALTER COLUMN card_klonk_damage SET NOT NULL,
    ALTER COLUMN card_priker_damage SET NOT NULL,
    ALTER COLUMN card_base_burst_damage SET NOT NULL,
    ALTER COLUMN card_burst_lojak_damage SET NOT NULL,
    ALTER COLUMN card_burst_takedar_damage SET NOT NULL,
    ALTER COLUMN card_burst_jukk_damage SET NOT NULL,
    ALTER COLUMN card_burst_sterl_damage SET NOT NULL,
    ALTER COLUMN card_burst_mohaca_damage SET NOT NULL,
    ALTER COLUMN card_burst_terro_damage SET NOT NULL,
    ALTER COLUMN card_burst_klonk_damage SET NOT NULL,
    ALTER COLUMN card_burst_priker_damage SET NOT NULL,
    ALTER COLUMN card_base_affliction_damage SET NOT NULL,
    ALTER COLUMN card_affliction_lojak_damage SET NOT NULL,
    ALTER COLUMN card_affliction_takedar_damage SET NOT NULL,
    ALTER COLUMN card_affliction_jukk_damage SET NOT NULL,
    ALTER COLUMN card_affliction_sterl_damage SET NOT NULL,
    ALTER COLUMN card_affliction_mohaca_damage SET NOT NULL,
    ALTER COLUMN card_affliction_terro_damage SET NOT NULL,
    ALTER COLUMN card_affliction_klonk_damage SET NOT NULL,
    ALTER COLUMN card_affliction_priker_damage SET NOT NULL,
    ALTER COLUMN gem_base_damage SET NOT NULL,
    ALTER COLUMN gem_head_damage SET NOT NULL,
    ALTER COLUMN gem_torso_damage SET NOT NULL,
    ALTER COLUMN gem_limbs_damage SET NOT NULL,
    ALTER COLUMN gem_armor_damage SET NOT NULL,
    ALTER COLUMN gem_head_armor_damage SET NOT NULL,
    ALTER COLUMN gem_torso_armor_damage SET NOT NULL,
    ALTER COLUMN gem_limbs_armor_damage SET NOT NULL,
    ALTER COLUMN gem_body_damage SET NOT NULL,
    ALTER COLUMN gem_head_body_damage SET NOT NULL,
    ALTER COLUMN gem_torso_body_damage SET NOT NULL,
    ALTER COLUMN gem_limbs_body_damage SET NOT NULL,
    ALTER COLUMN gem_lojak_damage SET NOT NULL,
    ALTER COLUMN gem_takedar_damage SET NOT NULL,
    ALTER COLUMN gem_jukk_damage SET NOT NULL,
    ALTER COLUMN gem_sterl_damage SET NOT NULL,
    ALTER COLUMN gem_mohaca_damage SET NOT NULL,
    ALTER COLUMN gem_terro_damage SET NOT NULL,
    ALTER COLUMN gem_klonk_damage SET NOT NULL,
    ALTER COLUMN gem_priker_damage SET NOT NULL,
    ALTER COLUMN gem_base_burst_damage SET NOT NULL,
    ALTER COLUMN gem_burst_lojak_damage SET NOT NULL,
    ALTER COLUMN gem_burst_takedar_damage SET NOT NULL,
    ALTER COLUMN gem_burst_jukk_damage SET NOT NULL,
    ALTER COLUMN gem_burst_sterl_damage SET NOT NULL,
    ALTER COLUMN gem_burst_mohaca_damage SET NOT NULL,
    ALTER COLUMN gem_burst_terro_damage SET NOT NULL,
    ALTER COLUMN gem_burst_klonk_damage SET NOT NULL,
    ALTER COLUMN gem_burst_priker_damage SET NOT NULL,
    ALTER COLUMN gem_base_affliction_damage SET NOT NULL,
    ALTER COLUMN gem_affliction_lojak_damage SET NOT NULL,
    ALTER COLUMN gem_affliction_takedar_damage SET NOT NULL,
    ALTER COLUMN gem_affliction_jukk_damage SET NOT NULL,
    ALTER COLUMN gem_affliction_sterl_damage SET NOT NULL,
    ALTER COLUMN gem_affliction_mohaca_damage SET NOT NULL,
    ALTER COLUMN gem_affliction_terro_damage SET NOT NULL,
    ALTER COLUMN gem_affliction_klonk_damage SET NOT NULL,
    ALTER COLUMN gem_affliction_priker_damage SET NOT NULL;

ALTER TABLE player_stats ADD CONSTRAINT player_stats_u16_ranges_check CHECK (
    player_raid_level BETWEEN 0 AND 65535 AND
    player_raid_base_damage BETWEEN 0 AND 65535 AND
    card_base_damage BETWEEN 0 AND 65535 AND
    card_head_damage BETWEEN 0 AND 65535 AND
    card_torso_damage BETWEEN 0 AND 65535 AND
    card_limbs_damage BETWEEN 0 AND 65535 AND
    card_armor_damage BETWEEN 0 AND 65535 AND
    card_head_armor_damage BETWEEN 0 AND 65535 AND
    card_torso_armor_damage BETWEEN 0 AND 65535 AND
    card_limbs_armor_damage BETWEEN 0 AND 65535 AND
    card_body_damage BETWEEN 0 AND 65535 AND
    card_head_body_damage BETWEEN 0 AND 65535 AND
    card_torso_body_damage BETWEEN 0 AND 65535 AND
    card_limbs_body_damage BETWEEN 0 AND 65535 AND
    card_lojak_damage BETWEEN 0 AND 65535 AND
    card_takedar_damage BETWEEN 0 AND 65535 AND
    card_jukk_damage BETWEEN 0 AND 65535 AND
    card_sterl_damage BETWEEN 0 AND 65535 AND
    card_mohaca_damage BETWEEN 0 AND 65535 AND
    card_terro_damage BETWEEN 0 AND 65535 AND
    card_klonk_damage BETWEEN 0 AND 65535 AND
    card_priker_damage BETWEEN 0 AND 65535 AND
    card_base_burst_damage BETWEEN 0 AND 65535 AND
    card_burst_lojak_damage BETWEEN 0 AND 65535 AND
    card_burst_takedar_damage BETWEEN 0 AND 65535 AND
    card_burst_jukk_damage BETWEEN 0 AND 65535 AND
    card_burst_sterl_damage BETWEEN 0 AND 65535 AND
    card_burst_mohaca_damage BETWEEN 0 AND 65535 AND
    card_burst_terro_damage BETWEEN 0 AND 65535 AND
    card_burst_klonk_damage BETWEEN 0 AND 65535 AND
    card_burst_priker_damage BETWEEN 0 AND 65535 AND
    card_base_affliction_damage BETWEEN 0 AND 65535 AND
    card_affliction_lojak_damage BETWEEN 0 AND 65535 AND
    card_affliction_takedar_damage BETWEEN 0 AND 65535 AND
    card_affliction_jukk_damage BETWEEN 0 AND 65535 AND
    card_affliction_sterl_damage BETWEEN 0 AND 65535 AND
    card_affliction_mohaca_damage BETWEEN 0 AND 65535 AND
    card_affliction_terro_damage BETWEEN 0 AND 65535 AND
    card_affliction_klonk_damage BETWEEN 0 AND 65535 AND
    card_affliction_priker_damage BETWEEN 0 AND 65535 AND
    gem_base_damage BETWEEN 0 AND 65535 AND
    gem_head_damage BETWEEN 0 AND 65535 AND
    gem_torso_damage BETWEEN 0 AND 65535 AND
    gem_limbs_damage BETWEEN 0 AND 65535 AND
    gem_armor_damage BETWEEN 0 AND 65535 AND
    gem_head_armor_damage BETWEEN 0 AND 65535 AND
    gem_torso_armor_damage BETWEEN 0 AND 65535 AND
    gem_limbs_armor_damage BETWEEN 0 AND 65535 AND
    gem_body_damage BETWEEN 0 AND 65535 AND
    gem_head_body_damage BETWEEN 0 AND 65535 AND
    gem_torso_body_damage BETWEEN 0 AND 65535 AND
    gem_limbs_body_damage BETWEEN 0 AND 65535 AND
    gem_lojak_damage BETWEEN 0 AND 65535 AND
    gem_takedar_damage BETWEEN 0 AND 65535 AND
    gem_jukk_damage BETWEEN 0 AND 65535 AND
    gem_sterl_damage BETWEEN 0 AND 65535 AND
    gem_mohaca_damage BETWEEN 0 AND 65535 AND
    gem_terro_damage BETWEEN 0 AND 65535 AND
    gem_klonk_damage BETWEEN 0 AND 65535 AND
    gem_priker_damage BETWEEN 0 AND 65535 AND
    gem_base_burst_damage BETWEEN 0 AND 65535 AND
    gem_burst_lojak_damage BETWEEN 0 AND 65535 AND
    gem_burst_takedar_damage BETWEEN 0 AND 65535 AND
    gem_burst_jukk_damage BETWEEN 0 AND 65535 AND
    gem_burst_sterl_damage BETWEEN 0 AND 65535 AND
    gem_burst_mohaca_damage BETWEEN 0 AND 65535 AND
    gem_burst_terro_damage BETWEEN 0 AND 65535 AND
    gem_burst_klonk_damage BETWEEN 0 AND 65535 AND
    gem_burst_priker_damage BETWEEN 0 AND 65535 AND
    gem_base_affliction_damage BETWEEN 0 AND 65535 AND
    gem_affliction_lojak_damage BETWEEN 0 AND 65535 AND
    gem_affliction_takedar_damage BETWEEN 0 AND 65535 AND
    gem_affliction_jukk_damage BETWEEN 0 AND 65535 AND
    gem_affliction_sterl_damage BETWEEN 0 AND 65535 AND
    gem_affliction_mohaca_damage BETWEEN 0 AND 65535 AND
    gem_affliction_terro_damage BETWEEN 0 AND 65535 AND
    gem_affliction_klonk_damage BETWEEN 0 AND 65535 AND
    gem_affliction_priker_damage BETWEEN 0 AND 65535
);

-- `stats` becomes a stale legacy copy from here on -- new writes stop
-- populating it (see player_stats_repo.rs). Dropped entirely in a later
-- contract-phase migration once the app has run cleanly against the
-- normalized columns for a while.
ALTER TABLE player_stats ALTER COLUMN stats DROP NOT NULL;

CREATE TABLE player_cards (
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    card_id card_name NOT NULL,
    level INTEGER NOT NULL CHECK (level BETWEEN 0 AND 65535),
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    PRIMARY KEY (player_id, card_id)
);

-- card_list JSON stores TT2's raw wire IDs (e.g. "BurstCount"), which differ
-- from several card_name enum labels (Rust variant names, e.g.
-- 'ClanshipBarrage') by design -- see cards.rs CardName::id(). Translate here.
INSERT INTO player_cards (player_id, card_id, level, enabled)
SELECT
    ps.player_id,
    (CASE (card_elem->>'card_id')
        WHEN 'MoonBeam' THEN 'MoonBeam'
        WHEN 'Fragmentize' THEN 'Fragmentize'
        WHEN 'SkullBash' THEN 'SkullBash'
        WHEN 'RazorWind' THEN 'RazorWind'
        WHEN 'WhipOfLightning' THEN 'WhipOfLightning'
        WHEN 'BurstCount' THEN 'ClanshipBarrage'
        WHEN 'Purify' THEN 'PurifyingBlast'
        WHEN 'LimbBurst' THEN 'PsychicShackles'
        WHEN 'FlakShot' THEN 'FlakShot'
        WHEN 'Haymaker' THEN 'CosmicHaymaker'
        WHEN 'ChainLightning' THEN 'ChainOfVengeance'
        WHEN 'MirrorForce' THEN 'MirrorForce'
        WHEN 'CelestialStatic' THEN 'CelestialStatic'
        WHEN 'Weaken' THEN 'GuardBreak'
        WHEN 'BarbedMorningstar' THEN 'BarbedMorningstar'
        WHEN 'BurningAttack' THEN 'BlazingInferno'
        WHEN 'PoisonAttack' THEN 'AcidDrench'
        WHEN 'DecayingAttack' THEN 'DecayingStrike'
        WHEN 'Fuse' THEN 'FusionBomb'
        WHEN 'Shadow' THEN 'GrimShadow'
        WHEN 'PlagueAttack' THEN 'ThrivingPlague'
        WHEN 'Disease' THEN 'Radioactivity'
        WHEN 'Swarm' THEN 'RavenousSwarm'
        WHEN 'RuinousRust' THEN 'RuinousRain'
        WHEN 'PowerBubble' THEN 'CorrosiveBubbles'
        WHEN 'RuneAttack' THEN 'Maelstrom'
        WHEN 'MagicPotion' THEN 'Amplify'
        WHEN 'SandsOfTime' THEN 'SandsOfTime'
        WHEN 'CosmicBarb' THEN 'ElectroZap'
        WHEN 'ExecutionersAxe' THEN 'CrushingInstinct'
        WHEN 'CrushingVoid' THEN 'InsanityVoid'
        WHEN 'MentalFocus' THEN 'RancidGas'
        WHEN 'ImpactAttack' THEN 'InspiringForce'
        WHEN 'InnerTruth' THEN 'SoulFire'
        WHEN 'FinisherAttack' THEN 'VictoryMarch'
        WHEN 'SuperheatMetal' THEN 'PrismaticRift'
        WHEN 'BurstBoost' THEN 'AncestralFavor'
        WHEN 'LimbSupport' THEN 'GraspingVines'
        WHEN 'TotemFairySkill' THEN 'TotemOfPower'
        WHEN 'TeamTactics' THEN 'TeamTactics'
        WHEN 'SpinalTap' THEN 'SkeletalSmash'
        WHEN 'AstralEcho' THEN 'AstralEcho'
        WHEN 'TriangleSupport' THEN 'RadiantKaleidoscope'
        WHEN 'BattleDrums' THEN 'BattleDrums'
     END)::card_name,
    (card_elem->>'level')::INTEGER,
    COALESCE((card_elem->>'enabled')::BOOLEAN, TRUE)
FROM player_stats ps, jsonb_array_elements(ps.stats->'card_list') AS card_elem;
