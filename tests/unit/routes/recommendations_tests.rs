use super::*;

#[test]
fn accepts_every_physical_deck_count() {
    for deck_count in 1..=MAX_RECOMMENDATION_DECK_COUNT as i32 {
        assert_eq!(
            validate_deck_count(deck_count).expect("valid count should pass"),
            deck_count as usize
        );
    }
}

#[test]
fn rejects_deck_counts_outside_the_card_pool_limit() {
    assert!(validate_deck_count(0).is_err());
    assert!(validate_deck_count(-1).is_err());
    assert!(validate_deck_count(15).is_err());
}

#[test]
fn generation_request_defaults_to_six_decks() {
    let request: GenerateRecommendationRequest =
        serde_json::from_str("{}").expect("empty generation request should be valid");
    assert_eq!(request.deck_count, DEFAULT_RECOMMENDATION_DECK_COUNT as i32);
    assert!(!request.include_body_phase);
}
