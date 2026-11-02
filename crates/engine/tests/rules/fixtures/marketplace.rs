//! Test-only cards and a table for Stinkyscape's Marketplace (#1853, R1853000–R1853005; #2250,
//! R2250002), all of the Boss-X set so a set lot draws among them, and `MARKET_TABLE` in the shape of
//! the set's (S21); the Resources are `fixtures/resources.rs`'. Ids `fx-mkt-…` from 18530 (the
//! hand-sale card 18610) collide with no other fixture file's.

use std::sync::LazyLock;

use jackioh_engine::testkit::*;

use super::catalog::{spell_def, unit_def};

/// The Marketplace's lowest price.
pub const FREE: &str = "fx-mkt-free";
/// A (4) 4/4: 40 Credits.
pub const FOUR: &str = "fx-mkt-four";
/// A (5) 5/5: 45 Credits.
pub const FIVE: &str = "fx-mkt-five";
/// The lot Call to Chaos (Scape Edition) adds.
pub const CHAOS: &str = "fx-mkt-chaos";
/// The fixed price it adds that lot at.
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

pub fn marketplace_catalog(base: CardDefs) -> CardDefs {
    let mut defs = base;
    for def in MARKET_DEFS.iter() {
        defs.insert(def.id.clone(), def.clone());
    }
    defs
}

/// The fixture table: the named three weighted 2, the (4) Unit alone 1, and the Boss-X set 1, so a
/// roll of three offers draws its lots by 2 : 1 : 1 (R2250002).
pub static MARKET_TABLE: &[MarketLot] = &[
    MarketLot {
        pool: MarketPool::Ids(&[FREE, FOUR, FIVE]),
        count: 2,
    },
    MarketLot {
        pool: MarketPool::Ids(&[FOUR]),
        count: 1,
    },
    MarketLot {
        pool: MarketPool::Set(SetName::BossX),
        count: 1,
    },
];

/// R1861000: a (1) Spell whose faces sell from the hand (`handSale`), as Stinkyscape #89 Spice does.
/// Kept out of `MARKET_DEFS`, so the table's tests quote the Resources alone.
pub const HAND_SALE: &str = "fx-mkt-hand-sale";

pub static HAND_SALE_DEF: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        18610,
        json!({ "id": HAND_SALE, "name": "Fixture Hand Sale", "set": "Core", "cost": 1 }),
    )
});

pub static HAND_SALE_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let face = || Script {
        static_flags: Some(StaticFlags {
            hand_sale: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    IndexMap::from([(
        HAND_SALE.to_string(),
        CardScripts {
            base: face(),
            radiant: face(),
        },
    )])
});
