//! SPEC §5.1's one catalog query, as the `packages/cards` surface every card script writes against.
//!
//! "The catalog needs one query function, `catalog.query({type, cost, costRange, tags, notTags,
//! rarity, set, excludeDefId})`, that every random-generation and Discover effect uses" (§5.1).
//! ONE function means one implementation: the filter lives in `crates/engine/src/catalog.rs` (the
//! engine needs it for Recruit, Discover and every `generate` effect), and this file is the thin
//! typed wrapper card scripts import. Nothing here filters, sorts, excludes tokens or reads a cost —
//! re-implementing any of that would make two pools out of §5.1's one, which is the bug this file
//! exists to prevent. What it does own is the card-facing contract below.
//!
//! The contract (proved by `tests/cross/query.rs`, which is the reference for card agents):
//!
//!   1. Tokens are out unless you ask. §5.1: "Random pools ('a random card', 'Discover a (2) cost
//!      card') never include Token-tagged cards". `query({})` is therefore every non-token card of
//!      every set (R380); tokens arrive only for a query that names the token pool — `tags:
//!      ["Token"]`, `rarity: "Token"`, `token: true`, `withTokens: true` (R382), or naming members
//!      outright via `defId` — and a Fruit pool holds the Grapes (R382).
//!      BUILD M4-T4 row 51.1 ("absent from every random pool") needs no extra argument: KY's Empty
//!      Notebook carries the KY tag, and `query({ tags: ["KY"] })` still leaves it out.
//!   2. The generating card is out when the card says so. §5.1: pools "never include the generating
//!      card's own definition, unless the card names the pool itself". That is `excludeDefId`, keyed
//!      by the catalog id because an index repeats across sets (R387, B2.2), and because it is the
//!      caller's own id, the caller passes it — see `pool()` below, which makes it impossible to
//!      forget. The exception is real: #95 Call to Chaos casts "a random Call
//!      to Chaos" from the tag "which includes #95", so #95 uses plain `query({ tags: [...] })`.
//!   3. Costs are read out of play (R65): "an embiggen card's printed cost is its base price and an
//!      X-cost card's is 0". `cost`, `costRange` and `query_cost` all read that one number, so #7's
//!      brackets, #51's brackets, #30's highest/lowest and #94's odd costs agree.
//!   4. The result is ordered set by set (Core first), each by SPEC §5 index, ascending, with no
//!      dependence on the order the registry handed the defs over. A seeded `rng.pick`/`rng.shuffle`
//!      over a pool therefore replays identically (§9.3, R58, R60).
//!   5. Only registered catalog cards are reachable. `register_all()` (src/lib.rs) registers the
//!      catalog before a game starts; until then every pool is empty. Fused and crafted definitions
//!      live on `state.transient_defs`, so no pool can ever generate one.
//!
//! Card scripts get this whole surface through `use crate::query::*;` (and the crate root's
//! re-export), so `catalog.query(...)`, `catalog.pool(...)`, `catalog.cost(...)` and
//! `catalog.trap_types` are always in reach even when only `catalog` is named, as the TS `catalog`
//! object was.

use jackioh_engine::{CardDef, CardType, CatalogQueryArgs};
use serde_json::Value;

/// §5.1's query arguments: `{ type, cost, costRange, tags, notTags, rarity, set, excludeDefId,
/// withTokens }`, plus the engine's identity fields (`defId`, `token`) for a pool a card names card by
/// card. `tags` means "has every listed tag"; `notTags` means "has none of them"; every
/// field narrows, and `{}` (`CardQuery::default()`) is the whole non-token catalog.
pub type CardQuery = CatalogQueryArgs;

/// §5.1's single pool source. Returns the matching definitions in §5 index order.
///
/// Random effects pick from this with `rng` (never the OS, CLAUDE.md rule 4); Discover passes
/// the result to a `PendingChoice`. A card that generates from a pool almost always wants `pool()`
/// instead, so that its own definition cannot come back out. TS's default argument `{}` is
/// `&CardQuery::default()`.
pub fn query(args: &CardQuery) -> Vec<&'static CardDef> {
    jackioh_engine::catalog::query(args)
}

