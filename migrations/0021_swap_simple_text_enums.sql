-- Swap the 5 TEXT+CHECK "enum-like" columns onto real Postgres ENUM types
-- (created in 0020). Existing values already satisfy each CHECK constraint,
-- so the USING casts below are lossless.

ALTER TABLE players ALTER COLUMN tt2_token_status DROP DEFAULT;
ALTER TABLE players DROP CONSTRAINT players_tt2_token_status_check;
ALTER TABLE players
    ALTER COLUMN tt2_token_status TYPE token_status USING tt2_token_status::token_status;
ALTER TABLE players ALTER COLUMN tt2_token_status SET DEFAULT 'missing'::token_status;

ALTER TABLE simulation_jobs DROP CONSTRAINT simulation_jobs_status_check;
ALTER TABLE simulation_jobs
    ALTER COLUMN status TYPE job_status USING status::job_status;

ALTER TABLE simulation_jobs ALTER COLUMN recompute_mode DROP DEFAULT;
ALTER TABLE simulation_jobs DROP CONSTRAINT simulation_jobs_recompute_mode_check;
ALTER TABLE simulation_jobs
    ALTER COLUMN recompute_mode TYPE recompute_mode USING recompute_mode::recompute_mode;
ALTER TABLE simulation_jobs ALTER COLUMN recompute_mode SET DEFAULT 'full'::recompute_mode;

ALTER TABLE simulation_deck_results ALTER COLUMN recommendation_phase DROP DEFAULT;
ALTER TABLE simulation_deck_results DROP CONSTRAINT simulation_deck_results_recommendation_phase_check;
ALTER TABLE simulation_deck_results
    ALTER COLUMN recommendation_phase TYPE recommendation_phase USING recommendation_phase::recommendation_phase;
ALTER TABLE simulation_deck_results
    ALTER COLUMN recommendation_phase SET DEFAULT 'current'::recommendation_phase;

ALTER TABLE deck_recommendations ALTER COLUMN recommendation_phase DROP DEFAULT;
ALTER TABLE deck_recommendations DROP CONSTRAINT deck_recommendations_recommendation_phase_check;
ALTER TABLE deck_recommendations
    ALTER COLUMN recommendation_phase TYPE recommendation_phase USING recommendation_phase::recommendation_phase;
ALTER TABLE deck_recommendations
    ALTER COLUMN recommendation_phase SET DEFAULT 'current'::recommendation_phase;

ALTER TABLE raid_attack_components DROP CONSTRAINT raid_attack_components_component_kind_check;
ALTER TABLE raid_attack_components
    ALTER COLUMN component_kind TYPE component_kind USING component_kind::component_kind;
