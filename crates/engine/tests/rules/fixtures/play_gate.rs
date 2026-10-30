//! Test-only cards for a card's play gate (`Script.play_allowed`, R1851000–R1851002, issue #1851):
//! Spells whose play needs an enemy Unit on the field though they declare no target (Meditative #53
//! Prestige's Radiant face). Ids `pg-…` from 18510, clear of every other fixture file (BUILD §0).

use std::sync::LazyLock;

use jackioh_engine::effects::{damage, destroy_all};
use jackioh_engine::testkit::*;

use super::harness::both;

fn def(index: u32, name: &str, type_: &str, keywords: Value) -> CardDef {
    let face = if type_ == "Unit" {
        json!({ "attack": 2, "health": 2, "keywords": keywords, "text": name })
    } else {
        json!({ "keywords": keywords, "text": name })
    };
    json_as(json!({
        "id": format!("pg-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (play gate)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": face,
        "radiant": face,
    }))
}

/// "Deal 1 damage to the enemy hero", played only while an enemy Unit stands on the field.
pub fn gated() -> CardDef {
    def(18510, "gated", "Spell", json!([]))
}

/// `gated` with Echo 1, whose Cry also destroys every enemy Unit, so its repeat finds none.
pub fn gated_echo() -> CardDef {
    def(18511, "gated-echo", "Spell", json!([]))
}

/// `gated` with Cloaked, so it may be set face-down and revealed (R783004, R783006).
pub fn gated_cloaked() -> CardDef {
    def(18512, "gated-cloaked", "Spell", json!([{ "kind": "Cloaked" }]))
}

/// "Deal 1 damage to the enemy hero", with no gate: a fusion's other ingredient.
pub fn plain_spell() -> CardDef {
    def(18513, "plain-spell", "Spell", json!([]))
}

pub fn dummy() -> CardDef {
    def(18514, "dummy", "Unit", json!([]))
}

fn enemy_unit_stands() -> PlayAllowedHook {
    play_allowed_hook(|state, player| !active_units_of(state, opponent_of(player)).is_empty())
}

fn hit_enemy_hero() -> Effect {
    damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 })))
}

pub static PLAY_GATE_DEFS: LazyLock<Vec<CardDef>> =
    LazyLock::new(|| vec![gated(), gated_echo(), gated_cloaked(), plain_spell(), dummy()]);

pub static PLAY_GATE_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let gated_script = Script {
        play_allowed: Some(enemy_unit_stands()),
        cry: Some(hook(|_ctx| vec![hit_enemy_hero()])),
        ..Script::default()
    };
    let mut table: IndexMap<String, CardScripts> = IndexMap::new();
    table.insert(gated().id, both(gated_script.clone()));
    table.insert(gated_cloaked().id, both(gated_script));
    table.insert(
        gated_echo().id,
        both(Script {
            play_allowed: Some(enemy_unit_stands()),
            static_flags: Some(StaticFlags {
                echo: Some(1),
                ..StaticFlags::default()
            }),
            cry: Some(hook(|_ctx| {
                vec![
                    hit_enemy_hero(),
                    destroy_all(json_as(json!({ "side": "enemy", "rows": ["units"] }))),
                ]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        plain_spell().id,
        both(Script {
            cry: Some(hook(|_ctx| vec![hit_enemy_hero()])),
            ..Script::default()
        }),
    );
    table.insert(dummy().id, both(Script::default()));
    table
});

pub fn register_play_gate() {
    let mut defs = registered_catalog().clone();
    for entry in PLAY_GATE_DEFS.iter() {
        defs.insert(entry.id.clone(), entry.clone());
    }
    register_catalog(defs);
    let mut all_scripts = registered_scripts().clone();
    all_scripts.extend(PLAY_GATE_SCRIPTS.clone());
    register_scripts(all_scripts);
}
