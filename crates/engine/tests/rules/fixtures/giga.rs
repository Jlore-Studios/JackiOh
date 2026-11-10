//! Test-only cards for GIGA (issue #3546, R3546000–R3546004): a GIGA Unit and Field Spell, a
//! plain Unit, a Cleaver, a Stack Unit, a lane-locked Unit, a Reborn GIGA and an embiggen Unit that
//! is GIGA only embiggened, so the engine never imports `crates/cards`. Ids are `giga-` from
//! 35460, clear of every other fixture file's.

use jackioh_engine::testkit::*;

use super::harness::both;

/// A (0) 2/6 Unit with GIGA.
pub const GIGA_UNIT: &str = "giga-unit";
/// A (0) Field Spell with GIGA.
pub const GIGA_FIELD: &str = "giga-field";
/// A (0) 1/3 Unit with no text.
pub const PLAIN: &str = "giga-plain";
/// A (0) 3/8 Unit with Cleave.
pub const CLEAVER: &str = "giga-cleaver";
/// A (0) 1/1 Unit with Stack.
pub const STACKER: &str = "giga-stacker";
/// A (0) 1/5 Unit only Units in its lane may attack.
pub const LANE_ONLY: &str = "giga-lane-only";
/// A (0) 1/1 Unit with GIGA and Reborn.
pub const REBORN_GIGA: &str = "giga-reborn";
/// A (0 embiggen 1) 2/2 Unit, GIGA only embiggened.
pub const GROWER: &str = "giga-grower";
/// A (0) Trap with GIGA, face-down in the backrow: the FaceDown view branch (R3546004).
pub const GIGA_TRAP: &str = "giga-trap";

fn card(
    index: u32,
    id: &str,
    type_: &str,
    stats: Option<(i32, i32)>,
    keywords: Value,
    cost: Value,
) -> CardDef {
    let (base, radiant) = match stats {
        Some((attack, health)) => (
            json!({ "attack": attack, "health": health, "keywords": keywords, "text": id }),
            json!({ "attack": 2 * attack, "health": 2 * health, "keywords": keywords, "text": id }),
        ),
        None => (
            json!({ "keywords": keywords, "text": id }),
            json!({ "keywords": keywords, "text": id }),
        ),
    };
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (giga fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": base,
        "radiant": radiant,
    }))
}

pub fn scripts() -> IndexMap<String, CardScripts> {
    let entries: Vec<(&str, Script)> = vec![
        (
            LANE_ONLY,
            Script {
                static_flags: Some(StaticFlags {
                    attacked_only_from_lane: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            },
        ),
        (
            GROWER,
            Script {
                conditional_keywords: Some(read_hook(|a| {
                    if a.self_.embiggened == Some(true) {
                        vec![Keyword::Giga]
                    } else {
                        vec![]
                    }
                })),
                ..Script::default()
            },
        ),
    ];
    let mut all: IndexMap<String, CardScripts> = entries
        .into_iter()
        .map(|(id, script)| (id.to_string(), both(script)))
        .collect();
    for id in [
        GIGA_UNIT,
        GIGA_FIELD,
        PLAIN,
        CLEAVER,
        STACKER,
        REBORN_GIGA,
        GIGA_TRAP,
    ] {
        all.insert(id.to_string(), both(Script::default()));
    }
    all
}

pub fn defs() -> Vec<CardDef> {
    vec![
        card(
            35460,
            GIGA_UNIT,
            "Unit",
            Some((2, 6)),
            json!([{ "kind": "GIGA" }]),
            json!(0),
        ),
        card(
            35461,
            GIGA_FIELD,
            "Field Spell",
            None,
            json!([{ "kind": "GIGA" }]),
            json!(0),
        ),
        card(
            35468,
            GIGA_TRAP,
            "Trap",
            None,
            json!([{ "kind": "GIGA" }]),
            json!(0),
        ),
        card(35462, PLAIN, "Unit", Some((1, 3)), json!([]), json!(0)),
        card(
            35463,
            CLEAVER,
            "Unit",
            Some((3, 8)),
            json!([{ "kind": "Cleave" }]),
            json!(0),
        ),
        card(
            35464,
            STACKER,
            "Unit",
            Some((1, 1)),
            json!([{ "kind": "Stack" }]),
            json!(0),
        ),
        card(35465, LANE_ONLY, "Unit", Some((1, 5)), json!([]), json!(0)),
        card(
            35466,
            REBORN_GIGA,
            "Unit",
            Some((1, 1)),
            json!([{ "kind": "GIGA" }, { "kind": "Reborn" }]),
            json!(0),
        ),
        card(
            35467,
            GROWER,
            "Unit",
            Some((2, 2)),
            json!([]),
            json!({ "base": 0, "embiggen": 1 }),
        ),
    ]
}

/// The engine's fixture catalog (`harness::setup_catalog`) with these cards and scripts on top.
pub fn register_giga_fixtures() {
    super::harness::setup_catalog();
    let mut catalog: CardDefs = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(scripts());
    register_scripts(merged);
}
