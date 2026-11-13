//! SPEC §5.1's one catalog query: the typed wrapper card scripts import.
//! The filter lives in `crates/engine/src/catalog.rs`; this file owns the card-facing contract.
//! Proved by `tests/cross/query.rs`:
//! 1. Tokens out unless asked (§5.1, R380, R382; row 51.1 KY's Empty Notebook left out).
//! 2. Generating card excluded when card says so (excludeDefId, R387, B2.2; #95 is exception).
//! 3. Printed costs read out of play (R65; 0 for X-cost, base for embiggen; #7, #30, #51, #94).
//! 4. Ascending §5 index order set by set; seeded RNG replays identically (§9.3, R58, R60).
//! 5. Registered catalog cards only; transient defs never generate.
//!
//! Scripts reach it through `use crate::query::*;`: [`query`], [`pool`], [`query_cost`], [`TRAP_TYPES`],
//! and the named pools [`POOLS`] lists, which `spec/CARDS.md` shows (#3804).

use jackioh_engine::{CardDef, CardType, CatalogQueryArgs, KeywordKind, OneOrMany, Rarity, SetName, Tag};

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

/// R278: "a random Jlockeed card" (C+ #48, C+ #52, M+ #4, S #130): every non-token card tagged
/// Jlockeed, from the sets that ship (R380, R1420); the card drawing it leaves itself out (R387).
pub fn jlockeed_cards() -> CardQuery {
    CardQuery {
        tags: Some(vec![Tag::Jlockeed]),
        ..CardQuery::default()
    }
}

/// R1862002: "Discover N `rarity` Weapons" (Stinkyscape #81 to #83, #85, #86): the Weapon-tagged cards
/// of that rarity. It names its set, so it reaches them whether or not the set ships (R1420).
pub fn weapons_of(rarity: Rarity) -> CardQuery {
    CardQuery {
        set: Some(OneOrMany::One(SetName::Stinkyscape)),
        tags: Some(vec![Tag::Weapon]),
        rarity: Some(OneOrMany::One(rarity)),
        ..CardQuery::default()
    }
}

/// R1421: "a random Prime card" (Meditative #31's Radiant face, #55 Dragon Fruit): every card tagged
/// Prime, each a token, which a pool that names the tag reaches.
pub fn prime_cards() -> CardQuery {
    CardQuery {
        tags: Some(vec![Tag::Prime]),
        ..CardQuery::default()
    }
}

/// R3869000: "a Terminus card" (S #4 Olivian Expanse, S #154 Dice of Doom, S+ #81, #82, #89, #156):
/// every card and token tagged Terminus, from the sets that ship (R380, R1420). Terminus is no
/// `POOL_TOKEN_TAGS` tag (R382), so the pool asks for its tokens.
pub fn terminus_cards() -> CardQuery {
    CardQuery {
        tags: Some(vec![Tag::Terminus]),
        with_tokens: Some(true),
        ..CardQuery::default()
    }
}

/// R3869001: "a Call to Chaos" (S #4.6 The Big Crunch, S #149 Wildar, S+ #154 Chaos Accelerator):
/// every edition, the cards tagged Call to Chaos, from the sets that ship (R380, R1420).
pub fn call_to_chaos_editions() -> CardQuery {
    CardQuery {
        tags: Some(vec![Tag::CallToChaos]),
        with_tokens: Some(true),
        ..CardQuery::default()
    }
}

/// R3869001: "a War Doctrine" (S #137 Starfleet Commander, S #176 Preemptive Strike): the tokens tagged
/// War Doctrine (R1857002), from the sets that ship.
pub fn war_doctrines() -> CardQuery {
    CardQuery {
        tags: Some(vec![Tag::WarDoctrine]),
        with_tokens: Some(true),
        ..CardQuery::default()
    }
}

/// One random pool `spec/CARDS.md` lists (#3804), which `cargo jackioh spec index` writes from this
/// table: the pool's members as the catalog holds them now and the cards whose scripts draw from it, so
/// that no spec row lists either by hand.
pub struct NamedPool {
    /// Its heading in `spec/CARDS.md`, which a spec row links as `[[CARDS#<name>]]`.
    pub name: &'static str,
    /// What decides its members, in one line of the spec's words, its sections and rulings cited as
    /// plain text (`R1444`), since `spec/CARDS.md` holds no `[[…]]` link for `spec check` to resolve.
    pub rule: &'static str,
    /// The function a script calls to draw from it. Every script under `src/scripts/` whose code calls
    /// it is a card that draws from the pool.
    pub call: &'static str,
    /// Its members by part, each part with its label (`""` for a pool of one part).
    pub parts: fn() -> PoolParts,
}

