//! AI decks (SPEC §9.9, SURFACE §9, R184, R186, R380): curve- and tag-aware weighted random draw of non-token
//! cards without replacement, meeting hard floors for themes, unit counts, and set leans (R1370).
//!
//! - Soft gate (R1390): excludes sets in `gated_sets` (default `AI_DECK_GATED_SETS`, #1884, #1885, #732).
//! - Quickdraw cap (R817000): caps Quickdraw cards at `max_quickdraw`.
//! - Strength (R1454, #750): weights cards by compiled win rate.
//! - Shaping (R976000): unban lane dealt-quality shaping via `shaped_weight` (`shadow_ban.rs`).

use std::cmp::Ordering;
use std::ops::{Index, IndexMut};

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::config::MAX_MANA;
use jackioh_engine::{
    CardDef, CardType, CatalogQueryArgs, Rng, SHIPPED_SETS, SetName, Tag, query, query_cost,
    registered_win_rates,
};
use serde::{Deserialize, Deserializer, Serialize};

use crate::config::{AI_DECK_GATED_SETS, AI_WIN_RATE};
use crate::shadow_ban::{SHADOW_BAN_IDS, shaped_weight};

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
    pub fn as_str(self) -> &'static str {
        match self {
            CostBucket::ZeroOne => "0-1",
            CostBucket::Two => "2",
            CostBucket::Three => "3",
            CostBucket::FourPlus => "4+",
        }
    }
}

/// One value per cost bucket.
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

/// Deserializes optional nullable string: absent -> None, null -> Some(None), string -> Some(Some(s)).
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
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub theme: Option<Option<String>>,
    /// The seat's handicap manaCap; default MAX_MANA. Shifts the curve and the uncastable test.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mana_cap: Option<i32>,
    /// R390: these ids' weights are multiplied by `by` (the sweep's pass 2, as `themeBoost` leans a theme).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boost: Option<AiDeckBoost>,
    /// R1370: set the deck leans on (`AI_DECK.leanMinShare` floor, `leanBoost`).
    /// Named set allows previewing unreleased sets (R1420); newest set options (R1372, R1373) pass `newest_shipped_set()` (R1371).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lean_set: Option<SetName>,
    /// R1390: soft-gated sets excluded from random draw (default `AI_DECK_GATED_SETS`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gated_sets: Option<Vec<SetName>>,
    /// #750 (R1454): weight scaling by compiled win rate from 0.0 to 1.0 (`AI_WIN_RATE`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strength: Option<f64>,
    /// R976000: multiply weights by unban lane's dealt-quality table `shaped_weight` (`shadow_ban.rs`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shape: bool,
}

/// R1390: the set a deck leans on under the gate `gated`: `lean` itself when the gate leaves it, else
/// the newest set that ships and is not gated (Classic+ under a gate on Meditative), else none.
pub fn ungated_lean(lean: Option<SetName>, gated: &[SetName]) -> Option<SetName> {
    let set = lean?;
    if !gated.contains(&set) {
        return Some(set);
    }
    SHIPPED_SETS
        .iter()
        .rev()
        .copied()
        .find(|shipped| !gated.contains(shipped))
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
    /// R1370: a deck that leans on a set holds at least ceil(size × leanMinShare) of its cards.
    pub lean_min_share: f64,
    /// R1370: weight for a card of the leaned set while the deck is short of `leanMinShare`.
    pub lean_boost: f64,
    /// R817000: a random deck holds at most this many Quickdraw cards, and Quickdraw is never a theme.
    pub max_quickdraw: i32,
}

pub const AI_DECK: AiDeck = AiDeck {
    curve: ByBucket {
        zero_one: 0.3,
        two: 0.33,
        three: 0.22,
        four_plus: 0.15,
    },
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
    // R1370 (#549): "at least half the deck", 13 of a human's 25 and 18 of a Hard deck's 35 (R1705000).
    lean_min_share: 0.5,
    // R1370: measured weight ensuring set cards are drawn naturally rather than forced by floor
    // across pool sizes (#1187, #1705). Boost stops once floor is met.
    lean_boost: 48.0,
    // R817000 (#817): "less Quickdraw bloat".
    max_quickdraw: 2,
};

