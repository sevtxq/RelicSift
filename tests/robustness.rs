mod common;
use common::*;
use relicsift::{
    analysis::{engine, rules},
    export, import,
    models::*,
};

#[test]
fn future_fields_cannot_collide_with_analysis_fields() {
    let mut relic = weak("one");
    for key in [
        "category",
        "score",
        "manual_override",
        "equipped",
        "rule_ids",
        "relic",
    ] {
        relic
            .extra
            .insert(key.into(), serde_json::json!({"preserved":"original"}));
    }
    let input = imported(vec![relic]);
    let decisions = engine::analyze(&input, &config(), &Default::default()).unwrap();
    let report = export::report(&input, config(), decisions).unwrap();
    let reopened = import::from_json(&serde_json::to_string(&report).unwrap()).unwrap();
    assert_eq!(input.archive.relics, reopened.archive.relics);
}
#[test]
fn huge_roll_counts_protect_instead_of_overflowing() {
    let mut relic = weak("one");
    for s in &mut relic.substats {
        s.count = Some(u32::MAX);
        s.step = Some(u32::MAX);
    }
    let decisions = analyze(vec![relic]);
    assert_ne!(decisions[0].category, Category::SafeDiscard);
    assert!(has_rule(&decisions[0], "P001"));
}
#[test]
fn configuration_typos_fail_instead_of_silently_removing_protection() {
    assert!(serde_json::from_str::<Config>(r#"{"protected_uidz":["one"]}"#).is_err());
    assert!(
        serde_json::from_str::<FutureProfile>(
            r#"{"name":"x","stats":["SPD"],"mainstats":[],"set_ids":[],"typo":true}"#
        )
        .is_err()
    );
}
#[test]
fn explicit_protection_blocks_manual_discard() {
    let relic = weak("one");
    let mut c = config();
    c.protected_uids.push("one".into());
    assert!(
        rules::validate_override(
            &relic,
            &c,
            &ManualOverride {
                category: Category::SafeDiscard,
                reason: "test".into()
            }
        )
        .is_err()
    );
}
#[test]
fn every_synergy_has_a_non_discard_counterexample() {
    for synergy in rules::SYNERGIES {
        let mut relic = weak("one");
        relic.slot = "Planar Sphere".into();
        relic.mainstat = "Quantum DMG Boost".into();
        relic.substats = synergy.stats.iter().map(|key| sub(key, 1, 0)).collect();
        for key in ["HP", "ATK", "DEF", "Effect RES_"] {
            if relic.substats.len() == 4 {
                break;
            }
            if !relic.substats.iter().any(|s| s.key == key) {
                relic.substats.push(sub(key, 1, 0));
            }
        }
        let decisions = analyze(vec![relic]);
        assert_ne!(
            decisions[0].category,
            Category::SafeDiscard,
            "{}",
            synergy.name
        );
        assert!(has_rule(&decisions[0], "P007"));
    }
}
#[test]
fn all_four_stats_at_plus_zero_are_not_given_a_fifth_unlock() {
    let decisions = analyze(vec![weak("one")]);
    for a in &decisions[0].archetypes {
        assert_eq!(a.projection.unlock_events, 0);
        assert_eq!(a.projection.remaining_events, 5);
    }
}
