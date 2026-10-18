use super::{archetypes, dominance, potential, rules};
use crate::models::*;
use std::collections::BTreeMap;

pub fn analyze(
    input: &Imported,
    config: &Config,
    overrides: &BTreeMap<String, ManualOverride>,
) -> Result<Vec<Decision>, String> {
    rules::validate_config(config)?;
    for (uid, over) in overrides {
        let Some(relic) = input.archive.relics.iter().find(|r| &r.uid == uid) else {
            return Err(format!("Override for unknown UID {uid}."));
        };
        rules::validate_override(relic, config, over)?;
    }
    let valid: Vec<_> = input.issues.iter().map(|i| i.is_empty()).collect();
    let mut cache = potential::ForecastCache::default();
    let projections: Vec<Vec<_>> = input
        .archive
        .relics
        .iter()
        .enumerate()
        .map(|(i, r)| {
            rules::ARCHETYPES
                .iter()
                .map(|a| potential::project(r, a, valid[i], &mut cache))
                .collect()
        })
        .collect();
    let comparisons = dominance::compare(&input.archive.relics, &projections, &valid);
    Ok(input.archive.relics.iter().enumerate().map(|(i, relic)| {
        let synergies = archetypes::synergies(relic);
        let mut uncertainty = input.issues[i].clone();
        if rules::set(&relic.set_id).is_none() { uncertainty.push("Set effects and usage profiles have not been fully reviewed; automatic discard is blocked.".into()); }
        if !config.verified_release_dates.contains_key(&relic.set_id) { uncertainty.push("No verified release date: no age is assumed for this set.".into()); }
        uncertainty.push("Upgrade probabilities use a conditional uniform model, not verified server probabilities.".into());
        let (automatic_category, mut rule_ids, mut explanation) = rules::decide(&rules::Evidence {
            relic, config, uncertain: !valid[i], synergy_count: synergies.len(), archetypes: &projections[i], dominators: comparisons[i].len(),
        });
        let manual_override = overrides.get(&relic.uid).cloned();
        let category = manual_override.as_ref().map_or(automatic_category, |o| o.category);
        if let Some(over) = &manual_override {
            rule_ids.insert(0, "M001".into());
            explanation = format!("Manual: {}. Automatic: {}. {}", over.reason, automatic_category.label(), explanation);
        }
        let score = projections[i].iter().filter(|a| a.mainstat_compatible).map(|a| a.projection.current).fold(0.0_f64, f64::max);
        Decision {
            relic: relic.clone(), equipped: relic.equipped(), category, automatic_category, manual_override, score, explanation, rule_ids,
            archetypes: projections[i].clone(), synergies, better_relics: comparisons[i].clone(), uncertainty,
            evidence: if !valid[i] { "Missing data: conservative protection" } else if automatic_category == Category::SafeDiscard { "Multiple independent heuristics and strict inventory dominance; no guarantee for future updates" } else { "Conservative heuristic using known data" }.into(),
        }
    }).collect())
}
