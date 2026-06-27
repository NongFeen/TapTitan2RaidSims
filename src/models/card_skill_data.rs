use std::collections::HashMap;
use std::str::FromStr;
use std::sync::OnceLock;

use csv::StringRecord;
use serde::{Deserialize, Serialize};

use super::cards::CardName;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardSkillRow {
    pub card_id: CardName,
    pub columns: HashMap<String, String>,
    pub name: String,
    pub is_active: bool,
    pub note: String,
    pub category: String,
    pub card_type: String,
    pub tier: u8,
    pub best_against: String,
    pub max_stacks: u32,
    pub duration: f64,
    pub chance: f64,
    pub max_chance: f64,
    pub spatial_length: f64,
    pub base_cooldown: f64,
    pub max_level: u16,
    pub color: String,
    pub bonus_type_a: Option<String>,
    pub a_values: Vec<f64>,
    pub bonus_type_b: Option<String>,
    pub b_values: Vec<f64>,
    pub bonus_type_c: Option<String>,
    pub bonus_amount_c: f64,
    pub bonus_type_d: Option<String>,
    pub bonus_amount_d: f64,
    pub bonus_type_e: Option<String>,
    pub bonus_amount_e: f64,
}

impl CardSkillRow {
    pub fn column(&self, name: &str) -> Option<&str> {
        self.columns.get(name).map(|value| value.as_str())
    }

    pub fn columns(&self) -> &HashMap<String, String> {
        &self.columns
    }

    pub fn value_a_at_level(&self, level: u16) -> Option<f64> {
        self.a_values.get(level.saturating_sub(1) as usize).copied()
    }

    pub fn value_b_at_level(&self, level: u16) -> Option<f64> {
        self.b_values.get(level.saturating_sub(1) as usize).copied()
    }
}

#[derive(Debug)]
pub struct CardSkillDatabase {
    rows: HashMap<CardName, CardSkillRow>,
}

static CARD_SKILLS: OnceLock<CardSkillDatabase> = OnceLock::new();

pub fn card_skill_database() -> &'static CardSkillDatabase {
    CARD_SKILLS.get_or_init(load_card_skill_database)
}

pub fn card_skill_row(card_id: CardName) -> Option<&'static CardSkillRow> {
    card_skill_database().rows.get(&card_id)
}

pub fn card_skill_value_a(card_id: CardName, level: u16) -> Option<f64> {
    card_skill_row(card_id)?.value_a_at_level(level)
}

pub fn card_skill_value_b(card_id: CardName, level: u16) -> Option<f64> {
    card_skill_row(card_id)?.value_b_at_level(level)
}

pub fn card_skill_bonustypeC(card_id: CardName) -> Option<&'static str> {
    card_skill_row(card_id)?.column("BonusTypeC")
}

pub fn card_skill_bonusamountC(card_id: CardName) -> Option<f64> {
    Some(card_skill_row(card_id)?.bonus_amount_c)
}

pub fn card_skill_bonustypeD(card_id: CardName) -> Option<&'static str> {
    card_skill_row(card_id)?.column("BonusTypeD")
}

pub fn card_skill_bonusamountD(card_id: CardName) -> Option<f64> {
    Some(card_skill_row(card_id)?.bonus_amount_d)
}
pub fn card_skill_bonustypeE(card_id: CardName) -> Option<&'static str> {
    card_skill_row(card_id)?.column("BonusTypeE")
}

pub fn card_skill_bonusamountE(card_id: CardName) -> Option<f64> {
    Some(card_skill_row(card_id)?.bonus_amount_e)
}

fn load_card_skill_database() -> CardSkillDatabase {
    let csv_data = include_str!("../../assets/taptitan/csv/RaidSkillInfo.csv");
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(csv_data.as_bytes());

    let headers = reader
        .headers()
        .expect("RaidSkillInfo.csv is missing a header row")
        .clone();

    let mut rows = HashMap::new();

    for record in reader.records() {
        let record = record.expect("Failed to read RaidSkillInfo.csv row");
        if !parse_bool(field(&headers, &record, "IsActive")) {
            continue;
        }

        let row = parse_row(&headers, &record);
        rows.insert(row.card_id, row);
    }

    CardSkillDatabase { rows }
}

