use super::rules::{self, Archetype};
use crate::models::Relic;

pub fn compatible(relic: &Relic, profile: &Archetype) -> bool {
    relic.slot == "Head"
        || relic.slot == "Hands"
        || profile.mainstats.contains(&relic.mainstat.as_str())
        || (profile.damage_sphere
            && relic.slot == "Planar Sphere"
            && rules::DAMAGE_MAINSTATS.contains(&relic.mainstat.as_str()))
}
pub fn affinity(relic: &Relic, profile: &Archetype) -> bool {
    rules::set(&relic.set_id).is_some_and(|s| s.affinities.contains(&profile.name))
}
pub fn weight(key: &str, profile: &Archetype) -> f64 {
    if profile.primary.contains(&key) {
        rules::PRIMARY_WEIGHT
    } else if profile.secondary.contains(&key) {
        rules::SECONDARY_WEIGHT
    } else if ["HP", "ATK", "DEF"].contains(&key) {
        rules::FLAT_WEIGHT
    } else {
        0.0
    }
}
pub fn multiplier(relic: &Relic, profile: &Archetype) -> f64 {
    if affinity(relic, profile) {
        rules::AFFINITY_WEIGHT
    } else {
        rules::NEUTRAL_SET_WEIGHT
    }
}
pub fn score<'a>(
    stats: impl Iterator<Item = &'a crate::models::Substat>,
    profile: &Archetype,
) -> f64 {
    stats
        .map(|s| {
            rules::stat(&s.key).map_or(0.0, |cfg| {
                s.value / (cfg.base + cfg.step) * weight(&s.key, profile)
            })
        })
        .sum()
}
pub fn synergies(relic: &Relic) -> Vec<String> {
    let pool = relic.known_pool();
    rules::SYNERGIES
        .iter()
        .filter(|synergy| {
            synergy
                .stats
                .iter()
                .all(|key| pool.iter().any(|s| &s.key == key && s.value > 0.0))
        })
        .map(|s| s.name.to_owned())
        .collect()
}
