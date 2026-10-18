//! Single source of truth: decisions, numerical parameters, profiles and English documentation.
//! The engine only calculates evidence; category selection happens here.
use crate::models::*;

pub const VERSION: &str = "1.0.0-conservative";

#[derive(Clone, Copy, Debug)]
pub struct ManualPolicy {
    pub id: &'static str,
    pub name: &'static str,
    pub priority: u16,
    pub require_reason: bool,
    pub forbid_protected_discard: bool,
    pub description: &'static str,
}
pub const MANUAL_POLICY: ManualPolicy = ManualPolicy {
    id: "M001",
    name: "Manual decision",
    priority: 1100,
    require_reason: true,
    forbid_protected_discard: true,
    description: "The user explicitly chooses a category and supplies a nonempty reason. Locked, equipped and UID-protected relics cannot be marked Safe Discard. The automatic recommendation is retained separately. Overrides survive reanalysis and result export/reimport; permanent storage requires exporting, without modifying the original file.",
};
pub fn validate_override(
    relic: &Relic,
    config: &Config,
    over: &ManualOverride,
) -> Result<(), String> {
    if MANUAL_POLICY.forbid_protected_discard
        && over.category == Category::SafeDiscard
        && (relic.protected() || config.protected_uids.contains(&relic.uid))
    {
        return Err(format!(
            "UID {} is protected: a Safe Discard override is not allowed.",
            relic.uid
        ));
    }
    if MANUAL_POLICY.require_reason && over.reason.trim().is_empty() {
        return Err(format!("Override for {} requires a reason.", relic.uid));
    }
    Ok(())
}

pub fn implementation_fingerprint() -> String {
    let mut source = Vec::new();
    for file in [
        include_bytes!("rules.rs").as_slice(),
        include_bytes!("archetypes.rs").as_slice(),
        include_bytes!("potential.rs").as_slice(),
        include_bytes!("dominance.rs").as_slice(),
        include_bytes!("engine.rs").as_slice(),
        include_bytes!("../import/mod.rs").as_slice(),
        include_bytes!("../models/mod.rs").as_slice(),
    ] {
        source.extend_from_slice(&(file.len() as u64).to_le_bytes());
        source.extend_from_slice(file);
    }
    crate::import::fingerprint(&source)
}

#[derive(Clone, Copy, Debug)]
pub struct Stat {
    pub key: &'static str,
    pub base: f64,
    pub step: f64,
}
pub const STATS: [Stat; 12] = [
    Stat {
        key: "HP",
        base: 33.87004,
        step: 4.233755,
    },
    Stat {
        key: "ATK",
        base: 16.935019,
        step: 2.116877,
    },
    Stat {
        key: "DEF",
        base: 16.935019,
        step: 2.116877,
    },
    Stat {
        key: "HP_",
        base: 3.4560002,
        step: 0.43200003,
    },
    Stat {
        key: "ATK_",
        base: 3.4560002,
        step: 0.43200003,
    },
    Stat {
        key: "DEF_",
        base: 4.32,
        step: 0.54,
    },
    Stat {
        key: "SPD",
        base: 2.0,
        step: 0.3,
    },
    Stat {
        key: "CRIT Rate_",
        base: 2.592,
        step: 0.32400002,
    },
    Stat {
        key: "CRIT DMG_",
        base: 5.184,
        step: 0.64800004,
    },
    Stat {
        key: "Effect Hit Rate_",
        base: 3.4560002,
        step: 0.43200003,
    },
    Stat {
        key: "Effect RES_",
        base: 3.4560002,
        step: 0.43200003,
    },
    Stat {
        key: "Break Effect_",
        base: 5.184,
        step: 0.64800004,
    },
];
pub fn stat(key: &str) -> Option<&'static Stat> {
    STATS.iter().find(|s| s.key == key)
}

pub const MAX_LEVEL: u8 = 15;
pub const EVENT_INTERVAL: u8 = 3;
pub const POOL_SIZE: usize = 4;
pub const TIER_COUNT: usize = 3;
pub const VALUE_TOLERANCE: f64 = 0.002;
pub const GOOD_SCORE: f64 = 5.0;
pub const GOOD_QUANTILE: f64 = 0.90;
pub const DOMINANCE_EPSILON: f64 = 0.000001;
pub const FLAT_WEIGHT: f64 = 0.12;
pub const PRIMARY_WEIGHT: f64 = 1.0;
pub const SECONDARY_WEIGHT: f64 = 0.35;
pub const AFFINITY_WEIGHT: f64 = 1.10;
pub const NEUTRAL_SET_WEIGHT: f64 = 1.0;

