use super::*;

#[test]
fn configured_seasonal_boosts_match_the_active_season() {
    assert_eq!(seasonal_level_boost(CardName::BlazingInferno), 20);
    assert_eq!(seasonal_level_boost(CardName::CorrosiveBubbles), 15);
    assert_eq!(seasonal_level_boost(CardName::RancidGas), 10);
    assert_eq!(seasonal_level_boost(CardName::MoonBeam), 0);
    assert_eq!(seasonal_effective_level(CardName::CorrosiveBubbles, 47), 62);
}
