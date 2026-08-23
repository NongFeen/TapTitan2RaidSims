UPDATE raid_cycle_state
SET
    next_reset_at = CASE
        WHEN started_at IS NOT NULL
         AND started_at = raid_started_at
         AND next_reset_at = raid_started_at + INTERVAL '12 hours'
         AND team_tactics_morale_boost = 0
         AND mirror_force_boost = 0
        THEN started_at - INTERVAL '5 hours'
        ELSE next_reset_at - INTERVAL '5 hours'
    END,
    started_at = started_at - INTERVAL '5 hours',
    raid_started_at = raid_started_at - INTERVAL '5 hours',
    updated_at = NOW();
