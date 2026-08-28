use super::*;
use crate::models::player_data::PlayerData;
use crate::services::taptitan::player_service::clean_data;
use std::collections::HashMap;
use std::path::Path;

/// Proves the deterministic single-tap machinery (`SimRng`, the tap-once
/// wrapper, and the loader plumbing above) actually runs end-to-end, using
/// an existing non-golden sample payload -- not an assertion against a real
/// measured number, just that the pipeline produces sensible, nonzero
/// output. `cases.toml` carries the real, verified comparisons.
#[test]
fn deterministic_single_tap_simulation_runs_end_to_end() {
    let mut payload: SimPayLoad =
        serde_json::from_str(include_str!("../../../../../playersim_data_sample.json"))
            .expect("sample payload should deserialize as a SimPayLoad");
    // Narrow to one target so there's no first-tap pattern ambiguity.
    payload.attackable_part = vec![BossPartName::Head];

    let result = SimService::run_deterministic_single_tap_simulation(payload, 1)
        .expect("sample deck should produce a valid attack pattern");
    let pattern = result
        .best_pattern
        .expect("single-tap simulation should produce a pattern result");

    assert!(
        pattern.average_damage > 0,
        "a guaranteed-proc single tap against a fresh boss should deal nonzero damage"
    );
}

/// Deserializes `tests/fixtures/sim_to_real/cases.toml` -- see that file for
/// the field-by-field format.
#[derive(Debug, serde::Deserialize)]
struct CasesFile {
    #[serde(default, rename = "case")]
    cases: Vec<Case>,
}

#[derive(Debug, serde::Deserialize)]
struct Case {
    name: String,
    /// Path (relative to `FIXTURES_DIR`) to a `{ player_raw_data, boss_data }`
    /// fixture -- `player_raw_data` is a raw TT2 player export (the same
    /// shape `clean_data` normally cleans on import), `boss_data` is a
    /// direct `Boss` JSON snapshot.
    payload: String,
    deck: Vec<CardName>,
    attackable_part: Vec<BossPartName>,
    #[serde(default)]
    mirror_force_boost: f64,
    /// Real damage measured in-game from the player's own tap alone
    /// (excludes every card's contribution).
    expected_tap_damage: u64,
    /// Real per-card damage measured in-game, keyed by card id. A card with
    /// no measurable contribution (e.g. a Support card, or one that can't
    /// proc in this scenario) should still be listed with `0`.
    expected_card_damage: HashMap<CardName, u64>,
}

/// A raw player export + a `Boss` snapshot, as captured by hand from a real
/// TT2 attack -- see `cases.toml`'s doc comment for how to produce one.
/// `title` is `PlayerRaidData::title` (the player's title, an additive bonus
/// to all raid damage -- see `damage_cache.rs`'s `raid_all_mult`) directly,
/// not part of the raw export `clean_data` reads, so it's supplied
/// separately here rather than defaulting to `clean_data`'s placeholder 0.0.
#[derive(Debug, serde::Deserialize)]
struct RawFixture {
    player_raw_data: PlayerData,
    boss_data: Boss,
    #[serde(default)]
    title: f32,
}

const FIXTURES_DIR: &str = "tests/fixtures/sim_to_real";

