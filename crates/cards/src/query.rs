//! SPEC §5.1's one catalog query: the typed wrapper card scripts import.
//! The filter lives in `crates/engine/src/catalog.rs`; this file owns the card-facing contract.
//! Proved by `tests/cross/query.rs`:
//! 1. Tokens out unless asked (§5.1, R380, R382; row 51.1 KY's Empty Notebook left out).
//! 2. Generating card excluded when card says so (excludeDefId, R387, B2.2; #95 is exception).
//! 3. Printed costs read out of play (R65; 0 for X-cost, base for embiggen; #7, #30, #51, #94).
//! 4. Ascending §5 index order set by set; seeded RNG replays identically (§9.3, R58, R60).
//! 5. Registered catalog cards only; transient defs never generate.
//! Scripts reach it through `use crate::query::*;`: [`query`], [`pool`], [`query_cost`], [`TRAP_TYPES`].

use jackioh_engine::{CardDef, CardType, CatalogQueryArgs, KeywordKind, OneOrMany, SetName};

/// §5.1's query arguments: `{ type, cost, costRange, tags, notTags, rarity, set, excludeDefId,
/// withTokens }`, plus the engine's identity fields (`defId`, `token`) for a pool a card names card by
/// card. `tags` means "has every listed tag"; `notTags` means "has none of them"; every
/// field narrows, and `{}` (`CardQuery::default()`) is the whole non-token catalog.
pub type CardQuery = CatalogQueryArgs;

/// §5.1's single pool source. Returns the matching definitions in §5 index order.
///
/// Random effects pick with `rng` (CLAUDE.md rule 4); Discover passes to `PendingChoice`.
/// A generating card wants `pool()` so its own definition cannot come back out.
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
/// Arguments are extended by `catalog::excluding_def_id` without serialization round-trips.
pub fn pool(own_id: &str, args: &CardQuery) -> Vec<&'static CardDef> {
    query(&jackioh_engine::catalog::excluding_def_id(
        None,
        args,
        Some(own_id),
    ))
}

/// MD-G1, R1444: "a random Luck-based card" (Meditative #101 Gachaholic): every non-token card that
/// prints Lucky on either face or flips a coin (`jackioh_engine::catalog::is_luck_based`), from the
/// sets that ship (R1420).
pub fn luck_based() -> CardQuery {
    CardQuery {
        luck_based: Some(true),
        ..CardQuery::default()
    }
}

/// R1178001: "a random embiggen card" (Meditative+ #23 Enormous Elderberry, #22 Mountaintop
/// Garden's class): every non-token card that prints two prices (`jackioh_engine::is_embiggen_card`),
/// from the sets that ship (R1420).
pub fn embiggen_cards() -> CardQuery {
    CardQuery {
        embiggen: Some(true),
        ..CardQuery::default()
    }
}

/// R1878000: "a random Starship Piece" (Stinkyscape #124, #125, #126): every non-token card that prints
/// Starship Piece on its base face, which leaves out the Dormant Starship (R1857001). It names its set,
/// the one that prints the keyword, so it reaches the pieces while the set is held (R1420).
pub fn starship_pieces() -> CardQuery {
    CardQuery {
        set: Some(OneOrMany::One(SetName::Stinkyscape)),
        keywords: Some(vec![KeywordKind::StarshipPiece]),
        ..CardQuery::default()
    }
}

/// Both trap types, for a pool or filter saying "Trap" (SPEC §8 #51, #85/R61, R35).
/// Prevents dropping #18 and #71.
pub const TRAP_TYPES: &[CardType] = &[CardType::Trap, CardType::FieldTrap];

/// R65's out-of-play cost of a definition: an X-cost card reads 0, an embiggen card reads its base
/// price, everything else its printed cost. This is what `cost` and `costRange` compare against, and
/// what a card script must use whenever it sorts, brackets or counts costs in a library, hand,
/// graveyard or pool. The in-play number is the engine's `mana::effective_cost`.
pub use jackioh_engine::catalog::query_cost;

// Never sort, filter or de-duplicate a pool after `query`: §5.1 forbids duplicating the filter.
