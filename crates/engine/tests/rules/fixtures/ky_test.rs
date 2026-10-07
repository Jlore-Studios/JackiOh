//! Port of `packages/engine/test/fixtures/kyTest.ts`.
//!
//! Test-only cards for Classic+ #42 KY's Test's question bank (`subsystems/kyTest.ts`, R420, R580): a
//! fixture KY's Test running `kyTestScript` over a fixture bank, and one card for each reward pool, so
//! the engine is proved without `packages/cards` (CLAUDE.md). The named rewards are registered under
//! the real ids `config.KY_TEST_REWARDS` names (The Coin, KY's Gift), as stand-ins.
//!
//! TS numbered the definitions from a module counter (`nextIndex = 3300`, one step per `def`); each
//! index is written out here in the order TS made them.

use std::sync::LazyLock;

use jackioh_engine::subsystems::ky_test::{KyTestProblem, ky_test_script};
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

fn def(index: u32, id: &str, type_: &str, extra: Value) -> CardDef {
    json_as(spread(
        json!({
            "id": id,
            "index": index.to_string(),
            "name": id,
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

fn pooled(index: u32, id: &str, tags: Value, cost: i32, rarity: &str) -> CardDef {
    def(index, id, "Spell", json!({ "tags": tags, "cost": cost, "rarity": rarity }))
}

/// Two Medium problems and one Hard one: enough to tell a bank draw from a fixed pick.
pub static FIXTURE_BANK: LazyLock<Vec<KyTestProblem>> = LazyLock::new(|| {
    json_as(json!([
        { "id": "m1", "difficulty": "Medium", "statement": "∫₀¹ 2x dx = ?", "options": ["1", "2", "1/2", "0"], "answer": "1" },
        { "id": "m2", "difficulty": "Medium", "statement": "∫₀¹∫₀¹ 1 dy dx = ?", "options": ["1", "2", "0", "1/2"], "answer": "1" },
        { "id": "h1", "difficulty": "Hard", "statement": "det [[2, 0], [0, 3]] = ?", "options": ["6", "5", "1", "0"], "answer": "6" },
    ]))
});

pub fn ky_test() -> CardDef {
    def(3301, "kt-test", "Spell", json!({ "tags": ["KY"], "rarity": "Legendary" }))
}

pub fn coin() -> CardDef {
    def(
        3302,
        "core-t-coin",
        "Spell",
        json!({ "tags": ["Token"], "rarity": "Token", "token": true, "cost": 0 }),
    )
}

pub fn gift() -> CardDef {
    def(
        3303,
        "classicplus-042-1",
        "Field Spell",
        json!({ "tags": ["KY", "Token"], "rarity": "Token", "token": true, "cost": 4 }),
    )
}

pub fn book() -> CardDef {
    pooled(3304, "kt-book", json!(["Book"]), 1, "Common")
}

pub fn ky_two() -> CardDef {
    pooled(3305, "kt-ky-two", json!(["KY"]), 2, "Common")
}

pub fn legend() -> CardDef {
    pooled(3306, "kt-legend", json!([]), 3, "Legendary")
}

pub fn four() -> CardDef {
    pooled(3307, "kt-four", json!([]), 4, "Common")
}

/// The fixture KY's Test with Quickdraw (§6.2), so a replayable game's opening hand holds it; in no reward pool.
pub fn ky_test_qd() -> CardDef {
    def(3308, "kt-test-qd", "Spell", json!({}))
}

fn defs() -> Vec<CardDef> {
    vec![ky_test(), ky_test_qd(), coin(), gift(), book(), ky_two(), legend(), four()]
}

fn scripts_table() -> IndexMap<String, CardScripts> {
    let made = ky_test_script(&FIXTURE_BANK);
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
    let mut table = IndexMap::new();
    table.insert(
        ky_test().id,
        CardScripts {
            base: script.clone(),
            radiant: script,
        },
    );
    table.insert(
        ky_test_qd().id,
        CardScripts {
            base: quick.clone(),
            radiant: quick,
        },
    );
    table
}

/// This file's definitions, by id (the brief's `catalog()`).
pub fn catalog() -> CardDefs {
    defs().into_iter().map(|card| (card.id.clone(), card)).collect()
}

/// This file's scripts, by id (the brief's `scripts()`).
pub fn scripts() -> IndexMap<String, CardScripts> {
    scripts_table()
}

pub fn register_ky_test_fixtures() {
    let mut all_defs = registered_catalog().clone();
    all_defs.extend(catalog());
    register_catalog(all_defs);
    let mut all_scripts = registered_scripts().clone();
    all_scripts.extend(scripts());
    register_scripts(all_scripts);
}
