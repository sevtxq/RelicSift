mod common;
use common::*;
use relicsift::{
    analysis::{archetypes, dominance, engine, rules},
    models::*,
};
#[test]
fn flat_and_percent_are_different_dimensions() {
    let r = weak("one");
    let a = &rules::ARCHETYPES[0];
    assert!(archetypes::weight("ATK_", a) > archetypes::weight("ATK", a));
    assert_eq!(rules::mainstat_subkey(&r), "ATK_");
    let mut head = r;
    head.slot = "Head".into();
    head.mainstat = "HP".into();
    assert_eq!(rules::mainstat_subkey(&head), "HP");
}
#[test]
fn count_and_step_follow_verified_export_formula() {
    let s = sub("ATK", 3, 4);
    assert!((s.value - 59.272565).abs() < 0.00001);
    let s = sub("CRIT Rate_", 3, 4);
    assert!((s.value - 9.07200008).abs() < 0.00001);
    let s = sub("SPD", 3, 4);
    assert!((s.value - 7.2).abs() < 0.00001);
}
#[test]
fn preview_is_not_counted_as_current_and_consumes_unlock_event() {
    let mut r = weak("one");
    let preview = r.substats.pop().unwrap();
    r.preview_substats = Some(vec![preview]);
    let input = imported(vec![r]);
    assert!(input.issues[0].is_empty());
    let d = engine::analyze(&input, &config(), &Default::default()).unwrap();
    for a in &d[0].archetypes {
        assert_eq!(a.projection.remaining_events, 5);
        assert_eq!(a.projection.unlock_events, 1);
        assert!(a.projection.expected.unwrap() >= a.projection.current);
    }
    let debuff = d[0]
        .archetypes
        .iter()
        .find(|a| a.name == "Debuff Support")
        .unwrap();
    assert!(debuff.projection.current < 0.5);
    assert!(debuff.projection.expected.unwrap() > 1.0);
}
#[test]
fn missing_preview_means_unknown_forecast_and_no_discard() {
    let mut r = weak("one");
    r.substats.pop();
    let d = analyze(vec![r]);
    assert_ne!(d[0].category, Category::SafeDiscard);
    assert!(
        d[0].archetypes
            .iter()
            .all(|a| a.projection.expected.is_none())
    );
}
#[test]
fn max_level_distribution_has_no_future_rolls() {
    let mut r = weak("one");
    r.level = 15;
    r.substats[0] = sub("HP", 6, 4);
    let input = imported(vec![r]);
    assert!(input.issues[0].is_empty());
    let d = engine::analyze(&input, &config(), &Default::default()).unwrap();
    for a in &d[0].archetypes {
        let p = &a.projection;
        assert_eq!(p.remaining_events, 0);
        assert_eq!(p.expected, Some(p.current));
        assert_eq!(p.good_p90, Some(p.current));
        assert_eq!(p.theoretical_max, Some(p.current));
    }
}
#[test]
fn intermediate_level_uses_remaining_milestones() {
    let mut r = weak("one");
    r.level = 4;
    r.substats[0] = sub("HP", 2, 0);
    let d = analyze(vec![r]);
    assert!(
        d[0].archetypes
            .iter()
            .all(|a| a.projection.remaining_events == 4)
    );
}
#[test]
fn exact_conditional_expectation_matches_one_roll_mean() {
    let mut r = weak("one");
    r.level = 12;
    r.substats[0] = sub("HP", 5, 0);
    let d = analyze(vec![r]);
    let a = d[0]
        .archetypes
        .iter()
        .find(|a| a.name == "Debuff Support")
        .unwrap();
    // Three flat weights .12 and one primary 1.0, neutral set => E(increment)=.34.
    assert!((a.projection.expected.unwrap() - a.projection.current - 0.34).abs() < 0.00001);
}
#[test]
fn forecast_current_expected_quantile_maximum_are_distinct() {
    let d = analyze(vec![weak("one")]);
    let p = &d[0]
        .archetypes
        .iter()
        .find(|a| a.name == "Debuff Support")
        .unwrap()
        .projection;
    assert!(p.current < p.expected.unwrap());
    assert!(p.expected.unwrap() < p.good_p90.unwrap());
    assert!(p.good_p90.unwrap() < p.theoretical_max.unwrap());
    assert!((0.0..=1.0).contains(&p.probability_good.unwrap()));
}
#[test]
fn strict_pareto_rejects_tradeoffs_and_equal_values() {
    let a = std::collections::BTreeMap::from([
        ("current:SPD".into(), 4.0),
        ("current:HP_".into(), 5.0),
    ]);
    let b = std::collections::BTreeMap::from([
        ("current:SPD".into(), 3.0),
        ("current:HP_".into(), 6.0),
    ]);
    assert!(!dominance::dominates(&a, &b));
    assert!(!dominance::dominates(&a, &a));
    let c = std::collections::BTreeMap::from([
        ("current:SPD".into(), 4.0),
        ("current:HP_".into(), 6.0),
    ]);
    assert!(dominance::dominates(&c, &a));
    assert!(dominance::dominates(&c, &b));
}
#[test]
fn dominance_never_compares_different_mainstat_set_level_or_pool() {
    let r = weak("one");
    let better = dominators(&r).remove(0);
    for mutation in 0..4 {
        let mut other = better.clone();
        match mutation {
            0 => other.mainstat = "HP".into(),
            1 => other.set_id = "103".into(),
            2 => {
                other.level = 3;
                other.substats[0] = sub("HP", 2, 2);
            }
            _ => other.substats[0] = sub("CRIT Rate_", 1, 2),
        };
        let d = analyze(vec![r.clone(), other]);
        assert!(d[0].better_relics.is_empty());
    }
}
#[test]
fn unknown_data_never_creates_dominance_evidence() {
    let r = weak("one");
    let mut other = dominators(&r).remove(0);
    other.substats[0].step = None;
    let d = analyze(vec![r, other]);
    assert!(d[0].better_relics.is_empty());
}
#[test]
fn mainstat_compatibility_prevents_impossible_profiles() {
    let mut r = weak("one");
    r.mainstat = "Outgoing Healing Boost".into();
    assert!(!archetypes::compatible(&r, &rules::ARCHETYPES[0]));
    assert!(archetypes::compatible(&r, &rules::ARCHETYPES[10]));
}
#[test]
fn set_affinity_is_soft_not_an_exclusion() {
    let mut r = crit("one");
    r.set_id = "103".into();
    let d = analyze(vec![r]);
    assert_eq!(d[0].category, Category::Keep);
    let hp = d[0].archetypes.iter().find(|a| a.name == "HP DPS").unwrap();
    assert!(hp.mainstat_compatible);
    assert!(!hp.set_affinity);
}
#[test]
fn deterministic_analysis_and_json_output() {
    let input = imported(vec![crit("one"), weak("two")]);
    let a = engine::analyze(&input, &config(), &Default::default()).unwrap();
    let b = engine::analyze(&input, &config(), &Default::default()).unwrap();
    assert_eq!(a, b);
    let a = relicsift::export::report(&input, config(), a).unwrap();
    let b = relicsift::export::report(&input, config(), b).unwrap();
    assert_eq!(
        serde_json::to_vec(&a).unwrap(),
        serde_json::to_vec(&b).unwrap()
    );
}
