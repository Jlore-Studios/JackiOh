//! The card catalog is static data shipped with the client (§9.4), so the engine reads it from a
//! registry rather than carrying it in GameState, which stays serializable. Fused and crafted
//! definitions live in `state.transientDefs` and win over the registry.
//!
//! Port of `packages/engine/src/catalog.ts`. Three changes of shape, none of behaviour (SURFACE §3,
//! §6.6, §8):
//!
//! - The registry is a `OnceLock`, set once by `register_catalog` (which `jackioh_cards::register_all`
//!   calls) and read-only after: a mutable static would let one game see another's cards when the
//!   tools run games on many threads. Under the `testkit` feature a thread-local override the testkit
//!   sets (`testkit::scenario::register_catalog`) is consulted first.
//! - Every reader that took TS's `TransientHolder` takes `Option<&GameState>`: the transient defs are
//!   the state's.
//! - The process-global digest table (TS `digestIngredients`, R468) is not ported, and with it
//!   `registerFusedIngredients`: a digest id's ingredients are read off the fused definition itself
//!   (`CardDef.ingredients`, which a Fuse always writes) in `state.transient_defs`. So the readers of a
//!   fused id's parts (`fused_id_specs`, `fused_id_parts`, `self_def_ids`, `excluding_def_id`) take the
//!   state too; without one, a digest id reads as no fused id, as an unknown digest did in TS.

use std::cmp::Ordering;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::config::{
    GLITCH_DEF_ID, GLITCH_ODDS_DENOMINATOR, GLITCH_ODDS_PER_SYSTEM_PLAY, GRAPE_ODDS, POOL_TOKEN_TAGS,
};
use crate::rng::Rng;
use crate::state::GameState;
use crate::wire::{
    CardCost, CardDef, CardDefs, CardType, CatalogQuery, CostRange, FusedIngredient, OneOrMany, Rarity,
    SHIPPED_SETS, SetName, Tag,
};

/// The registered catalog and its version (TS's two module `let`s, set together).
struct Registered {
    defs: CardDefs,
    version: String,
}

static REGISTERED: OnceLock<Registered> = OnceLock::new();

/// What `registered_catalog` answers before anything is registered (TS `let registered = {}`).
static EMPTY: OnceLock<CardDefs> = OnceLock::new();

/// SURFACE §8: the testkit's thread-local catalog, when the calling thread's test set one.
#[cfg(feature = "testkit")]
fn test_override() -> Option<&'static CardDefs> {
    crate::testkit::scenario::catalog_override()
}

#[cfg(not(feature = "testkit"))]
fn test_override() -> Option<&'static CardDefs> {
    None
}

/// The catalog every reader searches: a test's override, else the registered one, else none.
fn registered() -> &'static CardDefs {
    if let Some(defs) = test_override() {
        return defs;
    }
    match REGISTERED.get() {
        Some(registered) => &registered.defs,
        None => EMPTY.get_or_init(CardDefs::new),
    }
}

/// TS `registerCatalog(defs, catalogVersion = "test")`. Set once per process: a second call is
/// ignored (the registry is a `OnceLock`, SURFACE §3); a test that needs another catalog sets the
/// testkit's override instead (SURFACE §8).
pub fn register_catalog(defs: CardDefs, catalog_version: &str) {
    let _ = REGISTERED.set(Registered {
        defs,
        version: catalog_version.to_string(),
    });
}

pub fn registered_catalog() -> &'static CardDefs {
    registered()
}

/// Tokens and cards are also reachable by their §5 index ("43", "51.1", "T-rush") — within their set,
/// since an index is unique only there (B2.2: Core's "43" and Classic's "43" are two cards).
pub fn def_by_index(set: SetName, index: &str) -> Option<&'static CardDef> {
    registered()
        .values()
        .find(|def| def.set == set && def.index == index)
}

/// The registered catalog's version: "0" before one is registered, and TS's default "test" while a
/// test's override is in force.
pub fn catalog_version() -> &'static str {
    if test_override().is_some() {
        return "test";
    }
    match REGISTERED.get() {
        Some(registered) => &registered.version,
        None => "0",
    }
}

/// TS `TransientHolder = { transientDefs }`: what holds the transient definitions — the state.
pub type TransientHolder = GameState;

pub fn find_def<'a>(state: Option<&'a GameState>, def_id: &str) -> Option<&'a CardDef> {
    if let Some(def) = state.and_then(|state| state.transient_defs.get(def_id)) {
        return Some(def);
    }
    registered().get(def_id)
}

