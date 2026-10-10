//! Test-only cards for the Meditative riders part (docs/meditative-set.md M5, R1100–R1110): an
//! either-player Activate card (MD-D9, #54's shape, proved here while #54 waits for #525's Jade), a
//! controller-only one, a note log for ordering granted Deaths (MD-D13), a granter and a bearer, a
//! Book and an AI card for multi-pool fusion (MD-D14), and a `manaSpent` listener (MD-D26). The
//! engine never imports `packages/cards`; the real cards' tests cover the same cases again.
//!
//! Ids are prefixed `rider-` and indexed from 4200, so they cannot collide with another fixture
//! file's.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use serde_json::{Value, json};

use jackioh_engine::effects::{fuse_generated, gain_mana, grant_ability};
use jackioh_engine::testkit::*;

/// A Core Common `rider-<name>` at cost 0; a Unit is 2/2 (its Radiant face doubles them), with
/// `extra` over the definition.
fn def(name: &str, index: i32, extra: Value) -> CardDef {
    let mut rest = if extra.is_object() { extra } else { json!({}) };
    let mut take = |key: &str| rest.as_object_mut().and_then(|object| object.remove(key));
    let type_ = take("type").and_then(|v| v.as_str().map(str::to_string));
    let (face, radiant_face) = if type_.as_deref() == Some("Unit") || type_.is_none() {
        (
            json!({ "attack": 2, "health": 2, "keywords": [], "text": name }),
            json!({ "attack": 4, "health": 4, "keywords": [], "text": name }),
        )
    } else {
        (
            json!({ "keywords": [], "text": name }),
            json!({ "keywords": [], "text": name }),
        )
    };
    let mut card = json!({
        "id": format!("rider-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (riders)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face,
        "radiant": radiant_face,
    });
    if let (Some(target), Value::Object(rest)) = (card.as_object_mut(), rest) {
        for (key, value) in rest {
            target.insert(key, value);
        }
        if let Some(type_) = type_ {
            target.insert("type".to_string(), Value::String(type_));
        }
    }
    json_as(card)
}

/// An ability with a cost and no targets, modes or conditions.
fn ability(id: &str, label: &str, cost: Option<ActivationCost>, run: Hook) -> ActivationDecl {
    ActivationDecl {
        id: id.to_string(),
        label: label.to_string(),
        uses: ActivationUses::Count(1),
        cost,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run,
    }
}

// ---------------------------------------------------------------------------
// The note log
// ---------------------------------------------------------------------------

/// The note log's def: a Field Spell whose memory records what ran, in order.
pub static log_card: LazyLock<CardDef> = LazyLock::new(|| def("log", 4200, json!({ "type": "Field Spell" })));

/// The note log sits in p2's backrow lane 5, out of the way of every card under test.
pub const LOG_LANE: i32 = 5;

/// The log's steps, as stored (`memory.steps`), or none.
fn steps_of(memory: &IndexMap<String, Value>) -> Vec<String> {
    match memory.get("steps").and_then(Value::as_array) {
        Some(steps) => steps
            .iter()
            .filter_map(|step| step.as_str().map(str::to_string))
            .collect(),
        None => vec![],
    }
}

/// Everything the note log has recorded, in order.
pub fn notes(state: &GameState) -> Vec<String> {
    match state.players.p2.backrow.get((LOG_LANE - 1) as usize) {
        Some(Some(log)) => steps_of(&log.memory),
        _ => vec![],
    }
}

/// An effect that appends `entry` to the note log (nothing, while there is no log).
pub fn note(entry: impl Into<String>) -> Effect {
    let entry: String = entry.into();
    Effect::new("rider:note", move |ctx| {
        let Some(Some(log)) = ctx.state.players.p2.backrow.get_mut((LOG_LANE - 1) as usize) else {
            return;
        };
        let mut steps = steps_of(&log.memory);
        steps.push(entry.clone());
        log.memory.insert("steps".to_string(), json!(steps));
    })
}

// ---------------------------------------------------------------------------
// The cards
// ---------------------------------------------------------------------------

/// MD-D9: a Unit with "Activate: Spend (price). Gain 1 mana", open to either player, the price read
/// off the declared `price` number (2) so a tuning of it reaches the cost.
pub static open_mic: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "open",
        4201,
        json!({
            "cost": 3,
            "params": [{ "key": "price", "base": 2, "radiant": 2, "better": "up" }],
        }),
    )
});

fn open_ability() -> ActivationDecl {
    ability(
        "share",
        "Spend (2). Gain 1 mana",
        Some(ActivationCost {
            mana_param: Some("price"),
            either_player: Some(true),
            ..ActivationCost::default()
        }),
        hook(|_ctx| vec![gain_mana(json_as(json!({ "amount": 1 })))]),
    )
}

/// The same ability, controller-only: the "that card is not yours" comparison.
pub static closed_mic: LazyLock<CardDef> = LazyLock::new(|| def("closed", 4202, json!({ "cost": 1 })));

