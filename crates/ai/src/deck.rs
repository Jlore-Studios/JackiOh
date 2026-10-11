//! AI decks (SPEC §9.9, R184, R186, R380): a curve-aware, tag-aware random draw of distinct non-token
//! cards of every set, minus the shadow ban.
//!
//! The draw is weighted sampling without replacement. Every remaining card gets a weight that is the
//! product of the boosts in AI_DECK: its cost bucket is under or over its curve target, the deck is
//! still short of units, the card carries the deck's theme, the deck is still short of the set it
//! leans on (R1370), or the seat could never cast it. The floors are hard rather than weighted. A deck
//! that leans on no set has the two it always had: once the slots left equal the theme cards (then the
//! units) still owed, only theme cards (then the units among them) are eligible, so every themed deck
//! meets `themeMinShare` and every deck `minUnitShare` that its theme leaves room for, and every seed
//! deals the deck it always dealt. A deck that leans on a set keeps three together (R1370): once the
//! fewest cards that would meet the leaned set's `leanMinShare`, the theme's and the units' need every
//! slot left, only a card that some fewest way of meeting them takes is eligible, so all three hold
//! whatever the rng does whenever the cards left can meet them; a floor short of cards takes every card
//! it has, and when the slots cannot meet all three the leaned set goes before the theme before the
//! units. Everything comes from the rng passed in, so the same seed deals the same deck in any process,
//! and the lean adds no rng draw.
//!
//! The soft gate (R1390): the random draw never deals a card of a set in `gated_sets`, whose default is
//! `AI_DECK_GATED_SETS` (Meditative until the AI is trained on it), whatever ships or a thread
//! previews; a player's random deck passes `Some(vec![])` and is dealt from every set, and a lean on a
//! gated set leans on the newest set that ships and is not gated. An empty gate deals exactly what the
//! draw dealt before it.
//!
//! Port of `packages/ai/src/deck.ts` (SURFACE §9: `build_ai_deck(&mut Rng, i32, &AiDeckOptions)`). TS
//! threw on a bad request; this panics with the same message.

use std::cmp::Ordering;
use std::ops::{Index, IndexMut};

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::config::MAX_MANA;
use jackioh_engine::{
    CardDef, CardType, CatalogQueryArgs, Rng, SHIPPED_SETS, SetName, Tag, query, query_cost,
};
use serde::{Deserialize, Deserializer, Serialize};

use crate::config::AI_DECK_GATED_SETS;
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
    /// R1370: a set the deck leans on: at least `ceil(size × AI_DECK.leanMinShare)` of its cards come
    /// from it (a hard floor, as the theme's), and its cards weigh `leanBoost` more while the deck is
    /// short of them. `None`, the default, leans on nothing and deals exactly the deck it always did.
    ///
    /// The issue's `newest_share` (#549) is this field. It names the set rather than switching on "the
    /// newest set", because a caller may lean on a set that has not shipped (a tool previewing it,
    /// R1420); "More cards from the newest set" (R1372, R1373) passes `newest_shipped_set()` (R1371).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lean_set: Option<SetName>,
    /// R1390: the sets the random draw never deals from. Default `AI_DECK_GATED_SETS`, the AI's own
    /// decks' soft gate; pass `Some(vec![])`, as `banned`, for a player's random deck. An `include`
    /// still reaches a gated card (the sweep forces one in by name), and a `lean_set` the gate holds
    /// leans on `ungated_lean`'s set instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gated_sets: Option<Vec<SetName>>,
}

/// R1390: the set a deck leans on under the gate `gated`: `lean` itself when the gate leaves it, else
/// the newest set that ships and is not gated (Classic+ while Meditative is gated, before and after
/// Meditative ships), else none.
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
    // R1370 (#549): "at least half the deck", 10 of 20 and 15 of a Hard deck's 30.
    lean_min_share: 0.5,
    // R1370, chosen by measurement (`r1370_the_lean_boost_seldom_leaves_the_floor_to_force_a_card`
    // holds it). Classic+ is 78 of the 268 cards (29%), so with no boost a deck that leans on it would
    // hold about 6 of 20 by the draw's own weights, and the floor forces the rest in at the end, into
    // nine decks in ten. Over 1,000 seeds per weight and size (a human's 20 cards, Easy's 20, Medium's
    // 25 at cap 5 and Hard's 30 at cap 7), the floor had to force a card into a sixth of the decks or
    // more at 4 and below, 10–14% at 5, 7–10% at 6 and 6–8% at 8. 6 is the smallest of those weights at
    // which it forces a card into fewer than one deck in ten at every size (under a third of a card per
    // deck); 8 forces little less and deals more of the set past the floor (11.9 of 20, 15.1 of 25 and
    // 17.6 of 30, against 11.5, 14.8 and 17.2 at 6). The boost stops once the floor is met, so the rest
    // is dealt as without it.
    lean_boost: 6.0,
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

