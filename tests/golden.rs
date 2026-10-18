mod common;
use common::*;
use relicsift::models::*;
#[test]
fn representative_golden_decisions() {
    let mut locked = weak("locked");
    locked.lock = true;
    let mut unknown = weak("unknown");
    unknown.set_id = "future-set".into();
    let mut fallback = weak("fallback");
    fallback.set_id = "101".into();
    fallback.name = "Passerby of Wandering Cloud".into();
    let decisions = analyze(vec![locked, crit("hp-crit"), unknown, fallback]);
    let actual:Vec<_>=decisions.iter().map(|d|serde_json::json!({"uid":d.relic.uid,"category":d.category,"decisive_rule":d.rule_ids[0],"synergies":d.synergies})).collect();
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/decisions.golden.json")).unwrap();
    assert_eq!(serde_json::to_value(actual).unwrap(), expected);
}
#[test]
fn lower_rarity_is_conservatively_supported() {
    for rarity in 2..=4 {
        let mut r = weak("one");
        r.rarity = rarity;
        let d = analyze(vec![r]);
        assert_eq!(d[0].category, Category::PotentiallyUseful);
    }
}