pub const DAMAGE_MAINSTATS: [&str; 7] = [
    "Physical DMG Boost",
    "Fire DMG Boost",
    "Ice DMG Boost",
    "Lightning DMG Boost",
    "Wind DMG Boost",
    "Quantum DMG Boost",
    "Imaginary DMG Boost",
];
pub const MAINSTATS: [(&str, &[&str]); 6] = [
    ("Head", &["HP"]),
    ("Hands", &["ATK"]),
    (
        "Body",
        &[
            "HP",
            "ATK",
            "DEF",
            "CRIT Rate",
            "CRIT DMG",
            "Effect Hit Rate",
            "Outgoing Healing Boost",
        ],
    ),
    ("Feet", &["HP", "ATK", "DEF", "SPD"]),
    (
        "Planar Sphere",
        &[
            "HP",
            "ATK",
            "DEF",
            "Physical DMG Boost",
            "Fire DMG Boost",
            "Ice DMG Boost",
            "Lightning DMG Boost",
            "Wind DMG Boost",
            "Quantum DMG Boost",
            "Imaginary DMG Boost",
        ],
    ),
    (
        "Link Rope",
        &[
            "HP",
            "ATK",
            "DEF",
            "Break Effect",
            "Energy Regeneration Rate",
        ],
    ),
];
pub const RARE_MAINSTATS: [&str; 7] = [
    "SPD",
    "CRIT Rate",
    "CRIT DMG",
    "Effect Hit Rate",
    "Outgoing Healing Boost",
    "Energy Regeneration Rate",
    "Break Effect",
];

#[derive(Clone, Copy, Debug)]
pub struct Archetype {
    pub name: &'static str,
    pub primary: &'static [&'static str],
    pub secondary: &'static [&'static str],
    pub mainstats: &'static [&'static str],
    pub damage_sphere: bool,
}
pub const ARCHETYPES: [Archetype; 14] = [
    Archetype {
        name: "Crit DPS",
        primary: &["CRIT Rate_", "CRIT DMG_", "ATK_"],
        secondary: &["SPD"],
        mainstats: &["ATK", "CRIT Rate", "CRIT DMG", "SPD"],
        damage_sphere: true,
    },
    Archetype {
        name: "ATK DPS",
        primary: &["ATK_", "SPD"],
        secondary: &["CRIT Rate_", "CRIT DMG_"],
        mainstats: &["ATK", "SPD", "CRIT Rate", "CRIT DMG"],
        damage_sphere: true,
    },
    Archetype {
        name: "HP DPS",
        primary: &["HP_", "CRIT Rate_", "CRIT DMG_"],
        secondary: &["SPD"],
        mainstats: &["HP", "SPD", "CRIT Rate", "CRIT DMG"],
        damage_sphere: true,
    },
    Archetype {
        name: "DEF DPS",
        primary: &["DEF_", "CRIT Rate_", "CRIT DMG_"],
        secondary: &["SPD"],
        mainstats: &["DEF", "SPD", "CRIT Rate", "CRIT DMG"],
        damage_sphere: true,
    },
    Archetype {
        name: "Break DPS",
        primary: &["Break Effect_", "SPD"],
        secondary: &["ATK_", "Effect RES_"],
        mainstats: &["ATK", "HP", "DEF", "SPD", "Break Effect"],
        damage_sphere: false,
    },
    Archetype {
        name: "Hybrid Break/Crit",
        primary: &["Break Effect_", "CRIT Rate_", "CRIT DMG_"],
        secondary: &["SPD", "ATK_", "HP_", "DEF_"],
        mainstats: &[
            "ATK",
            "HP",
            "DEF",
            "SPD",
            "Break Effect",
            "CRIT Rate",
            "CRIT DMG",
        ],
        damage_sphere: true,
    },
    Archetype {
        name: "DoT DPS",
        primary: &["ATK_", "SPD", "Effect Hit Rate_"],
        secondary: &["Break Effect_"],
        mainstats: &["ATK", "SPD", "Effect Hit Rate"],
        damage_sphere: true,
    },
    Archetype {
        name: "Speed Support",
        primary: &["SPD", "HP_", "DEF_"],
        secondary: &["Effect RES_", "Break Effect_"],
        mainstats: &["HP", "DEF", "ATK", "SPD", "Energy Regeneration Rate"],
        damage_sphere: false,
    },
    Archetype {
        name: "Buff Support",
        primary: &["SPD", "CRIT DMG_"],
        secondary: &["HP_", "DEF_", "Effect RES_"],
        mainstats: &[
            "HP",
            "DEF",
            "ATK",
            "SPD",
            "CRIT DMG",
            "Energy Regeneration Rate",
        ],
        damage_sphere: false,
    },
    Archetype {
        name: "Debuff Support",
        primary: &["Effect Hit Rate_", "SPD"],
        secondary: &["HP_", "DEF_", "Effect RES_"],
        mainstats: &[
            "HP",
            "DEF",
            "SPD",
            "Effect Hit Rate",
            "Energy Regeneration Rate",
        ],
        damage_sphere: false,
    },
    Archetype {
        name: "HP Healer",
        primary: &["HP_", "SPD", "Effect RES_"],
        secondary: &["DEF_"],
        mainstats: &[
            "HP",
            "SPD",
            "Outgoing Healing Boost",
            "Energy Regeneration Rate",
        ],
        damage_sphere: false,
    },
    Archetype {
        name: "ATK Healer",
        primary: &["ATK_", "SPD", "Effect RES_"],
        secondary: &["HP_", "DEF_"],
        mainstats: &[
            "ATK",
            "SPD",
            "Outgoing Healing Boost",
            "Energy Regeneration Rate",
        ],
        damage_sphere: false,
    },
    Archetype {
        name: "Shield Sustain",
        primary: &["DEF_", "SPD", "Effect RES_"],
        secondary: &["HP_", "Effect Hit Rate_"],
        mainstats: &["DEF", "HP", "SPD", "Energy Regeneration Rate"],
        damage_sphere: false,
    },
    Archetype {
        name: "Hybrid Sustain/Break",
        primary: &["HP_", "DEF_", "Break Effect_"],
        secondary: &["SPD", "Effect RES_"],
        mainstats: &[
            "HP",
            "DEF",
            "SPD",
            "Break Effect",
            "Outgoing Healing Boost",
            "Energy Regeneration Rate",
        ],
        damage_sphere: false,
    },
];

