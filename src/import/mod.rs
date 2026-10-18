use crate::{analysis::rules, models::*};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::Path};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("File error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid JSON or incomplete relic (the entire import was canceled): {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Invalid(String),
}
pub fn fingerprint(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
pub fn read(path: &Path) -> Result<Imported, Error> {
    from_json(&std::fs::read_to_string(path)?)
}
pub fn from_json(text: &str) -> Result<Imported, Error> {
    let value: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))?;
    if value.get("export_schema_version").is_some() {
        let report: Report = serde_json::from_value(value)?;
        crate::export::validate(&report).map_err(Error::Invalid)?;
        let mut imported = from_archive(&report.original_json)?;
        for item in report.decisions() {
            if let Some(over) = &item.manual_override {
                imported
                    .overrides
                    .insert(item.relic.uid.clone(), over.clone());
            }
        }
        imported.saved_config = Some(report.config);
        Ok(imported)
    } else {
        from_archive(text)
    }
}
fn from_archive(text: &str) -> Result<Imported, Error> {
    let archive: Archive = serde_json::from_str(text.trim_start_matches('\u{feff}'))?;
    if archive.source != "reliquary_archiver" {
        return Err(Error::Invalid("Expected source = reliquary_archiver. Other scanners are not silently interpreted as Archiver exports.".into()));
    }
    let mut seen = BTreeSet::new();
    let mut issues = Vec::with_capacity(archive.relics.len());
    for relic in &archive.relics {
        if relic.uid.trim().is_empty() || !seen.insert(&relic.uid) {
            return Err(Error::Invalid(format!(
                "Empty or duplicate _uid {:?}; no relics were imported.",
                relic.uid
            )));
        }
        issues.push(rules::data_issues(relic));
    }
    let mut warnings = vec!["V1: conditional upgrade estimates, with no guarantee for future characters. Unknown sets remain protected.".into()];
    if archive.version != 4 || archive.build != "0.19.0" {
        warnings.push(format!(
            "Unverified Archiver version {}/{}: automatic discard recommendations are blocked.",
            archive.version, archive.build
        ));
        for issue in &mut issues {
            issue.push(
                "Export version has not been verified against the reference 4 / 0.19.0.".into(),
            );
        }
    }
    let uncertain = issues.iter().filter(|v| !v.is_empty()).count();
    if uncertain > 0 {
        warnings.push(format!("{uncertain} relics have uncertain data: at least Potentially Useful (protection rules may produce Keep)."));
    }
    Ok(Imported {
        archive,
        original_json: text.to_owned(),
        fingerprint: fingerprint(text.as_bytes()),
        warnings,
        issues,
        overrides: Default::default(),
        saved_config: None,
    })
}
