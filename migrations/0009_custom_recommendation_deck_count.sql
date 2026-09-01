ALTER TABLE deck_recommendations
DROP CONSTRAINT IF EXISTS deck_recommendations_deck_count_check;

ALTER TABLE deck_recommendations
ADD CONSTRAINT deck_recommendations_deck_count_range_check
CHECK (deck_count BETWEEN 1 AND 14);
