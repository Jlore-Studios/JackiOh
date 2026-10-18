//! Test-only cards for Meditative+ #20 KY's Competition's question sequence (`subsystems::ky_competition`,
//! R1184000, R1184001) and `summon`'s X (R1184003): `kc-competition` runs the card's own sequence and
//! summons `kc-kenny`, a token that prints 0/0 with X stats 1/1 (Radiant 2/2) and has Rush while its X
//! is at least `KENNY_RUSH_AT`, so the engine is proved without `crates/cards` (CLAUDE.md: the engine
//! never imports it). The real cards, Lil' Kenny's six thresholds included, are proved again in
//! `crates/cards/src/scripts/meditative_plus/`.

use jackioh_engine::subsystems::ky_competition::ky_competition_script;
use jackioh_engine::testkit::*;

/// The fixture Kenny's one threshold: Rush from this X on, read off its X, never its stats.
pub const KENNY_RUSH_AT: i32 = 4;

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

/// `kc-competition`: the fixture KY's Competition, a (1) Spell.
pub fn competition() -> CardDef {
    def(
        3501,
        "kc-competition",
        json!({ "tags": ["KY"], "rarity": "Rare" }),
    )
}

/// `kc-competition-qd`: the same with Quickdraw, so a replayed game deals it in the opening hand.
pub fn competition_qd() -> CardDef {
    def(
        3502,
        "kc-competition-qd",
        json!({ "tags": ["KY"], "rarity": "Rare" }),
    )
}

/// `kc-kenny`: the fixture Lil' Kenny, a (1) Unit token printing 0/0 with X stats 1/1 → 2/2.
pub fn kenny() -> CardDef {
    def(
        3503,
        "kc-kenny",
        json!({
            "type": "Unit",
            "tags": ["KY", "Token"],
            "rarity": "Token",
            "token": true,
            "base": { "attack": 0, "health": 0, "xStats": { "attack": 1, "health": 1 }, "keywords": [], "text": "kc-kenny" },
            "radiant": { "attack": 0, "health": 0, "xStats": { "attack": 2, "health": 2 }, "keywords": [], "text": "kc-kenny" },
        }),
    )
}

fn defs() -> Vec<CardDef> {
    vec![competition(), competition_qd(), kenny()]
}

fn scripts_table() -> IndexMap<String, CardScripts> {
    let made = ky_competition_script("kc-kenny");
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
    let rush = Script {
        conditional_keywords: Some(read_hook(|args| {
            if x_of(args.self_) >= KENNY_RUSH_AT {
                vec![Keyword::Rush]
            } else {
                Vec::new()
            }
        })),
        ..Script::default()
    };
    let mut table = IndexMap::new();
    table.insert(
        competition().id,
        CardScripts {
            base: script.clone(),
            radiant: script,
        },
    );
    table.insert(
        competition_qd().id,
        CardScripts {
            base: quick.clone(),
            radiant: quick,
        },
    );
    table.insert(
        kenny().id,
        CardScripts {
            base: rush.clone(),
            radiant: rush,
        },
    );
    table
}

pub fn register_ky_competition_fixtures() {
    let mut all_defs = registered_catalog().clone();
    all_defs.extend(defs().into_iter().map(|card| (card.id.clone(), card)));
    register_catalog(all_defs);
    let mut all_scripts = registered_scripts().clone();
    all_scripts.extend(scripts_table());
    register_scripts(all_scripts);
}
