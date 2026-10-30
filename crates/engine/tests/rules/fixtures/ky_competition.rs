//! Test-only cards for Meditative+ #20 KY's Competition's question sequence (`subsystems::ky_competition`,
//! R1184000, R1184001) and `summon`'s X (R1184003), without `crates/cards`: `kc-kenny` is a token printing
//! 0/0 with X stats 1/1 (Radiant 2/2) and Rush while its X is at least `KENNY_RUSH_AT`.

use jackioh_engine::subsystems::ky_competition::ky_competition_script;
use jackioh_engine::testkit::*;

use super::harness::spread;

/// Kenny has Rush from this X on, read off its X, never its stats.
pub const KENNY_RUSH_AT: i32 = 4;

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

pub fn competition() -> CardDef {
    def(
        3501,
        "kc-competition",
        json!({ "tags": ["KY"], "rarity": "Rare" }),
    )
}

/// With Quickdraw, so a replayed game deals it in the opening hand.
pub fn competition_qd() -> CardDef {
    def(
        3502,
        "kc-competition-qd",
        json!({ "tags": ["KY"], "rarity": "Rare" }),
    )
}

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
