//! Test-only cards for Anomalies, the board swap, and the Scanning Probes (issue #1855): a Spell that
//! exiles its caster's board and lays a preset in its place (`replace_board`, R1855000–R1855002), and
//! one that scans for an Anomaly (`scan_for_anomaly`, R1855003). Each does it through the engine's own
//! verbs, as a card script would, so the engine never imports `crates/cards`; Stinkyscape's Anomaly
//! Found! and its ten boards and the Scanning Probes (S20) are the cards that will use them.
//!
//! Ids are prefixed `an-` and indexed from 18580, so they collide with no other fixture file's.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects::{
    choose_mode, damage, delay, destroy_all, exile_all, replace_board, rotate, scan_for_anomaly,
};
use jackioh_engine::testkit::*;

use super::harness::{both, spread};
use super::resources::{ASTEROID, MYTHIC, RESOURCE_SCRIPTS, resources_catalog};

/// What the aimer's Cry deals its chosen target.
pub const AIM_DAMAGE: i32 = 1;
/// What the asker's Cry deals the enemy hero once its question is answered.
pub const ASK_DAMAGE: i32 = 4;
/// What the ward's Cry deals the enemy hero.
pub const WARD_DAMAGE: i32 = 5;
/// The two modes the modal's Cry deals to the enemy hero, by the mode chosen.
pub const MODE_TWO: i32 = 2;
pub const MODE_THREE: i32 = 3;
/// The lucky probe's printed Lucky.
pub const PROBE_LUCKY: i32 = 30;

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
        "id": format!("an-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (anomalies fixture)"),
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

/// A 2/3 Unit with no text.
pub static grunt: LazyLock<CardDef> = LazyLock::new(|| card(18580, "grunt", "Unit", Some((2, 3)), json!([])));
/// A 1/5 Unit with no text.
pub static wall: LazyLock<CardDef> = LazyLock::new(|| card(18581, "wall", "Unit", Some((1, 5)), json!([])));
/// A 1/3 Unit: "Cry: deal 1 damage to a unit or hero".
pub static aimer: LazyLock<CardDef> = LazyLock::new(|| card(18582, "aimer", "Unit", Some((1, 3)), json!([])));
/// A 1/3 Unit: "Cry: choose one — deal 2 to the enemy hero, or 3".
pub static modal: LazyLock<CardDef> = LazyLock::new(|| card(18583, "modal", "Unit", Some((1, 3)), json!([])));
/// A 1/3 Unit: "Cry: choose left or right; then deal 4 to the enemy hero" (a prompt its text opens).
pub static asker: LazyLock<CardDef> = LazyLock::new(|| card(18584, "asker", "Unit", Some((1, 3)), json!([])));
/// A 1/1 Unit: "Cry: exile your other permanents".
pub static banisher: LazyLock<CardDef> =
    LazyLock::new(|| card(18585, "banisher", "Unit", Some((1, 1)), json!([])));
/// A 2/2 Unit, Immune to Spells: "Cry: deal 5 to the enemy hero".
pub static ward: LazyLock<CardDef> = LazyLock::new(|| {
    card(
        18586,
        "ward",
        "Unit",
        Some((2, 2)),
        json!([{ "kind": "Immune to Spells" }]),
    )
});
/// A Trap with no script.
pub static trap: LazyLock<CardDef> = LazyLock::new(|| card(18587, "trap", "Trap", None, json!([])));
/// A Field Spell with no script.
pub static field: LazyLock<CardDef> = LazyLock::new(|| card(18588, "field", "Field Spell", None, json!([])));
/// A Spell with no script: a preset entry that is no permanent.
pub static spell: LazyLock<CardDef> = LazyLock::new(|| card(18589, "spell", "Spell", None, json!([])));
/// A Spell with no script: what a scan finds (Anomaly Found!'s place).
pub static found: LazyLock<CardDef> = LazyLock::new(|| card(18590, "found", "Spell", None, json!([])));
/// A Spell: "Scan for an Anomaly" (a Scanning Probe).
pub static probe: LazyLock<CardDef> = LazyLock::new(|| card(18591, "probe", "Spell", None, json!([])));
/// A Spell: the same scan, with Lucky 30.
pub static lucky_probe: LazyLock<CardDef> = LazyLock::new(|| {
    card(
        18592,
        "lucky-probe",
        "Spell",
        None,
        json!([{ "kind": "Lucky", "n": PROBE_LUCKY }]),
    )
});
/// A Spell: destroy every Unit.
pub static wipe: LazyLock<CardDef> = LazyLock::new(|| card(18593, "wipe", "Spell", None, json!([])));
/// A Spell: grunt in lane 1, a Radiant wall in Defense Position in lane 3, the trap in backrow 2 and
/// the field in backrow 4.
pub static anomaly: LazyLock<CardDef> = LazyLock::new(|| card(18594, "anomaly", "Spell", None, json!([])));
/// A Spell whose every entry fits nowhere: a row the card does not belong in, a Spell, lanes off the
/// row.
pub static misfit: LazyLock<CardDef> = LazyLock::new(|| card(18595, "misfit", "Spell", None, json!([])));
/// A Spell: the aimer, the modal and the asker in lanes 1 to 3.
pub static crowd: LazyLock<CardDef> = LazyLock::new(|| card(18596, "crowd", "Spell", None, json!([])));
/// A Spell: the banisher in lane 1 and the aimer in lane 2.
pub static exodus: LazyLock<CardDef> = LazyLock::new(|| card(18597, "exodus", "Spell", None, json!([])));
/// A Spell: the asteroid in lane 1, its Resources restricted to the Mythic one.
pub static mine: LazyLock<CardDef> = LazyLock::new(|| card(18598, "mine", "Spell", None, json!([])));
/// A 1/3 Unit that asks a question as it arrives (R151), and has no Cry.
pub static greeter: LazyLock<CardDef> =
    LazyLock::new(|| card(18599, "greeter", "Unit", Some((1, 3)), json!([])));
