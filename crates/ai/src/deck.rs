//! AI decks (SPEC §9.9, R184, R186, R380): a curve-aware, tag-aware random draw of distinct non-token
//! cards of every set, minus the shadow ban.
//!
//! The draw is weighted sampling without replacement. Every remaining card gets a weight that is the
//! product of the boosts in AI_DECK: its cost bucket is under or over its curve target, the deck is
//! still short of units, the card carries the deck's theme, or the seat could never cast it. Two
//! floors are hard rather than weighted: once the slots left equal the units (or theme cards) still
//! owed, only units (or theme cards) are eligible, so every deck meets `minUnitShare` and every themed
//! deck meets `themeMinShare` whatever the rng does. Everything comes from the rng passed in, so the
//! same seed deals the same deck in any process.
//!
//! Port of `packages/ai/src/deck.ts` (SURFACE §9: `build_ai_deck(&mut Rng, i32, &AiDeckOptions)`). TS
//! threw on a bad request; this panics with the same message.

use std::cmp::Ordering;
use std::ops::{Index, IndexMut};

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::config::MAX_MANA;
use jackioh_engine::{CardDef, CardType, CatalogQueryArgs, Rng, Tag, query, query_cost};
use serde::{Deserialize, Deserializer, Serialize};

use crate::shadow_ban::SHADOW_BAN_IDS;

/// `"0-1" | "2" | "3" | "4+"`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CostBucket {
    #[serde(rename = "0-1")]
    ZeroOne,
    #[serde(rename = "2")]
    Two,
    #[serde(rename = "3")]
    Three,
    #[serde(rename = "4+")]
    FourPlus,
}

impl CostBucket {
    /// The literal TS wrote.
    pub fn as_str(self) -> &'static str {
        match self {
            CostBucket::ZeroOne => "0-1",
            CostBucket::Two => "2",
            CostBucket::Three => "3",
            CostBucket::FourPlus => "4+",
        }
    }
}

/// `Record<CostBucket, T>`: one value per cost bucket, serialised `{ "0-1": …, "2": …, "3": …, "4+": … }`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
pub struct ByBucket<T> {
    #[serde(rename = "0-1")]
    pub zero_one: T,
    #[serde(rename = "2")]
    pub two: T,
    #[serde(rename = "3")]
    pub three: T,
    #[serde(rename = "4+")]
    pub four_plus: T,
}

impl<T> Index<CostBucket> for ByBucket<T> {
    type Output = T;

    fn index(&self, bucket: CostBucket) -> &T {
        match bucket {
            CostBucket::ZeroOne => &self.zero_one,
            CostBucket::Two => &self.two,
            CostBucket::Three => &self.three,
            CostBucket::FourPlus => &self.four_plus,
        }
    }
}

impl<T> IndexMut<CostBucket> for ByBucket<T> {
    fn index_mut(&mut self, bucket: CostBucket) -> &mut T {
        match bucket {
            CostBucket::ZeroOne => &mut self.zero_one,
            CostBucket::Two => &mut self.two,
            CostBucket::Three => &mut self.three,
            CostBucket::FourPlus => &mut self.four_plus,
        }
    }
}

/// R390: `boost`'s shape — these ids' weights are multiplied by `by`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AiDeckBoost {
    pub ids: Vec<String>,
    pub by: f64,
}

/// TS's `theme?: string | null` has three states: absent (roll one), `null` (none), a tag. Absent is
/// `None`; a present value, `null` included, is `Some`.
fn present<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(deserializer).map(Some)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AiDeckOptions {
    /// Default SHADOW_BAN_IDS; pass `Some(vec![])` for a human's random deck.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banned: Option<Vec<String>>,
    /// Ids forced in (the sweep); must be non-token and not banned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<String>>,
    /// A tag to lean on; `None` (TS undefined) = roll one (AI_DECK.themeChance), `Some(None)` (TS null) = none.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present")]
    pub theme: Option<Option<String>>,
    /// The seat's handicap manaCap; default MAX_MANA. Shifts the curve and the uncastable test.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mana_cap: Option<i32>,
    /// R390: these ids' weights are multiplied by `by` (the sweep's pass 2, as `themeBoost` leans a theme).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boost: Option<AiDeckBoost>,
}

