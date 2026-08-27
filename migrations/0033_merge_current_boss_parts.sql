-- current_boss and current_boss_parts are always loaded/written together as
-- a single "current sims boss" snapshot -- nothing in the codebase ever
-- queries one without the other. Since current_boss is a singleton (exactly
-- one row) and current_boss_parts is a small, fixed set (exactly 8 body
-- parts), fold the 8 part rows into 8-per-field columns on current_boss,
-- same shape as the player_cards flattening. radioactivity_afflicted_seconds
-- is dropped entirely rather than carried over: it's a per-simulation-run
-- tick accumulator (see Boss::update_persistent_affliction_timers, ticked at
-- 1/20s per simulated combat tick) that never survives past the single
-- simulation that produced it -- every write to the persisted boss either
-- hardcodes it to 0.0 (boss_from_raid_snapshot) or just copies the existing
-- (always-0) stored value forward (preserve_current_boss_values), so it has
-- never actually held a meaningful value in current_boss_parts.
ALTER TABLE current_boss
    ADD COLUMN head_part_state part_state,
    ADD COLUMN head_max_armor BIGINT,
    ADD COLUMN head_max_health BIGINT,
    ADD COLUMN head_current_armor BIGINT,
    ADD COLUMN head_current_health BIGINT,
    ADD COLUMN head_is_attackable BOOLEAN,
    ADD COLUMN torso_part_state part_state,
    ADD COLUMN torso_max_armor BIGINT,
    ADD COLUMN torso_max_health BIGINT,
    ADD COLUMN torso_current_armor BIGINT,
    ADD COLUMN torso_current_health BIGINT,
    ADD COLUMN torso_is_attackable BOOLEAN,
    ADD COLUMN left_shoulder_part_state part_state,
    ADD COLUMN left_shoulder_max_armor BIGINT,
    ADD COLUMN left_shoulder_max_health BIGINT,
    ADD COLUMN left_shoulder_current_armor BIGINT,
    ADD COLUMN left_shoulder_current_health BIGINT,
    ADD COLUMN left_shoulder_is_attackable BOOLEAN,
    ADD COLUMN right_shoulder_part_state part_state,
    ADD COLUMN right_shoulder_max_armor BIGINT,
    ADD COLUMN right_shoulder_max_health BIGINT,
    ADD COLUMN right_shoulder_current_armor BIGINT,
    ADD COLUMN right_shoulder_current_health BIGINT,
    ADD COLUMN right_shoulder_is_attackable BOOLEAN,
    ADD COLUMN left_hand_part_state part_state,
    ADD COLUMN left_hand_max_armor BIGINT,
    ADD COLUMN left_hand_max_health BIGINT,
    ADD COLUMN left_hand_current_armor BIGINT,
    ADD COLUMN left_hand_current_health BIGINT,
    ADD COLUMN left_hand_is_attackable BOOLEAN,
    ADD COLUMN right_hand_part_state part_state,
    ADD COLUMN right_hand_max_armor BIGINT,
    ADD COLUMN right_hand_max_health BIGINT,
    ADD COLUMN right_hand_current_armor BIGINT,
    ADD COLUMN right_hand_current_health BIGINT,
    ADD COLUMN right_hand_is_attackable BOOLEAN,
    ADD COLUMN left_leg_part_state part_state,
    ADD COLUMN left_leg_max_armor BIGINT,
    ADD COLUMN left_leg_max_health BIGINT,
    ADD COLUMN left_leg_current_armor BIGINT,
    ADD COLUMN left_leg_current_health BIGINT,
    ADD COLUMN left_leg_is_attackable BOOLEAN,
    ADD COLUMN right_leg_part_state part_state,
    ADD COLUMN right_leg_max_armor BIGINT,
    ADD COLUMN right_leg_max_health BIGINT,
    ADD COLUMN right_leg_current_armor BIGINT,
    ADD COLUMN right_leg_current_health BIGINT,
    ADD COLUMN right_leg_is_attackable BOOLEAN;

