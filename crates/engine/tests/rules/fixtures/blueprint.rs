//! Test-only cards for the Blueprint systems (MB26, R1280–R1284).
//!
//! Ids are prefixed `bp-` and indexed from 7100 up.

use std::sync::LazyLock;

use jackioh_engine::effects::{self, CaptureArgs};
use jackioh_engine::testkit::*;

fn spread(mut base: Value, extra: Value) -> Value {
    if let (Some(into), Value::Object(from)) = (base.as_object_mut(), extra) {
        for (key, value) in from {
            into.insert(key, value);
        }
    }
    base
}

fn def_json(index: u32, name: &str, type_: &str, extra: Value) -> Value {
    spread(
        json!({
            "id": format!("bp-{name}"),
            "index": index.to_string(),
            "name": format!("{name} (blueprint)"),
            "set": "Core",
            "type": type_,
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": { "keywords": [], "text": name },
            "radiant": { "keywords": [], "text": name },
        }),
        extra,
    )
}

fn def(index: u32, name: &str, type_: &str, extra: Value) -> CardDef {
    json_as(def_json(index, name, type_, extra))
}

fn unit(index: u32, name: &str, attack: i32, health: i32, extra: Value, keywords: Value) -> CardDef {
    let faces = json!({
        "base": { "attack": attack, "health": health, "keywords": keywords, "text": name },
        "radiant": { "attack": attack * 2, "health": health * 2, "keywords": keywords, "text": name },
    });
    def(index, name, "Unit", spread(faces, extra))
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

pub fn plot() -> CardDef {
    def(
        7100,
        "plot",
        "Unit",
        json!({
            "cost": 0,
            "base": { "attack": 0, "health": 3, "keywords": [{ "kind": "Can't attack" }], "text": "plot" },
            "radiant": { "attack": 0, "health": 8, "keywords": [{ "kind": "Can't attack" }], "text": "plot" },
            "params": [{ "key": "buffs", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1, "tunedOn": "radiant" }],
        }),
    )
}

pub fn church() -> CardDef {
    def(
        7101,
        "church",
        "Unit",
        json!({
            "cost": 3,
            "base": { "attack": 0, "health": 5, "keywords": [{ "kind": "Can't attack" }], "text": "church" },
            "radiant": { "attack": 0, "health": 10, "keywords": [{ "kind": "Can't attack" }, { "kind": "Divine Shield" }], "text": "church" },
            "params": [{ "key": "cheap", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 0 }],
        }),
    )
}

pub fn bunker() -> CardDef {
    def(
        7102,
        "bunker",
        "Unit",
        json!({
            "cost": 2,
            "base": { "attack": 5, "health": 10, "keywords": [{ "kind": "Armor", "n": 3 }, { "kind": "Can't attack" }, { "kind": "First Strike" }], "text": "bunker" },
            "radiant": { "attack": 10, "health": 30, "keywords": [{ "kind": "Armor", "n": 5 }, { "kind": "Can't attack" }, { "kind": "First Strike" }], "text": "bunker" },
            "params": [{ "key": "multiplier", "base": 3, "radiant": 3, "better": "up", "step": 1, "min": 1 }],
        }),
    )
}

pub fn cleaving_bunker() -> CardDef {
    unit(
        7103,
        "cleaving-bunker",
        2,
        10,
        json!({ "cost": 2 }),
        json!([{ "kind": "Cleave" }, { "kind": "Trample" }]),
    )
}

pub fn captor() -> CardDef {
    unit(7104, "captor", 0, 16, json!({ "cost": 4 }), json!([]))
}

pub fn cheap_unit() -> CardDef {
    unit(7105, "cheap-unit", 1, 1, json!({ "cost": 1 }), json!([]))
}

pub fn cheap_field() -> CardDef {
    def(7106, "cheap-field", "Field Spell", json!({ "cost": 1 }))
}

pub fn dear_unit() -> CardDef {
    unit(7107, "dear-unit", 4, 4, json!({ "cost": 4 }), json!([]))
}

pub fn worth_two() -> CardDef {
    unit(7108, "worth-two", 1, 1, json!({ "cost": 1 }), json!([]))
}

pub fn cry_unit() -> CardDef {
    unit(7109, "cry-unit", 1, 1, json!({ "cost": 1 }), json!([]))
}

pub fn big_unit() -> CardDef {
    unit(7110, "big-unit", 4, 10, json!({ "cost": 4 }), json!([]))
}

pub fn armored_big_unit() -> CardDef {
    unit(
        7111,
        "armored-big-unit",
        4,
        20,
        json!({ "cost": 4 }),
        json!([{ "kind": "Armor", "n": 2 }]),
    )
}

pub static BP_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        plot(),
        church(),
        bunker(),
        cleaving_bunker(),
        captor(),
        cheap_unit(),
        cheap_field(),
        dear_unit(),
        worth_two(),
        cry_unit(),
        big_unit(),
        armored_big_unit(),
    ]
});