/// A Spell: the greeter in lane 1 and the aimer in lane 2.
pub static welcome: LazyLock<CardDef> = LazyLock::new(|| card(18600, "welcome", "Spell", None, json!([])));
/// A 1/3 Unit: "Cry: rotate the board one step right" (§6.3 Rotate).
pub static spinner: LazyLock<CardDef> =
    LazyLock::new(|| card(18601, "spinner", "Unit", Some((1, 3)), json!([])));
/// A Spell: the spinner in lane 1 and the aimer in lane 2.
pub static whirl: LazyLock<CardDef> = LazyLock::new(|| card(18602, "whirl", "Spell", None, json!([])));
/// A 1/3 Unit: "Cry: your opponent chooses heads or tails; then you choose left or right; then deal
/// 4 to the enemy hero" (the other player's question first, then one for its caster).
pub static polite: LazyLock<CardDef> =
    LazyLock::new(|| card(18603, "polite", "Unit", Some((1, 3)), json!([])));
/// A Spell: the polite unit in lane 1 and the aimer in lane 2.
pub static parley: LazyLock<CardDef> = LazyLock::new(|| card(18604, "parley", "Spell", None, json!([])));
/// A 1/3 Unit: "Cry: at the end of this turn, choose left or right".
pub static planner: LazyLock<CardDef> =
    LazyLock::new(|| card(18605, "planner", "Unit", Some((1, 3)), json!([])));
/// A Spell: the planner in lane 1.
pub static scheme: LazyLock<CardDef> = LazyLock::new(|| card(18606, "scheme", "Spell", None, json!([])));

/// A Spell whose Cry replaces its caster's board with `board`.
fn anomaly_script(board: Value) -> CardScripts {
    let args = json!({ "board": board });
    both(Script {
        cry: Some(hook(move |_ctx| vec![replace_board(json_as(args.clone()))])),
        ..Script::default()
    })
}

/// A Spell whose Cry scans for the `found` card.
fn probe_script() -> CardScripts {
    both(Script {
        cry: Some(hook(|_ctx| {
            vec![scan_for_anomaly(json_as(json!({ "defId": found.id })))]
        })),
        ..Script::default()
    })
}

/// A Cry that deals `amount` to the enemy hero.
fn to_enemy_hero(amount: i32) -> Effect {
    damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))
}

