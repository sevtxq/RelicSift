mod common;
use common::*;
use relicsift::{analysis::engine, export, import, models::*};

#[test]
fn realistic_archive_shape_and_unknown_fields_roundtrip() {
    let mut relic = weak("900719925474099312345678901");
    relic
        .extra
        .insert("future_field".into(), serde_json::json!({"a":[1,2,3]}));
    relic.substats[0]
        .extra
        .insert("future_roll".into(), serde_json::json!(true));
    let input = imported(vec![relic.clone()]);
    assert!(input.issues[0].is_empty());
    assert_eq!(input.archive.relics[0], relic);
    let report = export::report(&input, config(), analyze(vec![relic.clone()])).unwrap();
    let bytes = serde_json::to_string(&report).unwrap();
    let restored = import::from_json(&bytes).unwrap();
    assert_eq!(restored.original_json, input.original_json);
    assert_eq!(restored.fingerprint, input.fingerprint);
    assert_eq!(restored.archive.relics[0], relic);
    assert_eq!(restored.archive.extra, input.archive.extra);
}
#[test]
fn duplicate_or_empty_uid_aborts_entire_import() {
    for ids in [["same", "same"], ["one", ""]] {
        let value = serde_json::json!({"source":"reliquary_archiver","build":"0.19.0","version":4,"relics":[weak(ids[0]),weak(ids[1])]});
        assert!(import::from_json(&value.to_string()).is_err());
    }
}
#[test]
fn malformed_relic_is_not_silently_dropped() {
    let mut value = serde_json::to_value(weak("one")).unwrap();
    value.as_object_mut().unwrap().remove("lock");
    let json = serde_json::json!({"source":"reliquary_archiver","build":"0.19.0","version":4,"relics":[weak("valid"),value]});
    assert!(import::from_json(&json.to_string()).is_err());
}
#[test]
fn numeric_uid_rejected_without_float_conversion() {
    let mut value = serde_json::to_value(weak("one")).unwrap();
    value["_uid"] = serde_json::json!(12345678901234567890_u64);
    let json = serde_json::json!({"source":"reliquary_archiver","build":"0.19.0","version":4,"relics":[value]});
    assert!(import::from_json(&json.to_string()).is_err());
}
#[test]
fn foreign_scanner_is_not_mistaken_for_archiver() {
    let json = serde_json::json!({"source":"HSR-Scanner","build":"0.19.0","version":4,"relics":[weak("one")]});
    assert!(import::from_json(&json.to_string()).is_err());
}
#[test]
fn optional_fields_absent_and_null_are_supported() {
    let mut value = serde_json::to_value(weak("one")).unwrap();
    value.as_object_mut().unwrap().remove("preview_substats");
    value.as_object_mut().unwrap().remove("reroll_substats");
    let json = serde_json::json!({"source":"reliquary_archiver","build":"0.19.0","version":4,"relics":[value]});
    let parsed = import::from_json(&json.to_string()).unwrap();
    assert!(parsed.archive.relics[0].preview_substats.is_none());
}
#[test]
fn unverified_version_imports_with_discard_protection() {
    let mut json: serde_json::Value =
        serde_json::from_str(&imported(vec![weak("one")]).original_json).unwrap();
    json["version"] = serde_json::json!(5);
    let input = import::from_json(&json.to_string()).unwrap();
    assert!(!input.issues[0].is_empty());
    let decision = engine::analyze(&input, &config(), &Default::default()).unwrap();
    assert!(has_rule(&decision[0], "P001"));
}
#[test]
fn utf8_bom_supported_and_fingerprinted_exactly() {
    let original = imported(vec![weak("one")]).original_json;
    let input = import::from_json(&format!("\u{feff}{original}")).unwrap();
    assert!(input.original_json.starts_with('\u{feff}'));
    assert_ne!(input.fingerprint, import::fingerprint(original.as_bytes()));
}
#[test]
fn counts_and_partition_validated() {
    let input = imported(vec![crit("one"), weak("two")]);
    let mut report =
        export::report(&input, config(), analyze(input.archive.relics.clone())).unwrap();
    assert_eq!(report.counts.total, 2);
    assert_eq!(
        report.keep.len() + report.potentially_useful.len() + report.safe_discard.len(),
        2
    );
    report.keep.clear();
    assert!(export::validate(&report).is_err());
}
#[test]
fn tampered_original_or_relic_is_rejected() {
    let input = imported(vec![crit("one")]);
    let report = export::report(&input, config(), analyze(input.archive.relics.clone())).unwrap();
    let mut bad = report.clone();
    bad.original_json.push(' ');
    assert!(export::validate(&bad).is_err());
    let mut bad = report;
    bad.keep[0].relic.uid = "tampered".into();
    assert!(export::validate(&bad).is_err());
}
#[test]
fn never_overwrite_input_or_existing_output() {
    let dir = std::env::temp_dir().join(format!("relicsift-export-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("input.json");
    let output = dir.join("output.json");
    let input = imported(vec![weak("one")]);
    std::fs::write(&path, &input.original_json).unwrap();
    let report = export::report(&input, config(), analyze(input.archive.relics.clone())).unwrap();
    assert!(export::write_new(&path, &report).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), input.original_json);
    export::write_new(&output, &report).unwrap();
    assert!(export::write_new(&output, &report).is_err());
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(output).unwrap();
    std::fs::remove_dir(dir).unwrap();
}
#[test]
fn overrides_survive_reanalysis_and_export_reimport() {
    let mut input = imported(vec![crit("one")]);
    input.overrides.insert(
        "one".into(),
        ManualOverride {
            category: Category::PotentiallyUseful,
            reason: "Personal planning".into(),
        },
    );
    let first = engine::analyze(&input, &config(), &input.overrides).unwrap();
    assert_eq!(first[0].category, Category::PotentiallyUseful);
    assert_eq!(first[0].automatic_category, Category::Keep);
    let report = export::report(&input, config(), first.clone()).unwrap();
    let reopened = import::from_json(&serde_json::to_string(&report).unwrap()).unwrap();
    let second = engine::analyze(&reopened, &config(), &reopened.overrides).unwrap();
    assert_eq!(first, second);
}
#[test]
fn manual_discard_of_protected_relic_rejected() {
    let mut r = weak("one");
    r.lock = true;
    let input = imported(vec![r]);
    let overrides = std::collections::BTreeMap::from([(
        "one".into(),
        ManualOverride {
            category: Category::SafeDiscard,
            reason: "Attempt".into(),
        },
    )]);
    assert!(engine::analyze(&input, &config(), &overrides).is_err());
}
#[test]
fn unknown_override_uid_or_empty_reason_rejected() {
    let input = imported(vec![weak("one")]);
    for (uid, reason) in [("missing", "reason"), ("one", "")] {
        let overrides = std::collections::BTreeMap::from([(
            uid.into(),
            ManualOverride {
                category: Category::Keep,
                reason: reason.into(),
            },
        )]);
        assert!(engine::analyze(&input, &config(), &overrides).is_err());
    }
}
