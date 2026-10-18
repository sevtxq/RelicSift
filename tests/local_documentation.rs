use relicsift::analysis::rules;

#[test]
#[ignore = "docs/ is local-only; run sync_rules, then this test explicitly with --ignored"]
fn generated_markdown_is_synchronized() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/SCORING_RULES.md");
    assert_eq!(
        std::fs::read_to_string(path)
            .expect("Generate local documentation first: cargo run --bin sync_rules."),
        rules::markdown(),
        "Rules changed: run cargo run --bin sync_rules."
    );
}
