use crate::models::cards::{Card, CardName};
use std::sync::OnceLock;

pub const CARD_PAIR_COUNT: usize = 44;
const CARD_PAIR_RULES_CSV: &str = include_str!("deck_pair_rules.csv");
static CARD_PAIR_RULES: OnceLock<[[bool; CARD_PAIR_COUNT]; CARD_PAIR_COUNT]> = OnceLock::new();

pub const CARD_PAIR_ORDER: [CardName; CARD_PAIR_COUNT] = [
    CardName::MoonBeam,            // 00
    CardName::Fragmentize,         // 01
    CardName::SkullBash,           // 02
    CardName::RazorWind,           // 03
    CardName::WhipOfLightning,     // 04
    CardName::ClanshipBarrage,     // 05
    CardName::PurifyingBlast,      // 06
    CardName::PsychicShackles,     // 07
    CardName::FlakShot,            // 08
    CardName::CosmicHaymaker,      // 09
    CardName::ChainOfVengeance,    // 10
    CardName::MirrorForce,         // 11
    CardName::CelestialStatic,     // 12
    CardName::GuardBreak,          // 13
    CardName::BarbedMorningstar,   // 14
    CardName::BlazingInferno,      // 15
    CardName::AcidDrench,          // 16
    CardName::DecayingStrike,      // 17
    CardName::FusionBomb,          // 18
    CardName::GrimShadow,          // 19
    CardName::ThrivingPlague,      // 20
    CardName::Radioactivity,       // 21
    CardName::RavenousSwarm,       // 22
    CardName::RuinousRain,         // 23
    CardName::CorrosiveBubbles,    // 24
    CardName::Maelstrom,           // 25
    CardName::Amplify,             // 26
    CardName::SandsOfTime,         // 27
    CardName::ElectroZap,          // 28
    CardName::CrushingInstinct,    // 29
    CardName::InsanityVoid,        // 30
    CardName::RancidGas,           // 31
    CardName::InspiringForce,      // 32
    CardName::SoulFire,            // 33
    CardName::VictoryMarch,        // 34
    CardName::PrismaticRift,       // 35
    CardName::AncestralFavor,      // 36
    CardName::GraspingVines,       // 37
    CardName::TotemOfPower,        // 38
    CardName::TeamTactics,         // 39
    CardName::SkeletalSmash,       // 40
    CardName::AstralEcho,          // 41
    CardName::RadiantKaleidoscope, // 42
    CardName::BattleDrums,         // 43
];

pub fn deck_passes_pair_table(deck: &[&Card]) -> bool {
    for left_index in 0..deck.len() {
        for right_index in (left_index + 1)..deck.len() {
            if !cards_can_pair(deck[left_index].card_id, deck[right_index].card_id) {
                return false;
            }
        }
    }

    true
}

pub fn cards_can_pair(left: CardName, right: CardName) -> bool {
    let left_index = card_pair_index(left);
    let right_index = card_pair_index(right);

    if left_index == right_index {
        return true;
    }

    card_pair_table()[left_index][right_index]
}

fn card_pair_table() -> &'static [[bool; CARD_PAIR_COUNT]; CARD_PAIR_COUNT] {
    CARD_PAIR_RULES.get_or_init(load_card_pair_table)
}

fn load_card_pair_table() -> [[bool; CARD_PAIR_COUNT]; CARD_PAIR_COUNT] {
    let mut table = [[true; CARD_PAIR_COUNT]; CARD_PAIR_COUNT];
    let mut reader = ::csv::ReaderBuilder::new()
        .trim(::csv::Trim::All)
        .from_reader(CARD_PAIR_RULES_CSV.as_bytes());

    let headers = reader
        .headers()
        .expect("deck_pair_rules.csv has invalid headers")
        .clone();
    validate_headers(&headers);

    for (row_index, record) in reader.records().enumerate() {
        let record = record.expect("deck_pair_rules.csv has invalid row data");
        validate_record_shape(row_index, &record);

        for col_index in 0..CARD_PAIR_COUNT {
            let cell = record
                .get(col_index + 1)
                .expect("deck_pair_rules.csv missing cell")
                .trim();

            if col_index < row_index {
                let allowed = parse_pair_cell(row_index, col_index, cell);
                table[row_index][col_index] = allowed;
                table[col_index][row_index] = allowed;
            } else if !cell.is_empty() && cell != "-" {
                panic!(
                    "deck_pair_rules.csv row {:02}, col {:02} must be '-' because only lower triangle cells are used",
                    row_index, col_index
                );
            }
        }
    }

    table
}

fn validate_headers(headers: &::csv::StringRecord) {
    if headers.len() != CARD_PAIR_COUNT + 1 {
        panic!(
            "deck_pair_rules.csv header must have {} columns, got {}",
            CARD_PAIR_COUNT + 1,
            headers.len()
        );
    }

    if headers.get(0) != Some("Card") {
        panic!("deck_pair_rules.csv first header must be Card");
    }

    for index in 0..CARD_PAIR_COUNT {
        let expected = card_pair_label(index);
        if headers.get(index + 1) != Some(expected.as_str()) {
            panic!(
                "deck_pair_rules.csv header column {} must be {}",
                index + 1,
                expected
            );
        }
    }
}

fn validate_record_shape(row_index: usize, record: &::csv::StringRecord) {
    if row_index >= CARD_PAIR_COUNT {
        panic!("deck_pair_rules.csv has more than {CARD_PAIR_COUNT} card rows");
    }

    if record.len() != CARD_PAIR_COUNT + 1 {
        panic!(
            "deck_pair_rules.csv row {:02} must have {} columns, got {}",
            row_index,
            CARD_PAIR_COUNT + 1,
            record.len()
        );
    }

    let expected_prefix = card_pair_label(row_index);
    let row_label = record.get(0).unwrap_or_default();
    if row_label != expected_prefix {
        panic!(
            "deck_pair_rules.csv row {:02} label must be '{}'",
            row_index, expected_prefix
        );
    }
}

fn parse_pair_cell(row_index: usize, col_index: usize, cell: &str) -> bool {
    match cell {
        "T" => true,
        "F" => false,
        _ => panic!(
            "deck_pair_rules.csv row {:02}, col {:02} must be T or F",
            row_index, col_index
        ),
    }
}

fn card_pair_index(card_name: CardName) -> usize {
    CARD_PAIR_ORDER
        .iter()
        .position(|card| *card == card_name)
        .expect("CardName missing from CARD_PAIR_ORDER")
}

fn card_pair_label(index: usize) -> String {
    format!("{:02} {:?}", index, CARD_PAIR_ORDER[index])
}

#[cfg(test)]
#[path = "../../../../tests/unit/services/taptitan/csv/deck_pair_rules_tests.rs"]
mod tests;
