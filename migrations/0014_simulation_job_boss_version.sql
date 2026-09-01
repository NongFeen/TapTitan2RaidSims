ALTER TABLE simulation_jobs
ADD COLUMN boss_version BIGINT;

UPDATE simulation_jobs
SET boss_version = COALESCE(
    (SELECT version FROM current_boss WHERE singleton=TRUE),
    0
);

ALTER TABLE simulation_jobs
ALTER COLUMN boss_version SET NOT NULL;

CREATE INDEX simulation_jobs_boss_version_idx
ON simulation_jobs (boss_version, created_at);
