UPDATE raid_cycle_state
SET
    started_at = started_at + INTERVAL '5 hours',
    raid_started_at = raid_started_at + INTERVAL '5 hours',
    next_reset_at = next_reset_at + INTERVAL '5 hours',
    updated_at = NOW();
