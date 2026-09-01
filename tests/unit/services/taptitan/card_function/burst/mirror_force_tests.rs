use super::clan_boost_multiplier;

#[test]
fn fractional_clan_boost_maps_to_the_expected_multiplier() {
    assert!((clan_boost_multiplier(0.0) - 1.0).abs() < f64::EPSILON);
    assert!((clan_boost_multiplier(0.35) - 1.35).abs() < f64::EPSILON);
}
