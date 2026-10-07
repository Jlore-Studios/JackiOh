//! Port of `packages/engine/test/fixtures/papaya.ts`.
//!
//! Test-only cards for Classic+ #62 KY's Papaya's curve targeting (SPEC §8.7 row 62, R422; B5 E32).
//! The fixture Spell runs the engine half of the card exactly as the real script does — `papayaBegin`
//! on resolve, `papayaAnswered` on each answer — so the subsystem is proved without `packages/cards`
//! (CLAUDE.md: the engine never imports it); the card's own test repeats the key cases.

use std::sync::LazyLock;

use jackioh_engine::subsystems::papaya::{PAPAYA_STEP, papaya_answered, papaya_begin};
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

fn def(id: &str, type_: &str, extra: Value) -> CardDef {
    json_as(spread(
        json!({
            "id": format!("pp-{id}"),
            "index": format!("pp-{id}"),
            "name": format!("{id} (papaya)"),
            "set": "Core",
            "type": type_,
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

/// The curve Spell: both faces run one script, the running face deciding the rows (R422).
pub fn curve() -> CardDef {
    def("curve", "Spell", json!({}))
}

/// A plain 2/2 to stand on cells.
pub fn body() -> CardDef {
    def(
        "body",
        "Unit",
        json!({
            "base": { "attack": 2, "health": 2, "keywords": [], "text": "2/2" },
            "radiant": { "attack": 4, "health": 4, "keywords": [], "text": "4/4" },
        }),
    )
}

/// A Trap with no text, set face-down.
pub fn snare() -> CardDef {
    def("snare", "Trap", json!({}))
}

/// A Field Spell, face-up in the backrow.
pub fn field() -> CardDef {
    def("field", "Field Spell", json!({}))
}

/// A unit token, which ceases to exist rather than reach the exile pile (R11).
pub fn token() -> CardDef {
    def(
        "token",
        "Unit",
        json!({
            "tags": ["Token"],
            "rarity": "Token",
            "token": true,
            "base": { "attack": 1, "health": 1, "keywords": [], "text": "1/1" },
            "radiant": { "attack": 2, "health": 2, "keywords": [], "text": "2/2" },
        }),
    )
}

/// The curve with Quickdraw, for a replayable game whose opening hand holds it (R225).
pub fn curve_quickdraw() -> CardDef {
    def("curve-qd", "Spell", json!({}))
}

fn curve_script() -> Script {
    Script {
        cry: Some(hook(|_ctx| papaya_begin())),
        resume: IndexMap::from([(PAPAYA_STEP, hook(|ctx| papaya_answered(ctx)))]),
        ..Script::default()
    }
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

pub static PAPAYA_DEFS: LazyLock<Vec<CardDef>> =
    LazyLock::new(|| vec![curve(), body(), snare(), field(), token(), curve_quickdraw()]);

fn papaya_scripts() -> IndexMap<String, CardScripts> {
    let mut table = IndexMap::new();
    table.insert(curve().id, both(curve_script()));
    table.insert(
        curve_quickdraw().id,
        both(Script {
            static_flags: Some(StaticFlags {
                quickdraw: Some(true),
                ..StaticFlags::default()
            }),
            ..curve_script()
        }),
    );
    table
}

/// This file's definitions, by id (the brief's `catalog()`).
pub fn catalog() -> CardDefs {
    PAPAYA_DEFS.iter().map(|card| (card.id.clone(), card.clone())).collect()
}

/// This file's scripts, by id (the brief's `scripts()`).
pub fn scripts() -> IndexMap<String, CardScripts> {
    papaya_scripts()
}

pub fn register_papaya_fixtures() {
    let mut all_defs = registered_catalog().clone();
    all_defs.extend(catalog());
    register_catalog(all_defs);
    let mut all_scripts = registered_scripts().clone();
    all_scripts.extend(scripts());
    register_scripts(all_scripts);
}
