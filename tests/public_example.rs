use relicsift::{analysis::engine, export, import, models::*};

fn example() -> Imported {
    import::from_json(include_str!("../examples/inventory.synthetic.json")).unwrap()
}

#[test]
fn documented_example_is_valid_and_entirely_synthetic() {
    let input = example();
    assert_eq!(input.archive.extra["metadata"]["synthetic"], true);
    assert_eq!(input.archive.relics.len(), 4);
    assert!(
        input
            .archive
            .relics
            .iter()
            .all(|r| r.uid.starts_with("synthetic-"))
    );
    assert!(input.issues.iter().all(Vec::is_empty));
    let config: Config = serde_json::from_str(include_str!("../examples/config.json")).unwrap();
    let decisions = engine::analyze(&input, &config, &Default::default()).unwrap();
    assert_eq!(
        decisions
            .iter()
            .filter(|d| d.category == Category::Keep)
            .count(),
        2
    );
    assert_eq!(
        decisions
            .iter()
            .filter(|d| d.category == Category::PotentiallyUseful)
            .count(),
        2
    );
    assert!(
        decisions
            .iter()
            .all(|d| d.category != Category::SafeDiscard)
    );
}

#[test]
fn documented_example_exports_and_reopens_without_loss() {
    let input = example();
    let config: Config = serde_json::from_str(include_str!("../examples/config.json")).unwrap();
    let decisions = engine::analyze(&input, &config, &Default::default()).unwrap();
    let report = export::report(&input, config, decisions).unwrap();
    let reopened = import::from_json(&serde_json::to_string(&report).unwrap()).unwrap();
    assert_eq!(reopened.archive.relics, input.archive.relics);
    assert_eq!(reopened.original_json, input.original_json);
    assert_eq!(report.counts.total, 4);
}
