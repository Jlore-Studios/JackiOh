//! Test-only cards for the systems Meditative+ batch 07 builds (issue #1185): a Unit's Attack and
//! Health set (`set_stats_all`, R1185002), a card on the field taken into your hand (`take_to_hand`,
//! R1185003), R429's play count raised (`raise_times_played`, R1185001), a board filled in Chinese
//! (`fill_board`'s `chinese` rider) and a draw that fills the hand with copies (`draw_priced`'s
//! `fillWithCopies`, R1185004). Each does it through the engine's own verbs, as a card script would,
//! so the engine never imports `crates/cards`; the real cards (M+ #1.1 Base Camp, #21 Harmony Spirit,
//! #24 Bob) test the same cases again.
//!
//! Ids are prefixed `b7-` and indexed from 11850, so they collide with no other fixture file's.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects;
// The verbs as a card script names them, through the prelude (CLAUDE.md rule 5).
use jackioh_engine::prelude::{draw_priced, fill_board, raise_times_played, set_stats_all, take_to_hand};
use jackioh_engine::testkit::*;

/// The Attack and Health the set fixtures set, Harmony Spirit's 3/3.
pub const SET_TO: i32 = 3;
/// How many plays the camp fixture adds to the summit fixture's count.
pub const RAISE_BY: i32 = 2;

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
        "id": format!("b7-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (batch 07 fixture)"),
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

/// A 2/4 Unit: "Cry: Set every other Unit's Attack and Health to 3/3" (M+ #21's shape).
pub static harmony: LazyLock<CardDef> =
    LazyLock::new(|| card(11850, "harmony", "Unit", Some((2, 4)), json!([])));
/// A 5/5 Immutable Unit (R23).
pub static stoic: LazyLock<CardDef> = LazyLock::new(|| {
    card(
        11851,
        "stoic",
        "Unit",
        Some((5, 5)),
        json!([{ "kind": "Immutable" }]),
    )
});
/// A 1/1 Unit: "Aura: Your other Units have +1/+1" (§10.4 layer 5).
pub static banner: LazyLock<CardDef> =
    LazyLock::new(|| card(11852, "banner", "Unit", Some((1, 1)), json!([])));
/// A 0/0 unit token that prints no Radiant form of its own: the Ghoul Token's shape, its X/X doubled on
/// its Radiant face (R349).
pub static ghoul: LazyLock<CardDef> = LazyLock::new(|| {
    json_as(json!({
        "id": "b7-ghoul",
        "index": "T-b7-ghoul",
        "name": "ghoul (batch 07 fixture)",
        "set": "Core",
        "type": "Unit",
        "tags": ["Token"],
        "rarity": "Token",
        "token": true,
        "cost": 0,
        "radiantFallback": true,
        "base": { "attack": 0, "health": 0, "keywords": [], "text": "X/X" },
        "radiant": { "attack": 0, "health": 0, "keywords": [], "text": "X/X" },
    }))
});
/// A 2/4 Unit: "Cry: Put a chosen enemy Unit into your hand" (M+ #24's second mode).
pub static kidnapper: LazyLock<CardDef> =
    LazyLock::new(|| card(11853, "kidnapper", "Unit", Some((2, 4)), json!([])));
/// A Spell: "Put a chosen Unit into your hand. It costs (0)" — either side, with a rider.
pub static snatch: LazyLock<CardDef> = LazyLock::new(|| card(11854, "snatch", "Spell", None, json!([])));
/// A Spell that counts its plays (`countsPlays`, R429): M #8 Reach the Summit's shape, with no text.
pub static summit: LazyLock<CardDef> = LazyLock::new(|| card(11855, "summit", "Spell", None, json!([])));
/// A Spell: "Every summit you own gets 2 more plays" (M+ #1.1 Base Camp's shape).
pub static camp: LazyLock<CardDef> = LazyLock::new(|| card(11856, "camp", "Spell", None, json!([])));
/// A Spell that does not count its plays: the camp fixture passes it by.
pub static plain_spell: LazyLock<CardDef> =
    LazyLock::new(|| card(11857, "plain-spell", "Spell", None, json!([])));
