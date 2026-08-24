-- Foundational enum types for the DB normalization redesign.
-- Purely additive: no existing table/column is touched by this migration.

CREATE TYPE token_status AS ENUM ('missing', 'configured', 'invalid');
CREATE TYPE job_status AS ENUM ('pending', 'running', 'optimizing', 'completed', 'failed');
CREATE TYPE recompute_mode AS ENUM ('full', 'phase_aware');
CREATE TYPE recommendation_phase AS ENUM ('current', 'void');
CREATE TYPE component_kind AS ENUM ('tap', 'card');

CREATE TYPE boss_part_name AS ENUM (
    'Head', 'Torso', 'LeftShoulder', 'RightShoulder', 'LeftHand', 'RightHand', 'LeftLeg', 'RightLeg'
);
CREATE TYPE boss_name AS ENUM (
    'Lojak', 'Takedar', 'Jukk', 'Sterl', 'Mohaca', 'Terro', 'Klonk', 'Priker'
);
CREATE TYPE part_state AS ENUM ('Cursed', 'Armor', 'Body', 'Skeleton');
CREATE TYPE curse_type AS ENUM ('None', 'BodyDamage', 'BurstDamage', 'AfflictionDamage');
CREATE TYPE global_raid_modifier AS ENUM (
    'None', 'BurstDamage', 'BurstChance', 'SupportEffect', 'AfflictionChance',
    'AfflictionDamage', 'AllDamage', 'AttackDuration', 'AfflictionDuration'
);

CREATE TYPE card_type AS ENUM ('Burst', 'Affliction', 'Support');

-- Exact declaration order of CardName in backend/src/models/cards.rs.
-- This order is documentation-only for the enum type itself (Postgres enums
-- are compared by creation order, not used here for that purpose), but the
-- separate `card_mask` bit-position order in Rust (CardName::iter()) must
-- never be changed independently of this list -- see cards_tests.rs pin.
CREATE TYPE card_name AS ENUM (
    'MoonBeam', 'Fragmentize', 'SkullBash', 'RazorWind', 'WhipOfLightning', 'ClanshipBarrage',
    'PurifyingBlast', 'PsychicShackles', 'FlakShot', 'CosmicHaymaker', 'ChainOfVengeance',
    'MirrorForce', 'CelestialStatic', 'GuardBreak', 'BarbedMorningstar',
    'BlazingInferno', 'AcidDrench', 'DecayingStrike', 'FusionBomb', 'GrimShadow', 'ThrivingPlague',
    'Radioactivity', 'RavenousSwarm', 'RuinousRain', 'CorrosiveBubbles', 'Maelstrom', 'Amplify',
    'SandsOfTime', 'ElectroZap',
    'CrushingInstinct', 'InsanityVoid', 'RancidGas', 'InspiringForce', 'SoulFire', 'VictoryMarch',
    'PrismaticRift', 'AncestralFavor', 'GraspingVines', 'TotemOfPower', 'TeamTactics',
    'SkeletalSmash', 'AstralEcho', 'RadiantKaleidoscope', 'BattleDrums'
);

CREATE TYPE research_kind AS ENUM ('card', 'gemstone');

-- Field names from RaidCardResearch/GemstoneResearch in
-- backend/src/models/player_raid_data.rs (both structs share this shape).
CREATE TYPE research_stat_key AS ENUM (
    'base_damage',
    'head_damage', 'torso_damage', 'limbs_damage',
    'armor_damage', 'head_armor_damage', 'torso_armor_damage', 'limbs_armor_damage',
    'body_damage', 'head_body_damage', 'torso_body_damage', 'limbs_body_damage',
    'lojak_damage', 'takedar_damage', 'jukk_damage', 'sterl_damage',
    'mohaca_damage', 'terro_damage', 'klonk_damage', 'priker_damage',
    'base_burst_damage', 'burst_lojak_damage', 'burst_takedar_damage', 'burst_jukk_damage',
    'burst_sterl_damage', 'burst_mohaca_damage', 'burst_terro_damage', 'burst_klonk_damage',
    'burst_priker_damage',
    'base_affliction_damage', 'affliction_lojak_damage', 'affliction_takedar_damage',
    'affliction_jukk_damage', 'affliction_sterl_damage', 'affliction_mohaca_damage',
    'affliction_terro_damage', 'affliction_klonk_damage', 'affliction_priker_damage'
);
