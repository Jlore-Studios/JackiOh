//! Test-only cards for ME-MARKET (Meditative #42 CN Flea Market, R1000–R1002): `nm-market` runs the
//! card's own night market over fixture shelves (`subsystems::night_market`), so the engine is proved
//! without `crates/cards` (CLAUDE.md: the engine never imports it). The Rock is registered under its
//! real id, `meditative-039-1`, as a stand-in, and each CN lot has a price the tests read.

use std::sync::LazyLock;

use jackioh_engine::subsystems::night_market::{MarketShelf, night_market_script};
use jackioh_engine::testkit::*;

/// The Auspicious Rock's real id (M #39.1), which the market names.
pub const ROCK: &str = "meditative-039-1";

/// TS's `{ ...a, ...b }` on two object literals: `b`'s keys replace `a`'s.
fn spread(mut base: Value, extra: Value) -> Value {
    if let (Some(into), Value::Object(from)) = (base.as_object_mut(), extra) {
        for (key, value) in from {
            into.insert(key, value);
        }
    }
    base
}

fn def(index: u32, id: &str, extra: Value) -> CardDef {
    json_as(spread(
        json!({
            "id": id,
            "index": index.to_string(),
            "name": id,
            "set": "Core",
            "type": "Spell",
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": { "keywords": [], "text": id },
            "radiant": { "keywords": [], "text": id },
        }),
        extra,
    ))
}

/// The market's declared yuan: 50, Radiant 80, as the card's.
fn yuan_params() -> Value {
    json!([{ "key": "yuan", "base": 50, "radiant": 80, "better": "up", "step": 10, "min": 10 }])
}

/// `nm-market`: the fixture night market, a CN Spell (so the stall's CN pool must leave it out, R387).
pub fn market() -> CardDef {
    def(
        3411,
        "nm-market",
        json!({ "tags": ["CN"], "rarity": "Legendary", "cost": 0, "params": yuan_params() }),
    )
}

/// `nm-market-qd`: the same market with Quickdraw, so a replayed game deals it in the opening hand.
pub fn market_qd() -> CardDef {
    def(
        3412,
        "nm-market-qd",
        json!({ "rarity": "Legendary", "cost": 0, "params": yuan_params() }),
    )
}

/// A (1) Common CN Spell: 15 yuan.
pub fn cn_one() -> CardDef {
    def(3413, "nm-cn-1", json!({ "tags": ["CN"], "cost": 1 }))
}

/// A (2) Rare CN Spell: 30 yuan.
pub fn cn_two() -> CardDef {
    def(
        3414,
        "nm-cn-2",
        json!({ "tags": ["CN"], "rarity": "Rare", "cost": 2 }),
    )
}

/// A (4) Legendary CN Spell: 60 yuan, over the base market's 50.
pub fn cn_four() -> CardDef {
    def(
        3415,
        "nm-cn-4",
        json!({ "tags": ["CN"], "rarity": "Legendary", "cost": 4 }),
    )
}

/// An X-cost Epic CN Spell: X counts 0 out of play (R65), so 15 yuan.
pub fn cn_x() -> CardDef {
    def(
        3416,
        "nm-cn-x",
        json!({ "tags": ["CN"], "rarity": "Epic", "cost": "X" }),
    )
}

/// The Rock stand-in: a (0) CN token printed Rare, 10 yuan.
pub fn rock() -> CardDef {
    def(
        3417,
        ROCK,
        json!({
            "tags": ["CN", "Token"], "rarity": "Token", "printedRarity": "Rare", "token": true, "cost": 0,
        }),
    )
}

/// An AI generated token, printing no rarity: ranked Common, so 15 yuan at (1).
pub fn ai_card() -> CardDef {
    def(
        3418,
        "nm-ai",
        json!({ "tags": ["AI", "Token"], "rarity": "Token", "token": true, "cost": 1 }),
    )
}

fn defs() -> Vec<CardDef> {
    vec![
        market(),
        market_qd(),
        cn_one(),
        cn_two(),
        cn_four(),
        cn_x(),
        rock(),
        ai_card(),
    ]
}

/// The card's shelves (the cards crate's #42 names the same three): three CN cards, two Rocks, one
/// AI generated card.
pub fn shelves() -> Vec<MarketShelf> {
    vec![
        MarketShelf::Pool {
            query: Box::new(json_as(json!({ "tags": ["CN"] }))),
            count: NIGHT_MARKET_CN_LOTS,
        },
        MarketShelf::Named {
            def_id: ROCK.to_string(),
            count: NIGHT_MARKET_ROCK_LOTS,
        },
        MarketShelf::Pool {
            query: Box::new(json_as(json!({ "tags": ["AI"], "token": true }))),
            count: NIGHT_MARKET_AI_LOTS,
        },
    ]
}

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let made = night_market_script(&shelves());
    let script = Script {
        cry: Some(made.cry.clone()),
        resume: made.resume.clone(),
        ..Script::default()
    };
    let quick = Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        ..script.clone()
    };
    IndexMap::from([
        (
            market().id,
            CardScripts {
                base: script.clone(),
                radiant: script,
            },
        ),
        (
            market_qd().id,
            CardScripts {
                base: quick.clone(),
                radiant: quick,
            },
        ),
    ])
});

/// This file's definitions, by id.
pub fn catalog() -> CardDefs {
    defs().into_iter().map(|card| (card.id.clone(), card)).collect()
}

/// This file's scripts, by id.
pub fn scripts() -> IndexMap<String, CardScripts> {
    SCRIPTS.clone()
}

/// Add these fixtures to whatever the harness registered.
pub fn register_night_market_fixtures() {
    let mut all_defs = registered_catalog().clone();
    all_defs.extend(catalog());
    register_catalog(all_defs);
    let mut all_scripts = registered_scripts().clone();
    all_scripts.extend(scripts());
    register_scripts(all_scripts);
}