#[derive(Clone, Copy, Debug)]
pub struct Synergy {
    pub name: &'static str,
    pub stats: &'static [&'static str],
}
pub const SYNERGIES: [Synergy; 10] = [
    Synergy {
        name: "Dual CRIT",
        stats: &["CRIT Rate_", "CRIT DMG_"],
    },
    Synergy {
        name: "ATK/Crit",
        stats: &["ATK_", "CRIT Rate_", "CRIT DMG_"],
    },
    Synergy {
        name: "HP/Crit",
        stats: &["HP_", "CRIT Rate_", "CRIT DMG_"],
    },
    Synergy {
        name: "DEF/Crit",
        stats: &["DEF_", "CRIT Rate_", "CRIT DMG_"],
    },
    Synergy {
        name: "Fast Break",
        stats: &["SPD", "Break Effect_"],
    },
    Synergy {
        name: "Fast debuffs",
        stats: &["SPD", "Effect Hit Rate_"],
    },
    Synergy {
        name: "Fast DoT/ATK",
        stats: &["SPD", "ATK_"],
    },
    Synergy {
        name: "Durable support",
        stats: &["SPD", "HP_", "DEF_"],
    },
    Synergy {
        name: "Fast HP sustain",
        stats: &["SPD", "HP_", "Effect RES_"],
    },
    Synergy {
        name: "Fast DEF sustain",
        stats: &["SPD", "DEF_", "Effect RES_"],
    },
];

#[derive(Clone, Copy, Debug)]
pub struct SetProfile {
    pub id: &'static str,
    pub name: &'static str,
    pub effect: &'static str,
    pub affinities: &'static [&'static str],
}
// Reviewed subset, intentionally not guessed for newer/special mechanics. No hard exclusions.
pub const SETS: [SetProfile; 18] = [
    SetProfile {
        id: "101",
        name: "Passerby of Wandering Cloud",
        effect: "Outgoing healing +10%; one skill point at the start of battle.",
        affinities: &["HP Healer", "ATK Healer"],
    },
    SetProfile {
        id: "102",
        name: "Musketeer of Wild Wheat",
        effect: "ATK +12%; four pieces add SPD +6% and Basic ATK damage +10%.",
        affinities: &["Crit DPS", "ATK DPS", "DoT DPS", "ATK Healer"],
    },
    SetProfile {
        id: "103",
        name: "Knight of Purity Palace",
        effect: "DEF +15%; four pieces increase shield absorption by 20%.",
        affinities: &["DEF DPS", "Shield Sustain"],
    },
    SetProfile {
        id: "106",
        name: "Guard of Wuthering Snow",
        effect: "Damage taken -8%; HP and energy recovery at turn start when HP is at most 50%.",
        affinities: &["Speed Support", "HP Healer", "Shield Sustain"],
    },
    SetProfile {
        id: "111",
        name: "Thief of Shooting Meteor",
        effect: "Break Effect +16%; four pieces add another 16% and energy on Weakness Break.",
        affinities: &["Break DPS", "Hybrid Break/Crit", "Hybrid Sustain/Break"],
    },
    SetProfile {
        id: "113",
        name: "Longevous Disciple",
        effect: "HP +12%; four pieces grant Crit Rate after being hit or consuming HP, up to two stacks.",
        affinities: &["HP DPS", "HP Healer"],
    },
    SetProfile {
        id: "114",
        name: "Messenger Traversing Hackerspace",
        effect: "SPD +6%; four pieces grant team SPD after an Ultimate targeting allies.",
        affinities: &["Speed Support", "Buff Support", "HP Healer", "ATK Healer"],
    },
    SetProfile {
        id: "116",
        name: "Prisoner in Deep Confinement",
        effect: "ATK +12%; four pieces ignore DEF based on DoTs on the target.",
        affinities: &["DoT DPS", "ATK DPS", "Crit DPS"],
    },
    SetProfile {
        id: "118",
        name: "Watchmaker, Master of Dream Machinations",
        effect: "Break Effect +16%; four pieces grant a team Break buff after an Ultimate targeting allies.",
        affinities: &["Break DPS", "Hybrid Sustain/Break", "Buff Support"],
    },
    SetProfile {
        id: "119",
        name: "Iron Cavalry Against the Scourge",
        effect: "Break Effect +16%; four pieces ignore DEF for Break/Super Break at 150/250% Break Effect.",
        affinities: &["Break DPS", "Hybrid Break/Crit"],
    },
    SetProfile {
        id: "301",
        name: "Space Sealing Station",
        effect: "ATK +12%; another ATK +12% at 120 SPD.",
        affinities: &["ATK DPS", "Crit DPS", "DoT DPS", "ATK Healer"],
    },
    SetProfile {
        id: "302",
        name: "Fleet of the Ageless",
        effect: "HP +12%; team ATK buff at 120 SPD.",
        affinities: &["Speed Support", "HP Healer", "Buff Support"],
    },
    SetProfile {
        id: "303",
        name: "Pan-Cosmic Commercial Enterprise",
        effect: "Effect Hit Rate +10%; additional ATK based on Effect Hit Rate.",
        affinities: &["DoT DPS", "Debuff Support"],
    },
    SetProfile {
        id: "304",
        name: "Belobog of the Architects",
        effect: "DEF +15%; another DEF +15% at 50% Effect Hit Rate.",
        affinities: &["Shield Sustain", "DEF DPS", "Debuff Support"],
    },
    SetProfile {
        id: "306",
        name: "Inert Salsotto",
        effect: "Crit Rate +8%; increased Ultimate and Follow-Up ATK damage at 50% Crit Rate.",
        affinities: &["Crit DPS", "ATK DPS", "HP DPS", "DEF DPS"],
    },
    SetProfile {
        id: "307",
        name: "Talia: Kingdom of Banditry",
        effect: "Break Effect +16%; another 20% at 145 SPD.",
        affinities: &["Break DPS", "Hybrid Break/Crit", "Hybrid Sustain/Break"],
    },
    SetProfile {
        id: "308",
        name: "Sprightly Vonwacq",
        effect: "Energy regeneration +5%; action advance at battle start with 120 SPD.",
        affinities: &["Speed Support", "Buff Support", "Debuff Support"],
    },
    SetProfile {
        id: "310",
        name: "Broken Keel",
        effect: "Effect RES +10%; team Crit DMG buff at 30% Effect RES.",
        affinities: &[
            "Speed Support",
            "Buff Support",
            "HP Healer",
            "Shield Sustain",
        ],
    },
];
pub fn set(id: &str) -> Option<&'static SetProfile> {
    SETS.iter().find(|s| s.id == id)
}

