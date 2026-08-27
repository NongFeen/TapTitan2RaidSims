-- Collapse raid_attack_components from one row per attack "component" (tap or
-- card hit, occasionally split further into separate Body/Armor rows for the
-- same card when a limb's cursed state flips mid-attack) down to one row per
-- attack, matching raid_attack_logs's own grain. `position` only existed to
-- keep those component rows orderable/unique; with one row per attack there's
-- nothing left for it to order. `part_damage` (the per-limb hit breakdown)
-- is dropped entirely -- nothing in the codebase ever reads it back.
--
-- A raid deck is always exactly 3 cards, so each attack always resolves to
-- (up to) 3 distinct card ids -- confirmed against every attack recorded so
-- far. Tap damage gets its own column since a tap has no card id/level.
-- card1-3 are nullable rather than NOT NULL: unlike our own simulation
-- output, this is live TT2 wire data, and a card simply not procing for a
-- given attack is a real (if so far unobserved) possibility worth tolerating
-- rather than crashing the event handler over.
ALTER TABLE raid_attack_components
    ADD COLUMN tap_damage NUMERIC(30,0),
    ADD COLUMN card1 TEXT,
    ADD COLUMN card1_level INTEGER,
    ADD COLUMN card1_damage NUMERIC(30,0),
    ADD COLUMN card2 TEXT,
    ADD COLUMN card2_level INTEGER,
    ADD COLUMN card2_damage NUMERIC(30,0),
    ADD COLUMN card3 TEXT,
    ADD COLUMN card3_level INTEGER,
    ADD COLUMN card3_damage NUMERIC(30,0);

WITH tap_totals AS (
    SELECT raid_id, player_id, attack_datetime, SUM(total_damage) AS tap_damage
    FROM raid_attack_components
    WHERE component_kind = 'tap'
    GROUP BY raid_id, player_id, attack_datetime
),
card_totals AS (
    SELECT raid_id, player_id, attack_datetime, card_id,
           MAX(card_level) AS card_level,
           SUM(total_damage) AS damage,
           MIN(position) AS first_position
    FROM raid_attack_components
    WHERE component_kind = 'card'
    GROUP BY raid_id, player_id, attack_datetime, card_id
),
ranked_cards AS (
    SELECT *, ROW_NUMBER() OVER (
        PARTITION BY raid_id, player_id, attack_datetime ORDER BY first_position
    ) AS card_rank
    FROM card_totals
),
pivoted AS (
    SELECT
        raid_id, player_id, attack_datetime,
        MAX(CASE WHEN card_rank = 1 THEN card_id END) AS card1,
        MAX(CASE WHEN card_rank = 1 THEN card_level END) AS card1_level,
        MAX(CASE WHEN card_rank = 1 THEN damage END) AS card1_damage,
        MAX(CASE WHEN card_rank = 2 THEN card_id END) AS card2,
        MAX(CASE WHEN card_rank = 2 THEN card_level END) AS card2_level,
        MAX(CASE WHEN card_rank = 2 THEN damage END) AS card2_damage,
        MAX(CASE WHEN card_rank = 3 THEN card_id END) AS card3,
        MAX(CASE WHEN card_rank = 3 THEN card_level END) AS card3_level,
        MAX(CASE WHEN card_rank = 3 THEN damage END) AS card3_damage
    FROM ranked_cards
    GROUP BY raid_id, player_id, attack_datetime
)
UPDATE raid_attack_components t
SET tap_damage = COALESCE(tt.tap_damage, 0),
    card1 = p.card1, card1_level = p.card1_level, card1_damage = p.card1_damage,
    card2 = p.card2, card2_level = p.card2_level, card2_damage = p.card2_damage,
    card3 = p.card3, card3_level = p.card3_level, card3_damage = p.card3_damage
FROM pivoted p
LEFT JOIN tap_totals tt USING (raid_id, player_id, attack_datetime)
WHERE t.raid_id = p.raid_id AND t.player_id = p.player_id AND t.attack_datetime = p.attack_datetime
  AND t.position = 0;

-- Every attack has exactly one position=0 row (its first component) to serve
-- as the survivor above; every other position for that attack is now a
-- redundant duplicate of data folded into the survivor row.
DELETE FROM raid_attack_components WHERE position <> 0;

ALTER TABLE raid_attack_components DROP CONSTRAINT raid_attack_components_pkey;
ALTER TABLE raid_attack_components
    DROP COLUMN position,
    DROP COLUMN component_kind,
    DROP COLUMN card_id,
    DROP COLUMN card_name,
    DROP COLUMN card_level,
    DROP COLUMN total_damage,
    DROP COLUMN part_damage;
ALTER TABLE raid_attack_components ALTER COLUMN tap_damage SET NOT NULL;
ALTER TABLE raid_attack_components ADD PRIMARY KEY (raid_id, player_id, attack_datetime);

-- component_kind only ever existed for this one now-dropped column.
DROP TYPE component_kind;
