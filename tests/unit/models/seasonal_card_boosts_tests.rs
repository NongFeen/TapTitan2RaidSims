use super::*;

#[test]
fn parses_card_boosts_from_the_env_format() {
    let boosts = parse_seasonal_card_boosts(" BurstBoost:10, Fragmentize : 20 ,,MirrorForce:20 ").unwrap();
    assert_eq!(boosts.len(), 3);
    assert_eq!(boosts[&CardName::from_str("BurstBoost").unwrap()], 10);
    assert_eq!(boosts[&CardName::Fragmentize], 20);
    assert_eq!(boosts[&CardName::MirrorForce], 20);
}

#[test]
fn empty_value_means_no_boosts() {
    assert!(parse_seasonal_card_boosts("").unwrap().is_empty());
}

#[test]
fn rejects_unknown_cards_and_bad_entries() {
    assert!(parse_seasonal_card_boosts("NotACard:10").unwrap_err().contains("unknown card ID"));
    assert!(parse_seasonal_card_boosts("MirrorForce").unwrap_err().contains("CardId:levels"));
    assert!(parse_seasonal_card_boosts("MirrorForce:ten").unwrap_err().contains("whole number"));
}