/// A named pool's members, part by part: each part's label and its definitions.
pub type PoolParts = Vec<(&'static str, Vec<&'static CardDef>)>;

/// One part, the whole pool.
fn whole(args: CardQuery) -> PoolParts {
    vec![("", query(&args))]
}

/// The rarities a Weapon is printed at, each a part of the Weapons' pool.
const WEAPON_RARITIES: [Rarity; 3] = [Rarity::Rare, Rarity::Epic, Rarity::Legendary];

/// Every pool `spec/CARDS.md` lists, in its order (#3804). A pool a spec row would list card by card
/// goes here, and the row states its rule and links its heading instead.
pub const POOLS: &[NamedPool] = &[
    NamedPool {
        name: "luck-based",
        rule: "The Luck-based cards (R1444): every non-token card that prints Lucky on either face or whose text says \"Flip a coin\", read off the definition, from the sets that ship (R380, R1420).",
        call: "luck_based",
        parts: || whole(luck_based()),
    },
    NamedPool {
        name: "embiggen",
        rule: "The embiggen cards (R1178001): every non-token card that prints two prices, from the sets that ship.",
        call: "embiggen_cards",
        parts: || whole(embiggen_cards()),
    },
    NamedPool {
        name: "starship-pieces",
        rule: "The Starship Pieces (R1878000): every non-token Stinkyscape card that prints Starship Piece on its base face, which leaves out the Dormant Starship (R1857001).",
        call: "starship_pieces",
        parts: || whole(starship_pieces()),
    },
    NamedPool {
        name: "jlockeed",
        rule: "The Jlockeed cards (R278): every non-token card tagged Jlockeed, from the sets that ship (R380, R1420); the card drawing it leaves itself out (R387).",
        call: "jlockeed_cards",
        parts: || whole(jlockeed_cards()),
    },
    NamedPool {
        name: "weapons",
        rule: "The Weapons (R1862002): the Weapon-tagged Stinkyscape cards of the rarity the text names, the pool naming its set (R1420).",
        call: "weapons_of",
        parts: || {
            WEAPON_RARITIES
                .iter()
                .map(|rarity| (rarity.as_str(), query(&weapons_of(*rarity))))
                .collect()
        },
    },
    NamedPool {
        name: "prime",
        rule: "The Prime cards (R1421): every card tagged Prime, each a token, which a pool that names the tag reaches.",
        call: "prime_cards",
        parts: || whole(prime_cards()),
    },
    NamedPool {
        name: "terminus",
        rule: "The Terminus cards (R3869000): every card and token tagged Terminus, from the sets that ship (R380, R1420).",
        call: "terminus_cards",
        parts: || whole(terminus_cards()),
    },
    NamedPool {
        name: "call-to-chaos",
        rule: "The Call to Chaos editions (R3869001): every card tagged Call to Chaos, from the sets that ship (R380, R1420).",
        call: "call_to_chaos_editions",
        parts: || whole(call_to_chaos_editions()),
    },
    NamedPool {
        name: "war-doctrines",
        rule: "The War Doctrines (R3869001): every token tagged War Doctrine (R1857002), from the sets that ship.",
        call: "war_doctrines",
        parts: || whole(war_doctrines()),
    },
    NamedPool {
        name: "resources",
        rule: "The Resources (R1840002): every Resource the catalog holds, shipped or not, rolled over the rarities they hold; a gain that names its own pool rolls over that one.",
        call: "gain_resources",
        parts: || {
            let ids = jackioh_engine::resources::resource_def_ids();
            vec![(
                "",
                jackioh_engine::catalog::registered_catalog()
                    .values()
                    .filter(|def| ids.contains(&def.id))
                    .collect(),
            )]
        },
    },
];

/// Both trap types, for a pool or filter saying "Trap" (SPEC §8 #51, #85/R61, R35).
/// Prevents dropping #18 and #71.
pub const TRAP_TYPES: &[CardType] = &[CardType::Trap, CardType::FieldTrap];

/// R65's out-of-play cost of a definition: an X-cost card reads 0, an embiggen card reads its base
/// price, everything else its printed cost. This is what `cost` and `costRange` compare against, and
/// what a card script must use whenever it sorts, brackets or counts costs in a library, hand,
/// graveyard or pool. The in-play number is the engine's `mana::effective_cost`.
pub use jackioh_engine::catalog::query_cost;

// Never sort, filter or de-duplicate a pool after `query`: §5.1 forbids duplicating the filter.
