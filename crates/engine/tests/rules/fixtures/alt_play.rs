//! Test-only cards for face-down plays as Traps (ME-ALTPLAY, R1040–R1046).
//!
//! Each reproduces through the effects library the shape of a Meditative card that the systems
//! exist for: Knowledge Breaker's Unit permission (#45), Paranoia's Spell permission (#99), a Spell
//! with a required target, a Spell with Echo, and a vanilla Unit to set. The engine never imports
//! `packages/cards`, so these are the engine's proof; the real cards' tests prove the same cases
//! again.
//!
//! Ids are prefixed `ap-` and indexed from 6040 up, clear of every other fixture file (BUILD §0).

use std::sync::{Arc, LazyLock};

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

/// TS's `{ ...a, ...b }` on two object literals: `b`'s keys replace `a`'s.
fn spread(mut base: Value, extra: Value) -> Value {
    if let (Some(into), Value::Object(from)) = (base.as_object_mut(), extra) {
        for (key, value) in from {
            into.insert(key, value);
        }
    }
    base
}

fn def_json(index: u32, name: &str, type_: &str, extra: Value) -> Value {
    spread(
        json!({
            "id": format!("ap-{name}"),
            "index": index.to_string(),
            "name": format!("{name} (alt play)"),
            "set": "Core",
            "type": type_,
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": { "keywords": [], "text": name },
            "radiant": { "keywords": [], "text": name },
        }),
        extra,
    )
}

fn def(index: u32, name: &str, type_: &str, extra: Value) -> CardDef {
    json_as(def_json(index, name, type_, extra))
}

fn unit(index: u32, name: &str, attack: i32, health: i32, extra: Value, keywords: Value) -> CardDef {
    let faces = json!({
        "base": { "attack": attack, "health": health, "keywords": keywords, "text": name },
        "radiant": { "attack": attack * 2, "health": health * 2, "keywords": keywords, "text": name },
    });
    def(index, name, "Unit", spread(faces, extra))
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn face_down_units() -> FaceDownPlayHook {
    Arc::new(|_args| {
        vec![FaceDownPlayPermission {
            units: Some(true),
            ..FaceDownPlayPermission::default()
        }]
    })
}

fn face_down_spells() -> FaceDownPlayHook {
    Arc::new(|_args| {
        vec![FaceDownPlayPermission {
            spells: Some(true),
            ..FaceDownPlayPermission::default()
        }]
    })
}

// ---------------------------------------------------------------------------
// The five cards
// ---------------------------------------------------------------------------

/// Knowledge Breaker's shape: a Unit granting the Unit face-down play, whose Cry damages a chosen
/// target.
pub fn breaker() -> CardDef {
    unit(6040, "breaker", 2, 2, json!({ "cost": 1 }), json!([]))
}

/// Paranoia's shape: a Field Spell granting the Spell face-down play; Radiant, it adds Echo 1.
pub fn paranoia() -> CardDef {
    def(6041, "paranoia", "Field Spell", json!({ "cost": 2 }))
}

/// A Spell with a required target that deals 2.
pub fn bolt() -> CardDef {
    def(6042, "bolt", "Spell", json!({ "cost": 1 }))
}

/// A Spell with Echo 1.
pub fn echo_spell() -> CardDef {
    def(6043, "echo-spell", "Spell", json!({ "cost": 1 }))
}

/// A vanilla Unit to set.
pub fn filler() -> CardDef {
    unit(6044, "filler", 2, 2, json!({ "cost": 1 }), json!([]))
}

pub static AP_DEFS: LazyLock<Vec<CardDef>> =
    LazyLock::new(|| vec![breaker(), paranoia(), bolt(), echo_spell(), filler()]);

pub static AP_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let mut table: IndexMap<String, CardScripts> = IndexMap::new();
    table.insert(
        breaker().id,
        both(Script {
            targets: vec![TargetDecl::target(1, 1, json!({ "of": ["unit", "hero"] }))],
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "chosen" }, "amount": 2 }),
                ))]
            })),
            face_down_play: Some(face_down_units()),
            ..Script::default()
        }),
    );
    table.insert(
        paranoia().id,
        CardScripts {
            base: Script {
                face_down_play: Some(face_down_spells()),
                ..Script::default()
            },
            radiant: Script {
                face_down_play: Some(Arc::new(|_args| {
                    vec![FaceDownPlayPermission {
                        spells: Some(true),
                        echo: Some(1),
                        ..FaceDownPlayPermission::default()
                    }]
                })),
                ..Script::default()
            },
        },
    );
    table.insert(
        bolt().id,
        both(Script {
            targets: vec![TargetDecl {
                required: Some(true),
                ..TargetDecl::target(1, 1, json!({ "of": ["unit"] }))
            }],
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "chosen" }, "amount": 2 }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        echo_spell().id,
        both(Script {
            static_flags: Some(json_as(json!({ "echo": 1 }))),
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": 1 }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(filler().id, both(Script::default()));
    table
});

/// This file's scripts, by id.
pub fn scripts() -> IndexMap<String, CardScripts> {
    AP_SCRIPTS.clone()
}

/// Merge this file's cards into whatever catalog and scripts the test registered first.
pub fn register_alt_play() {
    let mut defs = registered_catalog().clone();
    for entry in AP_DEFS.iter() {
        defs.insert(entry.id.clone(), entry.clone());
    }
    register_catalog(defs);
    let mut all_scripts = registered_scripts().clone();
    all_scripts.extend(scripts());
    register_scripts(all_scripts);
}