/// §5.1's "never include the generating card's own definition": the pool for card `own_id`, which is
/// `query` with `own_id` added to `excludeDefId` rather than replacing what the caller passed (R387).
///
/// ```text
/// pool("core-057", { tags: ["KY"] })         // #57 Conjure KY  -> Core #31, #51, #82
/// pool("core-083", { rarity: "Legendary" })  // #83 Transmogulate (R35) -> Core #52, #85, #87, #92, #93, #95
/// pool("core-067", { type: TRAP_TYPES })     // #67 Zoomerbin Oomen -> Core #18, #41, #60, #71, #85, #96
/// ```
///
/// TS's `excludeDefId` is `string | string[]`: absent becomes `[ownId]`, one string `s` becomes
/// `[s, ownId]`, and a list gets `ownId` appended. The arguments are extended on their own JSON (the
/// shape TS spread), so each of the three keeps TS's reading whatever Rust type holds the field.
pub fn pool(own_id: &str, args: &CardQuery) -> Vec<&'static CardDef> {
    let mut json = match serde_json::to_value(args) {
        Ok(json) => json,
        Err(error) => panic!("pool({own_id}): the query arguments are not JSON: {error}"),
    };
    let already = json.get("excludeDefId").cloned();
    let mut exclude_def_id: Vec<Value> = match already {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(list)) => list,
        Some(one) => vec![one],
    };
    exclude_def_id.push(Value::String(own_id.to_string()));
    if let Value::Object(map) = &mut json {
        map.insert("excludeDefId".to_string(), Value::Array(exclude_def_id));
    }
    let with_own: CardQuery = match serde_json::from_value(json) {
        Ok(args) => args,
        Err(error) => panic!("pool({own_id}): the query arguments do not read back: {error}"),
    };
    query(&with_own)
}

/// MD-G1, R1442: "a random Luck-based card" (Meditative #101 Gachaholic): every non-token card that
/// prints Lucky on either face or flips a coin (`jackioh_engine::catalog::is_luck_based`), from the
/// sets that ship (R1420).
pub fn luck_based() -> CardQuery {
    CardQuery {
        luck_based: Some(true),
        ..CardQuery::default()
    }
}

/// Both trap types, for a pool or filter that says "Trap". SPEC says "Field Trap counts as Trap" for
/// §8 #51 (KY's Private Tutor's type choice), #85/R61 (Unlicensed Experimentation's type match) and
/// R35 (Transmogulate's same-type replacement), so a `type: "Trap"` query — which matches the
/// `type` field exactly — would silently drop #18 and #71. Ask for both.
///
/// TS typed it as a mutable array only because `CatalogQuery["type"]` is `CardType | CardType[]`; it
/// is a constant here, and `query` never writes to it.
pub const TRAP_TYPES: &[CardType] = &[CardType::Trap, CardType::FieldTrap];

/// R65's out-of-play cost of a definition: an X-cost card reads 0, an embiggen card reads its base
/// price, everything else its printed cost. This is what `cost` and `costRange` compare against, and
/// what a card script must use whenever it sorts, brackets or counts costs in a library, hand,
/// graveyard or pool. The in-play number is the engine's `mana::effective_cost`, which starts from an
/// instance and adds `cost_mod`, discounts and Professor Curvature.
pub use jackioh_engine::catalog::query_cost;

/// The TS `catalog` object's type: `{ query, pool, cost: queryCost, trapTypes: TRAP_TYPES }`, so a
/// script writes `catalog.pool(ID, &args)` and `catalog.trap_types` as the TypeScript did. The three
/// functions are methods (no fn pointers in a pure crate's values) and the constant is a field.
#[derive(Clone, Copy, Debug)]
pub struct CatalogSurface {
    /// [`TRAP_TYPES`]: both trap types (TS `catalog.trapTypes`).
    pub trap_types: &'static [CardType],
}

impl CatalogSurface {
    /// §5.1's `catalog.query(...)`: [`query`].
    pub fn query(&self, args: &CardQuery) -> Vec<&'static CardDef> {
        query(args)
    }

    /// `catalog.pool(ownId, args)`: [`pool`], never offering `own_id` itself (R387).
    pub fn pool(&self, own_id: &str, args: &CardQuery) -> Vec<&'static CardDef> {
        pool(own_id, args)
    }

    /// `catalog.cost(def)`: [`query_cost`], R65's out-of-play cost.
    pub fn cost(&self, def: &CardDef) -> i32 {
        query_cost(def)
    }
}

/// §5.1's `catalog.query(...)`, as the object 110 card scripts call. Lower-case because it is the
/// TS name a script calls it by (SURFACE §4.2: a constant keeps its name); a value, not a module, so
/// it never collides with `jackioh_engine::catalog` in a script that names both.
#[allow(non_upper_case_globals)]
pub const catalog: CatalogSurface = CatalogSurface {
    trap_types: TRAP_TYPES,
};

// Never sort, filter or de-duplicate a pool after calling `query`: a second sort here would be the
// duplicated filter §5.1 forbids, and the ordering is the engine comparator's job. (It once
// returned NaN for two non-numeric indexes and left the shared tokens in insertion order; that is
// fixed in the engine and guarded by a regression test in `tests/cross/query.rs`.)
