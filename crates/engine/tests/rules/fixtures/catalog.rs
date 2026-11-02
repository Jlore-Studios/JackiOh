//! Script-less definitions for engine tests (BUILD §0), with JSON overrides (`json!({ "cost": 3 })`)
//! spread over the defaults key by key. Surface contract: docs/v0.3.0/SURFACE.md §4.1, §8.

use jackioh_engine::testkit::*;

use super::harness::spread;

fn take(overrides: &mut Value, key: &str) -> Option<Value> {
    overrides.as_object_mut().and_then(|object| object.remove(key))
}

/// A vanilla Unit `fx-<index>`, 2/2 unless `overrides` names `attack` or `health` (its Radiant face
/// doubles them); `overrides`' `keywords` go on both faces, every other key over the definition.
pub fn unit_def(index: i32, overrides: Value) -> CardDef {
    let mut rest = if overrides.is_object() {
        overrides
    } else {
        json!({})
    };
    let attack = take(&mut rest, "attack")
        .and_then(|v| v.as_i64())
        .map_or(2, |n| n as i32);
    let health = take(&mut rest, "health")
        .and_then(|v| v.as_i64())
        .map_or(2, |n| n as i32);
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

/// A Spell `fx-<index>` that does nothing, with `overrides` over it.
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

/// A 3/3 Rush unit token `fx-token-<name>`.
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
        // §7: a DISTINCT radiant face (as the real Rush Token's), so a test can tell it from the base.
        "radiant": { "attack": 6, "health": 6, "keywords": [{ "kind": "Rush" }], "text": "token" },
    }))
}

/// `count` distinct vanilla units, indexed from `from`.
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

/// The first `size` ids of a vanilla catalog, as a legal deck.
pub fn vanilla_deck(size: i32, from: i32) -> Vec<String> {
    (0..size).map(|i| format!("fx-{}", from + i)).collect()
}
