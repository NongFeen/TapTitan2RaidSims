ALTER TABLE simulation_deck_results
ADD COLUMN recommendation_phase TEXT NOT NULL DEFAULT 'current';

ALTER TABLE deck_recommendations
ADD COLUMN recommendation_phase TEXT NOT NULL DEFAULT 'current';

UPDATE simulation_deck_results AS d
SET recommendation_phase = 'void'
FROM simulation_jobs AS j
WHERE j.id = d.simulation_job_id
  AND COALESCE((j.payload->>'include_body_phase')::BOOLEAN, FALSE);

UPDATE deck_recommendations AS r
SET recommendation_phase = 'void'
FROM simulation_jobs AS j
WHERE j.id = r.simulation_job_id
  AND COALESCE((j.payload->>'include_body_phase')::BOOLEAN, FALSE);

ALTER TABLE simulation_deck_results
DROP CONSTRAINT simulation_deck_results_simulation_job_id_card_mask_key;

ALTER TABLE simulation_deck_results
ADD CONSTRAINT simulation_deck_results_job_mask_phase_key UNIQUE (
    simulation_job_id,
    card_mask,
    recommendation_phase
);

ALTER TABLE deck_recommendations
DROP CONSTRAINT deck_recommendations_job_count_required_cards_key;

ALTER TABLE deck_recommendations
ADD CONSTRAINT deck_recommendations_job_count_required_cards_phase_key UNIQUE (
    simulation_job_id,
    deck_count,
    must_include_mirror_force,
    must_include_team_tactics,
    recommendation_phase
);

ALTER TABLE simulation_deck_results
ADD CONSTRAINT simulation_deck_results_recommendation_phase_check
CHECK (recommendation_phase IN ('current', 'void'));

ALTER TABLE deck_recommendations
ADD CONSTRAINT deck_recommendations_recommendation_phase_check
CHECK (recommendation_phase IN ('current', 'void'));