fn parse_row(headers: &StringRecord, record: &StringRecord) -> CardSkillRow {
    let card_id = parse_card_name(field(headers, record, "CardID"));
    let columns = headers
        .iter()
        .zip(record.iter())
        .map(|(header, value)| (header.to_string(), value.to_string()))
        .collect::<HashMap<_, _>>();
    let a_values = parse_series(headers, record, "A", 150);
    let b_values = parse_series(headers, record, "B", 150);

    CardSkillRow {
        card_id,
        columns,
        name: field(headers, record, "Name").to_string(),
        is_active: parse_bool(field(headers, record, "IsActive")),
        note: field(headers, record, "Note").to_string(),
        category: field(headers, record, "Category").to_string(),
        card_type: field(headers, record, "Type").to_string(),
        tier: parse_u8(field(headers, record, "Tier")),
        best_against: field(headers, record, "BestAgainst").to_string(),
        max_stacks: parse_u32(field(headers, record, "MaxStacks")),
        duration: parse_f64(field(headers, record, "Duration")),
        chance: parse_f64(field(headers, record, "Chance")),
        max_chance: parse_f64(field(headers, record, "MaxChance")),
        spatial_length: parse_f64(field(headers, record, "SpatialLength")),
        base_cooldown: parse_f64(field(headers, record, "BaseCooldown")),
        max_level: parse_u16(field(headers, record, "MaxLevel")),
        color: field(headers, record, "Color").to_string(),
        bonus_type_a: parse_optional_text(field(headers, record, "BonusTypeA")),
        a_values,
        bonus_type_b: parse_optional_text(field(headers, record, "BonusTypeB")),
        b_values,
        bonus_type_c: parse_optional_text(field(headers, record, "BonusTypeC")),
        bonus_amount_c: parse_f64(field(headers, record, "BonusAmountC")),
        bonus_type_d: parse_optional_text(field(headers, record, "BonusTypeD")),
        bonus_amount_d: parse_f64(field(headers, record, "BonusAmountD")),
        bonus_type_e: parse_optional_text(field(headers, record, "BonusTypeE")),
        bonus_amount_e: parse_f64(field(headers, record, "BonusAmountE")),
    }
}

fn parse_series(
    headers: &StringRecord,
    record: &StringRecord,
    prefix: &str,
    count: usize,
) -> Vec<f64> {
    (1..=count)
        .map(|index| {
            let key = format!("{prefix}{index}");
            parse_f64(field(headers, record, &key))
        })
        .collect()
}

fn field<'a>(headers: &StringRecord, record: &'a StringRecord, name: &str) -> &'a str {
    let index = headers
        .iter()
        .position(|header| header == name)
        .unwrap_or_else(|| panic!("Missing CSV column: {name}"));
    record.get(index).unwrap_or("")
}

fn parse_card_name(value: &str) -> CardName {
    CardName::from_str(value).unwrap_or_else(|_| panic!("Unknown CardID in CSV: {value}"))
}

fn parse_bool(value: &str) -> bool {
    matches!(value, "TRUE" | "true" | "1")
}

fn parse_optional_text(value: &str) -> Option<String> {
    match value {
        "" | "None" => None,
        other => Some(other.to_string()),
    }
}

fn parse_f64(value: &str) -> f64 {
    value.parse::<f64>().unwrap_or(0.0)
}

fn parse_u32(value: &str) -> u32 {
    value.parse::<u32>().unwrap_or(0)
}

fn parse_u16(value: &str) -> u16 {
    value.parse::<u16>().unwrap_or(0)
}

fn parse_u8(value: &str) -> u8 {
    value.parse::<u8>().unwrap_or(0)
}
