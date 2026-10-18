fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs");
    std::fs::create_dir_all(&directory)?;
    let path = directory.join("SCORING_RULES.md");
    std::fs::write(&path, relicsift::analysis::rules::markdown())?;
    println!("Rule documentation updated: {}", path.display());
    Ok(())
}
