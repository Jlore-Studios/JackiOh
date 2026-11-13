//! Test-only cards for Flexible, Evasive and Frenzy (issue #3543, R3543000–R3543003): a Flexible
//! Unit, a Flying Unit, plain Units to fight, a Flexible Field Spell with stats (whose end-of-turn
//! text keeps running wherever it stands) and one without, Evasive Units (one with Taunt), and
//! Frenzied Units (one with Cleave), so the engine never imports `crates/cards`. Ids are `fef-`
//! from 35430, clear of every other fixture file's.

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

use super::harness::both;

/// A (0) 3/4 Unit with Flexible.
pub const FLEXER: &str = "fef-flexer";
/// A (0) 2/3 Unit with Flying.
pub const FLYER: &str = "fef-flyer";
/// A (0) 3/3 Unit with no text.
pub const GRUNT: &str = "fef-grunt";
/// A (0) 1/1 Unit with no text: what a 4-attack Unit kills.
pub const FRAIL: &str = "fef-frail";
/// A (0) 1/10 Unit with no text: what a 4-attack Unit does not kill.
pub const WALL: &str = "fef-wall";
/// A (0) 2/3 Field Spell with Flexible: its end of turn deals 1 to the enemy hero.
pub const FLEX_FIELD: &str = "fef-flex-field";
/// A (0) Field Spell with Flexible and no stats.
pub const BARE_FIELD: &str = "fef-bare-field";
/// A (0) 2/3 Unit with Evasive.
pub const EVADER: &str = "fef-evader";
/// A (0) 1/5 Unit with Evasive and Taunt.
pub const EVASIVE_TAUNT: &str = "fef-evasive-taunt";
/// A (0) 4/6 Unit with Frenzy.
pub const FRENZIED: &str = "fef-frenzied";
/// A (0) 4/6 Unit with Frenzy and Cleave.
pub const FRENZIED_CLEAVER: &str = "fef-frenzied-cleaver";

fn card(index: u32, id: &str, type_: &str, stats: Option<(i32, i32)>, keywords: Value) -> CardDef {
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
        "name": format!("{id} (flexible, evasive and frenzy fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": base,
        "radiant": radiant,
    }))
}

pub fn scripts() -> IndexMap<String, CardScripts> {
    let entries: Vec<(&str, Script)> = vec![(
        FLEX_FIELD,
        Script {
            end_of_turn: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": 1 }),
                ))]
            })),
            ..Script::default()
        },
    )];
    let mut all: IndexMap<String, CardScripts> = entries
        .into_iter()
        .map(|(id, script)| (id.to_string(), both(script)))
        .collect();
    for id in [
        FLEXER,
        FLYER,
        GRUNT,
        FRAIL,
        WALL,
        BARE_FIELD,
        EVADER,
        EVASIVE_TAUNT,
        FRENZIED,
        FRENZIED_CLEAVER,
    ] {
        all.insert(id.to_string(), both(Script::default()));
    }
    all
}

pub fn defs() -> Vec<CardDef> {
    vec![
        card(
            35430,
            FLEXER,
            "Unit",
            Some((3, 4)),
            json!([{ "kind": "Flexible" }]),
        ),
        card(35431, FLYER, "Unit", Some((2, 3)), json!([{ "kind": "Flying" }])),
        card(35432, GRUNT, "Unit", Some((3, 3)), json!([])),
        card(35433, FRAIL, "Unit", Some((1, 1)), json!([])),
        card(35434, WALL, "Unit", Some((1, 10)), json!([])),
        card(
            35435,
            FLEX_FIELD,
            "Field Spell",
            Some((2, 3)),
            json!([{ "kind": "Flexible" }]),
        ),
        card(
            35436,
            BARE_FIELD,
            "Field Spell",
            None,
            json!([{ "kind": "Flexible" }]),
        ),
        card(
            35437,
            EVADER,
            "Unit",
            Some((2, 3)),
            json!([{ "kind": "Evasive" }]),
        ),
        card(
            35438,
            EVASIVE_TAUNT,
            "Unit",
            Some((1, 5)),
            json!([{ "kind": "Evasive" }, { "kind": "Taunt" }]),
        ),
        card(
            35439,
            FRENZIED,
            "Unit",
            Some((4, 6)),
            json!([{ "kind": "Frenzy" }]),
        ),
        card(
            35440,
            FRENZIED_CLEAVER,
            "Unit",
            Some((4, 6)),
            json!([{ "kind": "Frenzy" }, { "kind": "Cleave" }]),
        ),
    ]
}

/// The engine's fixture catalog (`harness::setup_catalog`) with these cards and scripts on top.
pub fn register_fef_fixtures() {
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
