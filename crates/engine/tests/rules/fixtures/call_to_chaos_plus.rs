//! Test-only definitions for `subsystems/call_to_chaos_plus.rs` (C+ #73 Call to Chaos (Classic+
//! Edition)'s table, R423). The engine does not depend on `crates/cards`, so each pool its entries name
//! gets stand-ins under the tags, sets and indices the real cards carry.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::GRAPE_ODDS;
use jackioh_engine::testkit::*;

use super::catalog::{spell_def, unit_def};

/// C+ #73 itself and Core #95: the two editions the recursion and the deck replacement draw from (R28).
pub static plus: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        970,
        json!({
            "id": "fx-cp-plus",
            "index": "73",
            "name": "Fixture Call to Chaos (Classic+ Edition)",
            "set": "Classic+",
            "tags": ["Call to Chaos"],
            "rarity": "Legendary",
            "cost": 4,
        }),
    )
});
pub static core95: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        971,
        json!({
            "id": "fx-cp-core",
            "index": "95",
            "name": "Fixture Call to Chaos",
            "tags": ["Call to Chaos"],
            "rarity": "Legendary",
            "cost": 4,
        }),
    )
});

/// §7's Classic Golem stand-in: Classic+ index 73.1.
pub static golem: LazyLock<CardDef> = LazyLock::new(|| {
    unit_def(
        972,
        json!({
            "id": "fx-cp-golem",
            "index": "73.1",
            "name": "Fixture Classic Golem",
            "set": "Classic+",
            "tags": ["Token"],
            "rarity": "Token",
            "token": true,
            "cost": 4,
            "attack": 10,
            "health": 10,
        }),
    )
});

/// The pools: a Fruit and the five Grape tokens (R382), a Book of another set and a Book token, Classic cards.
pub static fruit: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        973,
        json!({ "id": "fx-cp-fruit", "name": "Fixture Fruit", "set": "Classic+", "tags": ["Fruit"] }),
    )
});
/// The five Grapes, stand-ins under their own ids: of the Fruit tokens a Fruit pool takes only these
/// (R382, `GRAPE_ODDS`), and a pick of one is rolled again on `GRAPE_ODDS`, which may name any of them.
pub static grapes: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    GRAPE_ODDS
        .iter()
        .enumerate()
        .map(|(at, odds)| {
            spell_def(
                974,
                json!({
                    "id": odds.def_id,
                    "index": format!("65.{}", at + 1),
                    "name": format!("Fixture Grape {}", at + 1),
                    "set": "Classic+",
                    "tags": ["Fruit", "Token"],
                    "rarity": "Token",
                    "token": true,
                }),
            )
        })
        .collect()
});
pub static book: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        975,
        json!({ "id": "fx-cp-book", "name": "Fixture Book", "set": "Classic", "tags": ["Book"], "cost": 2 }),
    )
});
pub static book_token: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        976,
        json!({
            "id": "fx-cp-book-token",
            "name": "Fixture Book Token",
            "tags": ["Book", "Token"],
            "rarity": "Token",
            "token": true,
        }),
    )
});
pub static classic: LazyLock<CardDef> = LazyLock::new(|| {
    unit_def(
        977,
        json!({ "id": "fx-cp-classic", "name": "Fixture Classic Unit", "set": "Classic", "cost": 3 }),
    )
});

/// A deck card no Fuse may keep (R23).
pub static immutable: LazyLock<CardDef> = LazyLock::new(|| {
    unit_def(
        978,
        json!({ "id": "fx-cp-immutable", "name": "Fixture Immutable Unit", "keywords": [{ "kind": "Immutable" }] }),
    )
});

/// A backrow Trap and an Indestructible Field Spell for the destroy entry.
pub static trap: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        979,
        json!({ "id": "fx-cp-trap", "name": "Fixture Trap", "type": "Trap" }),
    )
});
pub static hard_field: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        980,
        json!({
            "id": "fx-cp-hard",
            "name": "Fixture Indestructible Field Spell",
            "type": "Field Spell",
            "base": { "keywords": [{ "kind": "Indestructible" }], "text": "Indestructible" },
            "radiant": { "keywords": [{ "kind": "Indestructible" }], "text": "Indestructible" },
        }),
    )
});

/// Every stand-in above on top of `base`.
pub fn chaos_plus_catalog(base: CardDefs) -> CardDefs {
    let mut defs = base;
    for entry in [
        &*plus,
        &*core95,
        &*golem,
        &*fruit,
        &*book,
        &*book_token,
        &*classic,
        &*immutable,
        &*trap,
        &*hard_field,
    ] {
        defs.insert(entry.id.clone(), entry.clone());
    }
    for grape in grapes.iter() {
        defs.insert(grape.id.clone(), grape.clone());
    }
    defs
}
