-- Replace the last remaining JSONB blob on simulation_deck_results with real
-- columns. The `result` column (post-0027) only ever held a fixed 3-card
-- pattern shape -- {pattern, lowest_round_damage, highest_round_damage,
-- card_damage: [3 x {card, average_damage}]} -- so there's no reason to pay
-- for JSONB parsing on every read when the shape is exactly one row wide.
--
-- card1/2/3 are TEXT, not the `card_name` Postgres enum: CardName's
-- serde/wire representation (e.g. "Haymaker", what the frontend and
-- card_mask reconstruction already use everywhere) differs by design from
-- the enum's Rust-variant labels (e.g. "CosmicHaymaker" -- see cards.rs
-- CardName::id()). Storing the wire alias directly keeps this column
-- consistent with every other card-id string in the API instead of
-- introducing a second, mismatched representation.
ALTER TABLE simulation_deck_results
    ADD COLUMN pattern TEXT,
    ADD COLUMN card1 TEXT,
    ADD COLUMN card2 TEXT,
    ADD COLUMN card3 TEXT,
    ADD COLUMN card1_damage BIGINT,
    ADD COLUMN card2_damage BIGINT,
    ADD COLUMN card3_damage BIGINT,
    ADD COLUMN deck_lowest_damage BIGINT,
    ADD COLUMN deck_highest_damage BIGINT;

UPDATE simulation_deck_results
SET pattern = result->>'pattern',
    deck_lowest_damage = (result->>'lowest_round_damage')::BIGINT,
    deck_highest_damage = (result->>'highest_round_damage')::BIGINT,
    card1 = result->'card_damage'->0->>'card',
    card2 = result->'card_damage'->1->>'card',
    card3 = result->'card_damage'->2->>'card',
    card1_damage = (result->'card_damage'->0->>'average_damage')::BIGINT,
    card2_damage = (result->'card_damage'->1->>'average_damage')::BIGINT,
    card3_damage = (result->'card_damage'->2->>'average_damage')::BIGINT;

ALTER TABLE simulation_deck_results
    ALTER COLUMN pattern SET NOT NULL,
    ALTER COLUMN card1 SET NOT NULL,
    ALTER COLUMN card2 SET NOT NULL,
    ALTER COLUMN card3 SET NOT NULL,
    ALTER COLUMN card1_damage SET NOT NULL,
    ALTER COLUMN card2_damage SET NOT NULL,
    ALTER COLUMN card3_damage SET NOT NULL,
    ALTER COLUMN deck_lowest_damage SET NOT NULL,
    ALTER COLUMN deck_highest_damage SET NOT NULL;

-- `average_damage` (already a real column) is the deck's average damage --
-- identical to what was `result->'average_damage'` before 0027 dropped it as
-- a duplicate. No separate deck_average_damage column needed.
ALTER TABLE simulation_deck_results DROP COLUMN result;
