-- Promote total_attack_patterns to a real column (needed by incremental
-- recompute to rebuild SimRunResult's aggregate without deserializing the
-- full stored result), backfilled from the currently-stored full JSON.
ALTER TABLE simulation_deck_results ADD COLUMN total_attack_patterns INTEGER;
UPDATE simulation_deck_results
SET total_attack_patterns = COALESCE((result->>'total_attack_patterns')::INTEGER, 0);
ALTER TABLE simulation_deck_results ALTER COLUMN total_attack_patterns SET NOT NULL;

-- Narrow `result` down to only what nothing else already has: deck/deck_names
-- are fully reconstructible from card_mask, the deck-level average_damage
-- duplicates this row's own average_damage column, every _display string is
-- derived formatting the frontend never reads (it formats raw numbers
-- itself), card_name is derivable from card, simulation_phase/dependency_part_mask
-- already have their own real columns, and total_attack_patterns just moved
-- to one above.
UPDATE simulation_deck_results
SET result = jsonb_build_object(
    'pattern', result->'best_pattern'->>'pattern',
    'lowest_round_damage', (result->'best_pattern'->>'lowest_round_damage')::BIGINT,
    'highest_round_damage', (result->'best_pattern'->>'highest_round_damage')::BIGINT,
    'card_damage', (
        SELECT COALESCE(jsonb_agg(jsonb_build_object(
            'card', elem->>'card',
            'average_damage', (elem->>'average_damage')::BIGINT
        )), '[]'::jsonb)
        FROM jsonb_array_elements(result->'best_pattern'->'card_damage') AS elem
    )
);

-- cards duplicated what card_mask (a real column) already encodes.
ALTER TABLE simulation_deck_results DROP COLUMN cards;
