//! Test-only cards for Stinkyscape+ E5 (issue #3548, R3548000–R3548004): Tipsy spells and units,
//! plain and priced units, an X Spell, the Warhero Token, Activate abilities with and without
//! Refresh, and Field Spell and Unit cards giving infinite mana, so the engine never imports
//! `crates/cards`. Ids are `e5-` from 35480, clear of every other fixture file's.

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

use super::harness::both;

/// A (0) Spell with Tipsy 2.
pub const TIPSY_SPELL: &str = "e5-tipsy-spell";
/// A (0) 1/1 Unit with Taunt and Tipsy 5.
pub const TIPSY_UNIT: &str = "e5-tipsy-unit";
/// A (0) 2/3 Unit with no text.
pub const PLAIN: &str = "e5-plain";
/// A (9) 1/1 Unit with no text.
pub const EXPENSIVE: &str = "e5-expensive";
/// An (X) Spell with no text.
pub const X_SPELL: &str = "e5-x-spell";
/// The Warhero Token: a (0) Spell, worth the game-wide value.
pub const WARHERO: &str = WARHERO_TOKEN_DEF_ID;
/// A (0) Field Spell with "zap" (`Count(2)`) and nothing else.
pub const ACTIVATOR: &str = "e5-activator";
/// A (0) Field Spell with a mana-(3) ability (`Count(1)`).
pub const PRICED: &str = "e5-priced";
/// A (0) Field Spell with an extra-turn-cooldown ability (`Count(1)`).
pub const COOLDOWN: &str = "e5-cooldown";
/// A (0) Field Spell with the engine's Refresh ability (`Count(1)`).
pub const REFRESHER: &str = "e5-refresher";
/// A (0) Field Spell giving infinite mana.
pub const REACTOR: &str = "e5-reactor";
/// A (0) 2/2 Unit giving infinite mana, Dormant for 1 turn.
pub const SLEEPY_REACTOR: &str = "e5-sleepy-reactor";

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
        "name": format!("{id} (e5 fixture)"),
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

fn ability(id: &str, label: &str, uses: ActivationUses, cost: Option<ActivationCost>) -> ActivationDecl {
    ActivationDecl {
        id: id.to_string(),
        label: label.to_string(),
        uses,
        cost,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|_| vec![]),
    }
}

pub fn scripts() -> IndexMap<String, CardScripts> {
    let entries: Vec<(&str, Script)> = vec![
        (
            ACTIVATOR,
            Script {
                activations: vec![ability("zap", "Zap", ActivationUses::Count(2), None)],
                ..Script::default()
            },
        ),
        (
            PRICED,
            Script {
                activations: vec![ability(
                    "pricey",
                    "Pricey",
                    ActivationUses::Count(1),
                    Some(ActivationCost {
                        mana: Some(3),
                        ..Default::default()
                    }),
                )],
                ..Script::default()
            },
        ),
        (
            COOLDOWN,
            Script {
                activations: vec![ability(
                    "slow",
                    "Slow",
                    ActivationUses::Count(1),
                    Some(ActivationCost {
                        extra_turn_cooldown: Some(true),
                        ..Default::default()
                    }),
                )],
                ..Script::default()
            },
        ),
        (
            REFRESHER,
            Script {
                activations: vec![effects::refresh_ability(ActivationUses::Count(1))],
                ..Script::default()
            },
        ),
        (
            REACTOR,
            Script {
                static_flags: Some(StaticFlags {
                    infinite_mana: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            },
        ),
        (
            SLEEPY_REACTOR,
            Script {
                static_flags: Some(StaticFlags {
                    infinite_mana: Some(true),
                    slumber: Some(SlumberText {
                        turns: Some(1),
                        wakes_by: None,
                    }),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            },
        ),
    ];
    let mut all: IndexMap<String, CardScripts> = entries
        .into_iter()
        .map(|(id, script)| (id.to_string(), both(script)))
        .collect();
    for id in [TIPSY_SPELL, TIPSY_UNIT, PLAIN, EXPENSIVE, X_SPELL, WARHERO] {
        all.insert(id.to_string(), both(Script::default()));
    }
    all
}

pub fn defs() -> Vec<CardDef> {
    vec![
        card(
            35480,
            TIPSY_SPELL,
            "Spell",
            None,
            json!([{ "kind": "Tipsy", "n": 2 }]),
            json!(0),
        ),
        card(
            35481,
            TIPSY_UNIT,
            "Unit",
            Some((1, 1)),
            json!([{ "kind": "Taunt" }, { "kind": "Tipsy", "n": 5 }]),
            json!(0),
        ),
        card(35482, PLAIN, "Unit", Some((2, 3)), json!([]), json!(0)),
        card(35483, EXPENSIVE, "Unit", Some((1, 1)), json!([]), json!(9)),
        card(35484, X_SPELL, "Spell", None, json!([]), json!("X")),
        card(35485, WARHERO, "Spell", None, json!([]), json!(0)),
        card(35486, ACTIVATOR, "Field Spell", None, json!([]), json!(0)),
        card(35487, PRICED, "Field Spell", None, json!([]), json!(0)),
        card(35488, COOLDOWN, "Field Spell", None, json!([]), json!(0)),
        card(35489, REFRESHER, "Field Spell", None, json!([]), json!(0)),
        card(35490, REACTOR, "Field Spell", None, json!([]), json!(0)),
        card(35491, SLEEPY_REACTOR, "Unit", Some((2, 2)), json!([]), json!(0)),
    ]
}

/// The engine's fixture catalog (`harness::setup_catalog`) with these cards and scripts on top.
pub fn register_e5_fixtures() {
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