/// Panics when a def is missing: a card instance always has a definition.
pub fn def_of<'a>(state: Option<&'a GameState>, def_id: &str) -> &'a CardDef {
    match find_def(state, def_id) {
        Some(def) => def,
        None => panic!("unknown defId \"{def_id}\": register the catalog first"),
    }
}

// ---------------------------------------------------------------------------
// §5.1's one query function, and the cost every filter reads (R65).
// ---------------------------------------------------------------------------

/// R65 outside play: "an embiggen card's printed cost is its base price and an X-cost card's is 0".
/// Every pool, filter and comparison that looks at a *definition* rather than at a played instance
/// goes through this, so #7's cost brackets, #30's highest/lowest, #94's odd costs and Recruit all
/// read one number. The in-play calculation is `mana::effective_cost`, which starts from the instance.
pub fn query_cost(def: &CardDef) -> i32 {
    match def.cost {
        CardCost::X => 0,
        CardCost::Fixed(cost) => cost,
        CardCost::Embiggen { base, .. } => base,
    }
}

/// §5.1's `catalog.query({type, cost, costRange, tags, notTags, rarity, set, excludeDefId})`, plus the
/// identity fields a card needs to name a pool by hand. Every field is a narrowing filter, and `{}` is
/// every non-token card of every set (R380).
///
/// (TS `CatalogQuery & { defId?, token? }`: the wire query's fields written out beside the two of its
/// own, in that order, so a card writes the TS object literal as `json_as(json!({ … }))`.)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CatalogQueryArgs {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub type_: Option<OneOrMany<CardType>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_range: Option<CostRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_tags: Option<Vec<Tag>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rarity: Option<OneOrMany<Rarity>>,
    /// A set, or several ("Classic or Classic+"). Absent is every set (R380).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<OneOrMany<SetName>>,
    /// R387: never these definitions, by catalog id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude_def_id: Option<OneOrMany<String>>,
    /// R382: tokens may come out of this pool beside the cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_tokens: Option<bool>,
    /// A catalog id, or several, for a pool a script names card by card or builds from ids it holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub def_id: Option<OneOrMany<String>>,
    /// §5.1: `true` asks for tokens only, `false` forbids them (the default already does).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<bool>,
}

impl From<CatalogQuery> for CatalogQueryArgs {
    fn from(query: CatalogQuery) -> CatalogQueryArgs {
        CatalogQueryArgs {
            type_: query.type_,
            cost: query.cost,
            cost_range: query.cost_range,
            tags: query.tags,
            not_tags: query.not_tags,
            rarity: query.rarity,
            set: query.set,
            exclude_def_id: query.exclude_def_id,
            with_tokens: query.with_tokens,
            def_id: None,
            token: None,
        }
    }
}

fn as_list<T: PartialEq>(value: &Option<OneOrMany<T>>) -> &[T] {
    match value {
        Some(value) => value.as_slice(),
        None => &[],
    }
}

/// §5.1: "Random pools never include Token-tagged cards … unless the card names the pool itself".
/// A query names the token pool by asking for the Token tag or rarity, by setting `token: true`, or
/// by naming its members outright through `defId`. `withTokens` (R382) lets tokens in beside cards.
fn asks_for_tokens(args: &CatalogQueryArgs) -> bool {
    if let Some(token) = args.token {
        return token;
    }
    if args.with_tokens == Some(true) {
        return true;
    }
    if args.tags.as_ref().is_some_and(|tags| tags.contains(&Tag::Token)) {
        return true;
    }
    if as_list(&args.rarity).contains(&Rarity::Token) {
        return true;
    }
    !as_list(&args.def_id).is_empty()
}

fn is_token(def: &CardDef) -> bool {
    def.token || def.tags.contains(&Tag::Token)
}

/// R382: a pool named by one of `POOL_TOKEN_TAGS` (a Fruit pool) holds that tag's tokens too — the
/// five Grapes are Tokens "that can be generated by any Fruit card". No other pool reaches them.
fn tag_pool_takes_token(args: &CatalogQueryArgs, def: &CardDef) -> bool {
    let asked: &[Tag] = args.tags.as_deref().unwrap_or(&[]);
    asked
        .iter()
        .any(|tag| POOL_TOKEN_TAGS.contains(tag) && def.tags.contains(tag))
}

