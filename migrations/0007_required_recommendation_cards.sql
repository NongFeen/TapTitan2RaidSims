ALTER TABLE deck_recommendations
ADD COLUMN must_include_mirror_force_and_team_tactics BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE deck_recommendations
DROP CONSTRAINT deck_recommendations_simulation_job_id_deck_count_key;

ALTER TABLE deck_recommendations
ADD CONSTRAINT deck_recommendations_job_count_required_key UNIQUE (
    simulation_job_id,
    deck_count,
    must_include_mirror_force_and_team_tactics
);
