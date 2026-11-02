//! Test-only cards for layered summons and the lethal guard on a pile (issue #1842): several Units
//! summoned into one unit zone as a §3.2 Stack pile, bottom first (`summon_pile`, R1842000), and
//! R1204's lethal guard standing in such a pile (R1842001). Ids `ls-…` from 18420 are no other file's.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects::{buff, choose_mode, damage, destroy_all, summon_pile};
use jackioh_engine::testkit::*;

use super::harness::{both, spread};

/// What the bolt fixture deals: lethal to the ally and the middle, not to the guard through its Armor.
pub const BOLT: i32 = 10;
/// What the smite fixture deals: lethal to the guard through its Armor, and to a hero at 20.
pub const SMITE: i32 = 30;
pub const GUARD_ARMOR: i32 = 4;

fn card(index: u32, name: &str, type_: &str, stats: Option<(i32, i32)>, keywords: Value) -> CardDef {
    let (face, radiant) = match stats {
        Some((attack, health)) => (
            json!({ "attack": attack, "health": health, "keywords": keywords, "text": name }),
            json!({ "attack": 2 * attack, "health": 2 * health, "keywords": keywords, "text": name }),
        ),
        None => (
            json!({ "keywords": keywords, "text": name }),
            json!({ "keywords": keywords, "text": name }),
        ),
    };
    json_as(json!({
        "id": format!("ls-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (layered summons fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face,
        "radiant": radiant,
    }))
}

fn replacement(id: &str, on: ReplacementMoment, instead: Value) -> ReplacementDef {
    ReplacementDef {
        id: id.to_string(),
        on,
        where_: None,
        when: None,
        instead: json_as(instead),
        then: None,
        by: None,
    }
}

/// A pile's bottom layer (the Drone Battleship's Meltdown).
pub static bottom: LazyLock<CardDef> =
    LazyLock::new(|| card(18420, "bottom", "Unit", Some((3, 6)), json!([])));
/// A pile's middle layer (Propagation).
pub static middle: LazyLock<CardDef> =
    LazyLock::new(|| card(18421, "middle", "Unit", Some((2, 6)), json!([])));
/// What the guard watches over.
pub static ally: LazyLock<CardDef> = LazyLock::new(|| card(18422, "ally", "Unit", Some((1, 4)), json!([])));
/// "Fatal damage a friendly ally would take is redirected to this" (R1204, the Shield Layer's shape).
pub static guard: LazyLock<CardDef> = LazyLock::new(|| {
    card(
        18423,
        "guard",
        "Unit",
        Some((2, 12)),
        json!([{ "kind": "Armor", "n": GUARD_ARMOR }]),
    )
});
/// Asks a question as it arrives (R151).
pub static asker: LazyLock<CardDef> = LazyLock::new(|| card(18424, "asker", "Unit", Some((1, 1)), json!([])));
/// "Whenever you summon a Unit, give it +1/+1".
pub static watcher: LazyLock<CardDef> =
    LazyLock::new(|| card(18425, "watcher", "Unit", Some((1, 1)), json!([])));
/// A layer that is no Unit.
pub static trap: LazyLock<CardDef> = LazyLock::new(|| card(18426, "trap", "Trap", None, json!([])));
/// "Summon bottom, middle and guard as one pile, guard on top". Radiant: every layer Radiant.
pub static pile: LazyLock<CardDef> = LazyLock::new(|| card(18427, "pile", "Spell", None, json!([])));
/// The same pile into lane 3.
pub static lane_pile: LazyLock<CardDef> =
    LazyLock::new(|| card(18428, "lane-pile", "Spell", None, json!([])));
/// A pile of bottom, trap and guard.
pub static mixed_pile: LazyLock<CardDef> =
    LazyLock::new(|| card(18429, "mixed-pile", "Spell", None, json!([])));
/// A pile of asker, middle and guard.
pub static asking_pile: LazyLock<CardDef> =
    LazyLock::new(|| card(18430, "asking-pile", "Spell", None, json!([])));
pub static bolt: LazyLock<CardDef> = LazyLock::new(|| card(18431, "bolt", "Spell", None, json!([])));
pub static smite: LazyLock<CardDef> = LazyLock::new(|| card(18432, "smite", "Spell", None, json!([])));
/// Destroy every Unit.
pub static wipe: LazyLock<CardDef> = LazyLock::new(|| card(18433, "wipe", "Spell", None, json!([])));

/// Summons `def_ids` as one pile, with `extra` (`lane`, `radiant`) beside them.
fn pile_script(def_ids: Vec<String>, extra: Value) -> Script {
    let args = spread(json!({ "defIds": def_ids }), extra);
    Script {
        cry: Some(hook(move |_ctx| vec![summon_pile(json_as(args.clone()))])),
        ..Script::default()
    }
}

/// The pile the `pile` fixtures summon, bottom first.
fn layers() -> Vec<String> {
    vec![bottom.id.clone(), middle.id.clone(), guard.id.clone()]
}

fn hit(amount: i32) -> CardScripts {
    both(Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "of": ["unit", "hero"] }))],
        cry: Some(hook(move |_ctx| {
            vec![damage(json_as(
                json!({ "to": { "of": "chosen" }, "amount": amount }),
            ))]
        })),
        ..Script::default()
    })
}

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            guard.id.clone(),
            both(Script {
                replacements: vec![replacement(
                    "guard",
                    ReplacementMoment::LethalHit,
                    json!({ "redirect": "self" }),
                )],
                ..Script::default()
            }),
        ),
        (
            asker.id.clone(),
            both(Script {
                start_of_game: Some(hook(|_ctx| {
                    vec![choose_mode(json_as(
                        json!({ "options": ["left", "right"], "step": "picked" }),
                    ))]
                })),
                resume: IndexMap::from([("picked", hook(|_ctx| vec![]))]),
                ..Script::default()
            }),
        ),
        (
            watcher.id.clone(),
            both(Script {
                triggers: vec![TriggerDef::new(
                    "watch",
                    &[GameEventType::Summoned],
                    |ctx, event| match event {
                        GameEvent::Summoned {
                            player, instance_id, ..
                        } if *player == ctx.controller => vec![buff(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                            "attack": 1,
                            "health": 1,
                        })))],
                        _ => vec![],
                    },
                )],
                ..Script::default()
            }),
        ),
        (
            pile.id.clone(),
            CardScripts {
                base: pile_script(layers(), json!({})),
                radiant: pile_script(layers(), json!({ "radiant": true })),
            },
        ),
        (
            lane_pile.id.clone(),
            both(pile_script(layers(), json!({ "lane": 3 }))),
        ),
        (
            mixed_pile.id.clone(),
            both(pile_script(
                vec![bottom.id.clone(), trap.id.clone(), guard.id.clone()],
                json!({}),
            )),
        ),
        (
            asking_pile.id.clone(),
            both(pile_script(
                vec![asker.id.clone(), middle.id.clone(), guard.id.clone()],
                json!({}),
            )),
        ),
        (bolt.id.clone(), hit(BOLT)),
        (smite.id.clone(), hit(SMITE)),
        (
            wipe.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![destroy_all(json_as(json!({ "side": "any" })))])),
                ..Script::default()
            }),
        ),
    ])
});

pub static LS_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        bottom.clone(),
        middle.clone(),
        ally.clone(),
        guard.clone(),
        asker.clone(),
        watcher.clone(),
        trap.clone(),
        pile.clone(),
        lane_pile.clone(),
        mixed_pile.clone(),
        asking_pile.clone(),
        bolt.clone(),
        smite.clone(),
        wipe.clone(),
    ]
});

pub fn register_layered_fixtures() {
    super::harness::setup_catalog();
    let mut defs: CardDefs = registered_catalog().clone();
    for def in LS_DEFS.iter() {
        defs.insert(def.id.clone(), def.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}
