ALTER TABLE players
    ADD COLUMN player_token_ciphertext BYTEA,
    ADD COLUMN player_token_nonce BYTEA,
    ADD COLUMN tt2_last_fetched_at TIMESTAMPTZ,
    ADD COLUMN tt2_token_status TEXT NOT NULL DEFAULT 'missing'
        CHECK (tt2_token_status IN ('missing', 'configured', 'invalid'));

ALTER TABLE players
    ADD CONSTRAINT players_player_token_pair_check CHECK (
        (player_token_ciphertext IS NULL AND player_token_nonce IS NULL)
        OR
        (player_token_ciphertext IS NOT NULL AND player_token_nonce IS NOT NULL)
    );
