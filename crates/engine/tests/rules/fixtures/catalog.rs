//! Script-less definitions for engine tests: real cards arrive in M4, so M1–M3 tests use these
//! (BUILD §0). Each helper returns plain CardDefs, registered through createGame.
//!
//! Port of `packages/engine/test/fixtures/catalog.ts` (SURFACE §4.1, §8). TS's `Partial<CardDef>`
//! overrides are the TS object literal as JSON (`json!({ "cost": 3, "attack": 4 })`), spread over the
//! defaults key by key as TS's `{ ...defaults, ...overrides }` does; a defaulted TS argument is passed
//! explicitly (`vanilla_catalog(40, 1)`, `unit_def(1, json!({}))`).

use jackioh_engine::testkit::*;

/// TS `{ ...base, ...extra }`: every key of `extra` written over `base`.
fn spread(mut base: Value, extra: Value) -> Value {
    if let (Some(target), Value::Object(extra)) = (base.as_object_mut(), extra) {
        for (key, value) in extra {
            target.insert(key, value);
        }
    }
    base
}

/// TS `const { key, ...rest } = overrides`: takes `key` out of the literal.
fn take(overrides: &mut Value, key: &str) -> Option<Value> {
    overrides.as_object_mut().and_then(|object| object.remove(key))
}

/// A vanilla Unit `fx-<index>`, 2/2 unless `overrides` names `attack` or `health` (its Radiant face
/// doubles them), with `overrides`' `keywords` on both faces and every other key of `overrides` over
/// the definition. TS `unitDef(index, overrides = {})`.
pub fn unit_def(index: i32, overrides: Value) -> CardDef {
    let mut rest = if overrides.is_object() { overrides } else { json!({}) };
    let attack = take(&mut rest, "attack").and_then(|v| v.as_i64()).map_or(2, |n| n as i32);
    let health = take(&mut rest, "health").and_then(|v| v.as_i64()).map_or(2, |n| n as i32);
    let keywords = take(&mut rest, "keywords").unwrap_or_else(|| json!([]));
    json_as(spread(
        json!({
            "id": format!("fx-{index}"),
            "index": index.to_string(),
            "name": format!("Fixture Unit {index}"),
            "set": "Core",
            "type": "Unit",
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": { "attack": attack, "health": health, "keywords": keywords, "text": "vanilla" },
            "radiant": { "attack": attack * 2, "health": health * 2, "keywords": keywords, "text": "vanilla" },
        }),
        rest,
    ))
}

/// A Spell `fx-<index>` that does nothing, with `overrides` over it. TS `spellDef(index, overrides = {})`.
pub fn spell_def(index: i32, overrides: Value) -> CardDef {
    json_as(spread(
        json!({
            "id": format!("fx-{index}"),
            "index": index.to_string(),
            "name": format!("Fixture Spell {index}"),
            "set": "Core",
            "type": "Spell",
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": { "keywords": [], "text": "does nothing" },
            "radiant": { "keywords": [], "text": "does nothing" },
        }),
        overrides,
    ))
}

/// A 3/3 Rush unit token `fx-token-<name>`. TS `tokenDef(name, tags = ["Token"])`: pass
/// `[Tag::Token]` for the default.
pub fn token_def(name: &str, tags: impl Into<Vec<Tag>>) -> CardDef {
    let tags: Vec<Tag> = tags.into();
    json_as(json!({
        "id": format!("fx-token-{name}"),
        "index": format!("T-{name}"),
        "name": format!("Fixture {name} Token"),
        "set": "Core",
        "type": "Unit",
        "tags": tags,
        "rarity": "Token",
        "token": true,
        "cost": 1,
        "base": { "attack": 3, "health": 3, "keywords": [{ "kind": "Rush" }], "text": "token" },
        // §7 gives every unit token a DISTINCT radiant face now (the real Rush Token is 3/3 -> 6/6),
        // so the fixture carries one too. A fixture whose faces were identical could not tell a test
        // that reads the radiant face from one that silently read the base.
        "radiant": { "attack": 6, "health": 6, "keywords": [{ "kind": "Rush" }], "text": "token" },
    }))
}

/// `count` distinct vanilla units, indexed from `from`. TS `vanillaCatalog(count = 40, from = 1)`.
pub fn vanilla_catalog(count: i32, from: i32) -> CardDefs {
    let mut defs = CardDefs::new();
    for i in from..from + count {
        let def = unit_def(i, json!({}));
        defs.insert(def.id.clone(), def);
    }
    let token = token_def("rush", [Tag::Token]);
    defs.insert(token.id.clone(), token);
    defs
}

/// The first `size` ids of a vanilla catalog, as a legal deck. TS `vanillaDeck(size = 20, from = 1)`.
pub fn vanilla_deck(size: i32, from: i32) -> Vec<String> {
    (0..size).map(|i| format!("fx-{}", from + i)).collect()
}
