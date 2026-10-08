//! Test-only cards for ME-TRIG (docs/meditative-set.md M5; R820–R824): the trigger multipliers of
//! Meditative #9 Joint Filing and #10 Double Counting, and #12 Fear Mongerer's "trigger your End of
//! turn effects", each reproduced through the script hooks and the effects library so the engine is
//! proved without `crates/cards` (CLAUDE.md: the engine never depends on it). The real cards are
//! tested again in their own files.

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

fn def(index: u32, name: &str, type_: &str, extra: Value) -> CardDef {
    let mut base = json!({
        "id": format!("tm-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (trigger multipliers)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    });
    if let (Some(into), Value::Object(from)) = (base.as_object_mut(), extra) {
        for (key, value) in from {
            into.insert(key, value);
        }
    }
    json_as(base)
}

fn unit(index: u32, name: &str, attack: i32, health: i32) -> CardDef {
    def(
        index,
        name,
        "Unit",
        json!({
            "base": { "attack": attack, "health": health, "keywords": [], "text": name },
            "radiant": { "attack": attack * 2, "health": health * 2, "keywords": [], "text": name },
        }),
    )
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn hit_enemy_hero(amount: i32) -> Effect {
    effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))
}

fn hit_chosen(amount: i32) -> Effect {
    effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))
}

/// A Field Spell whose aura makes its controller's start- and end-of-turn hooks run 1 more time.
pub const JOINT: &str = "tm-joint";
/// The same, 2 more times.
pub const JOINT_TWO: &str = "tm-joint-two";
/// A Field Spell whose aura makes its controller's Cries and Deaths run 1 more time.
pub const DOUBLE: &str = "tm-double";
/// Units carrying a turn-hook multiplier of 1 and of 2, for a Fuse.
pub const EXTRA_UNIT_ONE: &str = "tm-extra-unit-one";
pub const EXTRA_UNIT_TWO: &str = "tm-extra-unit-two";
/// Start of turn and End of turn: 1 damage to the enemy hero.
pub const TICKER: &str = "tm-ticker";
/// End of turn: 1 damage to the enemy hero, then Bounce this.
pub const LEAVER: &str = "tm-leaver";
/// A Field Spell: at the start of your opponent's turn, 1 damage to the enemy hero (C #62's hook).
pub const LIVING: &str = "tm-living";
/// End of turn: summon a TICKER.
pub const SPAWNER: &str = "tm-spawner";
/// A Unit whose Cry deals 1 damage to its declared enemy Unit or hero.
pub const AIMER: &str = "tm-aimer";
/// A Spell: 1 damage to a declared Unit or hero, either side.
pub const BOLT: &str = "tm-bolt";
/// A vanilla 1/1.
pub const PAWN: &str = "tm-pawn";
/// A 1/1 whose Death deals 1 damage to the enemy hero.
pub const DIER: &str = "tm-dier";
/// A Spell: destroy every card on your side, units and backrow.
pub const NUKE: &str = "tm-nuke";
/// A Unit whose Cry deals 1, asks a mode (its answer deals 4), then deals 2: a list that parks its tail.
pub const ASKER: &str = "tm-asker";
/// A Spell: trigger the Cry of a declared Unit of yours (C #54 Rewind's shape).
pub const TRIGGERER: &str = "tm-triggerer";
/// A Unit whose Cry triggers its controller's End of turn effects twice.
pub const FEAR: &str = "tm-fear";

fn field_spell(index: u32, name: &str) -> CardDef {
    def(index, name, "Field Spell", json!({}))
}

fn spell(index: u32, name: &str) -> CardDef {
    def(index, name, "Spell", json!({}))
}

/// Every fixture definition.
pub fn catalog() -> CardDefs {
    [
        field_spell(9001, "joint"),
        field_spell(9002, "joint-two"),
        field_spell(9003, "double"),
        unit(9004, "extra-unit-one", 1, 1),
        unit(9005, "extra-unit-two", 1, 1),
        unit(9006, "ticker", 1, 3),
        unit(9007, "leaver", 1, 3),
        field_spell(9008, "living"),
        unit(9009, "spawner", 1, 3),
        unit(9010, "aimer", 1, 3),
        spell(9011, "bolt"),
        unit(9012, "pawn", 1, 1),
        unit(9013, "dier", 1, 1),
        spell(9014, "nuke"),
        unit(9015, "asker", 1, 3),
        spell(9016, "triggerer"),
        unit(9017, "fear", 2, 2),
    ]
    .into_iter()
    .map(|card| (card.id.clone(), card))
    .collect()
}

