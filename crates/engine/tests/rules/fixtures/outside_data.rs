//! Test-only cards for the Meditative+ batch "outside data and the autopilot" (issue #1183,
//! docs/meditative-plus-set.md MP5): the two-pick fuse (R1183001), the autopilot (R1183002,
//! R1183003) and the code-block Discover (R1183005). Each fixture is trimmed to the one property its
//! name promises, the same script on both faces; the real cards (M+ #2, #18, #25) and their tests
//! live in `crates/cards`.
//!
//! Ids are prefixed `od-` and indexed from 11830, so they cannot collide with another fixture file's.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects::{
    add_to_hand, autopilot, choose_mode, choose_target, damage, discover_from_catalog, discovered_code_card,
    fuse_generated,
};
use jackioh_engine::testkit::*;

use super::harness::setup_catalog;

/// The vanilla units the fuse and code fixtures name (`super::catalog::vanilla_catalog`'s).
pub const FIRST_UNIT: &str = "fx-1";
pub const SECOND_UNIT: &str = "fx-2";

/// A Core Common Spell `od-<name>` costing `cost`, with `extra` over the definition.
fn spell(name: &str, index: i32, cost: i32, extra: Value) -> CardDef {
    let mut card = json!({
        "id": format!("od-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (outside data)"),
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    });
    if let (Some(target), Value::Object(extra)) = (card.as_object_mut(), extra) {
        for (key, value) in extra {
            target.insert(key, value);
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

/// M+ #18's last clause as a fixture: "the AI plays the rest of your turn" (R1183002).
pub static offload: LazyLock<CardDef> = LazyLock::new(|| spell("offload", 11830, 1, json!({})));

/// The same, cast on draw (§2.4), as M+ #18 is.
pub static offload_on_draw: LazyLock<CardDef> =
    LazyLock::new(|| spell("offload-on-draw", 11831, 1, json!({})));

/// A (0) Spell that adds another of itself to its caster's hand: a turn that never runs out of
/// plays, for `AUTOPILOT_MAX_ACTIONS` (R1183003).
pub static refill: LazyLock<CardDef> = LazyLock::new(|| spell("refill", 11832, 0, json!({})));

/// A Spell that asks its caster for a target, any Unit or hero, and deals it 3 (R1183003's answers).
pub static zap: LazyLock<CardDef> = LazyLock::new(|| spell("zap", 11833, 1, json!({})));

/// A Spell whose question goes to the opponent (`choose_mode`'s `by: enemy`), so the autopilot's
/// turn waits on their answer (R1183002).
pub static ask_enemy: LazyLock<CardDef> = LazyLock::new(|| spell("ask-enemy", 11834, 1, json!({})));

/// A two-pick fuse of the two vanilla units, distinct, at (2) (R1183001).
pub static fuse_two: LazyLock<CardDef> = LazyLock::new(|| spell("fuse-two", 11835, 0, json!({})));

/// A two-pick fuse whose pools hold one card alone, distinct: the second pick has none (R1183001).
pub static fuse_same: LazyLock<CardDef> = LazyLock::new(|| spell("fuse-same", 11836, 0, json!({})));

/// A code Discover over the catalog, the pick added Radiant (R1183005).
pub static code_seer: LazyLock<CardDef> = LazyLock::new(|| spell("code-seer", 11837, 1, json!({})));

/// The resume step the target and the code fixtures re-enter.
const PICKED: &str = "picked";

/// The scripts of the fixtures, by def id.
pub static OUTSIDE_DATA_SCRIPTS: LazyLock<Vec<(String, CardScripts)>> = LazyLock::new(|| {
    vec![
        (
            offload.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![autopilot()])),
                ..Script::default()
            }),
        ),
        (
            offload_on_draw.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    cast_on_draw: Some(true),
                    ..StaticFlags::default()
                }),
                cry: Some(hook(|_ctx| vec![autopilot()])),
                ..Script::default()
            }),
        ),
        (
            refill.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![add_to_hand(json_as(json!({ "defId": "od-refill" })))]
                })),
                ..Script::default()
            }),
        ),
        (
            zap.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![choose_target(json_as(json!({
                        "step": PICKED,
                        "scope": { "side": "any", "of": ["unit", "hero"] },
                    })))]
                })),
                resume: IndexMap::from([(
                    PICKED,
                    hook(|_ctx| vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 3 })))]),
                )]),
                ..Script::default()
            }),
        ),
        (
            ask_enemy.id.clone(),
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
        (
            fuse_two.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![fuse_generated(json_as(json!({
                        "count": 2,
                        "pools": [{ "defId": [FIRST_UNIT, SECOND_UNIT] }, { "defId": [FIRST_UNIT, SECOND_UNIT] }],
                        "distinct": true,
                        "costOverride": 2,
                    })))]
                })),
                ..Script::default()
            }),
        ),
        (
            fuse_same.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![fuse_generated(json_as(json!({
                        "count": 2,
                        "pools": [{ "defId": [FIRST_UNIT] }, { "defId": [FIRST_UNIT] }],
                        "distinct": true,
                    })))]
                })),
                ..Script::default()
            }),
        ),
        (
            code_seer.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![discover_from_catalog(json_as(
                        json!({ "step": PICKED, "offer": "code" }),
                    ))]
                })),
                resume: IndexMap::from([(
                    PICKED,
                    hook(|ctx| match discovered_code_card(ctx) {
                        Some(picked) => {
                            vec![add_to_hand(json_as(json!({ "defId": picked, "radiant": true })))]
                        }
                        None => vec![],
                    }),
                )]),
                ..Script::default()
            }),
        ),
    ]
});

/// The shared fixture catalog with these cards, and their scripts, registered on this thread.
pub fn register_outside_data() {
    setup_catalog();
    let mut defs: CardDefs = registered_catalog().clone();
    for card in [
        &*offload,
        &*offload_on_draw,
        &*refill,
        &*zap,
        &*ask_enemy,
        &*fuse_two,
        &*fuse_same,
        &*code_seer,
    ] {
        defs.insert(card.id.clone(), card.clone());
    }
    register_catalog(defs);
    let mut scripts: IndexMap<String, CardScripts> = registered_scripts().clone();
    scripts.extend(OUTSIDE_DATA_SCRIPTS.clone());
    register_scripts(scripts);
}
