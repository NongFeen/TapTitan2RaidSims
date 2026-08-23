use super::*;

#[test]
fn each_selection_activates_at_most_one_modifier() {
    let selections = [
        GlobalRaidModifier::None,
        GlobalRaidModifier::BurstDamage,
        GlobalRaidModifier::BurstChance,
        GlobalRaidModifier::SupportEffect,
        GlobalRaidModifier::AfflictionChance,
        GlobalRaidModifier::AfflictionDamage,
        GlobalRaidModifier::AllDamage,
        GlobalRaidModifier::AttackDuration,
        GlobalRaidModifier::AfflictionDuration,
    ];

    for selected in selections {
        let modifiers = global_raid_modifiers(selected, None);
        let active_count = [
            modifiers.burst_damage_mult != 1.0,
            modifiers.burst_chance_mult != 1.0,
            modifiers.support_effect_mult != 1.0,
            modifiers.affliction_chance_mult != 1.0,
            modifiers.affliction_damage_mult != 1.0,
            modifiers.all_damage_mult != 1.0,
            modifiers.attack_duration_add_seconds != 0.0,
            modifiers.affliction_duration_mult != 1.0,
        ]
        .into_iter()
        .filter(|active| *active)
        .count();

        assert_eq!(
            active_count,
            usize::from(selected != GlobalRaidModifier::None),
            "unexpected active modifier count for {selected:?}",
        );
    }
}
