//! Fixture cards for Meditative #40 Feng Shui (R980–R987). The engine never imports
//! `packages/cards` (CLAUDE.md), so the element cycle, the judgement and Luck are proved on cards of
//! their shape here. Ids are `fs-*`, indices from 8600, registered on top of the shared fixture
//! catalog by `with_feng_shui`.

use std::sync::LazyLock;

use jackioh_engine::testkit::*;

fn def(index: u32, id: &str, type_: &str, extra: Value) -> CardDef {
    let mut rest = match extra {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    let attack = rest.remove("attack").and_then(|v| v.as_i64());
    let health = rest.remove("health").and_then(|v| v.as_i64());
    let keywords = rest.remove("keywords").unwrap_or_else(|| json!([]));
    let mut base = json!({ "keywords": keywords, "text": id });
    let mut radiant = json!({ "keywords": keywords, "text": format!("{id} radiant") });
    if type_ == "Unit" {
        let (attack, health) = (attack.unwrap_or(1), health.unwrap_or(1));
        if let (Some(base), Some(radiant)) = (base.as_object_mut(), radiant.as_object_mut()) {
            base.insert("attack".to_string(), json!(attack));
            base.insert("health".to_string(), json!(health));
            radiant.insert("attack".to_string(), json!(attack * 2));
            radiant.insert("health".to_string(), json!(health * 2));
        }
    }
    let mut card = json!({
        "id": format!("fs-{id}"),
        "index": index.to_string(),
        "name": format!("{id} (feng shui)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": base,
        "radiant": radiant,
    });
    if let Some(into) = card.as_object_mut() {
        for (key, value) in rest {
            into.insert(key, value);
        }
    }
    json_as(card)
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// TS's `FS` object: one definition per key. The units' indices end in 1–5 on purpose: 水 火 木 金
/// 土 in Hetu order (R980). The Trap ends in 2 (火); the judge ends in 0 (土).
pub struct Fs {
    /// A Unit of each element (indices 8601–8605: Water, Fire, Wood, Metal, Earth).
    pub water: CardDef,
    pub fire: CardDef,
    pub wood: CardDef,
    pub metal: CardDef,
    pub earth: CardDef,
    /// A Trap (Fire): a face-down play, never judged nor recorded (R982).
    pub trap: CardDef,
    /// A Field Spell (Earth) that judges plays and grants Luck 1 (R981, R987).
    pub judge: CardDef,
}

pub static FS: LazyLock<Fs> = LazyLock::new(|| Fs {
    water: def(8601, "water", "Unit", json!({})),
    fire: def(8602, "fire", "Unit", json!({})),
    wood: def(8603, "wood", "Unit", json!({})),
    earth: def(8605, "earth", "Unit", json!({})),
    metal: def(8604, "metal", "Unit", json!({})),
    trap: def(8612, "trap", "Trap", json!({})),
    judge: def(
        8620,
        "judge",
        "Field Spell",
        json!({ "params": [{ "key": "luck", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1 }] }),
    ),
});

/// `Object.values(FS)`, in `FS`'s key order.
pub static FS_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    let fs = &*FS;
    vec![
        fs.water.clone(),
        fs.fire.clone(),
        fs.wood.clone(),
        fs.metal.clone(),
        fs.earth.clone(),
        fs.trap.clone(),
        fs.judge.clone(),
    ]
});

fn scripts_table() -> IndexMap<String, CardScripts> {
    let fs = &*FS;
    let mut table: IndexMap<String, CardScripts> = IndexMap::new();
    for def in [&fs.water, &fs.fire, &fs.wood, &fs.metal, &fs.earth, &fs.trap] {
        table.insert(def.id.clone(), both(Script::default()));
    }
    table.insert(
        fs.judge.id.clone(),
        both(Script {
            static_flags: Some(StaticFlags {
                feng_shui: Some(true),
                luck: Some(1),
                ..StaticFlags::default()
            }),
            ..Script::default()
        }),
    );
    table
}

pub fn catalog() -> CardDefs {
    FS_DEFS
        .iter()
        .map(|card| (card.id.clone(), card.clone()))
        .collect()
}

pub fn scripts() -> IndexMap<String, CardScripts> {
    scripts_table()
}

pub fn register_feng_shui() {
    let mut all_defs = registered_catalog().clone();
    all_defs.extend(catalog());
    register_catalog(all_defs);
    let mut all_scripts = registered_scripts().clone();
    all_scripts.extend(scripts());
    register_scripts(all_scripts);
}

pub fn with_feng_shui(state: GameState) -> GameState {
    register_feng_shui();
    state
}
