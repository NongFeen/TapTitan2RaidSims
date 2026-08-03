CREATE TABLE player_stats (
    player_id UUID PRIMARY KEY REFERENCES players(id) ON DELETE CASCADE,
    revision BIGINT NOT NULL DEFAULT 1,
    stats JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO player_stats (player_id, revision, stats, created_at, updated_at)
SELECT DISTINCT ON (player_id) player_id, version, stats, created_at, created_at
FROM player_stat_versions
ORDER BY player_id, version DESC;

ALTER TABLE simulation_jobs DROP COLUMN player_stat_version_id;
DROP TABLE player_stat_versions;
