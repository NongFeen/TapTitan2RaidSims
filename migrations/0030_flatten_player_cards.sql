-- Collapse player_cards from one row per (player, card) into one row per
-- player: a dedicated level column per card (44 total, one per CardName
-- variant) plus a single bitmask column for which cards the player has
-- disabled, mirroring the card_mask convention already used by
-- simulation_deck_results (bit i = CardName::iter() position i). A NULL
-- level column means the card has never been reported for this player (not
-- owned yet) -- distinct from level 0, which player_cards already allowed.
ALTER TABLE player_cards
    ADD COLUMN moon_beam_level INTEGER,
    ADD COLUMN fragmentize_level INTEGER,
    ADD COLUMN skull_bash_level INTEGER,
    ADD COLUMN razor_wind_level INTEGER,
    ADD COLUMN whip_of_lightning_level INTEGER,
    ADD COLUMN clanship_barrage_level INTEGER,
    ADD COLUMN purifying_blast_level INTEGER,
    ADD COLUMN psychic_shackles_level INTEGER,
    ADD COLUMN flak_shot_level INTEGER,
    ADD COLUMN cosmic_haymaker_level INTEGER,
    ADD COLUMN chain_of_vengeance_level INTEGER,
    ADD COLUMN mirror_force_level INTEGER,
    ADD COLUMN celestial_static_level INTEGER,
    ADD COLUMN guard_break_level INTEGER,
    ADD COLUMN barbed_morningstar_level INTEGER,
    ADD COLUMN blazing_inferno_level INTEGER,
    ADD COLUMN acid_drench_level INTEGER,
    ADD COLUMN decaying_strike_level INTEGER,
    ADD COLUMN fusion_bomb_level INTEGER,
    ADD COLUMN grim_shadow_level INTEGER,
    ADD COLUMN thriving_plague_level INTEGER,
    ADD COLUMN radioactivity_level INTEGER,
    ADD COLUMN ravenous_swarm_level INTEGER,
    ADD COLUMN ruinous_rain_level INTEGER,
    ADD COLUMN corrosive_bubbles_level INTEGER,
    ADD COLUMN maelstrom_level INTEGER,
    ADD COLUMN amplify_level INTEGER,
    ADD COLUMN sands_of_time_level INTEGER,
    ADD COLUMN electro_zap_level INTEGER,
    ADD COLUMN crushing_instinct_level INTEGER,
    ADD COLUMN insanity_void_level INTEGER,
    ADD COLUMN rancid_gas_level INTEGER,
    ADD COLUMN inspiring_force_level INTEGER,
    ADD COLUMN soul_fire_level INTEGER,
    ADD COLUMN victory_march_level INTEGER,
    ADD COLUMN prismatic_rift_level INTEGER,
    ADD COLUMN ancestral_favor_level INTEGER,
    ADD COLUMN grasping_vines_level INTEGER,
    ADD COLUMN totem_of_power_level INTEGER,
    ADD COLUMN team_tactics_level INTEGER,
    ADD COLUMN skeletal_smash_level INTEGER,
    ADD COLUMN astral_echo_level INTEGER,
    ADD COLUMN radiant_kaleidoscope_level INTEGER,
    ADD COLUMN battle_drums_level INTEGER,
    ADD COLUMN disabled_card_mask BIGINT;

