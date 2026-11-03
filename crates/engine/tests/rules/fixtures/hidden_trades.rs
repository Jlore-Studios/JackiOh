//! Test-only cards for issue #1845's two engine primitives: hidden Nerfs (`degrade_secretly`, R1845000,
//! R1845001; Stinkyscape #123 Gokugains' shape) and the trade offer (`trade_offer`, R1845002; #77
//! Yuma's), through the engine's own verbs. Ids are prefixed `ht-` and indexed from 18450, so they
//! collide with no other fixture file's.

use std::sync::LazyLock;

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

use super::harness::both;

/// The sneak's hidden Nerfs (#123 Gokugains' three).
pub const SECRET_NERFS: i32 = 3;
/// A count no card survives with any stats to spare (#123's Radiant thirty).
pub const HEAVY_NERFS: i32 = 30;

fn card(index: u32, name: &str, type_: &str, cost: i32, face: Value) -> CardDef {
    json_as(json!({
        "id": format!("ht-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (hidden trades fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": face,
        "radiant": face,
    }))
}

/// A (0) 1/1 Unit: "Cry: a random Unit in your opponent's hand is Nerfed 3 times secretly" (#123).
pub static SNEAK: LazyLock<CardDef> = LazyLock::new(|| {
    card(
        18450,
        "sneak",
        "Unit",
        0,
        json!({ "attack": 1, "health": 1, "keywords": [], "text": "sneak" }),
    )
});
/// A (0) 1/1 Unit: "Cry: Trade offer your opponent at the start of their turn" (#77).
pub static TRADER: LazyLock<CardDef> = LazyLock::new(|| {
    card(
        18451,
        "trader",
        "Unit",
        0,
        json!({ "attack": 1, "health": 1, "keywords": [], "text": "trader" }),
    )
});
/// The trader with Lucky 1 printed (#77 Yuma's Radiant face): its offer's Discover is drawn twice (R1877001).
pub static LUCKY_TRADER: LazyLock<CardDef> = LazyLock::new(|| {
    card(
        18454,
        "lucky-trader",
        "Unit",
        0,
        json!({ "attack": 1, "health": 1, "keywords": [{ "kind": "Lucky", "n": 1 }], "text": "lucky trader" }),
    )
});
/// A (1) 2/2 Immutable Unit with no script, which no Nerf reaches (R23, R386).
pub static STOIC: LazyLock<CardDef> = LazyLock::new(|| {
    card(
        18452,
        "stoic",
        "Unit",
        1,
        json!({ "attack": 2, "health": 2, "keywords": [{ "kind": "Immutable" }], "text": "Immutable" }),
    )
});
/// A (0) Spell with no script: a hand card that is not a Unit.
pub static DUD: LazyLock<CardDef> =
    LazyLock::new(|| card(18453, "dud", "Spell", 0, json!({ "keywords": [], "text": "dud" })));

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            SNEAK.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::degrade_secretly(json_as(json!({
                        "scope": { "side": "enemy", "zones": ["hand"], "types": ["Unit"] },
                        "random": 1,
                        "times": SECRET_NERFS,
                    })))]
                })),
                ..Script::default()
            }),
        ),
        (
            TRADER.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![effects::trade_offer()])),
                ..Script::default()
            }),
        ),
        (
            LUCKY_TRADER.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![effects::trade_offer()])),
                ..Script::default()
            }),
        ),
    ])
});

pub static HIDDEN_TRADE_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        SNEAK.clone(),
        TRADER.clone(),
        LUCKY_TRADER.clone(),
        STOIC.clone(),
        DUD.clone(),
    ]
});

/// The engine's fixture catalog (`harness::setup_catalog`) with these cards and scripts on top.
pub fn register_hidden_trade_fixtures() {
    super::harness::setup_catalog();
    let mut defs: CardDefs = registered_catalog().clone();
    for def in HIDDEN_TRADE_DEFS.iter() {
        defs.insert(def.id.clone(), def.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}
