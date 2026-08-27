use super::*;
use crate::models::cards::CardName;
use strum::IntoEnumIterator;

fn test_key() -> String {
    STANDARD.encode([7_u8; 32])
}

#[test]
fn token_encryption_round_trip_uses_unique_nonces() {
    let cipher = TokenCipher::from_base64(&test_key()).unwrap();
    let (first, first_nonce) = cipher.encrypt("player-token").unwrap();
    let (second, second_nonce) = cipher.encrypt("player-token").unwrap();
    assert_ne!(first_nonce, second_nonce);
    assert_ne!(first, second);
    assert_eq!(
        cipher.decrypt(&first, &first_nonce).unwrap(),
        "player-token"
    );
}

#[test]
fn wrong_key_cannot_decrypt_token() {
    let cipher = TokenCipher::from_base64(&test_key()).unwrap();
    let other = TokenCipher::from_base64(&STANDARD.encode([8_u8; 32])).unwrap();
    let (encrypted, nonce) = cipher.encrypt("player-token").unwrap();
    assert!(other.decrypt(&encrypted, &nonce).is_err());
}

#[test]
fn raid_subscription_uses_player_tokens_array() {
    let body = RaidSubscriptionRequest {
        player_tokens: vec!["player-token-here"],
    };
    assert_eq!(
        serde_json::to_value(body).unwrap(),
        serde_json::json!({ "player_tokens": ["player-token-here"] })
    );
}

#[test]
fn public_player_data_maps_all_supported_raid_fields() {
    let cards = CardName::iter()
        .map(|card| PublicCard {
            level: 10,
            skill_name: card.id().to_string(),
        })
        .collect();
    let data = PublicPlayerData {
        player_code: "eqw9d34".to_string(),
        player_raid_level: "1395".to_string(),
        boosted_cards: vec![PublicBoostedCard {
            boost_level: 47,
            skill_name: "CosmicBarb".to_string(),
        }],
        raid_research_tree: HashMap::from([
            ("HeadDamage".to_string(), serde_json::json!(0.25)),
            ("ChestDamage".to_string(), serde_json::json!(0.17)),
            ("LimbDamage".to_string(), serde_json::json!(0.17)),
        ]),
        raid_research_bonuses: HashMap::from([
            ("RaidBaseDamage".to_string(), serde_json::json!(156)),
            ("BurstBaseDamage".to_string(), serde_json::json!(150)),
        ]),
        gemstone_research_tree_raid_bonuses: HashMap::from([
            ("Enemy1BurstBaseDamage".to_string(), serde_json::json!(10)),
            ("TorsoBaseDamage".to_string(), serde_json::json!(5)),
        ]),
        equipment_set: vec!["Jade", "Jukk", "Airforce", "Dancer", "RoseAnniversary"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        cards,
    };
    let cleaned = data.into_raid_data(1.5).unwrap();
    assert_eq!(cleaned.player_raid_level, 1395);
    assert_eq!(cleaned.player_raid_base_damage, 1496);
    assert_eq!(cleaned.card_list.len(), 44);
    assert_eq!(cleaned.title, 1.5);
    assert!(cleaned.raid_set.jade_anniversary);
    assert!(cleaned.raid_set.jukk_juggernaut);
    assert!(cleaned.raid_set.airforce_ace);
    assert!(cleaned.raid_set.dancer_venom);
    assert!(cleaned.raid_set.rose_anniversary);
    let boosted = cleaned
        .card_list
        .iter()
        .find(|card| card.card_id == CardName::ElectroZap)
        .unwrap();
    assert_eq!(boosted.level, 47);
    assert_eq!(cleaned.raid_card_research.base_damage, 156);
    assert_eq!(cleaned.gem_stone_research.burst_lojak_damage, 10);
}

#[test]
fn clan_data_equipment_set_names_are_normalized_to_match_manual_import() {
    let cards = CardName::iter()
        .map(|card| PublicCard {
            level: 10,
            skill_name: card.id().to_string(),
        })
        .collect();
    let data = PublicClanPlayerData {
        name: "Test Player".to_string(),
        player_code: "eqw9d34".to_string(),
        player_raid_level: serde_json::json!("1395"),
        boosted_cards: Vec::new(),
        raid_research_tree: HashMap::new(),
        raid_research_bonuses: HashMap::new(),
        gemstone_research_tree_raid_bonuses: HashMap::new(),
        // /raid/clan_data reports these under different names than the
        // manual-import export format uses ("Scorpion" for Dancer Venom,
        // "Runestone" for Jukk Juggernaut) and never reports Rose
        // Anniversary at all, even though every player has it.
        equipment_set: vec!["Jade", "Runestone", "Airforce", "Scorpion"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        cards,
    };
    let cleaned = data.into_raid_data(1.0).unwrap();
    assert!(cleaned.raid_set.jade_anniversary);
    assert!(cleaned.raid_set.jukk_juggernaut);
    assert!(cleaned.raid_set.airforce_ace);
    assert!(cleaned.raid_set.dancer_venom);
    assert!(cleaned.raid_set.rose_anniversary);
}
