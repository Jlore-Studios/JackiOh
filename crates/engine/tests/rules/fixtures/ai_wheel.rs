//! Test-only cards for Stinkyscape's "the AI at the wheel" (issue #1844, part of #1836): the
//! autopilot aura (R1844000), the Pareto hint (R1844001) and the helpful cast (R1844002), each trimmed
//! to the one property its name promises (the real cards, Stinkyscape #46, #47, #147, #147.2, come
//! with the set). The catalog is the vanilla Core units and these cards only, as `perfect_hand.rs`'s
//! is, so `CHARGER` is the only Charge card the perfect pool holds.
//! Ids are prefixed `aw-` and indexed from 18440, clear of every other fixture file's.

use std::sync::LazyLock;

use jackioh_engine::effects::{
    HelpfulCastArgs, choose_mode, choose_target, damage, helpful_cast, pareto_hint,
};
use jackioh_engine::testkit::*;

use super::catalog::vanilla_catalog;
use super::harness::{both, setup_catalog, spread};

/// `HALFCYCLER`'s count: the autopilot plays the first this-many actions of each turn.
pub const HALF_ACTIONS: i32 = 2;
/// `HALFCYCLER_MORE`'s, larger.
pub const MORE_ACTIONS: i32 = 3;
/// What `ASK` deals the target its caster picks.
pub const ASK_DAMAGE: i32 = 3;

fn def(name: &str, index: i32, type_: &str, cost: i32, extra: Value) -> CardDef {
    json_as(spread(
        json!({
            "id": format!("aw-{name}"),
            "index": index.to_string(),
            "name": format!("{name} (AI at the wheel)"),
            "set": "Core",
            "type": type_,
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": cost,
            "base": { "keywords": [], "text": name },
            "radiant": { "keywords": [], "text": name },
        }),
        extra,
    ))
}

fn unit_faces(name: &str, attack: i32, health: i32, keywords: Value) -> Value {
    json!({
        "base": { "attack": attack, "health": health, "keywords": keywords, "text": name },
        "radiant": { "attack": attack * 2, "health": health * 2, "keywords": keywords, "text": name },
    })
}

/// #46's aura: the autopilot plays the first `HALF_ACTIONS` actions of each turn.
pub static HALFCYCLER: LazyLock<CardDef> =
    LazyLock::new(|| def("halfcycler", 18440, "Field Spell", 1, json!({})));

/// The same, `MORE_ACTIONS` of them.
pub static HALFCYCLER_MORE: LazyLock<CardDef> =
    LazyLock::new(|| def("halfcycler-more", 18441, "Field Spell", 1, json!({})));

/// #47's aura: the autopilot plays each turn whole.
pub static FULLCYCLER: LazyLock<CardDef> =
    LazyLock::new(|| def("fullcycler", 18442, "Field Spell", 1, json!({})));

/// #147's Cry: the Pareto hint for the rest of the turn.
pub static HINT_UNIT: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "hint-unit",
        18443,
        "Unit",
        1,
        unit_faces("hint-unit", 1, 1, json!([])),
    )
});

/// #147.2's aura: the Pareto hint on each of its controller's turns.
pub static HINT_FIELD: LazyLock<CardDef> =
    LazyLock::new(|| def("hint-field", 18444, "Field Spell", 1, json!({})));

/// A Spell that does something helpful, the base face's way (R1844002).
pub static HELPER: LazyLock<CardDef> = LazyLock::new(|| def("helper", 18445, "Spell", 1, json!({})));

/// #147.2's start of turn: something helpful as its controller's turn begins.
pub static HELPER_FIELD: LazyLock<CardDef> =
    LazyLock::new(|| def("helper-field", 18446, "Field Spell", 1, json!({})));

/// A 4/4 Charge for 1 (Radiant 8/8): the one card of the pool that reaches a hero at 4 this turn.
pub static CHARGER: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "charger",
        18447,
        "Unit",
        1,
        unit_faces("charger", 4, 4, json!([{ "kind": "Charge" }])),
    )
});