/// The buckets in curve order; the last one, "4+", takes every cost above the others' ceilings.
const BUCKET_ORDER: [CostBucket; 4] = [
    CostBucket::ZeroOne,
    CostBucket::Two,
    CostBucket::Three,
    CostBucket::FourPlus,
];

/// The highest queryCost each bucket below "4+" holds, in curve order.
const BUCKET_CEILINGS: [(CostBucket, i32); 3] = [
    (CostBucket::ZeroOne, 1),
    (CostBucket::Two, 2),
    (CostBucket::Three, 3),
];

/// The tag every token carries; never a theme (the pool holds no tokens anyway, §2.6 L3).
const TOKEN_TAG: Tag = Tag::Token;

/// R817000: the tag a random deck holds at most `AI_DECK.maxQuickdraw` cards of, so never a theme.
const QUICKDRAW_TAG: Tag = Tag::Quickdraw;

/// R2511001: the Training Grounds Legacy cards' tag marks copies of Core cards, not a family that
/// plays together, so it is never a theme.
const LEGACY_TAG: Tag = Tag::Legacy;

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
    let mut targets: ByBucket<i32> = ByBucket {
        zero_one: 0,
        two: 0,
        three: 0,
        four_plus: 0,
    };
    let mut remainders: Vec<Remainder> = Vec::new();
    let mut assigned = 0;
    for (order, bucket) in BUCKET_ORDER.into_iter().enumerate() {
        let raw = shares[bucket] * f64::from(size);
        let whole = raw.floor();
        targets[bucket] = whole as i32;
        assigned += whole as i32;
        remainders.push(Remainder {
            bucket,
            fraction: raw - whole,
            order,
        });
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

/// The tags with at least AI_DECK.minThemeSize cards among `defs`, sorted by name; never Token,
/// Quickdraw (R817000) or Legacy (R2511001).
fn theme_candidates(defs: &[&CardDef]) -> Vec<String> {
    let mut counts: IndexMap<String, i32> = IndexMap::new();
    for def in defs {
        for tag in &def.tags {
            if *tag == TOKEN_TAG || *tag == QUICKDRAW_TAG || *tag == LEGACY_TAG {
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

/// R817000: whether a card counts toward `AI_DECK.maxQuickdraw`.
fn is_quickdraw(def: &CardDef) -> bool {
    def.tags.contains(&QUICKDRAW_TAG)
}

/// R1370: whether a card is of the set the deck leans on.
fn is_leaned(def: &CardDef, lean: Option<SetName>) -> bool {
    lean.is_some_and(|set| def.set == set)
}

/// Running tallies for cost buckets, units, theme, leaned set (R1370), and Quickdraw (R817000).
struct Tally<'a> {
    counts: ByBucket<i32>,
    units: i32,
    themed: i32,
    leaned: i32,
    quickdraw: i32,
    theme: Option<&'a str>,
    lean: Option<SetName>,
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
        if is_leaned(def, self.lean) {
            self.leaned += 1;
        }
        if is_quickdraw(def) {
            self.quickdraw += 1;
        }
    }
}

/// Narrowing for unleaned decks: forces theme and unit cards when slots left match owed counts.
fn narrow_unleaned<'a>(
    remaining: &[&'a CardDef],
    slots_left: i32,
    tally: &Tally<'_>,
    theme_needed: i32,
    units_needed: i32,
) -> Vec<&'a CardDef> {
    let mut eligible: Vec<&CardDef> = remaining.to_vec();
    if let Some(theme) = tally.theme
        && theme_needed - tally.themed >= slots_left
    {
        let themed_cards: Vec<&CardDef> = eligible
            .iter()
            .copied()
            .filter(|def| has_tag(def, theme))
            .collect();
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
    eligible
}

/// R1370: a leaning deck's three floors as bits of a card's class: the leaned set's, the theme's and
/// the units'. A class is the floors a card counts toward, 0 to 7.
const LEAN_BIT: usize = 1;
const THEME_BIT: usize = 2;
const UNIT_BIT: usize = 4;
const FLOOR_BITS: [usize; 3] = [LEAN_BIT, THEME_BIT, UNIT_BIT];
const CLASSES: usize = 8;

/// R1370: fallback priority for active floors when not all can be kept: lean set, then theme, then units.
const FLOORS_KEPT: [usize; 8] = [
    LEAN_BIT | THEME_BIT | UNIT_BIT,
    LEAN_BIT | THEME_BIT,
    LEAN_BIT | UNIT_BIT,
    LEAN_BIT,
    THEME_BIT | UNIT_BIT,
    THEME_BIT,
    UNIT_BIT,
    0,
];

fn class_of(def: &CardDef, theme: Option<&str>, lean: Option<SetName>) -> usize {
    let mut class = 0;
    if is_leaned(def, lean) {
        class |= LEAN_BIT;
    }
    if theme.is_some_and(|theme| has_tag(def, theme)) {
        class |= THEME_BIT;
    }
    if is_unit(def) {
        class |= UNIT_BIT;
    }
    class
}

/// R1370: fewest cards meeting all counts in `owed` given class availability (`avail`); `None` if unreachable.
fn min_slots(owed: [i32; 3], avail: &[i32; CLASSES]) -> Option<i32> {
    let [lean, theme, unit] = owed.map(|n| n.max(0));
    let all = avail[LEAN_BIT | THEME_BIT | UNIT_BIT].min(lean.max(theme).max(unit));
    let (lean, theme, unit) = ((lean - all).max(0), (theme - all).max(0), (unit - all).max(0));
    let mut best: Option<i32> = None;
    for with_theme in 0..=avail[LEAN_BIT | THEME_BIT].min(lean.max(theme)) {
        for with_unit in 0..=avail[LEAN_BIT | UNIT_BIT].min(lean.max(unit)) {
            let lean_only = (lean - with_theme - with_unit).max(0);
            if lean_only > avail[LEAN_BIT] {
                continue;
            }
            let theme_left = (theme - with_theme).max(0);
            let unit_left = (unit - with_unit).max(0);
            // Theme units first, as many as save a card, but enough that the single cards suffice.
            let fewest = (theme_left - avail[THEME_BIT])
                .max(unit_left - avail[UNIT_BIT])
                .max(0);
            let most = avail[THEME_BIT | UNIT_BIT].min(theme_left.max(unit_left));
            if fewest > most {
                continue;
            }
            let both = theme_left.min(unit_left).clamp(fewest, most);
            let cost = with_theme
                + with_unit
                + lean_only
                + both
                + (theme_left - both).max(0)
                + (unit_left - both).max(0);
            best = Some(best.map_or(cost, |best| best.min(cost)));
        }
    }
    best.map(|cost| all + cost)
}

/// `owed` with the floors outside `kept` owing nothing.
fn owed_within(owed: [i32; 3], kept: usize) -> [i32; 3] {
    let mut within = [0; 3];
    for (at, bit) in FLOOR_BITS.into_iter().enumerate() {
        if kept & bit != 0 {
            within[at] = owed[at].max(0);
        }
    }
    within
}

/// R1370: filters eligible cards for leaning deck to preserve feasibility of remaining floors in `FLOORS_KEPT`.
/// Restricts picks when slots left equal minimum cards required across active floors.
fn narrow_leaning<'a>(
    remaining: &[&'a CardDef],
    slots_left: i32,
    owed: [i32; 3],
    theme: Option<&str>,
    lean: Option<SetName>,
) -> Vec<&'a CardDef> {
    let classes: Vec<usize> = remaining.iter().map(|def| class_of(def, theme, lean)).collect();
    let mut avail = [0; CLASSES];
    for class in &classes {
        avail[*class] += 1;
    }
    // A floor short of cards owes only the cards it has left, so it takes every one of them.
    let mut owed = owed;
    for (at, bit) in FLOOR_BITS.into_iter().enumerate() {
        let left: i32 = (0..CLASSES)
            .filter(|class| class & bit != 0)
            .map(|class| avail[class])
            .sum();
        owed[at] = owed[at].max(0).min(left);
    }
    for kept in FLOORS_KEPT {
        let owed = owed_within(owed, kept);
        let Some(needed) = min_slots(owed, &avail) else {
            continue;
        };
        if needed > slots_left {
            continue;
        }
        if needed < slots_left {
            return remaining.to_vec();
        }
        let mut takes = [false; CLASSES];
        for (class, takes) in takes.iter_mut().enumerate() {
            if avail[class] == 0 {
                continue;
            }
            let mut after = avail;
            after[class] -= 1;
            let mut owed_after = owed;
            for (at, bit) in FLOOR_BITS.into_iter().enumerate() {
                if class & bit != 0 {
                    owed_after[at] = (owed_after[at] - 1).max(0);
                }
            }
            *takes = min_slots(owed_after, &after) == Some(needed - 1);
        }
        return remaining
            .iter()
            .zip(&classes)
            .filter(|(_, class)| takes[**class])
            .map(|(def, _)| *def)
            .collect();
    }
    remaining.to_vec()
}

/// #750: multiplies card weight by win-rate boost at `strength` (`AI_WIN_RATE`).
/// Unmatched cards or those under `min_games` default to 1.0.
fn win_rate_weight(id: &str, strength: f64) -> f64 {
    if strength <= 0.0 {
        return 1.0;
    }
    let Some(row) = registered_win_rates().cards.iter().find(|row| row.id == id) else {
        return 1.0;
    };
    if row.games < AI_WIN_RATE.min_games {
        return 1.0;
    }
    let rate = f64::from(row.wins) / f64::from(row.games);
    (1.0 + strength * AI_WIN_RATE.max_boost * (rate - 0.5) * 2.0)
        .clamp(AI_WIN_RATE.floor, 1.0 + AI_WIN_RATE.max_boost)
}

/// `build_ai_deck` result with count of picks forced by leaned set floor (R1370).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TracedAiDeck {
    pub deck: Vec<String>,
    pub lean_forced: i32,
}

/// Builds a deterministic AI deck of `size` cards; panics if pool is too small.
pub fn build_ai_deck(rng: &mut Rng, size: i32, options: &AiDeckOptions) -> Vec<String> {
    build_ai_deck_traced(rng, size, options).deck
}

/// `build_ai_deck`, with how many picks the leaned set's floor forced (R1370).
pub fn build_ai_deck_traced(rng: &mut Rng, size: i32, options: &AiDeckOptions) -> TracedAiDeck {
    let banned: IndexSet<String> = match &options.banned {
        Some(banned) => banned.iter().cloned().collect(),
        None => SHADOW_BAN_IDS.iter().map(|id| id.to_string()).collect(),
    };
    let include: &[String] = options.include.as_deref().unwrap_or(&[]);
    let mana_cap = options.mana_cap.unwrap_or(MAX_MANA);
    // R1390: the soft gate, the AI's own decks' unless the caller deals a player's deck.
    let gated: &[SetName] = options.gated_sets.as_deref().unwrap_or(AI_DECK_GATED_SETS);

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
        panic!(
            "buildAiDeck: {} included cards do not fit a {size}-card deck",
            include_defs.len()
        );
    }

    let pool: Vec<&CardDef> = every
        .iter()
        .copied()
        .filter(|def| {
            !banned.contains(&def.id) && !include_ids.contains(&def.id) && !gated.contains(&def.set)
        })
        .collect();
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
            if rolled && !candidates.is_empty() {
                rng.pick(&candidates).cloned()
            } else {
                None
            }
        }
        Some(theme) => theme.clone(),
    };

    let targets = curve_targets(size, mana_cap);
    let units_needed = (f64::from(size) * AI_DECK.min_unit_share).ceil() as i32;
    let theme_needed = if theme.is_none() {
        0
    } else {
        (f64::from(size) * AI_DECK.theme_min_share).ceil() as i32
    };
    // R1370: rounded up, so a human's 25-card deck owes 13, Medium's 30 15 and Hard's 35 18. R1390:
    // never on a gated set.
    let lean = ungated_lean(options.lean_set, gated);
    let lean_needed = if lean.is_none() {
        0
    } else {
        (f64::from(size) * AI_DECK.lean_min_share).ceil() as i32
    };
    let castable_ceiling = mana_cap + AI_DECK.cost_slack;
    let boosted: IndexSet<String> = options
        .boost
        .as_ref()
        .map(|boost| boost.ids.iter().cloned().collect())
        .unwrap_or_default();
    let boost_by = options.boost.as_ref().map_or(1.0, |boost| boost.by);
    // #750: 0.0 unless a ladder bot's deal asks, so every other deal is the one it always was.
    let strength = options
        .strength
        .filter(|strength| strength.is_finite())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    // R976000: the unban lane's deck shaping (shadow_ban.rs) only when the caller asks for it, never
    // because a ban list matches this AI's own, so a gate's two seats are dealt by one rule.
    let shape = options.shape;

    let mut deck: Vec<&CardDef> = include_defs.clone();
    let mut tally = Tally {
        counts: ByBucket {
            zero_one: 0,
            two: 0,
            three: 0,
            four_plus: 0,
        },
        units: 0,
        themed: 0,
        leaned: 0,
        quickdraw: 0,
        theme: theme.as_deref(),
        lean,
    };
    for def in &deck {
        tally.count(def);
    }

    let mut remaining: Vec<&CardDef> = pool;
    let mut lean_forced = 0;
    while (deck.len() as i32) < size {
        let slots_left = size - deck.len() as i32;
        // R817000: once the deck holds `maxQuickdraw` Quickdraw cards (its includes counted), no more are
        // dealt. Before that the draw is untouched, so a deck that would have held no more deals as it did.
        if tally.quickdraw >= AI_DECK.max_quickdraw {
            remaining.retain(|def| !is_quickdraw(def));
        }

        let eligible: Vec<&CardDef> = if lean.is_none() {
            narrow_unleaned(&remaining, slots_left, &tally, theme_needed, units_needed)
        } else {
            let owed = [
                lean_needed - tally.leaned,
                theme_needed - tally.themed,
                units_needed - tally.units,
            ];
            narrow_leaning(&remaining, slots_left, owed, theme.as_deref(), lean)
        };
        if eligible.len() < remaining.len() && eligible.iter().all(|def| is_leaned(def, lean)) {
            lean_forced += 1;
        }

        let weights: Vec<f64> = eligible
            .iter()
            .map(|def| {
                let mut weight = 1.0;
                let bucket = cost_bucket(def);
                weight *= if tally.counts[bucket] < targets[bucket] {
                    AI_DECK.curve_boost
                } else {
                    AI_DECK.curve_overflow
                };
                if is_unit(def) && tally.units < units_needed {
                    weight *= AI_DECK.unit_boost;
                }
                if let Some(theme) = theme.as_deref()
                    && has_tag(def, theme)
                {
                    weight *= AI_DECK.theme_boost;
                }
                // R1370: only while the deck is short of the set, so once the floor is met the rest
                // is dealt as without it.
                if is_leaned(def, lean) && tally.leaned < lean_needed {
                    weight *= AI_DECK.lean_boost;
                }
                if query_cost(def) > castable_ceiling {
                    weight *= AI_DECK.uncastable;
                }
                if boosted.contains(&def.id) {
                    weight *= boost_by;
                }
                if shape {
                    weight *= shaped_weight(&def.id);
                }
                weight *= win_rate_weight(&def.id, strength);
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

    TracedAiDeck {
        deck: deck.into_iter().map(|def| def.id.clone()).collect(),
        lean_forced,
    }
}
