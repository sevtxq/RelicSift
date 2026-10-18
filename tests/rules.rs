mod common;
use common::*;
use relicsift::{
    analysis::{engine, rules},
    models::*,
};

#[test]
fn registry_ids_priorities_and_archetypes_are_unique() {
    let registry = rules::registry();
    assert_eq!(
        registry
            .iter()
            .map(|r| r.id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        registry.len()
    );
    assert_eq!(
        registry
            .iter()
            .map(|r| r.priority)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        registry.len()
    );
    assert_eq!(
        rules::ARCHETYPES
            .iter()
            .map(|r| r.name)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        rules::ARCHETYPES.len()
    );
}
#[test]
fn k001_lock_and_equipment_override_all_weaknesses() {
    for lock in [true, false] {
        let mut r = weak("one");
        if lock {
            r.lock = true
        } else {
            r.location = "1202".into()
        };
        let d = analyze(vec![r]);
        assert_eq!(d[0].category, Category::Keep);
        assert!(has_rule(&d[0], "K001"));
        assert!(!has_rule(&d[0], "D001"));
    }
}
#[test]
fn k002_explicit_uid_protection() {
    let input = imported(vec![weak("one")]);
    let mut c = config();
    c.protected_uids.push("one".into());
    let d = engine::analyze(&input, &c, &Default::default()).unwrap();
    assert_eq!(d[0].category, Category::Keep);
    assert!(has_rule(&d[0], "K002"));
}
#[test]
fn k003_hp_def_and_atk_crit_synergies_work_on_all_sets() {
    for scale in ["HP_", "DEF_", "ATK_"] {
        let mut r = crit("one");
        r.set_id = "unknown".into();
        r.substats[2] = sub(scale, 1, 0);
        let d = analyze(vec![r]);
        assert_eq!(d[0].category, Category::Keep);
        assert!(has_rule(&d[0], "K003"));
    }
}
#[test]
fn k004_high_actual_quality_is_not_based_on_preview() {
    let mut r = weak("one");
    r.slot = "Head".into();
    r.mainstat = "HP".into();
    r.level = 15;
    r.substats = vec![
        sub("CRIT Rate_", 3, 3),
        sub("HP_", 4, 4),
        sub("DEF", 1, 0),
        sub("Effect Hit Rate_", 1, 0),
    ];
    let d = analyze(vec![r]);
    assert!(has_rule(&d[0], "K004"));
    assert_eq!(d[0].category, Category::Keep);
}
#[test]
fn k005_speed_uses_actual_value_only() {
    let mut r = weak("one");
    r.level = 9;
    r.substats = vec![
        sub("SPD", 4, 0),
        sub("HP", 1, 0),
        sub("ATK", 1, 0),
        sub("DEF", 1, 0),
    ];
    let d = analyze(vec![r]);
    assert_eq!(d[0].category, Category::Keep);
    assert!(has_rule(&d[0], "K005"));
}
#[test]
fn p001_unknown_or_inconsistent_data_prevents_discard() {
    let mut cases = Vec::new();
    let mut r = weak("one");
    r.substats[0].key = "New Stat_".into();
    cases.push(r);
    let mut r = weak("one");
    r.substats[0].step = Some(9);
    cases.push(r);
    let mut r = weak("one");
    r.substats[0].count = None;
    cases.push(r);
    let mut r = weak("one");
    r.substats[0].value += 1.0;
    cases.push(r);
    let mut r = weak("one");
    r.slot = "Future Slot".into();
    cases.push(r);
    let mut r = weak("one");
    r.substats.pop();
    cases.push(r);
    let mut r = weak("one");
    r.reroll_substats = Some(vec![sub("SPD", 1, 0)]);
    cases.push(r);
    for r in cases {
        let d = analyze(vec![r]);
        assert_ne!(d[0].category, Category::SafeDiscard);
        assert!(has_rule(&d[0], "P001"));
    }
}
#[test]
fn p002_unknown_set_is_protected_even_when_dominated() {
    let mut r = weak("one");
    r.set_id = "999999".into();
    let mut relics = dominators(&r);
    relics.push(r);
    let d = analyze(relics);
    let d = d.last().unwrap();
    assert!(has_rule(d, "P002"));
    assert_ne!(d.category, Category::SafeDiscard);
}
#[test]
fn p003_recent_dates_boundary_future_and_invalid_dates_protect() {
    let input = imported(vec![weak("one")]);
    for date in ["2026-10-18", "2026-07-20", "2027-01-01", "not-a-date"] {
        let mut c = config();
        c.verified_release_dates.insert("102".into(), date.into());
        let d = engine::analyze(&input, &c, &Default::default()).unwrap();
        assert!(has_rule(&d[0], "P003"), "{date}");
    }
    for date in ["2020-01-01", "2026-07-19"] {
        let mut c = config();
        c.verified_release_dates.insert("102".into(), date.into());
        let d = engine::analyze(&input, &c, &Default::default()).unwrap();
        assert!(!has_rule(&d[0], "P003"), "{date}");
    }
}
#[test]
fn p003_missing_date_is_never_invented_and_zero_disables_timer() {
    let input = imported(vec![weak("one")]);
    let mut c = config();
    let d = engine::analyze(&input, &c, &Default::default()).unwrap();
    assert!(!has_rule(&d[0], "P003"));
    assert!(d[0].uncertainty.iter().any(|u| u.contains("release date")));
    c.verified_release_dates
        .insert("102".into(), "2026-10-18".into());
    c.new_set_protection_days = 0;
    let d = engine::analyze(&input, &c, &Default::default()).unwrap();
    assert!(!has_rule(&d[0], "P003"));
}
#[test]
fn p004_specialized_mainstats_protect_two_bad_flats() {
    for (slot, mains) in rules::MAINSTATS {
        for main in mains {
            if !rules::RARE_MAINSTATS.contains(main) && !rules::DAMAGE_MAINSTATS.contains(main) {
                continue;
            }
            let mut r = weak("one");
            r.slot = slot.into();
            r.mainstat = (*main).into();
            let d = analyze(vec![r]);
            assert_ne!(d[0].category, Category::SafeDiscard);
            assert!(has_rule(&d[0], "P004"));
        }
    }
}
#[test]
fn p005_level_twelve_and_fifteen_protected() {
    for level in [12, 15] {
        let mut r = weak("one");
        r.level = level;
        r.substats[0] = sub("HP", 1 + u32::from(level / 3), 0);
        let d = analyze(vec![r]);
        assert!(has_rule(&d[0], "P005"));
        assert_ne!(d[0].category, Category::SafeDiscard);
    }
}
#[test]
fn p006_realistic_potential_is_protected() {
    let mut r = weak("one");
    r.slot = "Hands".into();
    r.substats = vec![
        sub("CRIT Rate_", 1, 0),
        sub("ATK_", 1, 0),
        sub("HP", 1, 0),
        sub("DEF", 1, 0),
    ];
    let d = analyze(vec![r]);
    assert!(has_rule(&d[0], "P006"));
    assert_ne!(d[0].category, Category::SafeDiscard);
}
#[test]
fn p007_break_speed_and_ehr_speed_survive_two_flats() {
    for key in ["Break Effect_", "Effect Hit Rate_", "ATK_"] {
        let mut r = weak("one");
        r.substats = vec![
            sub("SPD", 1, 0),
            sub(key, 1, 0),
            sub("HP", 1, 0),
            sub("DEF", 1, 0),
        ];
        let d = analyze(vec![r]);
        assert!(has_rule(&d[0], "P007"));
        assert_ne!(d[0].category, Category::SafeDiscard);
    }
}
#[test]
fn p008_future_profile_adds_protection_without_replacing_baselines() {
    let input = imported(vec![weak("one")]);
    let mut c = config();
    c.future_profiles.push(FutureProfile {
        name: "Future flat/EHR mechanic".into(),
        stats: vec!["HP".into(), "Effect Hit Rate_".into()],
        mainstats: vec!["ATK".into()],
        set_ids: vec!["102".into()],
    });
    let d = engine::analyze(&input, &c, &Default::default()).unwrap();
    assert!(has_rule(&d[0], "P008"));
    assert_eq!(d[0].archetypes.len(), rules::ARCHETYPES.len());
    c.future_profiles[0].set_ids = vec!["103".into()];
    let d = engine::analyze(&input, &c, &Default::default()).unwrap();
    assert!(!has_rule(&d[0], "P008"));
}
#[test]
fn d001_requires_all_guards_and_multiple_dominators() {
    let mut r = weak("weak");
    r.set_id = "101".into();
    r.name = "Passerby of Wandering Cloud".into();
    let mut relics = vec![r.clone()];
    relics.extend(dominators(&r));
    let d = analyze(relics);
    assert_eq!(d[0].better_relics.len(), 3);
    assert_eq!(d[0].category, Category::SafeDiscard, "{:?}", d[0]);
    assert!(has_rule(&d[0], "D001"));
    for count in 0..3 {
        let mut relics = vec![r.clone()];
        relics.extend(dominators(&r).into_iter().take(count));
        let d = analyze(relics);
        assert_ne!(d[0].category, Category::SafeDiscard);
    }
}
#[test]
fn p999_is_the_fallback_without_a_discard_quota() {
    let mut r = weak("one");
    r.set_id = "101".into();
    let d = analyze(vec![r]);
    assert_eq!(d[0].category, Category::PotentiallyUseful);
    assert!(has_rule(&d[0], "P999"));
}
#[test]
fn preexisting_discard_mark_has_no_influence() {
    let a = weak("one");
    let mut b = a.clone();
    b.discard = true;
    let da = analyze(vec![a]);
    let db = analyze(vec![b]);
    assert_eq!(da[0].category, db[0].category);
    assert_eq!(da[0].score, db[0].score);
    assert_eq!(da[0].rule_ids, db[0].rule_ids);
}
#[test]
fn invalid_analysis_date_and_invalid_future_stat_rejected() {
    let input = imported(vec![weak("one")]);
    let mut c = config();
    c.analysis_date = "tomorrow".into();
    assert!(engine::analyze(&input, &c, &Default::default()).is_err());
    c = config();
    c.future_profiles.push(FutureProfile {
        name: "Test".into(),
        stats: vec!["bogus".into()],
        mainstats: vec![],
        set_ids: vec![],
    });
    assert!(engine::analyze(&input, &c, &Default::default()).is_err());
}
