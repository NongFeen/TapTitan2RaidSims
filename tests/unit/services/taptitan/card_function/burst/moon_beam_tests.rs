use super::bonus_multiplier;
use crate::models::boss::BossPartName;

#[test]
fn favored_parts_get_their_own_bonus_and_other_parts_get_none() {
    // Torso uses bonus_c ("MoonBeamChestMult" in the card data).
    assert_eq!(bonus_multiplier(BossPartName::Torso, Some(1.5), Some(1.5)), 1.5);

    // The 4 arm parts use bonus_d ("MoonBeamArmMult"), not bonus_c.
    for part in [
        BossPartName::LeftHand,
        BossPartName::RightHand,
        BossPartName::LeftShoulder,
        BossPartName::RightShoulder,
    ] {
        assert_eq!(bonus_multiplier(part, Some(2.0), Some(1.5)), 1.5);
    }

    // Head and legs are not MoonBeam's favored parts: no bonus at all, even
    // though SkullBash (a mirror-image card with an identical base damage
    // curve) does get a bonus on Head. Regression test for a bug where this
    // previously defaulted to 1.5 for every unfavored part, making MoonBeam
    // deal identical damage to SkullBash on Head by coincidence.
    for part in [BossPartName::Head, BossPartName::LeftLeg, BossPartName::RightLeg] {
        assert_eq!(bonus_multiplier(part, Some(1.5), Some(1.5)), 1.0);
    }
}