/// `AI_DECK`'s shape.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiDeck {
    /// Share of each bucket at manaCap 4; each crystal above 4 moves `curveShiftPerMana` from "0-1" to "4+".
    pub curve: ByBucket<f64>,
    pub curve_shift_per_mana: f64,
    /// Allowed gap between a bucket's mean share over many seeds and its target.
    pub curve_tolerance: f64,
    pub min_unit_share: f64,
    /// Chance of rolling a theme when `theme` is undefined.
    pub theme_chance: f64,
    /// A tag needs this many cards in the pool to be a theme.
    pub min_theme_size: i32,
    pub theme_boost: f64,
    pub theme_min_share: f64,
    /// Weight for a card whose bucket is under target / already full.
    pub curve_boost: f64,
    pub curve_overflow: f64,
    /// Weight for a Unit while units < ceil(size × minUnitShare).
    pub unit_boost: f64,
    /// Weight for a card with queryCost > manaCap + costSlack.
    pub uncastable: f64,
    pub cost_slack: i32,
    /// The unbanned pool must hold at least this many cards.
    pub min_pool: i32,
}

pub const AI_DECK: AiDeck = AiDeck {
    curve: ByBucket { zero_one: 0.3, two: 0.33, three: 0.22, four_plus: 0.15 },
    curve_shift_per_mana: 0.025,
    curve_tolerance: 0.08,
    min_unit_share: 0.45,
    theme_chance: 0.35,
    min_theme_size: 6,
    theme_boost: 4.0,
    theme_min_share: 0.3,
    curve_boost: 3.0,
    curve_overflow: 0.15,
    unit_boost: 2.5,
    uncastable: 0.05,
    cost_slack: 1,
    min_pool: 45,
};

/// The buckets in curve order; the last one, "4+", takes every cost above the others' ceilings.
const BUCKET_ORDER: [CostBucket; 4] = [CostBucket::ZeroOne, CostBucket::Two, CostBucket::Three, CostBucket::FourPlus];

/// The highest queryCost each bucket below "4+" holds, in curve order.
const BUCKET_CEILINGS: [(CostBucket, i32); 3] = [(CostBucket::ZeroOne, 1), (CostBucket::Two, 2), (CostBucket::Three, 3)];

/// The tag every token carries; never a theme (the pool holds no tokens anyway, §2.6 L3).
const TOKEN_TAG: Tag = Tag::Token;

/// By queryCost (X → 0, embiggen → base).
pub fn cost_bucket(def: &CardDef) -> CostBucket {
    let cost = query_cost(def);
    for (bucket, ceiling) in BUCKET_CEILINGS {
        if cost <= ceiling {
            return bucket;
        }
    }
    CostBucket::FourPlus
}

/// The four target shares for a seat's mana cap, summing to 1.
fn curve_shares(mana_cap: i32) -> ByBucket<f64> {
    let mut shares = AI_DECK.curve;
    let wanted = f64::from((mana_cap - MAX_MANA).max(0)) * AI_DECK.curve_shift_per_mana;
    let moved = wanted.min(shares.zero_one);
    shares.zero_one -= moved;
    shares.four_plus += moved;
    shares
}

/// Integers summing to size (largest remainder).
pub fn curve_targets(size: i32, mana_cap: i32) -> ByBucket<i32> {
    struct Remainder {
        bucket: CostBucket,
        fraction: f64,
        order: usize,
    }
    let shares = curve_shares(mana_cap);
    let mut targets: ByBucket<i32> = ByBucket { zero_one: 0, two: 0, three: 0, four_plus: 0 };
    let mut remainders: Vec<Remainder> = Vec::new();
    let mut assigned = 0;
    for (order, bucket) in BUCKET_ORDER.into_iter().enumerate() {
        let raw = shares[bucket] * f64::from(size);
        let whole = raw.floor();
        targets[bucket] = whole as i32;
        assigned += whole as i32;
        remainders.push(Remainder { bucket, fraction: raw - whole, order });
    }
    // Largest fractional part first; a tie goes to the cheaper bucket, so the result is a total order.
    remainders.sort_by(|a, b| {
        if a.fraction != b.fraction {
            b.fraction.partial_cmp(&a.fraction).unwrap_or(Ordering::Equal)
        } else {
            a.order.cmp(&b.order)
        }
    });
    let mut i = 0usize;
    while assigned < size && !remainders.is_empty() {
        let Some(entry) = remainders.get(i) else {
            break;
        };
        targets[entry.bucket] += 1;
        assigned += 1;
        i = (i + 1) % remainders.len();
    }
    targets
}

