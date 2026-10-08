//! Fixtures for the engine half of patch v0.2.0's Core patches and cosmetics (test/corePatches.test.ts):
//! the play count R429 keeps (`timesPlayed`), the price a return that keeps it is noted with (R429,
//! R766, issue #557) and the mark R437 puts on a card a delayed effect waits for. Test-only
//! definitions and scripts, prefixed `cp-` and indexed from 4400, so they collide with no other file's.
//!
//! Port of `packages/engine/test/fixtures/corePatches.ts`. TS numbered its defs with a module counter
//! (`nextIndex`, from 4400, one per `def` call in file order); each def here states the index that
//! counter gave it.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects::{damage, delay};
use jackioh_engine::testkit::*;

/// TS `def(name, type, extra = {})`: a Core Common at cost 1, a 1/1 when it is a Unit (unless `extra`
/// names `attack` or `health`), both faces alike, every other key of `extra` over it.
fn def(name: &str, index: i32, type_: &str, extra: Value) -> CardDef {
    let mut rest = if extra.is_object() { extra } else { json!({}) };
    let attack = rest
        .as_object_mut()
        .and_then(|object| object.remove("attack"))
        .and_then(|v| v.as_i64())
        .map_or(1, |n| n as i32);
    let health = rest
        .as_object_mut()
        .and_then(|object| object.remove("health"))
        .and_then(|v| v.as_i64())
        .map_or(1, |n| n as i32);
    let face = if type_ == "Unit" {
        json!({ "attack": attack, "health": health, "keywords": [], "text": name })
    } else {
        json!({ "keywords": [], "text": name })
    };
    let mut base = json!({
        "id": format!("cp-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (core patches)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": face,
        "radiant": face,
    });
    if let (Some(target), Value::Object(rest)) = (base.as_object_mut(), rest) {
        for (key, value) in rest {
            target.insert(key, value);
        }
    }
    json_as(base)
}

/// A Spell that asks the engine to count its plays (R429).
pub static counted: LazyLock<CardDef> = LazyLock::new(|| def("counted", 4401, "Spell", json!({})));
/// A Spell that does not.
pub static uncounted: LazyLock<CardDef> = LazyLock::new(|| def("uncounted", 4402, "Spell", json!({})));
/// A Unit whose Cry marks an enemy permanent for a hit at the start of its controller's next turn (R437).
pub static marker: LazyLock<CardDef> = LazyLock::new(|| def("marker", 4403, "Unit", json!({ "health": 3 })));
/// A Trap that watches nothing: a face-down card to mark (R33).
pub static quiet_trap: LazyLock<CardDef> = LazyLock::new(|| def("quiet-trap", 4404, "Trap", json!({})));
/// R429, R766: a Spell with an end-of-turn return whose return keeps the climb it was played at
/// (`StaticFlags.returnKeepsPrice`, #31 KY's Math Equation's shape). Its return itself does nothing, so
/// the card stays in its graveyard and the note's lifetime can be read there.
pub static priced_return: LazyLock<CardDef> =
    LazyLock::new(|| def("priced-return", 4405, "Spell", json!({})));
/// The same return without the flag (#23 Reoccurring Dream's shape).
pub static plain_return: LazyLock<CardDef> = LazyLock::new(|| def("plain-return", 4406, "Spell", json!({})));

pub static CORE_PATCH_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        counted.clone(),
        uncounted.clone(),
        marker.clone(),
        quiet_trap.clone(),
        priced_return.clone(),
        plain_return.clone(),
    ]
});

pub fn core_patch_catalog() -> CardDefs {
    CORE_PATCH_DEFS
        .iter()
        .map(|card| (card.id.clone(), card.clone()))
        .collect()
}

/// R437: the mark the marker's delayed hit puts on its target.
pub const TEST_MARK: MarkSpec = MarkSpec {
    mark: "hit",
    color: "green",
};

const HIT: &str = "hit";

fn marker_script() -> Script {
    Script {
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "enemy", "of": ["unit", "backrow"] }),
        )],
        cry: Some(hook(|ctx| {
            let Some(Selection::Instance { instance_id }) = ctx.targets.first() else {
                return vec![];
            };
            vec![delay(json_as(json!({
                "at": { "phase": "start", "player": "self" },
                "step": HIT,
                "hook": RESUME_HOOK,
                "data": { "target": instance_id },
                "watch": instance_id,
                "mark": TEST_MARK,
            })))]
        })),
        resume: IndexMap::from([(
            HIT,
            hook(|ctx| match ctx.data.get("target").and_then(Value::as_str) {
                Some(target) => vec![damage(json_as(json!({
                    "to": { "of": "instance", "instanceId": target },
                    "amount": 1,
                })))],
                None => vec![],
            }),
        )]),
        ..Script::default()
    }
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

pub static CORE_PATCH_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            counted.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    counts_plays: Some(true),
                    ..StaticFlags::default()
                }),
                cry: Some(hook(|_ctx| vec![])),
                ..Script::default()
            }),
        ),
        (
            uncounted.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![])),
                ..Script::default()
            }),
        ),
        (marker.id.clone(), both(marker_script())),
        (quiet_trap.id.clone(), both(Script::default())),
        (
            priced_return.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    return_keeps_price: Some(true),
                    ..StaticFlags::default()
                }),
                cry: Some(hook(|_ctx| vec![])),
                end_of_turn: Some(hook(|_ctx| vec![])),
                ..Script::default()
            }),
        ),
        (
            plain_return.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![])),
                end_of_turn: Some(hook(|_ctx| vec![])),
                ..Script::default()
            }),
        ),
    ])
});