/// All thresholds used in category decisions are data in this registry.
#[derive(Clone, Debug)]
pub enum Condition {
    Protected,
    ExplicitProtection,
    StrongSynergy {
        minimum: usize,
    },
    StrongDeveloped {
        score: f64,
        core: usize,
    },
    Speed {
        minimum: f64,
    },
    UncertainData,
    UnknownSet,
    RecentSet,
    RareMainstat,
    Invested {
        level: u8,
    },
    OpenPotential {
        probability: f64,
        p90: f64,
    },
    PlausibleSynergy,
    FutureProfile {
        minimum: usize,
    },
    SafeCandidate {
        max_current: f64,
        max_p90: f64,
        max_probability: f64,
        min_dominators: usize,
        max_core: usize,
    },
    Fallback,
}
#[derive(Clone, Debug)]
pub struct Rule {
    pub id: &'static str,
    pub name: &'static str,
    pub priority: u16,
    pub category: Category,
    pub condition: Condition,
    pub description: &'static str,
    pub scope: &'static str,
    pub example: &'static str,
    pub exceptions: &'static str,
}
pub fn registry() -> Vec<Rule> {
    use Category::*;
    vec![
        Rule {
            id: "K001",
            name: "Existing protection",
            priority: 1000,
            category: Keep,
            condition: Condition::Protected,
            description: "Locked or equipped relics remain Keep. An existing discard flag is not evidence of quality.",
            scope: "All relics with lock or a nonempty location.",
            example: "A weak equipped +0 relic remains Keep.",
            exceptions: "Manual Safe Discard is forbidden; a manual Potentially Useful override is allowed.",
        },
        Rule {
            id: "K002",
            name: "Explicit user protection",
            priority: 990,
            category: Keep,
            condition: Condition::ExplicitProtection,
            description: "UIDs listed in protected_uids remain Keep.",
            scope: "Configuration field protected_uids.",
            example: "A sentimental relic is kept regardless of its score.",
            exceptions: "Manual Safe Discard is not allowed.",
        },
        Rule {
            id: "K003",
            name: "Especially versatile synergy",
            priority: 900,
            category: Keep,
            condition: Condition::StrongSynergy { minimum: 2 },
            description: "At least two detected synergies in the known stat pool protect valuable combinations. Preview values count as potential, not as current values.",
            scope: "SYNERGIES; all sets, including unknown sets.",
            example: "Crit Rate + Crit DMG + HP% provides both Dual CRIT and HP/Crit.",
            exceptions: "The main stat must be plausible for at least one archetype; score alone cannot trigger this rule.",
        },
        Rule {
            id: "K004",
            name: "Developed quality",
            priority: 890,
            category: Keep,
            condition: Condition::StrongDeveloped {
                score: 5.0,
                core: 2,
            },
            description: "A plausible archetype with at least two primary stats and sufficient current weighted roll equivalents is kept.",
            scope: "Current substats; all compatible archetypes.",
            example: "A developed HP/Crit relic remains valuable even on an ATK set.",
            exceptions: "Preview stats and theoretically perfect upgrades do not contribute to the current score.",
        },
        Rule {
            id: "K005",
            name: "High universal speed",
            priority: 880,
            category: Keep,
            condition: Condition::Speed { minimum: 7.0 },
            description: "A high current SPD substat protects regardless of set affinity.",
            scope: "Current SPD substat.",
            example: "7.2 SPD on an unusual set remains Keep.",
            exceptions: "SPD main stats are protected by P004; preview SPD is not added.",
        },
        Rule {
            id: "P001",
            name: "Data uncertainty",
            priority: 800,
            category: PotentiallyUseful,
            condition: Condition::UncertainData,
            description: "Unknown stats/slots, implausible values, incomplete roll metadata, rerolls, other rarities or unverified export versions prevent Safe Discard.",
            scope: "All data checks described under data model and limitations.",
            example: "step=8 with count=1 or an unknown stat key blocks discard.",
            exceptions: "Higher-priority Keep rules remain valid; structurally unreadable relics cancel the entire import.",
        },
        Rule {
            id: "P002",
            name: "Missing set coverage",
            priority: 790,
            category: PotentiallyUseful,
            condition: Condition::UnknownSet,
            description: "Sets outside the reviewed catalog are never automatically discarded. Their names alone do not establish known effects.",
            scope: "Set ID is not listed in SETS.",
            example: "A new Elation set remains protected until it has been reviewed.",
            exceptions: "Strong relics may already be Keep; generic archetypes are still displayed.",
        },
        Rule {
            id: "P003",
            name: "New sets",
            priority: 780,
            category: PotentiallyUseful,
            condition: Condition::RecentSet,
            description: "A configured verified release date protects through new_set_protection_days after release, inclusive. Future or invalid dates also protect. Missing dates are not guessed; P002 covers incompletely reviewed sets.",
            scope: "verified_release_dates, analysis_date, new_set_protection_days (default 90).",
            example: "A 30-day-old set with 90 days of protection remains at least Potentially Useful.",
            exceptions: "0 disables time-based protection; a missing date makes no age claim and leaves catalog/data rules in effect.",
        },
        Rule {
            id: "P004",
            name: "Specialized main stat",
            priority: 770,
            category: PotentiallyUseful,
            condition: Condition::RareMainstat,
            description: "Specialized main stats and all elemental damage spheres remain protected. This is a qualitative protection list, not a claimed drop probability.",
            scope: "RARE_MAINSTATS and DAMAGE_MAINSTATS.",
            example: "An energy rope with two flat stats is not automatically discarded.",
            exceptions: "Other main stats are not evidence for discard; Head HP and Hands ATK are fixed flat main stats.",
        },
        Rule {
            id: "P005",
            name: "Investment protection",
            priority: 760,
            category: PotentiallyUseful,
            condition: Condition::Invested { level: 12 },
            description: "Highly upgraded relics remain at least Potentially Useful.",
            scope: "All sets and rarities at or above the rule level.",
            example: "A +15 relic with weak rolls is retained for manual review.",
            exceptions: "Manual decisions remain available; automatic discard recommendations are blocked.",
        },
        Rule {
            id: "P006",
            name: "Realistic upgrade potential",
            priority: 750,
            category: PotentiallyUseful,
            condition: Condition::OpenPotential {
                probability: 0.05,
                p90: 4.5,
            },
            description: "At least one plausible archetype with sufficient conditional probability of a good score or a sufficient 90th percentile protects the relic. Missing forecasts are protected by P001.",
            scope: "All main-stat-compatible archetypes; forecasts are conditional on a known pool.",
            example: "A +0 Crit relic with useful additional stats remains protected.",
            exceptions: "The theoretical maximum alone cannot force Keep; the forecast explicitly assumes uniform distributions.",
        },
        Rule {
            id: "P007",
            name: "Plausible synergy",
            priority: 740,
            category: PotentiallyUseful,
            condition: Condition::PlausibleSynergy,
            description: "Every detected synergy prevents automatic discard.",
            scope: "SYNERGIES, including the preview pool.",
            example: "SPD + Break Effect remains protected even with two flat stats.",
            exceptions: "A synergy does not add a score bonus; multiple synergies are covered by K003.",
        },
        Rule {
            id: "P008",
            name: "Additional future profile",
            priority: 730,
            category: PotentiallyUseful,
            condition: Condition::FutureProfile { minimum: 2 },
            description: "An additional user profile protects when the main stat/set matches and at least two target stats are present. Profiles can add protection but cannot remove baseline archetypes.",
            scope: "future_profiles; empty main-stat/set lists allow all.",
            example: "A later verified HP/Break profile protects HP% + Break Effect.",
            exceptions: "No assumptions about Aha or other unverified character kits.",
        },
        Rule {
            id: "D001",
            name: "Conservative discard candidate",
            priority: 100,
            category: SafeDiscard,
            condition: Condition::SafeCandidate {
                max_current: 2.5,
                max_p90: 4.5,
                max_probability: 0.05,
                min_dominators: 3,
                max_core: 1,
            },
            description: "Only when ALL protection rules fail: low best current score, low good forecast outcome, low conditional good-outcome probability, at most one primary stat in every plausible archetype, and multiple strictly dominating alternatives. All conditions apply together; a total score alone never decides.",
            scope: "Only valid five-star relics with a reviewed set, a fully known pool and a complete forecast.",
            example: "A weak relic without synergies may be expendable when three alternatives are better in every stat/potential dimension.",
            exceptions: "Locked, equipped, unknown, specialized main stats, level +12 or higher, and relevant future synergies prevent this recommendation. There is no discard-count target.",
        },
        Rule {
            id: "P999",
            name: "Conservative fallback",
            priority: 0,
            category: PotentiallyUseful,
            condition: Condition::Fallback,
            description: "When no other evidence is sufficient, the relic remains potentially useful.",
            scope: "All remaining cases.",
            example: "A lack of dominating alternatives prevents discard even with a weak score.",
            exceptions: "No mathematical guarantee for future game updates; an intentionally conservative heuristic.",
        },
    ]
}

