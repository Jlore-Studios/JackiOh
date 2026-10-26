//! Test-only cards and a table for Stinkyscape's Marketplace (#1853, R1853000–R1853005): a (0) Spell, a
//! (4) and a (5) Unit and a (1) Spell standing in for Call to Chaos (Scape Edition)'s lot, all of the
//! Boss-X set so a set lot draws among them, and `MARKET_TABLE`, the fixture table in the shape of the
//! set's (S21): Resources, a pool of named cards, one named card at a fixed stock, and a set. No shipped
//! card reaches the Marketplace yet, so the engine never imports `crates/cards` for this; the Resources
//! are `fixtures/resources.rs`'.
//!
//! Ids are prefixed `fx-mkt-` and indexed from 18530, so they collide with no other fixture file's.

use std::sync::LazyLock;

use jackioh_engine::testkit::*;

use super::catalog::{spell_def, unit_def};

/// A (0) Spell: the Marketplace's lowest price.
pub const FREE: &str = "fx-mkt-free";
/// A (4) 4/4: 40 Credits.
pub const FOUR: &str = "fx-mkt-four";
/// A (5) 5/5: 45 Credits.
pub const FIVE: &str = "fx-mkt-five";
/// A (1) Spell, the lot Call to Chaos (Scape Edition) adds.
pub const CHAOS: &str = "fx-mkt-chaos";
/// The fixed price Call to Chaos (Scape Edition) adds its lot at.
pub const CHAOS_PRICE: i32 = 20;
/// The four fixture cards, the Boss-X set.
pub const MARKET_CARDS: [&str; 4] = [FREE, FOUR, FIVE, CHAOS];

pub static MARKET_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        spell_def(
            18530,
            json!({ "id": FREE, "name": "Fixture Free", "set": "Boss-X", "cost": 0 }),
        ),
        unit_def(
            18531,
            json!({ "id": FOUR, "name": "Fixture Four", "set": "Boss-X", "cost": 4, "attack": 4, "health": 4 }),
        ),
        unit_def(
            18532,
            json!({ "id": FIVE, "name": "Fixture Five", "set": "Boss-X", "cost": 5, "attack": 5, "health": 5 }),
        ),
        spell_def(
            18533,
            json!({ "id": CHAOS, "name": "Fixture Chaos", "set": "Boss-X", "cost": 1 }),
        ),
    ]
});

/// The Marketplace's cards on top of `base`.
pub fn marketplace_catalog(base: CardDefs) -> CardDefs {
    let mut defs = base;
    for def in MARKET_DEFS.iter() {
        defs.insert(def.id.clone(), def.clone());
    }
    defs
}

/// The fixture table: two Resources at stock 4–8, two of the named three at 1–3, the (4) Unit at 6, and
/// one card of the Boss-X set at 2 — six offers.
pub static MARKET_TABLE: &[MarketLot] = &[
    MarketLot {
        pool: MarketPool::Resources,
        count: 2,
        stock_min: 4,
        stock_max: 8,
    },
    MarketLot {
        pool: MarketPool::Ids(&[FREE, FOUR, FIVE]),
        count: 2,
        stock_min: 1,
        stock_max: 3,
    },
    MarketLot {
        pool: MarketPool::Ids(&[FOUR]),
        count: 1,
        stock_min: 6,
        stock_max: 6,
    },
    MarketLot {
        pool: MarketPool::Set(SetName::BossX),
        count: 1,
        stock_min: 2,
        stock_max: 2,
    },
];