fn matches_query(def: &CardDef, args: &CatalogQueryArgs, tokens_allowed: bool) -> bool {
    // R674: Glitch is in no pool, not even one that takes every token; only R673's roll makes one.
    if def.id == GLITCH_DEF_ID && !as_list(&args.def_id).contains(&def.id) {
        return false;
    }
    if !tokens_allowed && is_token(def) && !tag_pool_takes_token(args, def) {
        return false;
    }
    if let Some(token) = args.token
        && is_token(def) != token
    {
        return false;
    }

    let types = as_list(&args.type_);
    if !types.is_empty() && !types.contains(&def.type_) {
        return false;
    }
    let rarities = as_list(&args.rarity);
    if !rarities.is_empty() && !rarities.contains(&def.rarity) {
        return false;
    }
    let sets = as_list(&args.set);
    if !sets.is_empty() && !sets.contains(&def.set) {
        return false;
    }

    let def_ids = as_list(&args.def_id);
    if !def_ids.is_empty() && !def_ids.contains(&def.id) {
        return false;
    }
    // R387, §5.1: a random pool never offers the card that generated it, named by its id.
    if as_list(&args.exclude_def_id).contains(&def.id) {
        return false;
    }

    // `tags` means "has every listed tag"; `notTags` means "has none of them".
    if let Some(tags) = &args.tags
        && !tags.iter().all(|tag| def.tags.contains(tag))
    {
        return false;
    }
    if let Some(not_tags) = &args.not_tags
        && not_tags.iter().any(|tag| def.tags.contains(tag))
    {
        return false;
    }

    // R65: costs are read out of play, so X counts as 0 and an embiggen card as its base price.
    let cost = query_cost(def);
    if let Some(wanted) = args.cost
        && cost != wanted
    {
        return false;
    }
    if let Some(min) = args.cost_range.and_then(|range| range.min)
        && cost < min
    {
        return false;
    }
    if let Some(max) = args.cost_range.and_then(|range| range.max)
        && cost > max
    {
        return false;
    }
    true
}

/// R77, R102: the catalog ids a definition stands for when it generates — its own id, or, for a fused
/// definition (R179's `t-<n>:<a>+<b>`), every ingredient's, its fused ingredients' included, so a
/// fused card never generates any card it was made from (B4.1 rule 2). A crafted definition with no
/// ingredients stands for itself, which no pool can reach anyway.
pub fn self_def_ids(state: Option<&GameState>, def_id: &str) -> Vec<String> {
    match fused_id_parts(state, def_id) {
        None => vec![def_id.to_string()],
        Some(ingredients) => ingredients
            .iter()
            .flat_map(|id| self_def_ids(state, id))
            .collect(),
    }
}

/// R179: the ingredient ids a fused id (`t-<n>:<a>+<b>`, a fused ingredient in parentheses) names,
/// split at the top-level `+` signs; `None` for any other id. `fuse::fused_ingredients` is its reader
/// with R77's two-ingredient minimum. R469's Radiant mark (`<a>*`) is dropped here, since what a
/// card stands for is the definition, whichever face went in; R468's digest id (`t-<n>:#<hex>`)
/// reads its list off its fused definition in the state (the TS digest table is not ported).
pub fn fused_id_parts(state: Option<&GameState>, def_id: &str) -> Option<Vec<String>> {
    fused_id_specs(state, def_id).map(|specs| specs.into_iter().map(|spec| spec.def_id).collect())
}

/// R468: what follows `t-<n>:` in a digest id, which names its ingredients by a hash of them alone.
pub const FUSED_DIGEST_MARK: &str = "#";

/// R469: the mark an ingredient that went in on its Radiant face carries in a readable fused id.
pub const RADIANT_INGREDIENT_MARK: &str = "*";

/// `/^t-\d+:/`: the length of a fused id's head, or `None` for an id no Fuse minted.
pub fn fused_head_len(def_id: &str) -> Option<usize> {
    let rest = def_id.strip_prefix("t-")?;
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || rest.as_bytes().get(digits) != Some(&b':') {
        return None;
    }
    Some("t-".len() + digits + 1)
}

/// R468: whether a fused id is a digest of its ingredients rather than their names.
pub fn is_digest_id(def_id: &str) -> bool {
    match fused_head_len(def_id) {
        Some(head) => def_id[head..].starts_with(FUSED_DIGEST_MARK),
        None => false,
    }
}

pub(crate) fn copy_spec(entry: &FusedIngredient) -> FusedIngredient {
    FusedIngredient {
        def_id: entry.def_id.clone(),
        radiant: if entry.radiant == Some(true) {
            Some(true)
        } else {
            None
        },
    }
}