UPDATE current_boss cb
SET head_part_state = p.part_state,
    head_max_armor = p.max_armor,
    head_max_health = p.max_health,
    head_current_armor = p.current_armor,
    head_current_health = p.current_health,
    head_is_attackable = p.is_attackable
FROM current_boss_parts p WHERE p.part_name = 'Head';

UPDATE current_boss cb
SET torso_part_state = p.part_state,
    torso_max_armor = p.max_armor,
    torso_max_health = p.max_health,
    torso_current_armor = p.current_armor,
    torso_current_health = p.current_health,
    torso_is_attackable = p.is_attackable
FROM current_boss_parts p WHERE p.part_name = 'Torso';

UPDATE current_boss cb
SET left_shoulder_part_state = p.part_state,
    left_shoulder_max_armor = p.max_armor,
    left_shoulder_max_health = p.max_health,
    left_shoulder_current_armor = p.current_armor,
    left_shoulder_current_health = p.current_health,
    left_shoulder_is_attackable = p.is_attackable
FROM current_boss_parts p WHERE p.part_name = 'LeftShoulder';

UPDATE current_boss cb
SET right_shoulder_part_state = p.part_state,
    right_shoulder_max_armor = p.max_armor,
    right_shoulder_max_health = p.max_health,
    right_shoulder_current_armor = p.current_armor,
    right_shoulder_current_health = p.current_health,
    right_shoulder_is_attackable = p.is_attackable
FROM current_boss_parts p WHERE p.part_name = 'RightShoulder';

UPDATE current_boss cb
SET left_hand_part_state = p.part_state,
    left_hand_max_armor = p.max_armor,
    left_hand_max_health = p.max_health,
    left_hand_current_armor = p.current_armor,
    left_hand_current_health = p.current_health,
    left_hand_is_attackable = p.is_attackable
FROM current_boss_parts p WHERE p.part_name = 'LeftHand';

UPDATE current_boss cb
SET right_hand_part_state = p.part_state,
    right_hand_max_armor = p.max_armor,
    right_hand_max_health = p.max_health,
    right_hand_current_armor = p.current_armor,
    right_hand_current_health = p.current_health,
    right_hand_is_attackable = p.is_attackable
FROM current_boss_parts p WHERE p.part_name = 'RightHand';

UPDATE current_boss cb
SET left_leg_part_state = p.part_state,
    left_leg_max_armor = p.max_armor,
    left_leg_max_health = p.max_health,
    left_leg_current_armor = p.current_armor,
    left_leg_current_health = p.current_health,
    left_leg_is_attackable = p.is_attackable
FROM current_boss_parts p WHERE p.part_name = 'LeftLeg';

UPDATE current_boss cb
SET right_leg_part_state = p.part_state,
    right_leg_max_armor = p.max_armor,
    right_leg_max_health = p.max_health,
    right_leg_current_armor = p.current_armor,
    right_leg_current_health = p.current_health,
    right_leg_is_attackable = p.is_attackable
FROM current_boss_parts p WHERE p.part_name = 'RightLeg';

DROP TABLE current_boss_parts;

