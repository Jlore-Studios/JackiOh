//! Test-only cards for the Infinite Assault's Batch verbs (issue #4588): "your Units have +N
//! Attack" and "your Units have <keyword>" this turn (R4588000), doubling a Unit's stats (R4588005)
//! and a copy of the cheapest or the highest-Attack Unit (R4588001), each through the engine's own
//! verbs so the engine never imports `crates/cards`. Ids are `ba-` from 45880, so they collide with
//! no other fixture file's.

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

use super::harness::both;

/// What the rally lends its Units (R4588000).
pub const RALLY_ATTACK: i32 = 2;

/// A Spell: "Your Units have +2 Attack and Rush this turn."
pub const RALLY: &str = "ba-rally";
/// A Spell: "Double the Attack and Health of target Unit."
pub const DOUBLE: &str = "ba-double";
/// A Spell: "Summon a copy of your cheapest Unit."
pub const COPY_CHEAPEST: &str = "ba-copy-cheapest";
/// A Spell: "Summon a copy of your highest-Attack Unit."
pub const COPY_MIGHTIEST: &str = "ba-copy-mightiest";

fn card(index: u32, id: &str, type_: &str, stats: Option<(i32, i32)>) -> CardDef {
    let (base, radiant) = match stats {
        Some((attack, health)) => (
            json!({ "attack": attack, "health": health, "keywords": [], "text": id }),
            json!({ "attack": 2 * attack, "health": 2 * health, "keywords": [], "text": id }),
        ),
        None => (
            json!({ "keywords": [], "text": id }),
            json!({ "keywords": [], "text": id }),
        ),
    };
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (Batch fixture)"),
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

fn cry(effects: impl Fn() -> Vec<Effect> + Send + Sync + 'static) -> CardScripts {
    both(Script {
        cry: Some(hook(move |_ctx| effects())),
        ..Script::default()
    })
}

fn copy_script(extreme: effects::UnitExtreme) -> CardScripts {
    cry(move || {
        vec![effects::for_extreme_unit(effects::ForExtremeUnitArgs {
            extreme,
            player: None,
            each: std::sync::Arc::new(|instance_id: &str| {
                effects::summon_copy(json_as(json!({
                    "of": { "of": "instance", "instanceId": instance_id },
                    "player": "self",
                })))
            }),
        })]
    })
}

fn scripts() -> IndexMap<String, CardScripts> {
    IndexMap::from([
        (
            RALLY.to_string(),
            cry(|| {
                vec![
                    effects::give_units_attack_this_turn(json_as(json!({ "amount": RALLY_ATTACK }))),
                    effects::give_units_keyword_this_turn(json_as(json!({
                        "keyword": { "kind": "Rush" },
                    }))),
                ]
            }),
        ),
        (
            DOUBLE.to_string(),
            both(Script {
                targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
                cry: Some(hook(|_ctx| {
                    vec![effects::double_stats(json_as(json!({ "target": { "of": "chosen" } })))]
                })),
                ..Script::default()
            }),
        ),
        (
            COPY_CHEAPEST.to_string(),
            copy_script(effects::UnitExtreme::Cheapest),
        ),
        (
            COPY_MIGHTIEST.to_string(),
            copy_script(effects::UnitExtreme::HighestAttack),
        ),
    ])
}

fn defs() -> Vec<CardDef> {
    vec![
        card(45880, RALLY, "Spell", None),
        card(45881, DOUBLE, "Spell", None),
        card(45882, COPY_CHEAPEST, "Spell", None),
        card(45883, COPY_MIGHTIEST, "Spell", None),
    ]
}

pub fn register_batch() {
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