/// R179, R468, R469: the ingredients a fused id names, in order, each with its Radiant mark; `None`
/// for an id no Fuse minted, or a digest the state holds no definition of. A readable id is split at
/// its top-level `+` signs (a fused ingredient in parentheses, a Radiant one followed by `*`); a digest
/// id's list is its fused definition's `ingredients` (TS looked it up in the process's digest table,
/// which every minted or entered fused definition wrote).
pub fn fused_id_specs(state: Option<&GameState>, def_id: &str) -> Option<Vec<FusedIngredient>> {
    let head = fused_head_len(def_id)?;
    if is_digest_id(def_id) {
        let known = find_def(state, def_id)?.ingredients.as_ref()?;
        return Some(known.iter().map(copy_spec).collect());
    }
    let bytes = def_id.as_bytes();
    let mut specs: Vec<FusedIngredient> = Vec::new();
    let mut depth: i32 = 0;
    let mut start = head;
    for at in head..=bytes.len() {
        let ch = bytes.get(at).copied();
        if ch == Some(b'(') {
            depth += 1;
        } else if ch == Some(b')') {
            depth -= 1;
        } else if (ch == Some(b'+') && depth == 0) || at == bytes.len() {
            let mut part = &def_id[start..at];
            let radiant = part.ends_with(RADIANT_INGREDIENT_MARK);
            if radiant {
                part = &part[..part.len() - RADIANT_INGREDIENT_MARK.len()];
            }
            let name = if part.len() >= 2 && part.starts_with('(') && part.ends_with(')') {
                &part[1..part.len() - 1]
            } else {
                part
            };
            specs.push(FusedIngredient {
                def_id: name.to_string(),
                radiant: if radiant { Some(true) } else { None },
            });
            start = at + 1;
        }
    }
    if specs.iter().all(|spec| !spec.def_id.is_empty()) {
        Some(specs)
    } else {
        None
    }
}

/// §5.1, R387: "a random pool never includes the generating card's own definition". The running
/// card's ids (`self_def_ids`) are ADDED to whatever the caller already excludes, never put in its
/// place: a card whose text names its own exclusion keeps it when the card running that text is a
/// fused one (R77, R102).
pub fn excluding_def_id(
    state: Option<&GameState>,
    args: &CatalogQueryArgs,
    def_id: Option<&str>,
) -> CatalogQueryArgs {
    let Some(def_id) = def_id else {
        return args.clone();
    };
    let already: Vec<String> = as_list(&args.exclude_def_id).to_vec();
    let added: Vec<String> = self_def_ids(state, def_id)
        .into_iter()
        .filter(|id| !already.contains(id))
        .collect();
    if added.is_empty() {
        return args.clone();
    }
    let mut excluded = already;
    excluded.extend(added);
    CatalogQueryArgs {
        exclude_def_id: Some(OneOrMany::Many(excluded)),
        ..args.clone()
    }
}

/// §5's index as a number, so "2" sorts before "10" and a token index ("T-rush") sorts last.
/// (`Number.parseFloat`, SURFACE §4.4.4: no catalog index hits JS's lenient-prefix case.)
fn index_rank(index: &str) -> f64 {
    match index.parse::<f64>() {
        Ok(parsed) if parsed.is_finite() => parsed,
        _ => f64::INFINITY,
    }
}

/// B2.2: a set's place in the catalog order; a set the order does not name sorts last.
fn set_rank(set: SetName) -> usize {
    SHIPPED_SETS
        .iter()
        .position(|shipped| *shipped == set)
        .unwrap_or(SHIPPED_SETS.len())
}

