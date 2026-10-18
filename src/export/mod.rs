use crate::{analysis::rules, import, models::*};
use std::{collections::BTreeSet, io::Write, path::Path};

pub const SCHEMA_VERSION: u32 = 1;
pub fn report(
    input: &Imported,
    config: Config,
    decisions: Vec<Decision>,
) -> Result<Report, String> {
    let mut result = Report {
        export_schema_version: SCHEMA_VERSION,
        application: "RelicSift".into(),
        application_version: env!("CARGO_PKG_VERSION").into(),
        rules_version: rules::VERSION.into(),
        rules_fingerprint: import::fingerprint(rules::markdown().as_bytes()),
        input_fingerprint: input.fingerprint.clone(),
        original_json: input.original_json.clone(),
        config,
        counts: Counts::default(),
        warnings: input.warnings.clone(),
        keep: Vec::new(),
        potentially_useful: Vec::new(),
        safe_discard: Vec::new(),
    };
    for decision in decisions {
        match decision.category {
            Category::Keep => result.keep.push(decision),
            Category::PotentiallyUseful => result.potentially_useful.push(decision),
            Category::SafeDiscard => result.safe_discard.push(decision),
        }
    }
    // Stable UID order, independent of UI sorting and filtering.
    result.keep.sort_by(|a, b| a.relic.uid.cmp(&b.relic.uid));
    result
        .potentially_useful
        .sort_by(|a, b| a.relic.uid.cmp(&b.relic.uid));
    result
        .safe_discard
        .sort_by(|a, b| a.relic.uid.cmp(&b.relic.uid));
    result.counts = Counts {
        total: input.archive.relics.len(),
        keep: result.keep.len(),
        potentially_useful: result.potentially_useful.len(),
        safe_discard: result.safe_discard.len(),
        protected_or_equipped: input
            .archive
            .relics
            .iter()
            .filter(|r| r.protected())
            .count(),
    };
    validate(&result)?;
    Ok(result)
}

pub fn validate(report: &Report) -> Result<(), String> {
    if report.export_schema_version != SCHEMA_VERSION || report.application != "RelicSift" {
        return Err("Unknown RelicSift result schema.".into());
    }
    if import::fingerprint(report.original_json.as_bytes()) != report.input_fingerprint {
        return Err("Original data does not match the input fingerprint.".into());
    }
    let original: Archive =
        serde_json::from_str(report.original_json.trim_start_matches('\u{feff}'))
            .map_err(|e| e.to_string())?;
    let originals: std::collections::BTreeMap<_, _> =
        original.relics.iter().map(|r| (&r.uid, r)).collect();
    let mut ids = BTreeSet::new();
    for (group, expected) in [
        (&report.keep, Category::Keep),
        (&report.potentially_useful, Category::PotentiallyUseful),
        (&report.safe_discard, Category::SafeDiscard),
    ] {
        for item in group {
            if item.category != expected
                || !ids.insert(&item.relic.uid)
                || originals
                    .get(&item.relic.uid)
                    .is_none_or(|r| **r != item.relic)
                || item.equipped != item.relic.equipped()
            {
                return Err("Invalid category partition, original relic or UID uniqueness.".into());
            }
            if expected == Category::SafeDiscard
                && (item.relic.protected()
                    || report.config.protected_uids.contains(&item.relic.uid))
            {
                return Err("Protected relic found in Safe Discard.".into());
            }
            if let Some(over) = &item.manual_override {
                rules::validate_override(&item.relic, &report.config, over)?;
                if over.category != item.category {
                    return Err("Inconsistent manual override.".into());
                }
            } else if item.category != item.automatic_category {
                return Err(
                    "Category differs from the automatic decision without an override.".into(),
                );
            }
        }
    }
    if ids.len() != original.relics.len()
        || originals.len() != original.relics.len()
        || report.counts
            != (Counts {
                total: ids.len(),
                keep: report.keep.len(),
                potentially_useful: report.potentially_useful.len(),
                safe_discard: report.safe_discard.len(),
                protected_or_equipped: original.relics.iter().filter(|r| r.protected()).count(),
            })
    {
        return Err("At least one relic is missing or the counts do not match.".into());
    }
    Ok(())
}

/// Create-new is deliberate: never replace an input, prior export, symlink target or hardlink.
pub fn write_new(path: &Path, report: &Report) -> Result<(), String> {
    validate(report)?;
    let bytes = serde_json::to_vec_pretty(report).map_err(|e| e.to_string())?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            format!("Export requires a new filename; existing files are never overwritten: {e}")
        })?;
    if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
        // A partial file is left visibly incomplete; never report success or overwrite it.
        return Err(format!(
            "Write error; the result file may be incomplete: {error}"
        ));
    }
    Ok(())
}
