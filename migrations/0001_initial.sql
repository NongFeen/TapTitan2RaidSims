CREATE TABLE players (
    id UUID PRIMARY KEY,
    external_id TEXT UNIQUE,
    display_name TEXT NOT NULL,
    selected BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE player_stat_versions (
    id UUID PRIMARY KEY,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    version BIGINT NOT NULL,
    stats JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (player_id, version)
);

CREATE INDEX player_stat_versions_latest_idx
    ON player_stat_versions (player_id, version DESC);

CREATE TABLE raids (
    id UUID PRIMARY KEY,
    external_id TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'completed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE raid_bosses (
    id UUID PRIMARY KEY,
    raid_id UUID NOT NULL REFERENCES raids(id) ON DELETE CASCADE,
    external_event_id TEXT NOT NULL UNIQUE,
    version BIGINT NOT NULL,
    boss_data JSONB NOT NULL,
    attackable_parts JSONB NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    spawned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (raid_id, version)
);

CREATE INDEX raid_bosses_active_idx ON raid_bosses (active, spawned_at DESC);

CREATE TABLE simulation_jobs (
    id UUID PRIMARY KEY,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    player_stat_version_id UUID NOT NULL REFERENCES player_stat_versions(id),
    raid_boss_id UUID REFERENCES raid_bosses(id),
    deduplication_key TEXT NOT NULL UNIQUE,
    simulator_version TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'optimizing', 'completed', 'failed')),
    payload JSONB NOT NULL,
    result JSONB,
    error_message TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX simulation_jobs_status_idx ON simulation_jobs (status, created_at);
CREATE INDEX simulation_jobs_player_idx ON simulation_jobs (player_id, created_at DESC);

CREATE TABLE simulation_deck_results (
    id UUID PRIMARY KEY,
    simulation_job_id UUID NOT NULL REFERENCES simulation_jobs(id) ON DELETE CASCADE,
    cards JSONB NOT NULL,
    card_mask BIGINT NOT NULL,
    average_damage NUMERIC(24, 0) NOT NULL,
    result JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (simulation_job_id, card_mask)
);

CREATE INDEX simulation_deck_results_job_damage_idx
    ON simulation_deck_results (simulation_job_id, average_damage DESC);

CREATE TABLE deck_recommendations (
    id UUID PRIMARY KEY,
    simulation_job_id UUID NOT NULL REFERENCES simulation_jobs(id) ON DELETE CASCADE,
    deck_count INTEGER NOT NULL CHECK (deck_count IN (6, 9)),
    total_average_damage NUMERIC(24, 0) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (simulation_job_id, deck_count)
);

CREATE TABLE deck_recommendation_items (
    recommendation_id UUID NOT NULL REFERENCES deck_recommendations(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    simulation_deck_result_id UUID NOT NULL REFERENCES simulation_deck_results(id) ON DELETE CASCADE,
    PRIMARY KEY (recommendation_id, position),
    UNIQUE (recommendation_id, simulation_deck_result_id)
);
