-- Cards a player has raised with the raid crystal, as reported by TT2's
-- `boosted_cards`. `player_cards` keeps storing each card's level after this
-- boost (what the simulations use); this table keeps the player's own level
-- and the boosted level so the frontend can show the boost separately.
CREATE TABLE player_raid_crystal_boosts (
    player_id TEXT NOT NULL REFERENCES players(player_id) ON DELETE CASCADE,
    card_id card_name NOT NULL,
    base_level INTEGER NOT NULL CHECK (base_level BETWEEN 0 AND 65535),
    boost_level INTEGER NOT NULL CHECK (boost_level BETWEEN 0 AND 65535),
    PRIMARY KEY (player_id, card_id)
);
