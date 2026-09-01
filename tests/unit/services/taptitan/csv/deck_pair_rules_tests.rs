use super::*;

#[test]
fn deck_pair_csv_loads() {
    let table = card_pair_table();
    assert!(table[0][1]);
    assert!(table[CARD_PAIR_COUNT - 1][0]);
}
