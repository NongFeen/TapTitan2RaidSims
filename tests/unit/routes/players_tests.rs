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

#[test]
fn raw_export_keeps_raid_crystal_boosts_with_the_players_own_level() {
    use std::str::FromStr;
    let raw: crate::models::player_data::PlayerData =
        serde_json::from_str(include_str!("../../../playerDataSample.json"))
            .expect("raw player sample should deserialize");
    let stats = crate::services::taptitan::player_service::clean_data(&raw);
    let weaken = crate::models::cards::CardName::from_str("Weaken").unwrap();

    let boost = stats
        .raid_crystal_boosts
        .iter()
        .find(|boost| boost.card_id == weaken)
        .expect("Weaken is crystal-boosted in the sample");
    assert_eq!((boost.base_level, boost.boost_level), (33, 48));
    let card = stats.card_list.iter().find(|card| card.card_id == weaken).unwrap();
    assert_eq!(card.level, 48, "card_list keeps the boosted level the sims use");
    assert_eq!(stats.raid_crystal_boosts.len(), 8);
}