pub struct Evidence<'a> {
    pub relic: &'a Relic,
    pub config: &'a Config,
    pub uncertain: bool,
    pub synergy_count: usize,
    pub archetypes: &'a [ArchetypeResult],
    pub dominators: usize,
}

impl Condition {
    pub fn matches(&self, e: &Evidence<'_>) -> bool {
        let plausible: Vec<_> = e
            .archetypes
            .iter()
            .filter(|a| a.mainstat_compatible)
            .collect();
        match self {
            Self::Protected => e.relic.protected(),
            Self::ExplicitProtection => e.config.protected_uids.contains(&e.relic.uid),
            Self::StrongSynergy { minimum } => !plausible.is_empty() && e.synergy_count >= *minimum,
            Self::StrongDeveloped { score, core } => plausible
                .iter()
                .any(|a| a.projection.current >= *score && a.relevant_stats >= *core),
            Self::Speed { minimum } => e
                .relic
                .substats
                .iter()
                .any(|s| s.key == "SPD" && s.value >= *minimum),
            Self::UncertainData => e.uncertain || plausible.is_empty(),
            Self::UnknownSet => set(&e.relic.set_id).is_none(),
            Self::RecentSet => recent_set(e.relic, e.config),
            Self::RareMainstat => {
                RARE_MAINSTATS.contains(&e.relic.mainstat.as_str())
                    || DAMAGE_MAINSTATS.contains(&e.relic.mainstat.as_str())
            }
            Self::Invested { level } => e.relic.level >= *level,
            Self::OpenPotential { probability, p90 } => plausible.iter().any(|a| {
                a.projection
                    .probability_good
                    .is_some_and(|v| v >= *probability)
                    || a.projection.good_p90.is_some_and(|v| v >= *p90)
            }),
            Self::PlausibleSynergy => e.synergy_count > 0,
            Self::FutureProfile { minimum } => e.config.future_profiles.iter().any(|p| {
                (p.mainstats.is_empty() || p.mainstats.contains(&e.relic.mainstat))
                    && (p.set_ids.is_empty() || p.set_ids.contains(&e.relic.set_id))
                    && e.relic
                        .known_pool()
                        .iter()
                        .filter(|s| p.stats.contains(&s.key))
                        .count()
                        >= *minimum
            }),
            Self::SafeCandidate {
                max_current,
                max_p90,
                max_probability,
                min_dominators,
                max_core,
            } => {
                !e.uncertain
                    && set(&e.relic.set_id).is_some()
                    && !plausible.is_empty()
                    && e.dominators >= *min_dominators
                    && plausible.iter().all(|a| {
                        a.projection.current < *max_current
                            && a.relevant_stats <= *max_core
                            && a.projection.good_p90.is_some_and(|v| v < *max_p90)
                            && a.projection
                                .probability_good
                                .is_some_and(|v| v < *max_probability)
                    })
            }
            Self::Fallback => true,
        }
    }
}

