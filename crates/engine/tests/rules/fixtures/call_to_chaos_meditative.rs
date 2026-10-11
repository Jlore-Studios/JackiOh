//! Test-only definitions for `subsystems/call_to_chaos_meditative.rs` (Meditative #95 Call to Chaos
//! (Meditative Edition)'s table, R1240–R1244). The engine does not depend on `crates/cards`, so each pool
//! its entries name — CN cards, the Prime tokens (R1421), the Acclaimed permanents, the "Call to Chaos"
//! pool of every edition, the Jade Beauty and CN Golem tokens — gets stand-ins under the tags, sets and
//! indices the real cards carry; the real card's test covers the same cases against the real catalog
//! (`crates/cards/src/scripts/meditative/c095_call_to_chaos_meditative_edition.rs`). They sit on top of
//! C+ #73's stand-ins (`call_to_chaos_plus.rs`), whose two editions and Immutable card they reuse.

use std::sync::LazyLock;

use jackioh_engine::testkit::*;

use super::catalog::{spell_def, unit_def};

/// Meditative #95 itself, the third edition (R1240): in a set that has not shipped (R1420).
pub static MED: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        990,
        json!({
            "id": "fx-cm-med",
            "index": "95",
            "name": "Fixture Call to Chaos (Meditative Edition)",
            "set": "Meditative",
            "tags": ["Call to Chaos"],
            "rarity": "Legendary",
            "cost": 4,
        }),
    )
});

/// §7's CN Golem and Jade Beauty stand-ins: Meditative indices 95.1 and 39.5.
pub static GOLEM: LazyLock<CardDef> = LazyLock::new(|| {
    unit_def(
        991,
        json!({
            "id": "fx-cm-golem",
            "index": "95.1",
            "name": "Fixture CN Golem",
            "set": "Meditative",
            "tags": ["CN", "Token"],
            "rarity": "Token",
            "token": true,
            "cost": 4,
            "attack": 10,
            "health": 10,
        }),
    )
});
pub static JADE: LazyLock<CardDef> = LazyLock::new(|| {
    unit_def(
        992,
        json!({
            "id": "fx-cm-jade",
            "index": "39.5",
            "name": "Fixture Jade Beauty",
            "set": "Meditative",
            "tags": ["CN", "Token"],
            "rarity": "Token",
            "token": true,
            "cost": 10,
            "attack": 20,
            "health": 20,
        }),
    )
});

/// The pools: a CN card, a Prime token (R1421), an Acclaimed permanent, an Acclaimed token and an
/// Acclaimed Spell (R1243: neither is in the pool).
pub static CN: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        993,
        json!({ "id": "fx-cm-cn", "name": "Fixture CN Spell", "tags": ["CN"], "cost": 3 }),
    )
});
pub static PRIME: LazyLock<CardDef> = LazyLock::new(|| {
    unit_def(
        994,
        json!({
            "id": "fx-cm-prime",
            "name": "Fixture Prime",
            "set": "Classic+",
            "tags": ["Prime", "Token"],
            "rarity": "Token",
            "token": true,
            "cost": 5,
        }),
    )
});
pub static ACCLAIMED: LazyLock<CardDef> = LazyLock::new(|| {
    unit_def(
        995,
        json!({
            "id": "fx-cm-acclaimed",
            "name": "Fixture Acclaimed Unit",
            "set": "Classic",
            "tags": ["Acclaimed"],
            "cost": 5,
        }),
    )
});
pub static ACCLAIMED_TOKEN: LazyLock<CardDef> = LazyLock::new(|| {
    unit_def(
        996,
        json!({
            "id": "fx-cm-acclaimed-token",
            "name": "Fixture Acclaimed Token",
            "tags": ["Acclaimed", "Token"],
            "rarity": "Token",
            "token": true,
        }),
    )
});
pub static ACCLAIMED_SPELL: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        997,
        json!({ "id": "fx-cm-acclaimed-spell", "name": "Fixture Acclaimed Spell", "tags": ["Acclaimed"] }),
    )
});

/// Every stand-in above on top of `base`.
pub fn chaos_med_catalog(base: CardDefs) -> CardDefs {
    let mut defs = base;
    for entry in [
        &*MED,
        &*GOLEM,
        &*JADE,
        &*CN,
        &*PRIME,
        &*ACCLAIMED,
        &*ACCLAIMED_TOKEN,
        &*ACCLAIMED_SPELL,
    ] {
        defs.insert(entry.id.clone(), entry.clone());
    }
    defs
}
