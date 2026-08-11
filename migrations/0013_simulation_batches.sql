CREATE TABLE simulation_batches (
    id UUID PRIMARY KEY,
    include_body_phase BOOLEAN NOT NULL DEFAULT FALSE,
    requested_count INTEGER NOT NULL CHECK (requested_count > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE simulation_batch_jobs (
    batch_id UUID NOT NULL REFERENCES simulation_batches(id) ON DELETE CASCADE,
    simulation_job_id UUID NOT NULL REFERENCES simulation_jobs(id) ON DELETE CASCADE,
    created_for_batch BOOLEAN NOT NULL,
    PRIMARY KEY (batch_id, simulation_job_id)
);

CREATE INDEX simulation_batch_jobs_batch_idx
ON simulation_batch_jobs (batch_id);
