use relicsift::{analysis::engine, export, import, models::Config};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() < 2 || args.len() > 3 {
        eprintln!("Usage: analyze <archive-or-result.json> <new-result.json> [configuration.json]");
        std::process::exit(2);
    }
    let input = import::read(std::path::Path::new(&args[0]))?;
    let config: Config = if args.len() == 3 {
        serde_json::from_slice(&std::fs::read(&args[2])?)?
    } else {
        input.saved_config.clone().unwrap_or_default()
    };
    let start = std::time::Instant::now();
    let decisions = engine::analyze(&input, &config, &input.overrides)?;
    let report = export::report(&input, config, decisions)?;
    export::write_new(std::path::Path::new(&args[1]), &report)?;
    println!(
        "{} relics: {} Keep / {} Potentially Useful / {} Safe Discard ({:.2}s)",
        report.counts.total,
        report.counts.keep,
        report.counts.potentially_useful,
        report.counts.safe_discard,
        start.elapsed().as_secs_f64()
    );
    Ok(())
}
