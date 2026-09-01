CREATE TABLE tt2_clan_sync_state (
    singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    clan_code TEXT,
    clan_name TEXT,
    last_fetched_at TIMESTAMPTZ,
    last_player_count INTEGER NOT NULL DEFAULT 0 CHECK (last_player_count >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO tt2_clan_sync_state (singleton)
VALUES (TRUE)
ON CONFLICT (singleton) DO NOTHING;