/// Runs every case in `cases.toml` through the deterministic single-tap
/// simulation and checks tap damage plus every card's damage against the
/// real, human-measured TT2 numbers recorded for that case. Passes
/// trivially (0 cases run) until a real, verified case is added.
#[test]
fn sim_to_real_golden_cases() {
    let cases_path = Path::new(FIXTURES_DIR).join("cases.toml");
    let cases_toml = std::fs::read_to_string(&cases_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", cases_path.display()));
    let cases_file: CasesFile = toml::from_str(&cases_toml)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", cases_path.display()));

    // Run every case (even after one mismatches) so a single failing test
    // run shows the full picture across all parts/cards at once -- much
    // easier to spot a pattern (e.g. "always off by the same %") than
    // stopping at the first mismatch.
    let mut all_failures = Vec::new();
    for case in &cases_file.cases {
        if let Err(failures) = run_case(case) {
            all_failures.push(format!("case '{}':\n{}", case.name, failures.join("\n")));
        }
    }

    assert!(all_failures.is_empty(), "\n{}", all_failures.join("\n\n"));
}

fn run_case(case: &Case) -> Result<(), Vec<String>> {
    let payload_path = Path::new(FIXTURES_DIR).join(&case.payload);
    let payload_json = std::fs::read_to_string(&payload_path).unwrap_or_else(|error| {
        panic!(
            "case '{}': failed to read payload {}: {error}",
            case.name,
            payload_path.display()
        )
    });
    let fixture: RawFixture = serde_json::from_str(&payload_json).unwrap_or_else(|error| {
        panic!(
            "case '{}': failed to parse payload {}: {error}",
            case.name,
            payload_path.display()
        )
    });

    let mut player_raid_data = clean_data(&fixture.player_raw_data);
    player_raid_data.title = fixture.title;

    let payload = SimPayLoad {
        player_raid_data,
        boss_data: fixture.boss_data,
        attackable_part: case.attackable_part.clone(),
        usable_card: case.deck.clone(),
        include_body_phase: false,
        mirror_force_boost: case.mirror_force_boost,
    };

    let result = SimService::run_deterministic_single_tap_simulation(payload, 1).unwrap_or_else(
        || panic!("case '{}': deck has no valid attack patterns for this boss", case.name),
    );
    let pattern = result
        .best_pattern
        .as_ref()
        .unwrap_or_else(|| panic!("case '{}': simulation produced no pattern result", case.name));

    
    let actual_tap_damage = pattern.tap_damage;

    println!(
        "[sim-to-real] case '{}': sim total={} tap={} (expected {}) cards={:?}",
        case.name,
        pattern.average_damage,
        actual_tap_damage,
        case.expected_tap_damage,
        pattern
            .card_damage
            .iter()
            .map(|card| (card.card, card.average_damage))
            .collect::<Vec<_>>()
    );

    let mut failures = Vec::new();
    check_component("tap", actual_tap_damage, case.expected_tap_damage, &mut failures);
    for (&card_name, &expected) in &case.expected_card_damage {
        let actual = pattern
            .card_damage
            .iter()
            .find(|card| card.card == card_name)
            .map_or(0, |card| card.average_damage);
        check_component(&format!("{card_name:?}"), actual, expected, &mut failures);
    }

    if failures.is_empty() { Ok(()) } else { Err(failures) }
}

/// The largest unit TT2's own in-game display would shorten `value` into
/// (1 = shown in full, no shortening).
fn display_unit(value: u64) -> u64 {
    if value >= 1_000_000_000_000 {
        1_000_000_000_000
    } else if value >= 1_000_000_000 {
        1_000_000_000
    } else if value >= 1_000_000 {
        1_000_000
    } else if value >= 1_000 {
        1_000
    } else {
        1
    }
}

/// Truncates `value` to the same precision a 2-decimal K/M/B/T display would
/// show it at (e.g. 124985 -> "124.98K" -> 124980, never rounded up) --
/// TT2's own shortened display truncates rather than rounds, and this
/// matches that exactly. Zeroes out every digit that reading such a display
/// back into a raw number can't recover. Pure integer arithmetic -- no
/// floating point, so no representation surprises.
fn truncate_to_display_precision(value: u64) -> u64 {
    let unit = display_unit(value);
    if unit == 1 {
        return value;
    }
    let step = unit / 100; // one "cent" of the shortened display
    (value / step) * step
}

fn check_component(label: &str, actual: u64, expected: u64, failures: &mut Vec<String>) {
    let actual_truncated = truncate_to_display_precision(actual);
    let expected_truncated = truncate_to_display_precision(expected);
    if actual_truncated != expected_truncated {
        failures.push(format!(
            "  {label}: expected {expected} (~{expected_truncated}), got {actual} (~{actual_truncated})"
        ));
    }
}