/// R1370: whether a card is of the set the deck leans on.
fn is_leaned(def: &CardDef, lean: Option<SetName>) -> bool {
    lean.is_some_and(|set| def.set == set)
}

/// The running tallies `count` keeps (TS's closure over `counts`, `units` and `themed`), and the
/// leaned set's (R1370).
struct Tally<'a> {
    counts: ByBucket<i32>,
    units: i32,
    themed: i32,
    leaned: i32,
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
    }
}

/// The narrowing a deck that leans on no set has always had, kept exactly so every seed still deals
/// the deck it dealt: once the theme cards still owed reach the slots left only theme cards are
/// eligible, and once the units still owed do, only the units among those (when there are any).
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

/// R1370: when the floors cannot all be kept, the floors a leaning deck still keeps, most first: the
/// leaned set before the theme before the units (the theme already outranked the units, above).
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

/// R1370: the fewest cards that still meet every count in `owed` (the leaned set's, the theme's and
/// the units', in `FLOOR_BITS` order), drawing at most `avail[class]` cards of each class; `None` when
/// no number of them can. Exact: a card of all three classes is never worse than any other card, so
/// as many of those as can help are taken first; then every count of the two pairs that serve the
/// leaned set is tried, and the pair and the single cards that serve the theme and the units are
/// worked out from what is left.
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

/// R1370: the cards the next pick of a leaning deck may take. A floor whose cards are fewer than it
/// owes owes only those, so it takes all it has. The floors it keeps are then the first set in
/// `FLOORS_KEPT` that the slots left can still meet; while they can be met with a slot to spare, every
/// card may be taken, and once they need every slot left, only a card of a class that some fewest way
/// of meeting them takes, so the floors stay within reach to the last card. "Narrow to the cards that
/// meet every floor owed, else to as many floors as can be met", made exact: a card that meets two
/// floors is taken over one that meets one only when the slots left need it, and a floor is given up
/// only when the slots left cannot meet it beside the floors before it.
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

/// `build_ai_deck`'s deck and how many of its picks the leaned set's floor forced (R1370): a pick
/// whose eligible cards were narrowed to the set's alone, because the slots left had come down to the
/// cards the floors still owed. The measurement `AI_DECK.leanBoost` was chosen by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TracedAiDeck {
    pub deck: Vec<String>,
    pub lean_forced: i32,
}

/// Distinct non-token ids of every set, exactly `size`, deterministic for the rng. Panics (TS threw)
/// if the pool is too small.
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
    // R1370: rounded up, so a handicap's 25-card deck owes 13 and a 30-card one 15. R1390: never on a
    // gated set.
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
    // The unban lane's deck shaping (shadow_ban.rs): exactly the seat an arena or a promotion deals
    // for this AI — `banned` set to this AI's own SHADOW_BAN_IDS under a handicap's `mana_cap`, no
    // `include` or `boost` — gets its weights multiplied by the dealt-quality the lane's records
    // measured. A seat under any other options (the parent's own list, a human's none, a test's or
    // the sweep's own) deals as before. The mana cap is what keeps "a human's none" honest once the
    // ban list is empty: a player's random deck passes `banned: []` with no cap, which an empty
    // SHADOW_BAN_IDS would otherwise set-match like this AI's own.
    let shape = options.include.is_none()
        && options.boost.is_none()
        && options.mana_cap.is_some()
        && options.banned.as_ref().is_some_and(|listed| {
            listed.len() == SHADOW_BAN_IDS.len()
                && SHADOW_BAN_IDS
                    .iter()
                    .all(|id| listed.iter().any(|entry| entry == id))
        });

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