/// The tags with at least AI_DECK.minThemeSize cards among `defs`, sorted by name.
fn theme_candidates(defs: &[&CardDef]) -> Vec<String> {
    let mut counts: IndexMap<String, i32> = IndexMap::new();
    for def in defs {
        for tag in &def.tags {
            if *tag == TOKEN_TAG {
                continue;
            }
            *counts.entry(tag.as_str().to_string()).or_insert(0) += 1;
        }
    }
    let mut themes: Vec<String> = counts
        .into_iter()
        .filter(|(_, count)| *count >= AI_DECK.min_theme_size)
        .map(|(tag, _)| tag)
        .collect();
    themes.sort();
    themes
}

/// One index into `weights`, chosen with probability proportional to its weight. TS answered
/// `weights.length - 1` past the end, -1 for an empty list: here `None`. The draw is taken either way.
fn weighted_index(weights: &[f64], rng: &mut Rng) -> Option<usize> {
    let total = weights.iter().fold(0.0, |sum, weight| sum + weight);
    let roll = rng.next() * total;
    let mut cumulative = 0.0;
    for (i, weight) in weights.iter().enumerate() {
        cumulative += weight;
        if roll < cumulative {
            return Some(i);
        }
    }
    weights.len().checked_sub(1)
}

fn has_tag(def: &CardDef, tag: &str) -> bool {
    def.tags.iter().any(|t| t.as_str() == tag)
}

fn is_unit(def: &CardDef) -> bool {
    def.type_ == CardType::Unit
}

/// The running tallies `count` keeps (TS's closure over `counts`, `units` and `themed`).
struct Tally<'a> {
    counts: ByBucket<i32>,
    units: i32,
    themed: i32,
    theme: Option<&'a str>,
}

impl Tally<'_> {
    fn count(&mut self, def: &CardDef) {
        self.counts[cost_bucket(def)] += 1;
        if is_unit(def) {
            self.units += 1;
        }
        if let Some(theme) = self.theme
            && has_tag(def, theme)
        {
            self.themed += 1;
        }
    }
}

