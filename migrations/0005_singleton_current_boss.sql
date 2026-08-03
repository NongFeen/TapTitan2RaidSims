CREATE TABLE current_boss (
    singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    version BIGINT NOT NULL DEFAULT 1,
    boss_data JSONB NOT NULL,
    attackable_parts JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO current_boss (singleton, version, boss_data, attackable_parts, created_at, updated_at)
SELECT TRUE, version, boss_data, attackable_parts, spawned_at, updated_at
FROM raid_bosses
WHERE active = TRUE
ORDER BY updated_at DESC
LIMIT 1;

ALTER TABLE simulation_jobs DROP COLUMN raid_boss_id;
DROP TABLE raid_bosses;
DROP TABLE raids;
