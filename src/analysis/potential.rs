use super::{archetypes, rules};
use crate::models::{ArchetypeResult, Projection, Relic};
use std::collections::BTreeMap;

// Quantized convolution avoids unstable float keys. Cache depends on increments, not relic UID.
type Distribution = Vec<(i64, f64)>;
#[derive(Default)]
pub struct ForecastCache(BTreeMap<(u8, Vec<i64>), Distribution>);
const PRECISION: f64 = 1_000_000.0;

impl ForecastCache {
    fn distribution(&mut self, events: u8, increments: Vec<i64>) -> &Distribution {
        self.0
            .entry((events, increments))
            .or_insert_with_key(|(events, increments)| {
                let mut distribution = BTreeMap::from([(0_i64, 1.0)]);
                for _ in 0..*events {
                    let mut next = BTreeMap::new();
                    for (sum, probability) in &distribution {
                        for increment in increments {
                            *next.entry(sum + increment).or_insert(0.0) +=
                                probability / increments.len() as f64;
                        }
                    }
                    distribution = next;
                }
                distribution.into_iter().collect()
            })
    }
}

pub fn project(
    relic: &Relic,
    profile: &rules::Archetype,
    valid: bool,
    cache: &mut ForecastCache,
) -> ArchetypeResult {
    let factor = archetypes::multiplier(relic, profile);
    let current = archetypes::score(relic.substats.iter(), profile) * factor;
    let remaining = rules::MAX_LEVEL.saturating_sub(relic.level) / rules::EVENT_INTERVAL;
    // Events already consumed are floor(level/3), including levels between milestones.
    let remaining = if relic.level <= rules::MAX_LEVEL {
        rules::MAX_LEVEL / rules::EVENT_INTERVAL - relic.level / rules::EVENT_INTERVAL
    } else {
        remaining
    };
    let unlock = rules::POOL_SIZE.saturating_sub(relic.substats.len()) as u8;
    let mut result = ArchetypeResult {
        name: profile.name.to_owned(),
        mainstat_compatible: archetypes::compatible(relic, profile),
        set_affinity: archetypes::affinity(relic, profile),
        relevant_stats: relic
            .known_pool()
            .iter()
            .filter(|s| profile.primary.contains(&s.key.as_str()))
            .count(),
        projection: Projection {
            current,
            expected: None,
            good_p90: None,
            theoretical_max: None,
            probability_good: None,
            remaining_events: remaining,
            unlock_events: unlock,
            assumption: "No reliable forecast: incomplete or uncertain stat pool or roll metadata."
                .into(),
        },
    };
    if !valid || remaining < unlock {
        return result;
    }
    let pool = relic.known_pool();
    if pool.len() != rules::POOL_SIZE {
        return result;
    }
    let baseline = archetypes::score(pool.iter().copied(), profile) * factor;
    let mut increments = Vec::new();
    for sub in &pool {
        let Some(cfg) = rules::stat(&sub.key) else {
            return result;
        };
        for tier in 0..rules::TIER_COUNT {
            let increment = (cfg.base + tier as f64 * cfg.step) / (cfg.base + cfg.step)
                * archetypes::weight(&sub.key, profile)
                * factor;
            increments.push((increment * PRECISION).round() as i64);
        }
    }
    increments.sort();
    let distribution = cache.distribution(remaining - unlock, increments);
    let mut expected = 0.0;
    let mut cumulative = 0.0;
    let mut p90 = None;
    let mut probability_good = 0.0;
    for &(sum, probability) in distribution {
        let value = baseline + sum as f64 / PRECISION;
        expected += value * probability;
        cumulative += probability;
        if p90.is_none() && cumulative >= rules::GOOD_QUANTILE {
            p90 = Some(value);
        }
        if value >= rules::GOOD_SCORE {
            probability_good += probability;
        }
    }
    let theoretical = baseline
        + distribution
            .last()
            .map_or(0.0, |(v, _)| *v as f64 / PRECISION);
    result.projection.expected = Some(expected);
    result.projection.good_p90 = p90.or(Some(theoretical));
    result.projection.theoretical_max = Some(theoretical);
    result.projection.probability_good = Some(probability_good.clamp(0.0, 1.0));
    result.projection.assumption = "Conditional model: four known stats; unlock pending preview stats first, then assume independent uniform stat targets and quality tiers. Server probabilities are unverified; this does not predict drops or characters.".into();
    result
}