ALTER TABLE current_boss
    ALTER COLUMN head_part_state SET NOT NULL,
    ALTER COLUMN head_max_armor SET NOT NULL,
    ALTER COLUMN head_max_health SET NOT NULL,
    ALTER COLUMN head_current_armor SET NOT NULL,
    ALTER COLUMN head_current_health SET NOT NULL,
    ALTER COLUMN head_is_attackable SET NOT NULL,
    ALTER COLUMN torso_part_state SET NOT NULL,
    ALTER COLUMN torso_max_armor SET NOT NULL,
    ALTER COLUMN torso_max_health SET NOT NULL,
    ALTER COLUMN torso_current_armor SET NOT NULL,
    ALTER COLUMN torso_current_health SET NOT NULL,
    ALTER COLUMN torso_is_attackable SET NOT NULL,
    ALTER COLUMN left_shoulder_part_state SET NOT NULL,
    ALTER COLUMN left_shoulder_max_armor SET NOT NULL,
    ALTER COLUMN left_shoulder_max_health SET NOT NULL,
    ALTER COLUMN left_shoulder_current_armor SET NOT NULL,
    ALTER COLUMN left_shoulder_current_health SET NOT NULL,
    ALTER COLUMN left_shoulder_is_attackable SET NOT NULL,
    ALTER COLUMN right_shoulder_part_state SET NOT NULL,
    ALTER COLUMN right_shoulder_max_armor SET NOT NULL,
    ALTER COLUMN right_shoulder_max_health SET NOT NULL,
    ALTER COLUMN right_shoulder_current_armor SET NOT NULL,
    ALTER COLUMN right_shoulder_current_health SET NOT NULL,
    ALTER COLUMN right_shoulder_is_attackable SET NOT NULL,
    ALTER COLUMN left_hand_part_state SET NOT NULL,
    ALTER COLUMN left_hand_max_armor SET NOT NULL,
    ALTER COLUMN left_hand_max_health SET NOT NULL,
    ALTER COLUMN left_hand_current_armor SET NOT NULL,
    ALTER COLUMN left_hand_current_health SET NOT NULL,
    ALTER COLUMN left_hand_is_attackable SET NOT NULL,
    ALTER COLUMN right_hand_part_state SET NOT NULL,
    ALTER COLUMN right_hand_max_armor SET NOT NULL,
    ALTER COLUMN right_hand_max_health SET NOT NULL,
    ALTER COLUMN right_hand_current_armor SET NOT NULL,
    ALTER COLUMN right_hand_current_health SET NOT NULL,
    ALTER COLUMN right_hand_is_attackable SET NOT NULL,
    ALTER COLUMN left_leg_part_state SET NOT NULL,
    ALTER COLUMN left_leg_max_armor SET NOT NULL,
    ALTER COLUMN left_leg_max_health SET NOT NULL,
    ALTER COLUMN left_leg_current_armor SET NOT NULL,
    ALTER COLUMN left_leg_current_health SET NOT NULL,
    ALTER COLUMN left_leg_is_attackable SET NOT NULL,
    ALTER COLUMN right_leg_part_state SET NOT NULL,
    ALTER COLUMN right_leg_max_armor SET NOT NULL,
    ALTER COLUMN right_leg_max_health SET NOT NULL,
    ALTER COLUMN right_leg_current_armor SET NOT NULL,
    ALTER COLUMN right_leg_current_health SET NOT NULL,
    ALTER COLUMN right_leg_is_attackable SET NOT NULL;

ALTER TABLE current_boss ADD CONSTRAINT current_boss_part_ranges_check CHECK (
    head_max_armor >= 0 AND head_max_health >= 0 AND head_current_armor >= 0 AND head_current_health >= 0 AND
    torso_max_armor >= 0 AND torso_max_health >= 0 AND torso_current_armor >= 0 AND torso_current_health >= 0 AND
    left_shoulder_max_armor >= 0 AND left_shoulder_max_health >= 0 AND left_shoulder_current_armor >= 0 AND left_shoulder_current_health >= 0 AND
    right_shoulder_max_armor >= 0 AND right_shoulder_max_health >= 0 AND right_shoulder_current_armor >= 0 AND right_shoulder_current_health >= 0 AND
    left_hand_max_armor >= 0 AND left_hand_max_health >= 0 AND left_hand_current_armor >= 0 AND left_hand_current_health >= 0 AND
    right_hand_max_armor >= 0 AND right_hand_max_health >= 0 AND right_hand_current_armor >= 0 AND right_hand_current_health >= 0 AND
    left_leg_max_armor >= 0 AND left_leg_max_health >= 0 AND left_leg_current_armor >= 0 AND left_leg_current_health >= 0 AND
    right_leg_max_armor >= 0 AND right_leg_max_health >= 0 AND right_leg_current_armor >= 0 AND right_leg_current_health >= 0
);
