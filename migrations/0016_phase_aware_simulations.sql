ALTER TABLE simulation_jobs
ADD COLUMN recompute_mode TEXT NOT NULL DEFAULT 'full'
    CHECK (recompute_mode IN ('full', 'phase_aware')),
ADD COLUMN phase_change_mask SMALLINT NOT NULL DEFAULT 0
    CHECK (phase_change_mask BETWEEN 0 AND 255),
ADD COLUMN base_job_id UUID REFERENCES simulation_jobs(id) ON DELETE SET NULL;

ALTER TABLE simulation_deck_results
ADD COLUMN dependency_part_mask SMALLINT
    CHECK (dependency_part_mask BETWEEN 0 AND 255);

CREATE INDEX simulation_jobs_base_job_idx
ON simulation_jobs (base_job_id)
WHERE base_job_id IS NOT NULL;