pub fn decide(e: &Evidence<'_>) -> (Category, Vec<String>, String) {
    let rules = registry();
    let mut hits: Vec<_> = rules.iter().filter(|r| r.condition.matches(e)).collect();
    hits.sort_by_key(|r| std::cmp::Reverse(r.priority));
    // A discard predicate is not reported as triggered when any stronger protective rule applies.
    if hits
        .iter()
        .any(|r| r.priority > 100 && r.condition.matches(e))
    {
        hits.retain(|r| r.category != Category::SafeDiscard);
    }
    let first = hits[0]; // P999 always matches.
    let ids = hits
        .iter()
        .filter(|r| r.id != "P999" || first.id == "P999")
        .map(|r| r.id.to_owned())
        .collect();
    (
        first.category,
        ids,
        format!("{}: {}", first.name, first.description),
    )
}

pub fn parse_date(text: &str) -> Option<time::Date> {
    let fmt = time::format_description::parse_borrowed::<2>("[year]-[month]-[day]").ok()?;
    time::Date::parse(text, &fmt).ok()
}
pub fn validate_config(config: &Config) -> Result<(), String> {
    if parse_date(&config.analysis_date).is_none() {
        return Err("Analysis date must use YYYY-MM-DD.".into());
    }
    for p in &config.future_profiles {
        if p.name.trim().is_empty() || p.stats.iter().any(|s| stat(s).is_none()) {
            return Err("Future profiles require a name and known substat keys.".into());
        }
    }
    Ok(())
}
fn recent_set(relic: &Relic, config: &Config) -> bool {
    if config.new_set_protection_days == 0 {
        return false;
    }
    let Some(release) = config.verified_release_dates.get(&relic.set_id) else {
        return false;
    };
    match (parse_date(release), parse_date(&config.analysis_date)) {
        (Some(release), Some(today)) => {
            (today - release).whole_days() <= i64::from(config.new_set_protection_days)
        }
        _ => true,
    }
}

pub fn mainstat_subkey(relic: &Relic) -> String {
    if (relic.slot == "Head" && relic.mainstat == "HP")
        || (relic.slot == "Hands" && relic.mainstat == "ATK")
        || relic.mainstat == "SPD"
    {
        relic.mainstat.clone()
    } else {
        format!("{}_", relic.mainstat)
    }
}

