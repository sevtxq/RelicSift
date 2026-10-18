use relicsift::{analysis::engine, export, import, models::*};

#[test]
#[ignore = "requires a private local archive; set RELICSIFT_TEST_ARCHIVE explicitly"]
fn private_reference_inventory_full_integration() {
    let path = std::env::var_os("RELICSIFT_TEST_ARCHIVE")
        .map(std::path::PathBuf::from)
        .expect("Set RELICSIFT_TEST_ARCHIVE to a private local Reliquary archive.");
    let input = import::read(&path).expect("Private reference archive must be readable.");
    let raw: serde_json::Value =
        serde_json::from_str(input.original_json.trim_start_matches('\u{feff}')).unwrap();
    let original_count = raw["relics"].as_array().unwrap().len();
    assert_eq!(input.archive.relics.len(), original_count);
    assert!(original_count > 0, "Use a nonempty reference archive.");

    let config = Config {
        analysis_date: "2026-10-18".into(),
        ..Config::default()
    };
    let first = engine::analyze(&input, &config, &Default::default()).unwrap();
    let second = engine::analyze(&input, &config, &Default::default()).unwrap();
    assert_eq!(first, second);
    assert!(
        first
            .iter()
            .filter(|d| d.relic.protected())
            .all(|d| d.category == Category::Keep)
    );

    let report = export::report(&input, config, first).unwrap();
    assert_eq!(report.counts.total, original_count);
    export::validate(&report).unwrap();
    let reopened = import::from_json(&serde_json::to_string(&report).unwrap()).unwrap();
    assert_eq!(reopened.archive.relics, input.archive.relics);
    assert_eq!(reopened.original_json, input.original_json);
    assert_eq!(reopened.fingerprint, input.fingerprint);
}
