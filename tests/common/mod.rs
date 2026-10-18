#![allow(dead_code)]
use relicsift::{
    analysis::{engine, rules},
    import,
    models::*,
};

pub fn sub(key: &str, count: u32, step: u32) -> Substat {
    let cfg = rules::stat(key).unwrap();
    Substat {
        key: key.into(),
        value: f64::from(count) * cfg.base + f64::from(step) * cfg.step,
        count: Some(count),
        step: Some(step),
        extra: Default::default(),
    }
}
pub fn weak(uid: &str) -> Relic {
    Relic {
        uid: uid.into(),
        set_id: "102".into(),
        name: "Musketeer of Wild Wheat".into(),
        slot: "Body".into(),
        rarity: 5,
        level: 0,
        mainstat: "ATK".into(),
        substats: vec![
            sub("HP", 1, 0),
            sub("ATK", 1, 0),
            sub("DEF", 1, 0),
            sub("Effect Hit Rate_", 1, 0),
        ],
        preview_substats: None,
        reroll_substats: None,
        location: String::new(),
        lock: false,
        discard: false,
        extra: Default::default(),
    }
}
pub fn crit(uid: &str) -> Relic {
    let mut r = weak(uid);
    r.slot = "Head".into();
    r.mainstat = "HP".into();
    r.substats = vec![
        sub("CRIT Rate_", 1, 0),
        sub("CRIT DMG_", 1, 1),
        sub("HP_", 1, 2),
        sub("DEF", 1, 0),
    ];
    r
}
pub fn imported(relics: Vec<Relic>) -> Imported {
    import::from_json(&serde_json::json!({"source":"reliquary_archiver","build":"0.19.0","version":4,"relics":relics,"metadata":{"large_id":90071992547409931234567890_u128.to_string()},"unknown":{"retain":true}}).to_string()).unwrap()
}
pub fn config() -> Config {
    Config {
        analysis_date: "2026-10-18".into(),
        ..Config::default()
    }
}
pub fn analyze(relics: Vec<Relic>) -> Vec<Decision> {
    let input = imported(relics);
    engine::analyze(&input, &config(), &Default::default()).unwrap()
}
pub fn has_rule(decision: &Decision, id: &str) -> bool {
    decision.rule_ids.iter().any(|r| r == id)
}
pub fn dominators(r: &Relic) -> Vec<Relic> {
    (1..=3)
        .map(|i| {
            let mut better = r.clone();
            better.uid = format!("better-{i}");
            better.substats = r
                .substats
                .iter()
                .enumerate()
                .map(|(index, s)| {
                    sub(
                        &s.key,
                        s.count.unwrap(),
                        if (i == 3 && index == 0) || i == 2 {
                            2
                        } else {
                            1
                        },
                    )
                })
                .collect();
            better
        })
        .collect()
}
