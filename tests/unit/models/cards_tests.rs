#[test]
fn card_name_declaration_order_is_pinned() {
    // `simulation_deck_results.card_mask` encodes a deck as a bitmask where
    // bit N = the Nth `CardName` in this enum's declaration order (see
    // `recommendation.rs`). Reordering or inserting a variant in the middle
    // of `CardName` would silently change the meaning of every previously
    // stored `card_mask` value. New cards must only ever be appended.
    use strum::IntoEnumIterator;

    let names: Vec<super::CardName> = super::CardName::iter().collect();
    assert_eq!(names.len(), 44, "CardName variant count changed");
    assert_eq!(names[0], super::CardName::MoonBeam);
    assert_eq!(names[14], super::CardName::BarbedMorningstar);
    assert_eq!(names[15], super::CardName::BlazingInferno);
    assert_eq!(names[28], super::CardName::ElectroZap);
    assert_eq!(names[29], super::CardName::CrushingInstinct);
    assert_eq!(names[43], super::CardName::BattleDrums);
}

#[test]
fn missing_enabled_flag_defaults_to_true_and_false_is_persisted() {
    let card: super::Card = serde_json::from_value(serde_json::json!({
        "card_id": "MoonBeam",
        "cardtype": "Burst",
        "level": 10
    }))
    .expect("legacy stored card should deserialize");
    assert!(card.enabled);

    let mut disabled = card;
    disabled.enabled = false;
    let stored = serde_json::to_value(disabled).expect("card should serialize");
    assert_eq!(stored["enabled"], false);
}
