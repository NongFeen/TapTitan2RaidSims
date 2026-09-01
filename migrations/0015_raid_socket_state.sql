CREATE TABLE raid_current_state (
    raid_id BIGINT PRIMARY KEY,
    clan_code TEXT NOT NULL,
    resulting_titan_index INTEGER,
    current_enemy_id TEXT,
    refresh_required BOOLEAN NOT NULL DEFAULT FALSE,
    raid_data JSONB,
    titan_targets JSONB,
    raw_sub_cycle JSONB,
    received_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE raid_cycle_state (
    raid_id BIGINT PRIMARY KEY,
    clan_code TEXT NOT NULL,
    started_at TIMESTAMPTZ,
    raid_started_at TIMESTAMPTZ NOT NULL,
    next_reset_at TIMESTAMPTZ NOT NULL,
    morale DOUBLE PRECISION NOT NULL DEFAULT 0,
    team_tactics_morale_boost DOUBLE PRECISION NOT NULL DEFAULT 0,
    mirror_force_boost DOUBLE PRECISION NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE raid_attack_logs (
    id UUID PRIMARY KEY,
    raid_id BIGINT NOT NULL,
    clan_code TEXT NOT NULL,
    player_id UUID REFERENCES players(id) ON DELETE SET NULL,
    player_code TEXT NOT NULL,
    player_name TEXT NOT NULL,
    cycle INTEGER NOT NULL,
    attack_datetime TIMESTAMPTZ NOT NULL,
    attacked_titan_index INTEGER NOT NULL,
    resulting_titan_index INTEGER NOT NULL,
    enemy_id TEXT NOT NULL,
    tap_damage NUMERIC(30, 0) NOT NULL DEFAULT 0,
    total_damage NUMERIC(30, 0) NOT NULL,
    raw_payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (raid_id, player_code, attack_datetime)
);

CREATE INDEX raid_attack_logs_player_time_idx
ON raid_attack_logs (player_code, attack_datetime DESC);

CREATE INDEX raid_attack_logs_raid_cycle_idx
ON raid_attack_logs (raid_id, cycle, attack_datetime);

CREATE TABLE raid_attack_components (
    attack_log_id UUID NOT NULL REFERENCES raid_attack_logs(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    component_kind TEXT NOT NULL CHECK (component_kind IN ('tap', 'card')),
    card_id TEXT,
    card_name TEXT NOT NULL,
    card_level INTEGER,
    total_damage NUMERIC(30, 0) NOT NULL,
    part_damage JSONB NOT NULL,
    PRIMARY KEY (attack_log_id, position)
);

ALTER TABLE current_boss
    ADD COLUMN source_raid_id BIGINT,
    ADD COLUMN source_titan_index INTEGER,
    ADD COLUMN source_enemy_id TEXT;