/// A Spell: "Fill your board with Rush Tokens in Chinese" (M+ #21's Radiant face).
pub static party: LazyLock<CardDef> = LazyLock::new(|| card(11858, "party", "Spell", None, json!([])));
/// A Spell: "Draw 1. Fill your hand with copies of it." (M+ #24's fourth mode).
pub static xerox: LazyLock<CardDef> = LazyLock::new(|| card(11859, "xerox", "Spell", None, json!([])));
/// A Spell with Cast on draw: 1 damage to the enemy hero, so a draw puts nothing in the hand (R58).
pub static spark: LazyLock<CardDef> = LazyLock::new(|| card(11860, "spark", "Spell", None, json!([])));

fn both(script: impl Fn() -> Script) -> CardScripts {
    CardScripts {
        base: script(),
        radiant: script(),
    }
}

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            harmony.id.clone(),
            both(|| Script {
                cry: Some(hook(|_ctx| {
                    vec![set_stats_all(json_as(json!({
                        "side": "any",
                        "excludeSelf": true,
                        "attack": SET_TO,
                        "health": SET_TO,
                    })))]
                })),
                ..Script::default()
            }),
        ),
        (
            banner.id.clone(),
            both(|| Script {
                aura: Some(aura_hook(|a| {
                    let controller = a.self_.controller;
                    let me = a.self_.id.clone();
                    vec![AuraEntry {
                        applies: Box::new(move |unit: &CardInstance| {
                            unit.controller == controller && unit.id != me && unit.zone.z() == ZoneName::Field
                        }),
                        mod_: StatMod {
                            attack: Some(1),
                            max_health: Some(1),
                            ..StatMod::default()
                        },
                    }]
                })),
                ..Script::default()
            }),
        ),
        (
            kidnapper.id.clone(),
            both(|| Script {
                targets: vec![TargetDecl::target(
                    1,
                    1,
                    json!({ "side": "enemy", "of": ["unit"] }),
                )],
                cry: Some(hook(|_ctx| {
                    vec![take_to_hand(json_as(json!({ "target": { "of": "chosen" } })))]
                })),
                ..Script::default()
            }),
        ),
        (
            snatch.id.clone(),
            both(|| Script {
                targets: vec![TargetDecl::target(1, 1, json!({ "of": ["unit"] }))],
                cry: Some(hook(|_ctx| {
                    vec![take_to_hand(json_as(
                        json!({ "target": { "of": "chosen" }, "costOverride": 0 }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            summit.id.clone(),
            both(|| Script {
                static_flags: Some(StaticFlags {
                    counts_plays: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            }),
        ),
        (
            camp.id.clone(),
            both(|| Script {
                cry: Some(hook(|_ctx| {
                    vec![
                        raise_times_played(json_as(json!({ "defId": "b7-summit", "amount": RAISE_BY }))),
                        raise_times_played(json_as(json!({ "defId": "b7-plain-spell", "amount": RAISE_BY }))),
                    ]
                })),
                ..Script::default()
            }),
        ),
        (
            party.id.clone(),
            both(|| Script {
                cry: Some(hook(|_ctx| {
                    vec![fill_board(json_as(
                        json!({ "defId": "fx-token-rush", "chinese": true }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            xerox.id.clone(),
            both(|| Script {
                cry: Some(hook(|_ctx| {
                    vec![draw_priced(json_as(json!({ "fillWithCopies": true })))]
                })),
                ..Script::default()
            }),
        ),
        (
            spark.id.clone(),
            both(|| Script {
                static_flags: Some(StaticFlags {
                    cast_on_draw: Some(true),
                    ..StaticFlags::default()
                }),
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": 1 }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
    ])
});

pub static B07_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        harmony.clone(),
        stoic.clone(),
        banner.clone(),
        ghoul.clone(),
        kidnapper.clone(),
        snatch.clone(),
        summit.clone(),
        camp.clone(),
        plain_spell.clone(),
        party.clone(),
        xerox.clone(),
        spark.clone(),
    ]
});

/// The engine's fixture catalog (`harness::setup_catalog`) with these cards and scripts on top.
pub fn register_b07_fixtures() {
    super::harness::setup_catalog();
    let mut defs: CardDefs = registered_catalog().clone();
    for def in B07_DEFS.iter() {
        defs.insert(def.id.clone(), def.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}
