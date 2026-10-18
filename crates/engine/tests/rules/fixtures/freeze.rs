//! Test-only cards for Frozen and Freeze (SPEC §6.1, §6.3, R1174000): each freezes, or does what a
//! freeze test needs done to a Frozen Unit, through the engine's own verbs. No shipped card freezes
//! yet (M+ #24 Bob is the first, #1185), so the engine never imports `crates/cards` for this.
//!
//! Ids are prefixed `fz-` and indexed from 11740, so they collide with no other fixture file's.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

fn def(index: u32, name: &str, type_: &str, keywords: Value) -> CardDef {
    let (face, radiant) = if type_ == "Unit" {
        (
            json!({ "attack": 2, "health": 4, "keywords": keywords, "text": name }),
            json!({ "attack": 4, "health": 8, "keywords": keywords, "text": name }),
        )
    } else {
        (
            json!({ "keywords": keywords, "text": name }),
            json!({ "keywords": keywords, "text": name }),
        )
    };
    json_as(json!({
        "id": format!("fz-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (freeze fixture)"),
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

/// A Spell: "Freeze a character" — a Unit or a hero, so a test can aim it at a hero (§6.3 Freeze).
pub static chill: LazyLock<CardDef> = LazyLock::new(|| def(11740, "chill", "Spell", json!([])));
/// A Spell: "Freeze every enemy Unit" (M+ #24 Bob's choice, as a Spell, so Immune to Spells stops it).
pub static blizzard: LazyLock<CardDef> = LazyLock::new(|| def(11741, "blizzard", "Spell", json!([])));
/// A 2/4 Unit: "Cry: Freeze every enemy Unit", Bob's shape on a Unit.
pub static frost: LazyLock<CardDef> = LazyLock::new(|| def(11742, "frost", "Unit", json!([])));
/// A 2/4 Unit: "End of turn: Freeze each other friendly Unit, then destroy this" — a freeze in its
/// controller's End phase, once.
pub static shiver: LazyLock<CardDef> = LazyLock::new(|| def(11743, "shiver", "Unit", json!([])));
/// A Trap: "When an enemy Unit attacks, Freeze it" — a freeze inside a declared attack's window.
pub static ice_trap: LazyLock<CardDef> = LazyLock::new(|| def(11744, "ice-trap", "Trap", json!([])));
/// A Spell: "Steal a Unit" (§6.3 Steal, R171).
pub static grab: LazyLock<CardDef> = LazyLock::new(|| def(11745, "grab", "Spell", json!([])));
/// A Spell: "Vanilla a Unit" (§6.3 Vanilla).
pub static hush: LazyLock<CardDef> = LazyLock::new(|| def(11746, "hush", "Spell", json!([])));
/// A Spell: "Bounce a Unit" (§6.3 Bounce).
pub static recall: LazyLock<CardDef> = LazyLock::new(|| def(11747, "recall", "Spell", json!([])));
/// A Spell: "Switch a Unit's position" — an effect's switch (R20).
pub static shove: LazyLock<CardDef> = LazyLock::new(|| def(11748, "shove", "Spell", json!([])));
/// A Spell: "Your Units attack the enemy hero" — forced attacks (R53).
pub static goad: LazyLock<CardDef> = LazyLock::new(|| def(11749, "goad", "Spell", json!([])));
/// A Spell: "A Unit may attack again this turn" (B5 E35, `may_attack_again`): a fresh exertion.
pub static rally: LazyLock<CardDef> = LazyLock::new(|| def(11750, "rally", "Spell", json!([])));
/// A 2/4 Unit with Immune to Spells (§6.1).
pub static warded: LazyLock<CardDef> =
    LazyLock::new(|| def(11751, "warded", "Unit", json!([{ "kind": "Immune to Spells" }])));

fn both(script: impl Fn() -> Script) -> CardScripts {
    CardScripts {
        base: script(),
        radiant: script(),
    }
}

/// A declared pick of one Unit, either side.
fn a_unit() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "of": ["unit"] }))]
}

fn chosen() -> Value {
    json!({ "target": { "of": "chosen" } })
}

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            chill.id.clone(),
            both(|| Script {
                targets: vec![TargetDecl::target(1, 1, json!({ "of": ["unit", "hero"] }))],
                cry: Some(hook(|_ctx| vec![effects::freeze(json_as(chosen()))])),
                ..Script::default()
            }),
        ),
        (
            blizzard.id.clone(),
            both(|| Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::freeze_all(json_as(json!({ "side": "enemy" })))]
                })),
                ..Script::default()
            }),
        ),
        (
            frost.id.clone(),
            both(|| Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::freeze_all(json_as(json!({ "side": "enemy" })))]
                })),
                ..Script::default()
            }),
        ),
        (
            shiver.id.clone(),
            both(|| Script {
                end_of_turn: Some(hook(|_ctx| {
                    vec![
                        effects::freeze_all(json_as(json!({ "side": "self", "excludeSelf": true }))),
                        effects::destroy(json_as(json!({ "target": { "of": "self" } }))),
                    ]
                })),
                ..Script::default()
            }),
        ),
        (
            ice_trap.id.clone(),
            both(|| Script {
                triggers: vec![
                    TriggerDef::new("ice", &[GameEventType::AttackDeclared], |_ctx, event| {
                        let GameEvent::AttackDeclared { attacker_id, .. } = event else {
                            return Vec::new();
                        };
                        vec![effects::freeze(json_as(
                            json!({ "target": { "of": "instance", "instanceId": attacker_id } }),
                        ))]
                    })
                    .with_when(|_ctx, event| matches!(event, GameEvent::AttackDeclared { forced: false, .. })),
                ],
                ..Script::default()
            }),
        ),
        (
            grab.id.clone(),
            both(|| Script {
                targets: a_unit(),
                cry: Some(hook(|_ctx| vec![effects::steal(json_as(chosen()))])),
                ..Script::default()
            }),
        ),
        (
            hush.id.clone(),
            both(|| Script {
                targets: a_unit(),
                cry: Some(hook(|_ctx| vec![effects::vanilla(json_as(chosen()))])),
                ..Script::default()
            }),
        ),
        (
            recall.id.clone(),
            both(|| Script {
                targets: a_unit(),
                cry: Some(hook(|_ctx| vec![effects::bounce(json_as(chosen()))])),
                ..Script::default()
            }),
        ),
        (
            shove.id.clone(),
            both(|| Script {
                targets: a_unit(),
                cry: Some(hook(|_ctx| vec![effects::switch_position_of(json_as(chosen()))])),
                ..Script::default()
            }),
        ),
        (
            goad.id.clone(),
            both(|| Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::forced_attacks_on(json_as(
                        json!({ "target": { "of": "enemyHero" }, "attackers": "self" }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            rally.id.clone(),
            both(|| Script {
                targets: a_unit(),
                cry: Some(hook(|_ctx| vec![effects::may_attack_again(json_as(chosen()))])),
                ..Script::default()
            }),
        ),
    ])
});

pub static FREEZE_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        chill.clone(),
        blizzard.clone(),
        frost.clone(),
        shiver.clone(),
        ice_trap.clone(),
        grab.clone(),
        hush.clone(),
        recall.clone(),
        shove.clone(),
        goad.clone(),
        rally.clone(),
        warded.clone(),
    ]
});

/// The engine's fixture catalog (`harness::setup_catalog`) with these cards and scripts on top.
pub fn register_freeze_fixtures() {
    super::harness::setup_catalog();
    let mut defs: CardDefs = registered_catalog().clone();
    for card in FREEZE_DEFS.iter() {
        defs.insert(card.id.clone(), card.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}
