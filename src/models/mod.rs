use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Substat {
    pub key: String,
    pub value: f64,
    pub count: Option<u32>,
    pub step: Option<u32>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Relic {
    #[serde(rename = "_uid")]
    pub uid: String,
    pub set_id: String,
    pub name: String,
    pub slot: String,
    pub rarity: u8,
    pub level: u8,
    pub mainstat: String,
    pub substats: Vec<Substat>,
    pub preview_substats: Option<Vec<Substat>>,
    pub reroll_substats: Option<Vec<Substat>>,
    pub location: String,
    pub lock: bool,
    pub discard: bool,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Relic {
    pub fn equipped(&self) -> bool {
        !self.location.is_empty()
    }
    pub fn protected(&self) -> bool {
        self.lock || self.equipped()
    }
    /// Preview entries are pending new affixes; never add their values to current stats.
    pub fn known_pool(&self) -> Vec<&Substat> {
        self.substats
            .iter()
            .chain(self.preview_substats.iter().flatten())
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Archive {
    pub source: String,
    pub build: String,
    pub version: u32,
    pub relics: Vec<Relic>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Keep,
    #[default]
    PotentiallyUseful,
    SafeDiscard,
}
impl Category {
    pub const ALL: [Self; 3] = [Self::Keep, Self::PotentiallyUseful, Self::SafeDiscard];
    pub fn label(self) -> &'static str {
        match self {
            Self::Keep => "Keep",
            Self::PotentiallyUseful => "Potentially Useful",
            Self::SafeDiscard => "Safe Discard",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ManualOverride {
    pub category: Category,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// ISO calendar date. Persisted in every report, never read by the engine from the clock.
    pub analysis_date: String,
    pub new_set_protection_days: u32,
    /// Only add dates from verified sources. No release date is inferred from an ID.
    pub verified_release_dates: BTreeMap<String, String>,
    pub protected_uids: Vec<String>,
    /// Additive future profiles can protect relics; never remove baseline archetypes.
    pub future_profiles: Vec<FutureProfile>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            analysis_date: time::OffsetDateTime::now_utc().date().to_string(),
            new_set_protection_days: 90,
            verified_release_dates: BTreeMap::new(),
            protected_uids: Vec::new(),
            future_profiles: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FutureProfile {
    pub name: String,
    pub stats: Vec<String>,
    pub mainstats: Vec<String>,
    /// Empty permits any set. These profiles only create additional protection.
    pub set_ids: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Imported {
    pub archive: Archive,
    pub original_json: String,
    pub fingerprint: String,
    pub warnings: Vec<String>,
    pub issues: Vec<Vec<String>>,
    pub overrides: BTreeMap<String, ManualOverride>,
    pub saved_config: Option<Config>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Projection {
    pub current: f64,
    pub expected: Option<f64>,
    pub good_p90: Option<f64>,
    pub theoretical_max: Option<f64>,
    pub probability_good: Option<f64>,
    pub remaining_events: u8,
    pub unlock_events: u8,
    pub assumption: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ArchetypeResult {
    pub name: String,
    pub mainstat_compatible: bool,
    pub set_affinity: bool,
    pub relevant_stats: usize,
    pub projection: Projection,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Decision {
    pub relic: Relic,
    pub equipped: bool,
    pub category: Category,
    pub automatic_category: Category,
    pub manual_override: Option<ManualOverride>,
    pub score: f64,
    pub explanation: String,
    pub rule_ids: Vec<String>,
    pub archetypes: Vec<ArchetypeResult>,
    pub synergies: Vec<String>,
    pub better_relics: Vec<String>,
    pub uncertainty: Vec<String>,
    pub evidence: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Counts {
    pub total: usize,
    pub keep: usize,
    pub potentially_useful: usize,
    pub safe_discard: usize,
    pub protected_or_equipped: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub export_schema_version: u32,
    pub application: String,
    pub application_version: String,
    pub rules_version: String,
    pub rules_fingerprint: String,
    pub input_fingerprint: String,
    /// Exact original UTF-8 input, including unknown fields and account metadata.
    pub original_json: String,
    pub config: Config,
    pub counts: Counts,
    pub warnings: Vec<String>,
    pub keep: Vec<Decision>,
    pub potentially_useful: Vec<Decision>,
    pub safe_discard: Vec<Decision>,
}
impl Report {
    pub fn decisions(&self) -> impl Iterator<Item = &Decision> {
        self.keep
            .iter()
            .chain(&self.potentially_useful)
            .chain(&self.safe_discard)
    }
}