/// Distinct non-token ids of every set, exactly `size`, deterministic for the rng. Panics (TS threw)
/// if the pool is too small.
pub fn build_ai_deck(rng: &mut Rng, size: i32, options: &AiDeckOptions) -> Vec<String> {
    let banned: IndexSet<String> = match &options.banned {
        Some(banned) => banned.iter().cloned().collect(),
        None => SHADOW_BAN_IDS.iter().map(|id| id.to_string()).collect(),
    };
    let include: &[String] = options.include.as_deref().unwrap_or(&[]);
    let mana_cap = options.mana_cap.unwrap_or(MAX_MANA);

    // `query` never returns tokens unless asked, so this is §2.6 L3's deck-legal pool of every set
    // (R184, R380).
    let every: Vec<&CardDef> = query(&CatalogQueryArgs::default());
    let by_id: IndexMap<&str, &CardDef> = every.iter().map(|def| (def.id.as_str(), *def)).collect();

    let mut include_defs: Vec<&CardDef> = Vec::new();
    let mut include_ids: IndexSet<String> = IndexSet::new();
    for id in include {
        let Some(def) = by_id.get(id.as_str()).copied() else {
            panic!("buildAiDeck: include \"{id}\" is not a non-token card");
        };
        if banned.contains(id) {
            panic!("buildAiDeck: include \"{id}\" is banned");
        }
        if include_ids.contains(id) {
            panic!("buildAiDeck: include \"{id}\" is listed twice");
        }
        include_ids.insert(id.clone());
        include_defs.push(def);
    }
    if include_defs.len() as i32 > size {
        panic!("buildAiDeck: {} included cards do not fit a {size}-card deck", include_defs.len());
    }

    let pool: Vec<&CardDef> =
        every.iter().copied().filter(|def| !banned.contains(&def.id) && !include_ids.contains(&def.id)).collect();
    if ((include_defs.len() + pool.len()) as i32) < size {
        panic!(
            "buildAiDeck: a {size}-card deck needs {size} distinct cards, but only {} non-token cards are unbanned",
            include_defs.len() + pool.len()
        );
    }

    let theme: Option<String> = match &options.theme {
        None => {
            // The chance is always rolled, so the draws after it sit at the same cursor whatever the pool.
            let rolled = rng.chance(AI_DECK.theme_chance);
            let mut both: Vec<&CardDef> = include_defs.clone();
            both.extend(pool.iter().copied());
            let candidates = theme_candidates(&both);
            if rolled && !candidates.is_empty() { rng.pick(&candidates).cloned() } else { None }
        }
        Some(theme) => theme.clone(),
    };

    let targets = curve_targets(size, mana_cap);
    let units_needed = (f64::from(size) * AI_DECK.min_unit_share).ceil() as i32;
    let theme_needed = if theme.is_none() { 0 } else { (f64::from(size) * AI_DECK.theme_min_share).ceil() as i32 };
    let castable_ceiling = mana_cap + AI_DECK.cost_slack;
    let boosted: IndexSet<String> = options.boost.as_ref().map(|boost| boost.ids.iter().cloned().collect()).unwrap_or_default();
    let boost_by = options.boost.as_ref().map_or(1.0, |boost| boost.by);

    let mut deck: Vec<&CardDef> = include_defs.clone();
    let mut tally = Tally {
        counts: ByBucket { zero_one: 0, two: 0, three: 0, four_plus: 0 },
        units: 0,
        themed: 0,
        theme: theme.as_deref(),
    };
    for def in &deck {
        tally.count(def);
    }

    let mut remaining: Vec<&CardDef> = pool;
    while (deck.len() as i32) < size {
        let slots_left = size - deck.len() as i32;

        let mut eligible: Vec<&CardDef> = remaining.clone();
        if let Some(theme) = theme.as_deref()
            && theme_needed - tally.themed >= slots_left
        {
            let themed_cards: Vec<&CardDef> = eligible.iter().copied().filter(|def| has_tag(def, theme)).collect();
            if !themed_cards.is_empty() {
                eligible = themed_cards;
            }
        }
        if units_needed - tally.units >= slots_left {
            let unit_cards: Vec<&CardDef> = eligible.iter().copied().filter(|def| is_unit(def)).collect();
            if !unit_cards.is_empty() {
                eligible = unit_cards;
            }
        }

        let weights: Vec<f64> = eligible
            .iter()
            .map(|def| {
                let mut weight = 1.0;
                let bucket = cost_bucket(def);
                weight *= if tally.counts[bucket] < targets[bucket] { AI_DECK.curve_boost } else { AI_DECK.curve_overflow };
                if is_unit(def) && tally.units < units_needed {
                    weight *= AI_DECK.unit_boost;
                }
                if let Some(theme) = theme.as_deref()
                    && has_tag(def, theme)
                {
                    weight *= AI_DECK.theme_boost;
                }
                if query_cost(def) > castable_ceiling {
                    weight *= AI_DECK.uncastable;
                }
                if boosted.contains(&def.id) {
                    weight *= boost_by;
                }
                weight
            })
            .collect();

        let Some(chosen) = weighted_index(&weights, rng).and_then(|at| eligible.get(at).copied()) else {
            panic!("buildAiDeck: ran out of cards at {} of {size}", deck.len());
        };
        deck.push(chosen);
        tally.count(chosen);
        remaining.retain(|def| def.id != chosen.id);
    }

    deck.into_iter().map(|def| def.id.clone()).collect()
}
