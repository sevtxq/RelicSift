use super::rules;
use crate::models::{ArchetypeResult, Relic};
use std::collections::BTreeMap;

fn key(relic: &Relic) -> (String, String, String, u8, u8, Vec<String>) {
    let mut stats: Vec<_> = relic.known_pool().iter().map(|s| s.key.clone()).collect();
    stats.sort();
    (
        relic.set_id.clone(),
        relic.slot.clone(),
        relic.mainstat.clone(),
        relic.rarity,
        relic.level,
        stats,
    )
}
fn dimensions(relic: &Relic, archetypes: &[ArchetypeResult]) -> BTreeMap<String, f64> {
    let mut result = BTreeMap::new();
    for stat in &relic.substats {
        result.insert(format!("current:{}", stat.key), stat.value);
    }
    for stat in relic.known_pool() {
        result.insert(format!("pool:{}", stat.key), stat.value);
    }
    for archetype in archetypes {
        for (kind, value) in [
            ("expected", archetype.projection.expected),
            ("p90", archetype.projection.good_p90),
            ("maximum", archetype.projection.theoretical_max),
        ] {
            if let Some(value) = value {
                result.insert(format!("{}:{kind}", archetype.name), value);
            }
        }
    }
    result
}
/// Strict Pareto comparison: no weighted-sum substitute, identical dimensional support.
pub fn dominates(better: &BTreeMap<String, f64>, worse: &BTreeMap<String, f64>) -> bool {
    better.len() == worse.len()
        && worse.iter().all(|(key, value)| {
            better
                .get(key)
                .is_some_and(|b| *b + rules::DOMINANCE_EPSILON >= *value)
        })
        && worse.iter().any(|(key, value)| {
            (key.starts_with("current:") || key.starts_with("pool:"))
                && better
                    .get(key)
                    .is_some_and(|b| *b > *value + rules::DOMINANCE_EPSILON)
        })
}
pub fn compare(
    relics: &[Relic],
    projections: &[Vec<ArchetypeResult>],
    valid: &[bool],
) -> Vec<Vec<String>> {
    let mut buckets: BTreeMap<_, Vec<usize>> = BTreeMap::new();
    let vectors: Vec<_> = relics
        .iter()
        .zip(projections)
        .map(|(r, a)| dimensions(r, a))
        .collect();
    for (index, relic) in relics.iter().enumerate() {
        if valid[index] {
            buckets.entry(key(relic)).or_default().push(index);
        }
    }
    let mut result = vec![Vec::new(); relics.len()];
    for bucket in buckets.values() {
        for &worse in bucket {
            for &better in bucket {
                if better != worse && dominates(&vectors[better], &vectors[worse]) {
                    result[worse].push(relics[better].uid.clone());
                }
            }
            result[worse].sort();
        }
    }
    result
}
