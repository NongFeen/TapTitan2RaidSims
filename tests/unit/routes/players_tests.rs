use super::*;

#[test]
fn accepts_raw_player_export_for_stats_update() {
    let request: UpdatePlayerStatsRequest =
        serde_json::from_str(include_str!("../../../playerDataSample.json"))
            .expect("raw player sample should deserialize");
    let UpdatePlayerStatsRequest::Raw(raw) = request else {
        panic!("raw sample was mistaken for cleaned stats");
    };
    let cleaned = clean_data(&raw);
    validate_stats(&cleaned).expect("converted raw player stats should be valid");

    let level = |card_name| {
        cleaned
            .card_list
            .iter()
            .find(|card| card.card_id == card_name)
            .map(|card| card.level)
            .expect("sample card should exist")
    };
    assert_eq!(level(crate::models::cards::CardName::GuardBreak), 48);
    assert_eq!(level(crate::models::cards::CardName::BarbedMorningstar), 48);
    assert_eq!(level(crate::models::cards::CardName::ElectroZap), 47);
    assert_eq!(level(crate::models::cards::CardName::CorrosiveBubbles), 47);
    assert_eq!(level(crate::models::cards::CardName::BattleDrums), 49);
    assert_eq!(level(crate::models::cards::CardName::CrushingInstinct), 49);
    assert_eq!(level(crate::models::cards::CardName::SoulFire), 49);
    assert_eq!(level(crate::models::cards::CardName::RancidGas), 49);
    assert_eq!(level(crate::models::cards::CardName::MoonBeam), 48);
}

#[test]
fn accepts_older_raw_export_without_boosted_cards() {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("../../../playerDataSample.json"))
            .expect("raw player sample should be valid JSON");
    value
        .as_object_mut()
        .expect("raw player sample should be an object")
        .remove("boostedCards");

    let raw: crate::models::player_data::PlayerData =
        serde_json::from_value(value).expect("older raw export should still deserialize");
    assert!(raw.boosted_cards.is_empty());
}

#[test]
fn tt2_refresh_preserves_existing_card_preferences() {
    let request: UpdatePlayerStatsRequest =
        serde_json::from_str(include_str!("../../../playerDataSample.json"))
            .expect("raw player sample should deserialize");
    let UpdatePlayerStatsRequest::Raw(raw) = request else {
        panic!("raw sample was mistaken for cleaned stats");
    };
    let mut existing = clean_data(&raw);
    let mut refreshed = existing.clone();
    let disabled_card = existing.card_list[0].card_id;
    existing.card_list[0].enabled = false;

    preserve_card_preferences(&mut refreshed, &existing);

    assert!(
        !refreshed
            .card_list
            .iter()
            .find(|card| card.card_id == disabled_card)
            .expect("disabled card should still exist")
            .enabled
    );
    assert!(
        refreshed
            .card_list
            .iter()
            .filter(|card| card.card_id != disabled_card)
            .all(|card| card.enabled)
    );
}
