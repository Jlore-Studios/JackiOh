//! Port of `packages/engine/test/fixtures/twiceForward.ts`.
//!
//! Test-only definitions and scripts for `subsystems/twiceForward.ts` (C+ #74 Twice Forward One Step
//! Backwards, R425). The engine does not depend on `packages/cards`; the real card's test covers the same
//! cases again (packages/cards/test/classic-plus/074-twice-forward-one-step-backwards.test.ts).

use std::sync::LazyLock;

use jackioh_engine::effects;
use jackioh_engine::subsystems::twice_forward::twice_forward_trigger;
use jackioh_engine::testkit::*;

use super::catalog::{spell_def, unit_def};

/// #74's stand-in: a (2) Field Trap printing Brittle 2 (Radiant 4), every 2 plays, +1 Brittle.
pub fn forward() -> CardDef {
    spell_def(
        960,
        json!({
            "id": "fx-tf-forward",
            "name": "Fixture Twice Forward",
            "type": "Field Trap",
            "cost": 2,
            "params": [
                { "key": "plays", "base": 2, "radiant": 2, "better": "down", "step": 1, "min": 2 },
                { "key": "brittleGain", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1 },
            ],
            "base": { "keywords": [{ "kind": "Brittle", "n": 2 }], "text": "Brittle 2" },
            "radiant": { "keywords": [{ "kind": "Brittle", "n": 4 }], "text": "Brittle 4" },
        }),
    )
}

/// The opponent's plays: a Spell that stays in the graveyard, one that exiles itself, a Trap.
pub fn spell() -> CardDef {
    spell_def(961, json!({ "id": "fx-tf-spell", "name": "Fixture Plain Spell" }))
}

pub fn self_exiler() -> CardDef {
    spell_def(962, json!({ "id": "fx-tf-exiler", "name": "Fixture Self-Exiling Spell" }))
}

pub fn trap() -> CardDef {
    spell_def(963, json!({ "id": "fx-tf-trap", "name": "Fixture Trap", "type": "Trap" }))
}

/// A Unit whose end-of-turn line hits the enemy hero for 1, and one whose Cry hits it for 5.
pub fn turner() -> CardDef {
    unit_def(
        964,
        json!({ "id": "fx-tf-turner", "name": "Fixture End-Of-Turn Unit", "attack": 1, "health": 1 }),
    )
}

pub fn crier() -> CardDef {
    unit_def(965, json!({ "id": "fx-tf-crier", "name": "Fixture Cry Unit", "attack": 1, "health": 1 }))
}

/// A Spell whose resolution casts the plain Spell (R70): the cast is the later play, though it resolves first.
pub fn caster() -> CardDef {
    spell_def(966, json!({ "id": "fx-tf-caster", "name": "Fixture Casting Spell" }))
}

pub const TURNER_DAMAGE: i32 = 1;
pub const CRIER_DAMAGE: i32 = 5;

pub fn twice_forward_catalog(base: CardDefs) -> CardDefs {
    let mut defs = base;
    for def in [forward(), spell(), self_exiler(), trap(), turner(), crier(), caster()] {
        defs.insert(def.id.clone(), def);
    }
    defs
}

/// `castNew({ def })`: a new card of that definition, cast by the running card's controller.
fn cast_new_of(def_id: String) -> Effect {
    effects::cast_new(effects::CastNewArgs {
        def: effects::CastNewDef::from(def_id),
        radiant: None,
        how: effects::CastHow::default(),
    })
}

fn same(script: impl Fn() -> Script) -> CardScripts {
    CardScripts {
        base: script(),
        radiant: script(),
    }
}

pub static TWICE_FORWARD_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let mut table = IndexMap::new();
    table.insert(
        forward().id,
        CardScripts {
            base: Script {
                triggers: vec![twice_forward_trigger(json_as(json!({ "radiantCopy": false })))],
                ..Script::default()
            },
            radiant: Script {
                triggers: vec![twice_forward_trigger(json_as(json!({ "radiantCopy": true })))],
                ..Script::default()
            },
        },
    );
    table.insert(
        self_exiler().id,
        same(|| Script {
            cry: Some(hook(|_ctx| vec![effects::exile(json_as(json!({ "target": { "of": "self" } })))])),
            ..Script::default()
        }),
    );
    table.insert(
        caster().id,
        same(|| Script {
            cry: Some(hook(|_ctx| vec![cast_new_of(spell().id)])),
            ..Script::default()
        }),
    );
    table.insert(
        turner().id,
        same(|| Script {
            end_of_turn: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": TURNER_DAMAGE }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        crier().id,
        same(|| Script {
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": CRIER_DAMAGE }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table
});
