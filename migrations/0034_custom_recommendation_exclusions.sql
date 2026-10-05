-- Custom (excluded-card) recommendations are now saved as rows too. Each row
-- records the exclusion mask it was built with; 0 means "no exclusions", which
-- is the normal recommendation every existing row already is.
ALTER TABLE deck_recommendations
    ADD COLUMN excluded_cards_mask BIGINT NOT NULL DEFAULT 0;

ALTER TABLE deck_recommendations
    DROP CONSTRAINT IF EXISTS deck_recommendations_job_count_required_cards_phase_key;

ALTER TABLE deck_recommendations
    ADD CONSTRAINT deck_recommendations_job_count_required_cards_phase_key UNIQUE (
        simulation_job_id,
        deck_count,
        must_include_mirror_force,
        must_include_team_tactics,
        recommendation_phase,
        excluded_cards_mask
    );