WITH bit_index (card_id, bit_index) AS (
    VALUES
        ('MoonBeam'::card_name, 0),
        ('Fragmentize'::card_name, 1),
        ('SkullBash'::card_name, 2),
        ('RazorWind'::card_name, 3),
        ('WhipOfLightning'::card_name, 4),
        ('ClanshipBarrage'::card_name, 5),
        ('PurifyingBlast'::card_name, 6),
        ('PsychicShackles'::card_name, 7),
        ('FlakShot'::card_name, 8),
        ('CosmicHaymaker'::card_name, 9),
        ('ChainOfVengeance'::card_name, 10),
        ('MirrorForce'::card_name, 11),
        ('CelestialStatic'::card_name, 12),
        ('GuardBreak'::card_name, 13),
        ('BarbedMorningstar'::card_name, 14),
        ('BlazingInferno'::card_name, 15),
        ('AcidDrench'::card_name, 16),
        ('DecayingStrike'::card_name, 17),
        ('FusionBomb'::card_name, 18),
        ('GrimShadow'::card_name, 19),
        ('ThrivingPlague'::card_name, 20),
        ('Radioactivity'::card_name, 21),
        ('RavenousSwarm'::card_name, 22),
        ('RuinousRain'::card_name, 23),
        ('CorrosiveBubbles'::card_name, 24),
        ('Maelstrom'::card_name, 25),
        ('Amplify'::card_name, 26),
        ('SandsOfTime'::card_name, 27),
        ('ElectroZap'::card_name, 28),
        ('CrushingInstinct'::card_name, 29),
        ('InsanityVoid'::card_name, 30),
        ('RancidGas'::card_name, 31),
        ('InspiringForce'::card_name, 32),
        ('SoulFire'::card_name, 33),
        ('VictoryMarch'::card_name, 34),
        ('PrismaticRift'::card_name, 35),
        ('AncestralFavor'::card_name, 36),
        ('GraspingVines'::card_name, 37),
        ('TotemOfPower'::card_name, 38),
        ('TeamTactics'::card_name, 39),
        ('SkeletalSmash'::card_name, 40),
        ('AstralEcho'::card_name, 41),
        ('RadiantKaleidoscope'::card_name, 42),
        ('BattleDrums'::card_name, 43)
),
disabled_masks AS (
    SELECT pc.player_id, SUM(1::bigint << bi.bit_index) AS disabled_card_mask
    FROM player_cards pc JOIN bit_index bi ON bi.card_id = pc.card_id
    WHERE NOT pc.enabled
    GROUP BY pc.player_id
),
pivoted AS (
    SELECT player_id,
        MAX(CASE WHEN card_id='MoonBeam' THEN level END) AS moon_beam_level,
        MAX(CASE WHEN card_id='Fragmentize' THEN level END) AS fragmentize_level,
        MAX(CASE WHEN card_id='SkullBash' THEN level END) AS skull_bash_level,
        MAX(CASE WHEN card_id='RazorWind' THEN level END) AS razor_wind_level,
        MAX(CASE WHEN card_id='WhipOfLightning' THEN level END) AS whip_of_lightning_level,
        MAX(CASE WHEN card_id='ClanshipBarrage' THEN level END) AS clanship_barrage_level,
        MAX(CASE WHEN card_id='PurifyingBlast' THEN level END) AS purifying_blast_level,
        MAX(CASE WHEN card_id='PsychicShackles' THEN level END) AS psychic_shackles_level,
        MAX(CASE WHEN card_id='FlakShot' THEN level END) AS flak_shot_level,
        MAX(CASE WHEN card_id='CosmicHaymaker' THEN level END) AS cosmic_haymaker_level,
        MAX(CASE WHEN card_id='ChainOfVengeance' THEN level END) AS chain_of_vengeance_level,
        MAX(CASE WHEN card_id='MirrorForce' THEN level END) AS mirror_force_level,
        MAX(CASE WHEN card_id='CelestialStatic' THEN level END) AS celestial_static_level,
        MAX(CASE WHEN card_id='GuardBreak' THEN level END) AS guard_break_level,
        MAX(CASE WHEN card_id='BarbedMorningstar' THEN level END) AS barbed_morningstar_level,
        MAX(CASE WHEN card_id='BlazingInferno' THEN level END) AS blazing_inferno_level,
        MAX(CASE WHEN card_id='AcidDrench' THEN level END) AS acid_drench_level,
        MAX(CASE WHEN card_id='DecayingStrike' THEN level END) AS decaying_strike_level,
        MAX(CASE WHEN card_id='FusionBomb' THEN level END) AS fusion_bomb_level,
        MAX(CASE WHEN card_id='GrimShadow' THEN level END) AS grim_shadow_level,
        MAX(CASE WHEN card_id='ThrivingPlague' THEN level END) AS thriving_plague_level,
        MAX(CASE WHEN card_id='Radioactivity' THEN level END) AS radioactivity_level,
        MAX(CASE WHEN card_id='RavenousSwarm' THEN level END) AS ravenous_swarm_level,
        MAX(CASE WHEN card_id='RuinousRain' THEN level END) AS ruinous_rain_level,
        MAX(CASE WHEN card_id='CorrosiveBubbles' THEN level END) AS corrosive_bubbles_level,
        MAX(CASE WHEN card_id='Maelstrom' THEN level END) AS maelstrom_level,
        MAX(CASE WHEN card_id='Amplify' THEN level END) AS amplify_level,
        MAX(CASE WHEN card_id='SandsOfTime' THEN level END) AS sands_of_time_level,
        MAX(CASE WHEN card_id='ElectroZap' THEN level END) AS electro_zap_level,
        MAX(CASE WHEN card_id='CrushingInstinct' THEN level END) AS crushing_instinct_level,
        MAX(CASE WHEN card_id='InsanityVoid' THEN level END) AS insanity_void_level,
        MAX(CASE WHEN card_id='RancidGas' THEN level END) AS rancid_gas_level,
        MAX(CASE WHEN card_id='InspiringForce' THEN level END) AS inspiring_force_level,
        MAX(CASE WHEN card_id='SoulFire' THEN level END) AS soul_fire_level,
        MAX(CASE WHEN card_id='VictoryMarch' THEN level END) AS victory_march_level,
        MAX(CASE WHEN card_id='PrismaticRift' THEN level END) AS prismatic_rift_level,
        MAX(CASE WHEN card_id='AncestralFavor' THEN level END) AS ancestral_favor_level,
        MAX(CASE WHEN card_id='GraspingVines' THEN level END) AS grasping_vines_level,
        MAX(CASE WHEN card_id='TotemOfPower' THEN level END) AS totem_of_power_level,
        MAX(CASE WHEN card_id='TeamTactics' THEN level END) AS team_tactics_level,
        MAX(CASE WHEN card_id='SkeletalSmash' THEN level END) AS skeletal_smash_level,
        MAX(CASE WHEN card_id='AstralEcho' THEN level END) AS astral_echo_level,
        MAX(CASE WHEN card_id='RadiantKaleidoscope' THEN level END) AS radiant_kaleidoscope_level,
        MAX(CASE WHEN card_id='BattleDrums' THEN level END) AS battle_drums_level
    FROM player_cards
    GROUP BY player_id
)
UPDATE player_cards t
SET
    moon_beam_level = p.moon_beam_level,
    fragmentize_level = p.fragmentize_level,
    skull_bash_level = p.skull_bash_level,
    razor_wind_level = p.razor_wind_level,
    whip_of_lightning_level = p.whip_of_lightning_level,
    clanship_barrage_level = p.clanship_barrage_level,
    purifying_blast_level = p.purifying_blast_level,
    psychic_shackles_level = p.psychic_shackles_level,
    flak_shot_level = p.flak_shot_level,
    cosmic_haymaker_level = p.cosmic_haymaker_level,
    chain_of_vengeance_level = p.chain_of_vengeance_level,
    mirror_force_level = p.mirror_force_level,
    celestial_static_level = p.celestial_static_level,
    guard_break_level = p.guard_break_level,
    barbed_morningstar_level = p.barbed_morningstar_level,
    blazing_inferno_level = p.blazing_inferno_level,
    acid_drench_level = p.acid_drench_level,
    decaying_strike_level = p.decaying_strike_level,
    fusion_bomb_level = p.fusion_bomb_level,
    grim_shadow_level = p.grim_shadow_level,
    thriving_plague_level = p.thriving_plague_level,
    radioactivity_level = p.radioactivity_level,
    ravenous_swarm_level = p.ravenous_swarm_level,
    ruinous_rain_level = p.ruinous_rain_level,
    corrosive_bubbles_level = p.corrosive_bubbles_level,
    maelstrom_level = p.maelstrom_level,
    amplify_level = p.amplify_level,
    sands_of_time_level = p.sands_of_time_level,
    electro_zap_level = p.electro_zap_level,
    crushing_instinct_level = p.crushing_instinct_level,
    insanity_void_level = p.insanity_void_level,
    rancid_gas_level = p.rancid_gas_level,
    inspiring_force_level = p.inspiring_force_level,
    soul_fire_level = p.soul_fire_level,
    victory_march_level = p.victory_march_level,
    prismatic_rift_level = p.prismatic_rift_level,
    ancestral_favor_level = p.ancestral_favor_level,
    grasping_vines_level = p.grasping_vines_level,
    totem_of_power_level = p.totem_of_power_level,
    team_tactics_level = p.team_tactics_level,
    skeletal_smash_level = p.skeletal_smash_level,
    astral_echo_level = p.astral_echo_level,
    radiant_kaleidoscope_level = p.radiant_kaleidoscope_level,
    battle_drums_level = p.battle_drums_level,
    disabled_card_mask = COALESCE(dm.disabled_card_mask, 0)
