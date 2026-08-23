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