pub static BP_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let mut table: IndexMap<String, CardScripts> = IndexMap::new();
    table.insert(
        plot().id,
        CardScripts {
            base: Script {
                static_flags: Some(StaticFlags {
                    stack_base: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            },
            radiant: Script {
                static_flags: Some(StaticFlags {
                    stack_base: Some(true),
                    stack_base_buffs: Some(1),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            },
        },
    );
    table.insert(
        church().id,
        both(Script {
            static_flags: Some(StaticFlags {
                tribute: Some(5),
                tribute_cheap: Some(1),
                ..StaticFlags::default()
            }),
            ..Script::default()
        }),
    );
    table.insert(
        bunker().id,
        both(Script {
            static_flags: Some(StaticFlags {
                tribute: Some(2),
                lane_multiplier: Some(3),
                ..StaticFlags::default()
            }),
            ..Script::default()
        }),
    );
    table.insert(
        cleaving_bunker().id,
        both(Script {
            static_flags: Some(StaticFlags {
                lane_multiplier: Some(3),
                ..StaticFlags::default()
            }),
            ..Script::default()
        }),
    );
    table.insert(
        captor().id,
        both(Script {
            triggers: vec![
                TriggerDef::new(
                    "bp-captor-capture",
                    &[GameEventType::CardResolved],
                    |_ctx, event| {
                        let GameEvent::CardResolved {
                            instance_id,
                            permanent,
                            ..
                        } = event
                        else {
                            return vec![];
                        };
                        if *permanent {
                            vec![effects::capture(CaptureArgs {
                                instance_id: instance_id.clone(),
                            })]
                        } else {
                            vec![]
                        }
                    },
                )
                .with_when(|ctx, event| {
                    let GameEvent::CardResolved {
                        player,
                        def_id,
                        permanent,
                        ..
                    } = event
                    else {
                        return false;
                    };
                    if *player == ctx.controller {
                        return false;
                    }
                    if def_of(Some(&*ctx.state), def_id).type_ != CardType::Unit {
                        return false;
                    }
                    *permanent
                }),
            ],
            ..Script::default()
        }),
    );
    table.insert(cheap_unit().id, both(Script::default()));
    table.insert(cheap_field().id, both(Script::default()));
    table.insert(dear_unit().id, both(Script::default()));
    table.insert(
        worth_two().id,
        both(Script {
            static_flags: Some(StaticFlags {
                tribute_worth: Some(2),
                ..StaticFlags::default()
            }),
            ..Script::default()
        }),
    );
    table.insert(
        cry_unit().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": 1 }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(big_unit().id, both(Script::default()));
    table.insert(armored_big_unit().id, both(Script::default()));
    table
});

pub fn scripts() -> IndexMap<String, CardScripts> {
    BP_SCRIPTS.clone()
}

pub fn register_blueprint() {
    let mut defs = registered_catalog().clone();
    for entry in BP_DEFS.iter() {
        defs.insert(entry.id.clone(), entry.clone());
    }
    register_catalog(defs);
    let mut all_scripts = registered_scripts().clone();
    all_scripts.extend(scripts());
    register_scripts(all_scripts);
}