FROM pivoted p
LEFT JOIN disabled_masks dm ON dm.player_id = p.player_id
WHERE t.player_id = p.player_id
  AND t.card_id = (SELECT MIN(card_id) FROM player_cards p2 WHERE p2.player_id = t.player_id);

-- Every player has exactly one survivor row (the one with the
-- lexicographically-first card_id) carrying the pivoted values above; every
-- other row for that player is now a redundant duplicate.
DELETE FROM player_cards t
WHERE t.card_id <> (SELECT MIN(card_id) FROM player_cards p2 WHERE p2.player_id = t.player_id);

ALTER TABLE player_cards DROP CONSTRAINT player_cards_pkey;
ALTER TABLE player_cards
    DROP COLUMN card_id,
    DROP COLUMN level,
    DROP COLUMN enabled;
ALTER TABLE player_cards
    ALTER COLUMN disabled_card_mask SET NOT NULL,
    ALTER COLUMN disabled_card_mask SET DEFAULT 0;
ALTER TABLE player_cards ADD PRIMARY KEY (player_id);
ALTER TABLE player_cards ADD CONSTRAINT player_cards_level_ranges_check CHECK (
    (moon_beam_level IS NULL OR (moon_beam_level BETWEEN 0 AND 65535)) AND
    (fragmentize_level IS NULL OR (fragmentize_level BETWEEN 0 AND 65535)) AND
    (skull_bash_level IS NULL OR (skull_bash_level BETWEEN 0 AND 65535)) AND
    (razor_wind_level IS NULL OR (razor_wind_level BETWEEN 0 AND 65535)) AND
    (whip_of_lightning_level IS NULL OR (whip_of_lightning_level BETWEEN 0 AND 65535)) AND
    (clanship_barrage_level IS NULL OR (clanship_barrage_level BETWEEN 0 AND 65535)) AND
    (purifying_blast_level IS NULL OR (purifying_blast_level BETWEEN 0 AND 65535)) AND
    (psychic_shackles_level IS NULL OR (psychic_shackles_level BETWEEN 0 AND 65535)) AND
    (flak_shot_level IS NULL OR (flak_shot_level BETWEEN 0 AND 65535)) AND
    (cosmic_haymaker_level IS NULL OR (cosmic_haymaker_level BETWEEN 0 AND 65535)) AND
    (chain_of_vengeance_level IS NULL OR (chain_of_vengeance_level BETWEEN 0 AND 65535)) AND
    (mirror_force_level IS NULL OR (mirror_force_level BETWEEN 0 AND 65535)) AND
    (celestial_static_level IS NULL OR (celestial_static_level BETWEEN 0 AND 65535)) AND
    (guard_break_level IS NULL OR (guard_break_level BETWEEN 0 AND 65535)) AND
    (barbed_morningstar_level IS NULL OR (barbed_morningstar_level BETWEEN 0 AND 65535)) AND
    (blazing_inferno_level IS NULL OR (blazing_inferno_level BETWEEN 0 AND 65535)) AND
    (acid_drench_level IS NULL OR (acid_drench_level BETWEEN 0 AND 65535)) AND
    (decaying_strike_level IS NULL OR (decaying_strike_level BETWEEN 0 AND 65535)) AND
    (fusion_bomb_level IS NULL OR (fusion_bomb_level BETWEEN 0 AND 65535)) AND
    (grim_shadow_level IS NULL OR (grim_shadow_level BETWEEN 0 AND 65535)) AND
    (thriving_plague_level IS NULL OR (thriving_plague_level BETWEEN 0 AND 65535)) AND
    (radioactivity_level IS NULL OR (radioactivity_level BETWEEN 0 AND 65535)) AND
    (ravenous_swarm_level IS NULL OR (ravenous_swarm_level BETWEEN 0 AND 65535)) AND
    (ruinous_rain_level IS NULL OR (ruinous_rain_level BETWEEN 0 AND 65535)) AND
    (corrosive_bubbles_level IS NULL OR (corrosive_bubbles_level BETWEEN 0 AND 65535)) AND
    (maelstrom_level IS NULL OR (maelstrom_level BETWEEN 0 AND 65535)) AND
    (amplify_level IS NULL OR (amplify_level BETWEEN 0 AND 65535)) AND
    (sands_of_time_level IS NULL OR (sands_of_time_level BETWEEN 0 AND 65535)) AND
    (electro_zap_level IS NULL OR (electro_zap_level BETWEEN 0 AND 65535)) AND
    (crushing_instinct_level IS NULL OR (crushing_instinct_level BETWEEN 0 AND 65535)) AND
    (insanity_void_level IS NULL OR (insanity_void_level BETWEEN 0 AND 65535)) AND
    (rancid_gas_level IS NULL OR (rancid_gas_level BETWEEN 0 AND 65535)) AND
    (inspiring_force_level IS NULL OR (inspiring_force_level BETWEEN 0 AND 65535)) AND
    (soul_fire_level IS NULL OR (soul_fire_level BETWEEN 0 AND 65535)) AND
    (victory_march_level IS NULL OR (victory_march_level BETWEEN 0 AND 65535)) AND
    (prismatic_rift_level IS NULL OR (prismatic_rift_level BETWEEN 0 AND 65535)) AND
    (ancestral_favor_level IS NULL OR (ancestral_favor_level BETWEEN 0 AND 65535)) AND
    (grasping_vines_level IS NULL OR (grasping_vines_level BETWEEN 0 AND 65535)) AND
    (totem_of_power_level IS NULL OR (totem_of_power_level BETWEEN 0 AND 65535)) AND
    (team_tactics_level IS NULL OR (team_tactics_level BETWEEN 0 AND 65535)) AND
    (skeletal_smash_level IS NULL OR (skeletal_smash_level BETWEEN 0 AND 65535)) AND
    (astral_echo_level IS NULL OR (astral_echo_level BETWEEN 0 AND 65535)) AND
    (radiant_kaleidoscope_level IS NULL OR (radiant_kaleidoscope_level BETWEEN 0 AND 65535)) AND
    (battle_drums_level IS NULL OR (battle_drums_level BETWEEN 0 AND 65535))
);