/// One preset entry: `extra` (`radiant`, `position`, `memory`) beside its zone.
fn entry(def: &CardDef, row: &str, lane: i32, extra: Value) -> Value {
    spread(json!({ "defId": def.id, "row": row, "lane": lane }), extra)
}

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            aimer.id.clone(),
            both(Script {
                targets: vec![TargetDecl::target(1, 1, json!({ "of": ["unit", "hero"] }))],
                cry: Some(hook(|_ctx| {
                    vec![damage(json_as(
                        json!({ "to": { "of": "chosen" }, "amount": AIM_DAMAGE }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            modal.id.clone(),
            both(Script {
                modes: vec![ModeDecl {
                    kind: PromptKind::Mode,
                    options: vec!["two".into(), "three".into()],
                }],
                cry: Some(hook(|ctx| {
                    if ctx.modes.first().map(String::as_str) == Some("three") {
                        vec![to_enemy_hero(MODE_THREE)]
                    } else {
                        vec![to_enemy_hero(MODE_TWO)]
                    }
                })),
                ..Script::default()
            }),
        ),
        (
            asker.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![choose_mode(json_as(
                        json!({ "options": ["left", "right"], "step": "picked" }),
                    ))]
                })),
                resume: IndexMap::from([("picked", hook(|_ctx| vec![to_enemy_hero(ASK_DAMAGE)]))]),
                ..Script::default()
            }),
        ),
        (
            polite.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![choose_mode(json_as(
                        json!({ "options": ["heads", "tails"], "by": "enemy", "step": "theirs" }),
                    ))]
                })),
                resume: IndexMap::from([
                    (
                        "theirs",
                        hook(|_ctx| {
                            vec![choose_mode(json_as(
                                json!({ "options": ["left", "right"], "step": "mine" }),
                            ))]
                        }),
                    ),
                    ("mine", hook(|_ctx| vec![to_enemy_hero(ASK_DAMAGE)])),
                ]),
                ..Script::default()
            }),
        ),
        (
            planner.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![delay(json_as(json!({
                        "at": { "phase": "end", "player": "self" },
                        "step": "later",
                    })))]
                })),
                delayed: Some(hook(|_ctx| {
                    vec![choose_mode(json_as(
                        json!({ "options": ["left", "right"], "step": "planned" }),
                    ))]
                })),
                resume: IndexMap::from([("planned", hook(|_ctx| vec![]))]),
                ..Script::default()
            }),
        ),
        (
            banisher.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![exile_all(json_as(json!({ "side": "self", "excludeSelf": true })))]
                })),
                ..Script::default()
            }),
        ),
        (
            ward.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![to_enemy_hero(WARD_DAMAGE)])),
                ..Script::default()
            }),
        ),
        (
            greeter.id.clone(),
            both(Script {
                start_of_game: Some(hook(|_ctx| {
                    vec![choose_mode(json_as(
                        json!({ "options": ["hello", "goodbye"], "step": "greeted" }),
                    ))]
                })),
                resume: IndexMap::from([("greeted", hook(|_ctx| vec![]))]),
                ..Script::default()
            }),
        ),
        (
            spinner.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![rotate(json_as(json!({ "direction": "right" })))]
                })),
                ..Script::default()
            }),
        ),
        (probe.id.clone(), probe_script()),
        (lucky_probe.id.clone(), probe_script()),
        (
            wipe.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![destroy_all(json_as(json!({ "side": "any" })))])),
                ..Script::default()
            }),
        ),
        (
            anomaly.id.clone(),
            anomaly_script(json!([
                entry(&grunt, "units", 1, json!({})),
                entry(&wall, "units", 3, json!({ "radiant": true, "position": "DEF" })),
                entry(&trap, "backrow", 2, json!({})),
                entry(&field, "backrow", 4, json!({})),
            ])),
        ),
        (
            misfit.id.clone(),
            anomaly_script(json!([
                entry(&grunt, "backrow", 1, json!({})),
                entry(&trap, "units", 2, json!({})),
                entry(&spell, "units", 3, json!({})),
                entry(&grunt, "units", 6, json!({})),
                entry(&grunt, "units", 0, json!({})),
            ])),
        ),
        (
            crowd.id.clone(),
            anomaly_script(json!([
                entry(&aimer, "units", 1, json!({})),
                entry(&modal, "units", 2, json!({})),
                entry(&asker, "units", 3, json!({})),
            ])),
        ),
        (
            exodus.id.clone(),
            anomaly_script(json!([
                entry(&banisher, "units", 1, json!({})),
                entry(&aimer, "units", 2, json!({})),
            ])),
        ),
        (
            mine.id.clone(),
            anomaly_script(json!([{
                "defId": ASTEROID,
                "row": "units",
                "lane": 1,
                "memory": { "resourcePool": [MYTHIC] },
            }])),
        ),
        (
            whirl.id.clone(),
            anomaly_script(json!([
                entry(&spinner, "units", 1, json!({})),
                entry(&aimer, "units", 2, json!({})),
            ])),
        ),
        (
            parley.id.clone(),
            anomaly_script(json!([
                entry(&polite, "units", 1, json!({})),
                entry(&aimer, "units", 2, json!({})),
            ])),
        ),
        (
            scheme.id.clone(),
            anomaly_script(json!([entry(&planner, "units", 1, json!({}))])),
        ),
        (
            welcome.id.clone(),
            anomaly_script(json!([
                entry(&greeter, "units", 1, json!({})),
                entry(&aimer, "units", 2, json!({})),
            ])),
        ),
    ])
});

pub static AN_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        grunt.clone(),
        wall.clone(),
        aimer.clone(),
        modal.clone(),
        asker.clone(),
        banisher.clone(),
        ward.clone(),
        trap.clone(),
        field.clone(),
        spell.clone(),
        found.clone(),
        probe.clone(),
        lucky_probe.clone(),
        wipe.clone(),
        anomaly.clone(),
        misfit.clone(),
        crowd.clone(),
        exodus.clone(),
        mine.clone(),
        greeter.clone(),
        welcome.clone(),
        spinner.clone(),
        whirl.clone(),
        polite.clone(),
        parley.clone(),
        planner.clone(),
        scheme.clone(),
    ]
});

/// The engine's fixture catalog (`harness::setup_catalog`) with the Resources and the asteroid
/// (`fixtures/resources.rs`) and these cards and scripts on top.
pub fn register_anomaly_fixtures() {
    super::harness::setup_catalog();
    let mut defs: CardDefs = resources_catalog(registered_catalog().clone());
    for def in AN_DEFS.iter() {
        defs.insert(def.id.clone(), def.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(RESOURCE_SCRIPTS.clone());
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}