pub fn data_issues(relic: &Relic) -> Vec<String> {
    let mut out = Vec::new();
    if relic.rarity != 5 {
        out.push(
            "V1 models upgrades only for five-star relics; other rarities are protected.".into(),
        );
    }
    if relic.level > MAX_LEVEL
        || relic.rarity == 0
        || relic.rarity > 5
        || relic.level > relic.rarity.saturating_mul(EVENT_INTERVAL)
    {
        out.push("Implausible rarity or upgrade level.".into());
    }
    if !MAINSTATS
        .iter()
        .any(|(slot, mains)| *slot == relic.slot && mains.contains(&relic.mainstat.as_str()))
    {
        out.push("Unknown slot/main-stat combination.".into());
    }
    let pool = relic.known_pool();
    if pool.len() != POOL_SIZE {
        out.push("The full pool of four distinct stats is not known.".into());
    }
    let mut keys = std::collections::BTreeSet::new();
    let forbidden = mainstat_subkey(relic);
    for s in &pool {
        if !keys.insert(&s.key) || s.key == forbidden {
            out.push(format!(
                "Duplicate substat or a substat identical to the main stat: {}.",
                s.key
            ));
        }
        if !s.value.is_finite() || s.value <= 0.0 {
            out.push("Nonpositive or nonfinite stat value.".into());
        }
        match (stat(&s.key), s.count, s.step) {
            (Some(cfg), Some(count), Some(step))
                if count > 0 && count <= 9 && step <= 2 * count =>
            {
                if relic.rarity == 5
                    && (s.value - (f64::from(count) * cfg.base + f64::from(step) * cfg.step)).abs()
                        > VALUE_TOLERANCE
                {
                    out.push(format!("{} conflicts with count/step/base value.", s.key));
                }
            }
            _ => out.push(format!(
                "{}: unknown stat or incomplete/implausible roll metadata.",
                s.key
            )),
        }
    }
    if relic.rarity == 5 && relic.level <= MAX_LEVEL {
        let events = relic.level / EVENT_INTERVAL;
        let count: u64 = relic
            .substats
            .iter()
            .map(|s| u64::from(s.count.unwrap_or(0)))
            .sum();
        if count != 3 + u64::from(events) && count != 4 + u64::from(events) {
            out.push(
                "The sum of current roll counts conflicts with the level and a 3/4-stat start."
                    .into(),
            );
        }
        if !(3..=4).contains(&relic.substats.len())
            || (events > 0 && relic.substats.len() != POOL_SIZE)
        {
            out.push(
                "The number of current substats conflicts with the five-star upgrade model.".into(),
            );
        }
    }
    if relic
        .preview_substats
        .iter()
        .flatten()
        .any(|s| s.count != Some(1))
    {
        out.push(
            "Preview entries are not single new-stat rolls; their semantics are uncertain.".into(),
        );
    }
    if relic
        .reroll_substats
        .as_ref()
        .is_some_and(|s| !s.is_empty())
    {
        out.push("A reroll alternative is present; it is kept separate from current stats and protected from discard.".into());
    }
    out.sort();
    out.dedup();
    out
}

