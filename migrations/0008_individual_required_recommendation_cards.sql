ALTER TABLE deck_recommendations
ADD COLUMN must_include_mirror_force BOOLEAN NOT NULL DEFAULT FALSE,
ADD COLUMN must_include_team_tactics BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE deck_recommendations
SET must_include_mirror_force = must_include_mirror_force_and_team_tactics,
    must_include_team_tactics = must_include_mirror_force_and_team_tactics;

ALTER TABLE deck_recommendations
DROP CONSTRAINT deck_recommendations_job_count_required_key;

ALTER TABLE deck_recommendations
DROP COLUMN must_include_mirror_force_and_team_tactics;

ALTER TABLE deck_recommendations
ADD CONSTRAINT deck_recommendations_job_count_required_cards_key UNIQUE (
    simulation_job_id,
    deck_count,
    must_include_mirror_force,
    must_include_team_tactics
);