fn closed_ability() -> ActivationDecl {
    ability(
        "share",
        "Spend (1). Gain 1 mana",
        Some(ActivationCost {
            mana: Some(1),
            ..ActivationCost::default()
        }),
        hook(|_ctx| vec![gain_mana(json_as(json!({ "amount": 1 })))]),
    )
}

/// MD-D13: a Unit whose Death notes "own": the bearer a grant is granted to.
pub static bearer: LazyLock<CardDef> = LazyLock::new(|| def("bearer", 4203, json!({})));

fn bearer_script() -> Script {
    Script {
        death: Some(hook(|_ctx| vec![note("own")])),
        ..Script::default()
    }
}

/// MD-D13: a Unit that grants `rider-granter#noteDeath` ("notes `granted`") to the first unit chosen.
pub static granter: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "granter",
        4204,
        json!({
            "base": { "attack": 2, "health": 2, "keywords": [], "text": "granter", "grants": { "noteDeath": "Death: notes `granted`." } },
            "radiant": { "attack": 4, "health": 4, "keywords": [], "text": "granter", "grants": { "noteDeath": "Death: notes `granted`." } },
        }),
    )
});

pub const GRANT_KEY: &str = "rider-granter#noteDeath";

fn granter_script() -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
        cry: Some(hook(|_ctx| {
            vec![grant_ability(json_as(json!({
                "target": { "of": "chosen" },
                "grant": GRANT_KEY,
            })))]
        })),
        grants: IndexMap::from([("noteDeath", hook(|_ctx| vec![note("granted")]))]),
        ..Script::default()
    }
}

/// MD-D14: a Book Spell, one ingredient of the multi-pool fusion.
pub static book: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "book",
        4205,
        json!({ "type": "Spell", "tags": ["Book"], "base": { "keywords": [], "text": "book" }, "radiant": { "keywords": [], "text": "book" } }),
    )
});

/// MD-D14: an AI Unit, the fusion's other ingredient.
pub static mind: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "mind",
        4206,
        json!({ "tags": ["AI"], "base": { "attack": 1, "health": 1, "keywords": [], "text": "mind" }, "radiant": { "attack": 2, "health": 2, "keywords": [], "text": "mind" } }),
    )
});

/// MD-D14: a Spell whose Cry fuses one Book and one AI card into the hand.
pub static fuser: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "fuser",
        4207,
        json!({ "type": "Spell", "base": { "keywords": [], "text": "fuse" }, "radiant": { "keywords": [], "text": "fuse" } }),
    )
});

fn fuser_script() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![fuse_generated(json_as(json!({
                "count": 2,
                "pools": [{ "tags": ["Book"] }, { "tags": ["AI"] }],
            })))]
        })),
        ..Script::default()
    }
}

/// MD-D26: a Unit that answers `manaSpent` by noting it.
pub static ear: LazyLock<CardDef> = LazyLock::new(|| def("ear", 4208, json!({})));

fn ear_script() -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "rider-ear",
            &[GameEventType::ManaSpent],
            |_ctx, _event| vec![note("heard")],
        )],
        ..Script::default()
    }
}

pub static RIDER_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let plain = || CardScripts {
        base: Script::default(),
        radiant: Script::default(),
    };
    IndexMap::from([
        (log_card.id.clone(), plain()),
        (
            open_mic.id.clone(),
            CardScripts {
                base: Script {
                    activations: vec![open_ability()],
                    ..Script::default()
                },
                radiant: Script {
                    activations: vec![open_ability()],
                    ..Script::default()
                },
            },
        ),
        (
            closed_mic.id.clone(),
            CardScripts {
                base: Script {
                    activations: vec![closed_ability()],
                    ..Script::default()
                },
                radiant: Script {
                    activations: vec![closed_ability()],
                    ..Script::default()
                },
            },
        ),
        (
            bearer.id.clone(),
            CardScripts {
                base: bearer_script(),
                radiant: bearer_script(),
            },
        ),
        (
            granter.id.clone(),
            CardScripts {
                base: granter_script(),
                radiant: granter_script(),
            },
        ),
        (book.id.clone(), plain()),
        (mind.id.clone(), plain()),
        (
            fuser.id.clone(),
            CardScripts {
                base: fuser_script(),
                radiant: fuser_script(),
            },
        ),
        (
            ear.id.clone(),
            CardScripts {
                base: ear_script(),
                radiant: ear_script(),
            },
        ),
    ])
});

/// Every riders fixture on top of `base`.
pub fn riders_catalog(base: CardDefs) -> CardDefs {
    let mut defs = base;
    for entry in [
        &log_card,
        &open_mic,
        &closed_mic,
        &bearer,
        &granter,
        &book,
        &mind,
        &fuser,
        &ear,
    ] {
        defs.insert(entry.id.clone(), (**entry).clone());
    }
    defs
}