fn turn_extra(extra: i32) -> Script {
    Script {
        turn_hook_extra: Some(read_hook(move |_args| extra)),
        ..Script::default()
    }
}

/// Every fixture script.
pub fn scripts() -> IndexMap<String, CardScripts> {
    let enemy_unit_or_hero = TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit", "hero"] }));
    let any_unit_or_hero = TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }));
    let entries: Vec<(&str, Script)> = vec![
        (JOINT, turn_extra(1)),
        (JOINT_TWO, turn_extra(2)),
        (
            DOUBLE,
            Script {
                cry_death_extra: Some(read_hook(|_args| 1)),
                ..Script::default()
            },
        ),
        (EXTRA_UNIT_ONE, turn_extra(1)),
        (EXTRA_UNIT_TWO, turn_extra(2)),
        (
            TICKER,
            Script {
                start_of_turn: Some(hook(|_ctx| vec![hit_enemy_hero(1)])),
                end_of_turn: Some(hook(|_ctx| vec![hit_enemy_hero(1)])),
                ..Script::default()
            },
        ),
        (
            LEAVER,
            Script {
                end_of_turn: Some(hook(|_ctx| {
                    vec![
                        hit_enemy_hero(1),
                        effects::bounce(json_as(json!({ "target": { "of": "self" } }))),
                    ]
                })),
                ..Script::default()
            },
        ),
        (
            LIVING,
            Script {
                start_of_opponent_turn: Some(hook(|_ctx| vec![hit_enemy_hero(1)])),
                ..Script::default()
            },
        ),
        (
            SPAWNER,
            Script {
                end_of_turn: Some(hook(|_ctx| vec![effects::summon(json_as(json!({ "defId": TICKER })))])),
                ..Script::default()
            },
        ),
        (
            AIMER,
            Script {
                targets: vec![enemy_unit_or_hero],
                cry: Some(hook(|_ctx| vec![hit_chosen(1)])),
                ..Script::default()
            },
        ),
        (
            BOLT,
            Script {
                targets: vec![any_unit_or_hero],
                cry: Some(hook(|_ctx| vec![hit_chosen(1)])),
                ..Script::default()
            },
        ),
        (PAWN, Script::default()),
        (
            DIER,
            Script {
                death: Some(hook(|_ctx| vec![hit_enemy_hero(1)])),
                ..Script::default()
            },
        ),
        (
            NUKE,
            Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::destroy_all(json_as(
                        json!({ "side": "self", "rows": ["units", "backrow"] }),
                    ))]
                })),
                ..Script::default()
            },
        ),
        (
            ASKER,
            Script {
                cry: Some(hook(|_ctx| {
                    vec![
                        hit_enemy_hero(1),
                        effects::choose_mode(json_as(
                            json!({ "options": ["left", "right"], "step": "after", "prompt": "asker" }),
                        )),
                        hit_enemy_hero(2),
                    ]
                })),
                resume: IndexMap::from([("after", hook(|_ctx| vec![hit_enemy_hero(4)]))]),
                ..Script::default()
            },
        ),
        (
            TRIGGERER,
            Script {
                targets: vec![TargetDecl::target(1, 1, json!({ "side": "ally", "of": ["unit"] }))],
                cry: Some(hook(|_ctx| {
                    vec![effects::trigger_cry(json_as(json!({ "target": { "of": "chosen" } })))]
                })),
                ..Script::default()
            },
        ),
        (
            FEAR,
            Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::trigger_turn_hooks(effects::TriggerTurnHooksArgs { times: 2 })]
                })),
                ..Script::default()
            },
        ),
    ];
    entries
        .into_iter()
        .map(|(id, script)| (id.to_string(), both(script)))
        .collect()
}

/// The real catalog, for anchors, and these fixtures with their scripts, on this test's thread.
pub fn register() {
    let mut defs: CardDefs = serde_json::from_str(include_str!("../../../../cards/catalog.json"))
        .expect("crates/cards/catalog.json parses as CardDefs");
    defs.extend(catalog());
    register_catalog(defs);
    register_scripts(scripts());
}