pub fn markdown() -> String {
    let mut out = format!(
        "# RelicSift scoring rules\n\nGenerated from `src/analysis/rules.rs`. Do not edit by hand.\n\nRule version: **{VERSION}**. Regenerate with `cargo run --bin sync_rules`. The explicit local documentation test requires a byte-for-byte match. The docs directory is intentionally untracked.\n\n## Decision order\n\nThe first matching rule in descending priority determines the automatic category. Additional matches are evidence, not extra decisions. Protection has priority over D001. Scores are weighted roll equivalents, not percentage quality or simulated damage.\n\n"
    );
    out.push_str(&format!("Implementation fingerprint: `{}`. Changes to the rules, calculations, import validator or configuration model require regeneration. The fingerprint does not replace semantic tests.\n\n## {} — {}\n\n- Category: explicitly chosen by the user\n- Priority: {} (applied after automatic analysis)\n- Description, scope and protection: {}\n- Structured parameters: `{:?}`\n- Affected fields: individual UID, lock, location, protected_uids, manual reason.\n- Example: manually move a kept relic to Potentially Useful with a reason.\n- Exception: a protected UID cannot be manually marked Safe Discard.\n\n", implementation_fingerprint(), MANUAL_POLICY.id, MANUAL_POLICY.name, MANUAL_POLICY.priority, MANUAL_POLICY.description, MANUAL_POLICY));
    for rule in registry() {
        out.push_str(&format!("## {} — {}\n\n- Category: **{}**\n- Priority: {}\n- Description and effect: {}\n- Scope: {}\n- Actual condition and parameters: `{:?}`\n- Example: {}\n- Exceptions and protection: {}\n\n", rule.id, rule.name, rule.category.label(), rule.priority, rule.description, rule.scope, rule.condition, rule.example, rule.exceptions));
    }
    out.push_str(&format!("## Data model and calculation parameters\n\nForecasts cover five-star relics only: maximum level {MAX_LEVEL}; one event every {EVENT_INTERVAL} levels; {POOL_SIZE} substats; {TIER_COUNT} quality tiers. Current roll counts must equal 3 or 4 starting rolls plus floor(level/{EVENT_INTERVAL}). Missing stat slots are unlocked before improvement rolls. Preview entries are known pending stats, not current stats. Reroll alternatives remain separate.\n\nExport formula: value = count × base + step × step_value. Percentage values use percentage points. count includes the initial contribution; step sums quality tiers from 0 through 2 × count. Formula tolerance: {VALUE_TOLERANCE}. Missing metadata prevents discard. Structural parse errors and empty/duplicate IDs cancel the entire import. IDs remain strings. Automatic discard is verified only for source=reliquary_archiver, build=0.19.0, version=4. Other builds/versions import with protection. Extra fields and exact original bytes remain preserved.\n\n### Conditional upgrade forecast\n\nThe distribution is exact **under an explicitly stated model assumption**: with four known stats, each improvement independently selects a uniform target stat and one of {TIER_COUNT} uniform quality tiers. These are **not verified server probabilities**. Value tiers come from game-data tables. Fixed preview entries are unlocked first. Unknown future stats are not fabricated: the forecast is null and P001 protects. Expected value, quantile {GOOD_QUANTILE}, theoretical maximum, and probability of at least {GOOD_SCORE} points are calculated separately for each archetype. Complete convolution is used, not random Monte Carlo sampling. Increments are rounded to 10^-6 points for numerical keys; this is precision, not a decision weight. No remaining upgrades yields a single-point distribution.\n\n### Scores and archetypes\n\nScore = stat value / average five-star roll value × stat weight. Primary weight {PRIMARY_WEIGHT}; secondary weight {SECONDARY_WEIGHT}; flat weight {FLAT_WEIGHT}; all others 0. Set affinity multiplies by {AFFINITY_WEIGHT}, otherwise {NEUTRAL_SET_WEIGHT}. There are no hard set exclusions or additive synergy bonuses. Head and Hands are compatible with every archetype. Other main stats must appear in the profile; elemental spheres require damage_sphere=true. Primary-stat counts use the known pool; current scores use current stats only. Profiles are generic heuristic models, not verified character builds.\n\n"));
    out.push_str("| Archetype | Primary stats | Secondary stats | Main stats excluding Head/Hands | Elemental sphere |\n|---|---|---|---|---|\n");
    for a in ARCHETYPES {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            a.name,
            a.primary.join(", "),
            a.secondary.join(", "),
            a.mainstats.join(", "),
            a.damage_sphere
        ));
    }
    out.push_str("\n### Synergies (all listed stats required)\n\n");
    for s in SYNERGIES {
        out.push_str(&format!("- {}: {}.\n", s.name, s.stats.join(" + ")));
    }
    out.push_str(&format!(
        "\n### Main stats\n\nSpecialized protection list: {}. Also protected: {}.\n\n",
        RARE_MAINSTATS.join(", "),
        DAMAGE_MAINSTATS.join(", ")
    ));
    for (slot, mains) in MAINSTATS {
        out.push_str(&format!("- {slot}: {}.\n", mains.join(", ")));
    }
    out.push_str(
        "\n### Five-star roll values (export units)\n\n| Stat | Base | Step |\n|---|---:|---:|\n",
    );
    for s in STATS {
        out.push_str(&format!("| {} | {} | {} |\n", s.key, s.base, s.step));
    }
    out.push_str("\n### Reviewed set catalog\n\nAffinities are conservative heuristic mappings, not claims that activation conditions are met in a team. Set bonuses are not added as active combat stats. Other archetypes remain possible. No release dates are supplied or guessed. New and complex sets outside this reviewed subset remain protected.\n\n| ID | Set | Verified effect summary | Affine profiles |\n|---|---|---|---|\n");
    for s in SETS {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            s.id,
            s.name,
            s.effect,
            s.affinities.join(", ")
        ));
    }
    out.push_str(&format!("\n## Multidimensional dominance\n\nComparisons require identical set, slot, main stat, rarity **and level**, fully valid data and the same known substat keys. Every current stat value, known-pool value and every archetype's expected value, 90th percentile and theoretical maximum must be at least as good. At least one actual stat/pool value must exceed the other by more than {DOMINANCE_EPSILON}. Flat and percentage stats stay separate. Equality is not dominance. Different stat pools, levels or incomplete data are not compared. All strictly dominating UIDs are reported without a top-N limit. Dominance alone never determines a category. Buckets limit comparisons to functionally matching relics.\n\n## Evidence and limitations\n\nNo dependency on an account or existing character build. No claims about unreleased kits or unknown set effects. An existing discard flag does not lower quality. Model expectations are not guarantees. Missing information yields at least Potentially Useful. Manual decisions remain distinct from automatic recommendations. Configuration and input/rule fingerprints are stored. The engine reads neither the clock nor the network; the analysis date is explicit configuration.\n\n## Sources\n\n- [Reliquary Archiver export model](https://github.com/IceDynamix/reliquary-archiver/blob/main/src/export/fribbels/models.rs) and [export formula](https://github.com/IceDynamix/reliquary-archiver/blob/main/src/export/fribbels/converters.rs), checked 2026-10-18.\n- [Dimbreath RelicSubAffixConfig](https://gitlab.com/Dimbreath/turnbasedgamedata/-/blob/main/ExcelOutput/RelicSubAffixConfig.json): numerical roll bases and tiers, checked 2026-10-18.\n- [StarRailRes relic sets](https://github.com/Mar-7th/StarRailRes/blob/master/index_new/en/relic_sets.json): set IDs and effect references, checked 2026-10-18.\n- [Fribbels HSR Optimizer](https://fribbels.github.io/hsr-optimizer/) and [HSR Scanner Discarder](https://github.com/ashleney/HSR-Scanner-Discarder): technical references; no scoring thresholds or game automation copied.\n"));
    out
}