/// A 1/1 for 4: never the judge's best while a vanilla 2/2 for 1 is playable.
pub static DEAR: LazyLock<CardDef> =
    LazyLock::new(|| def("dear", 18448, "Unit", 4, unit_faces("dear", 1, 1, json!([]))));

/// Asks its caster for any Unit or hero and deals it 3 (`outside_data`'s zap).
pub static ASK: LazyLock<CardDef> = LazyLock::new(|| def("ask", 18449, "Spell", 1, json!({})));

/// Its question goes to the opponent (`outside_data`'s ask-enemy).
pub static ASK_ENEMY: LazyLock<CardDef> = LazyLock::new(|| def("ask-enemy", 18450, "Spell", 1, json!({})));

/// The resume step the two questions re-enter.
const PICKED: &str = "picked";

fn flagged(flags: StaticFlags) -> CardScripts {
    both(Script {
        static_flags: Some(flags),
        ..Script::default()
    })
}

pub static AI_WHEEL_SCRIPTS: LazyLock<Vec<(String, CardScripts)>> = LazyLock::new(|| {
    vec![
        (
            HALFCYCLER.id.clone(),
            flagged(StaticFlags {
                pilots_actions: Some(HALF_ACTIONS),
                ..StaticFlags::default()
            }),
        ),
        (
            HALFCYCLER_MORE.id.clone(),
            flagged(StaticFlags {
                pilots_actions: Some(MORE_ACTIONS),
                ..StaticFlags::default()
            }),
        ),
        (
            FULLCYCLER.id.clone(),
            flagged(StaticFlags {
                pilots_turn: Some(true),
                ..StaticFlags::default()
            }),
        ),
        (
            HINT_UNIT.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![pareto_hint()])),
                ..Script::default()
            }),
        ),
        (
            HINT_FIELD.id.clone(),
            flagged(StaticFlags {
                hints_plays: Some(true),
                ..StaticFlags::default()
            }),
        ),
        (
            HELPER.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![helpful_cast(HelpfulCastArgs::default())])),
                ..Script::default()
            }),
        ),
        (
            HELPER_FIELD.id.clone(),
            both(Script {
                start_of_turn: Some(hook(|_ctx| vec![helpful_cast(HelpfulCastArgs::default())])),
                ..Script::default()
            }),
        ),
        (
            ASK.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![choose_target(json_as(json!({
                        "step": PICKED,
                        "scope": { "side": "any", "of": ["unit", "hero"] },
                    })))]
                })),
                resume: IndexMap::from([(
                    PICKED,
                    hook(|_ctx| {
                        vec![damage(json_as(
                            json!({ "to": { "of": "chosen" }, "amount": ASK_DAMAGE }),
                        ))]
                    }),
                )]),
                ..Script::default()
            }),
        ),
        (
            ASK_ENEMY.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![choose_mode(json_as(json!({
                        "options": ["left", "right"],
                        "step": PICKED,
                        "by": "enemy",
                    })))]
                })),
                resume: IndexMap::from([(PICKED, hook(|_ctx| vec![]))]),
                ..Script::default()
            }),
        ),
    ]
});

/// Registers the catalog the header names, with the scripts, on this thread.
pub fn register_ai_wheel() {
    setup_catalog();
    let mut defs = vanilla_catalog(2 * DECK_SIZE, 1);
    for card in [
        &*HALFCYCLER,
        &*HALFCYCLER_MORE,
        &*FULLCYCLER,
        &*HINT_UNIT,
        &*HINT_FIELD,
        &*HELPER,
        &*HELPER_FIELD,
        &*CHARGER,
        &*DEAR,
        &*ASK,
        &*ASK_ENEMY,
    ] {
        defs.insert(card.id.clone(), card.clone());
    }
    register_catalog(defs);
    let mut scripts: IndexMap<String, CardScripts> = registered_scripts().clone();
    scripts.extend(AI_WHEEL_SCRIPTS.clone());
    register_scripts(scripts);
}