/// §5.1's single source of random pools and Discover: every definition the filters allow, set by set
/// (catalog order, `SHIPPED_SETS`: Core, Classic, Classic+), each set in §5 index order. The order is a strict total order (set, index as a
/// number, then the index itself, then the unique catalog id), so it never depends on the order the
/// registry happened to hand the defs over and a seeded `rng.pick`/`rng.shuffle` over the result
/// replays identically (R58, R60, §9.3).
///
/// Only the registered catalog is searched: fused and crafted definitions live on the instance's
/// `transientDefs` and are not catalog cards, so no pool can generate one.
pub fn query(args: &CatalogQueryArgs) -> Vec<&'static CardDef> {
    let tokens_allowed = asks_for_tokens(args);
    let mut pool: Vec<&'static CardDef> = registered()
        .values()
        .filter(|def| matches_query(def, args, tokens_allowed))
        .collect();
    pool.sort_by(|a, b| {
        let set_a = set_rank(a.set);
        let set_b = set_rank(b.set);
        if set_a != set_b {
            return set_a.cmp(&set_b);
        }
        // Compared, not subtracted: two token indexes are both +Infinity, and `Infinity - Infinity`
        // is NaN, which `sort` reads as 0 while `NaN !== 0` is true — so subtracting returned early
        // with NaN and skipped both tie-breaks, leaving the tokens in registry insertion order and
        // breaking the total order this function promises (R58, R60, §9.3).
        let rank_a = index_rank(&a.index);
        let rank_b = index_rank(&b.index);
        if rank_a != rank_b {
            return if rank_a < rank_b {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        if a.index != b.index {
            return a.index.cmp(&b.index);
        }
        a.id.cmp(&b.id)
    });
    pool
}

// ---------------------------------------------------------------------------
// Grapes (C+ #65, #66; R382)
// ---------------------------------------------------------------------------

/// R382, BUILD §2: one Grape, rolled by `GRAPE_ODDS` — one draw of the match rng over the percents'
/// sum, walked in the table's order. `lucky` extra rolls (§6.1 Lucky X) keep the best, and the best is
/// the later entry, since the table runs from worst to best (Rotten < Normal < Large < Golden < Mythic).
/// Returns the Grape's def id. (TS's `lucky = 0` default: pass 0.)
pub fn roll_grape(rng: &mut Rng, lucky: i32) -> String {
    let roll = |rng: &mut Rng| -> usize {
        let total: i32 = GRAPE_ODDS.iter().map(|grape| grape.percent).sum();
        let mut at = rng.int(total);
        for (i, grape) in GRAPE_ODDS.iter().enumerate() {
            if at < grape.percent {
                return i;
            }
            at -= grape.percent;
        }
        GRAPE_ODDS.len().saturating_sub(1)
    };
    let extra = lucky.max(0);
    let index = if extra == 0 {
        roll(rng)
    } else {
        rng.lucky(extra, roll, |a, b| a.max(b))
    };
    let grape = GRAPE_ODDS.get(index).or_else(|| GRAPE_ODDS.last());
    grape.map(|grape| grape.def_id.to_string()).unwrap_or_default()
}

/// The def ids `GRAPE_ODDS` rolls: a pool pick naming one of these is re-rolled (R382).
fn is_grape_def_id(def_id: &str) -> bool {
    GRAPE_ODDS.iter().any(|grape| grape.def_id == def_id)
}

/// R673: what the Glitch roll reads — the match's count of "… in the System" plays
/// (`GameState.systemPlays`). TS typed it `{ readonly systemPlays?: number }` and every caller passed
/// the state, so it is the state.
pub type GlitchOdds = GameState;

/// R382: draw the generated card from a pool — as `rng.pick` today, except that a Grape pick is
/// rolled again on `GRAPE_ODDS` and the Grape that roll names is generated instead, so the Grape
/// rarity pool persists across every kind of Grape generation. One extra rng draw, only when a
/// Grape was picked; every other pick draws exactly as before. A caller generating into a hand or a
/// deck passes the state as `glitch`, and the pick may then become Glitch (R673).
pub fn pick_generated<'a>(
    rng: &mut Rng,
    pool: &[&'a CardDef],
    glitch: Option<&GlitchOdds>,
) -> Option<&'a CardDef> {
    let picked: Option<&'a CardDef> = rng.pick(pool).copied();
    let def = match picked {
        Some(grape) if is_grape_def_id(&grape.id) => {
            let rolled = roll_grape(rng, 0);
            Some(find_def(None, &rolled).unwrap_or(grape))
        }
        other => other,
    };
    match (def, glitch) {
        (Some(def), Some(glitch)) => Some(glitch_or_not(rng, def, glitch)),
        (def, _) => def,
    }
}

/// R673: after a card is picked for a hand or a deck, n/10000 that it is Glitch instead, n the match's
/// System plays. One rng draw, and only when n is above 0, so a match no System card was played in
/// draws exactly as before; a catalog without Glitch keeps the pick.
pub fn glitch_or_not<'a>(rng: &mut Rng, def: &'a CardDef, glitch: &GlitchOdds) -> &'a CardDef {
    let plays = glitch.system_plays.unwrap_or(0);
    if plays <= 0 {
        return def;
    }
    let roll = rng.int(GLITCH_ODDS_DENOMINATOR);
    if roll >= plays * GLITCH_ODDS_PER_SYSTEM_PLAY {
        return def;
    }
    find_def(None, GLITCH_DEF_ID).unwrap_or(def)
}
