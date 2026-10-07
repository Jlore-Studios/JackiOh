//! Test-only cards for Activate (docs/classic-sets.md B3.2, R384). Each reproduces one shape of the
//! Classic and Classic+ cards that use the keyword through the engine's own verbs: a targeted ping
//! (Classic+ #76.1 Brother Ping), an "Activate 2" on a Unit, a ♾️ Tribute cost that reads the tributed
//! unit's Attack (Classic #21 Turtinator), a random-discard cost (Classic #15 Nose Hunter), "Tribute
//! this" on an Indestructible Field Spell (Classic #84 Lockdown), a mana price (Heroic Power's "spend
//! (X)", the Heroic Power patch), modes with mode-bound targets and a delayed destroy (Classic #20 The Power to
//! Punish), a condition the text sets (Classic #7 InfiniScepter), an ability that asks mid-list,
//! abilities a card has only on some instances (the Heroic Power patch's rolled power), and abilities a
//! fused card carries, each read in its ingredient's place (R102). The engine never imports
//! `packages/cards`; the real cards' tests cover the same cases again.
//!
//! Ids are prefixed `act-` and indexed from 4100, so they cannot collide with another fixture file's.
//!
//! Port of `packages/engine/test/fixtures/activate.ts`. Each exported def is a `pub static` under TS's
//! name snake_cased (`log_card.id`, `low_teller.id`); TS's module counter (`nextIndex`, from 4100, one
//! per `def` call in file order) is each def's stated index.

#![allow(non_upper_case_globals)]

use std::borrow::Borrow;
use std::sync::LazyLock;

use jackioh_engine::effects::{damage, destroy_at_next_turn_start, discard_random, draw, gain_mana};
use jackioh_engine::subsystems::activate::activation_paid;
use jackioh_engine::subsystems::hero_power::{POWER_RESUME, STEADY_SHOT_PARAM, hero_power, power_abilities, roll_power};
use jackioh_engine::testkit::*;

/// TS `def(name, type, extra = {})`: a Core Common at cost 0 named after `name`; a Unit is 2/2 unless
/// `extra` names `attack` or `health` (its Radiant face doubles them), `extra`'s `keywords` go on both
/// faces, and every other key of `extra` is written over the definition.
fn def(name: &str, index: i32, type_: &str, extra: Value) -> CardDef {
    let mut rest = if extra.is_object() { extra } else { json!({}) };
    let mut take = |key: &str| rest.as_object_mut().and_then(|object| object.remove(key));
    let attack = take("attack").and_then(|v| v.as_i64()).map_or(2, |n| n as i32);
    let health = take("health").and_then(|v| v.as_i64()).map_or(2, |n| n as i32);
    let keywords = take("keywords").unwrap_or_else(|| json!([]));
    let (face, radiant_face) = if type_ == "Unit" {
        (
            json!({ "attack": attack, "health": health, "keywords": keywords, "text": name }),
            json!({ "attack": attack * 2, "health": health * 2, "keywords": keywords, "text": name }),
        )
    } else {
        (
            json!({ "keywords": keywords, "text": name }),
            json!({ "keywords": keywords, "text": name }),
        )
    };
    let mut card = json!({
        "id": format!("act-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (activate)"),
        "set": "Core",
        "type": type_,
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
    }
    json_as(card)
}

// ---------------------------------------------------------------------------
// The note log: a Field Spell whose memory records what ran, in order.
// ---------------------------------------------------------------------------

pub static log_card: LazyLock<CardDef> = LazyLock::new(|| def("log", 4101, "Field Spell", json!({})));
/// The note log sits in p2's backrow lane 5, out of the way of every card under test.
pub const LOG_LANE: i32 = 5;

/// The log's steps, as stored (`memory.steps`), or none.
fn steps_of(memory: &IndexMap<String, Value>) -> Vec<String> {
    match memory.get("steps").and_then(Value::as_array) {
        Some(steps) => steps.iter().filter_map(|step| step.as_str().map(str::to_string)).collect(),
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
    Effect::new("act:note", move |ctx| {
        let Some(Some(log)) = ctx.state.players.p2.backrow.get_mut((LOG_LANE - 1) as usize) else {
            return;
        };
        let mut steps = steps_of(&log.memory);
        steps.push(entry.clone());
        log.memory.insert("steps".to_string(), json!(steps));
    })
}

/// §10.6: a prompt for the controller with one answer, whose answer re-enters `step`.
pub fn ask_controller(step: impl Into<String>) -> Effect {
    let step: String = step.into();
    Effect::new("act:ask", move |ctx| {
        let resume = resume_self(&*ctx, &step, Default::default());
        let player = ctx.controller;
        open_prompt(
            ctx,
            json_as(json!({
                "player": player,
                "kind": "target",
                "prompt": "the ability asks its controller",
                "options": [{ "key": "none", "label": "nothing", "selection": { "pick": "none" } }],
                "resume": resume,
            })),
        );
    })
}

/// TS `faces(base, radiant = base)`.
fn faces(base: Script) -> CardScripts {
    CardScripts {
        base: base.clone(),
        radiant: base,
    }
}

fn faces_of(base: Script, radiant: Script) -> CardScripts {
    CardScripts { base, radiant }
}

/// TS `ANY_TARGET`: one unit or hero.
fn any_target() -> TargetDecl {
    TargetDecl::target(1, 1, json!({ "of": ["unit", "hero"] }))
}

/// An ability with no cost, targets, modes or conditions: the TS literal `{ id, label, uses, run }`.
fn ability(id: &str, label: &str, uses: ActivationUses, run: Hook) -> ActivationDecl {
    ActivationDecl {
        id: id.to_string(),
        label: label.to_string(),
        uses,
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run,
    }
}

fn notes_hook(entries: &'static [&'static str]) -> Hook {
    hook(move |_ctx| entries.iter().map(|entry| note(*entry)).collect())
}

/// TS `String(value)` for a value read back out of a card's memory (`undefined` when there is none).
fn js_string<V: Borrow<Value>>(value: Option<V>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(value) => match value.borrow() {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        },
    }
}

// ---------------------------------------------------------------------------
// The cards
// ---------------------------------------------------------------------------

/// Classic+ #76.1's shape: "Activate: Deal 1 damage" (Radiant "Activate 2: Deal 2 damage").
pub static pinger: LazyLock<CardDef> = LazyLock::new(|| def("pinger", 4102, "Field Spell", json!({})));
fn ping(uses: i32, amount: i32) -> ActivationDecl {
    ActivationDecl {
        targets: vec![any_target()],
        ..ability(
            "ping",
            &format!("Deal {amount} damage"),
            ActivationUses::Count(uses),
            hook(move |_ctx| vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))]),
        )
    }
}

/// A Unit with "Activate 2: Gain 1 mana", for summoning sickness and "Activate N".
pub static sentry: LazyLock<CardDef> =
    LazyLock::new(|| def("sentry", 4103, "Unit", json!({ "attack": 2, "health": 2 })));

/// Classic #21's shape: "Activate ♾️: Tribute a Unit. Deal damage equal to its Attack to any target."
pub static turtle: LazyLock<CardDef> =
    LazyLock::new(|| def("turtle", 4104, "Unit", json!({ "attack": 5, "health": 4 })));

/// Classic #15's shape: "Activate: Discard a random card. …"
pub static nose: LazyLock<CardDef> = LazyLock::new(|| def("nose", 4105, "Unit", json!({ "attack": 3, "health": 1 })));

/// Classic #84's shape: Indestructible, "Activate: Tribute this."
pub static lockdown: LazyLock<CardDef> = LazyLock::new(|| {
    def("lockdown", 4106, "Field Spell", json!({ "keywords": [{ "kind": "Indestructible" }] }))
});

/// Classic #89's shape: to target this, a player must also discard 2 cards (B5 E5, R450).
pub static ghost: LazyLock<CardDef> = LazyLock::new(|| def("ghost", 4107, "Unit", json!({ "attack": 5, "health": 6 })));

/// A mana price, the Heroic Power patch's "Activate: Spend (2): Draw 1".
pub static merchant: LazyLock<CardDef> = LazyLock::new(|| def("merchant", 4108, "Field Spell", json!({})));
pub const MERCHANT_PRICE: i32 = 2;

/// Classic #20's shape: "Activate: Choose one: Deal 2 damage; your opponent discards a card; or choose
/// a Unit, which is destroyed at the start of your next turn" (Radiant: all enemy Units then).
pub static punisher: LazyLock<CardDef> = LazyLock::new(|| def("punisher", 4109, "Field Spell", json!({})));
pub const PUNISH_MODES: [&str; 3] = ["damage", "discard", "doom"];
fn punish(radiant: bool) -> Script {
    let damage_target = TargetDecl {
        for_modes: Some(vec!["damage".to_string()]),
        ..any_target()
    };
    let targets = if radiant {
        vec![damage_target]
    } else {
        vec![
            damage_target,
            TargetDecl {
                for_modes: Some(vec!["doom".to_string()]),
                ..TargetDecl::target(1, 1, json!({ "of": ["unit"] }))
            },
        ]
    };
    Script {
        activations: vec![ActivationDecl {
            modes: vec![ModeDecl {
                kind: PromptKind::Mode,
                options: PUNISH_MODES.iter().map(|mode| mode.to_string()).collect(),
            }],
            targets,
            ..ability(
                "punish",
                "Choose one",
                ActivationUses::Count(1),
                hook(move |ctx| match ctx.modes.first().map(String::as_str) {
                    Some("damage") => vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 2 })))],
                    Some("discard") => vec![discard_random(json_as(json!({ "player": "enemy" })))],
                    Some("doom") => {
                        if radiant {
                            vec![destroy_at_next_turn_start(json_as(json!({ "scope": { "side": "enemy" } })))]
                        } else {
                            vec![destroy_at_next_turn_start(json_as(json!({ "target": { "of": "chosen" } })))]
                        }
                    }
                    _ => vec![],
                }),
            )
        }],
        ..Script::default()
    }
}

/// An ability that asks mid-list: `note(ask)`, a prompt, `note(ask:tail)`; the answer notes too.
pub static asker: LazyLock<CardDef> =
    LazyLock::new(|| def("asker", 4110, "Field Spell", json!({ "tags": ["Quickdraw"] })));

/// Classic #7's shape: "Activate: Cast a copy of that Spell" — nothing remembered, no activation.
pub static scepter: LazyLock<CardDef> = LazyLock::new(|| def("scepter", 4111, "Field Spell", json!({})));
pub const SCEPTER_KEY: &str = "stored";

/// The Heroic Power patch's shape: one ability per power, and a copy has only the one it rolled
/// (`has`), beside one it always has. The rolled one is `memory.pick`.
pub static chooser: LazyLock<CardDef> = LazyLock::new(|| def("chooser", 4112, "Field Spell", json!({})));
pub const PICK_KEY: &str = "pick";

/// A unit whose Death asks, so a Tribute cost that takes it pauses before the ability's effect.
pub static mourner: LazyLock<CardDef> =
    LazyLock::new(|| def("mourner", 4113, "Unit", json!({ "attack": 1, "health": 1 })));

/// A Trap with an ability: face-down in the backrow it has no text anyone can use.
pub static trapper: LazyLock<CardDef> = LazyLock::new(|| def("trapper", 4114, "Trap", json!({})));

/// "Activate ♾️" with no cost, to reach the cap.
pub static endless: LazyLock<CardDef> = LazyLock::new(|| def("endless", 4115, "Field Spell", json!({})));

/// Heroic Power wired as Core #98 is (R752), so `activate` and its alias reach the real power.
pub static heroic: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "heroic",
        4116,
        "Field Spell",
        json!({
            "cost": 0,
            "tags": ["Quickdraw"],
            "params": [{ "key": STEADY_SHOT_PARAM, "base": 2, "radiant": 4, "better": "up", "step": 2, "min": 1 }],
        }),
    )
});

/// Classic #7's shape read the way the real card reads it (R102): the ability is usable once its own
/// text has remembered something (`recalled`), and notes what it remembered, so a fusion shows which
/// ingredient's memory each of its abilities reads.
pub static keeper: LazyLock<CardDef> = LazyLock::new(|| def("keeper", 4117, "Field Spell", json!({})));
pub const KEEPER_KEY: &str = "kept";

/// Classic #20's numbered shape (B3.4): "Activate: Deal {damage} damage", read through `param`.
pub const ZAPPER_DAMAGE: i32 = 3;
pub static zapper: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "zapper",
        4118,
        "Field Spell",
        json!({
            "params": [{ "key": "damage", "base": ZAPPER_DAMAGE, "radiant": ZAPPER_DAMAGE * 2, "better": "up", "step": 1, "min": 1 }],
        }),
    )
});
/// A card that declares the same number, smaller, and has no ability: fused ahead of a zapper (R102).
pub static spark: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "spark",
        4119,
        "Field Spell",
        json!({ "params": [{ "key": "damage", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 }] }),
    )
});

/// Two cards whose abilities ask the same step and answer with the number their own text declares,
/// so a fusion holding both shows which ingredient a stored question comes back to (R102, R113).
pub const LOW_TELL: i32 = 1;
pub const HIGH_TELL: i32 = 5;
pub static low_teller: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "low-teller",
        4120,
        "Field Spell",
        json!({ "params": [{ "key": "tell", "base": LOW_TELL, "radiant": LOW_TELL * 2, "better": "up", "step": 1, "min": 1 }] }),
    )
});
pub static high_teller: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "high-teller",
        4121,
        "Field Spell",
        json!({ "params": [{ "key": "tell", "base": HIGH_TELL, "radiant": HIGH_TELL * 2, "better": "up", "step": 1, "min": 1 }] }),
    )
});

/// The tellers' script: "Activate: ask; the answer notes {tell}", read by the step the answer re-enters.
fn teller() -> CardScripts {
    faces(Script {
        activations: vec![ability(
            "tell",
            "Ask, then tell {tell}",
            ActivationUses::Count(1),
            hook(|_ctx| vec![ask_controller("told")]),
        )],
        resume: IndexMap::from([("told", hook(|ctx| vec![note(format!("told:{}", param(&*ctx, "tell")))]))]),
        ..Script::default()
    })
}

pub static ACTIVATE_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        log_card.clone(),
        pinger.clone(),
        sentry.clone(),
        turtle.clone(),
        nose.clone(),
        lockdown.clone(),
        ghost.clone(),
        merchant.clone(),
        punisher.clone(),
        asker.clone(),
        scepter.clone(),
        chooser.clone(),
        mourner.clone(),
        trapper.clone(),
        endless.clone(),
        heroic.clone(),
        keeper.clone(),
        zapper.clone(),
        spark.clone(),
        low_teller.clone(),
        high_teller.clone(),
    ]
});

fn quickdraw() -> Option<StaticFlags> {
    Some(StaticFlags {
        quickdraw: Some(true),
        ..StaticFlags::default()
    })
}

fn heroic_face(radiant: bool) -> Script {
    Script {
        static_flags: quickdraw(),
        start_of_game: Some(hook(|_ctx| vec![roll_power()])),
        activations: power_abilities(radiant),
        resume: IndexMap::from([(POWER_RESUME, hook(hero_power))]),
        ..Script::default()
    }
}

pub static ACTIVATE_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (log_card.id.clone(), faces(Script::default())),
        (
            pinger.id.clone(),
            faces_of(
                Script {
                    activations: vec![ping(1, 1)],
                    ..Script::default()
                },
                Script {
                    activations: vec![ping(2, 2)],
                    ..Script::default()
                },
            ),
        ),
        (
            ghost.id.clone(),
            faces(Script {
                targeting_discards: Some(read_hook(|_a| 2)),
                ..Script::default()
            }),
        ),
        (
            sentry.id.clone(),
            faces(Script {
                activations: vec![ability(
                    "surge",
                    "Gain 1 mana",
                    ActivationUses::Count(2),
                    hook(|_ctx| vec![gain_mana(json_as(json!({ "amount": 1 })))]),
                )],
                ..Script::default()
            }),
        ),
        (
            turtle.id.clone(),
            faces(Script {
                activations: vec![ActivationDecl {
                    cost: Some(ActivationCost {
                        tribute: Some(1),
                        ..ActivationCost::default()
                    }),
                    targets: vec![any_target()],
                    ..ability(
                        "eat",
                        "Tribute a Unit. Deal damage equal to its Attack to any target",
                        ActivationUses::Unlimited,
                        hook(|ctx| {
                            let amount = activation_paid(&*ctx).tributed.first().map_or(0, |unit| unit.attack);
                            vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))]
                        }),
                    )
                }],
                ..Script::default()
            }),
        ),
        (
            nose.id.clone(),
            faces(Script {
                activations: vec![ActivationDecl {
                    cost: Some(ActivationCost {
                        discard_random: Some(1),
                        ..ActivationCost::default()
                    }),
                    ..ability("sniff", "Discard a random card", ActivationUses::Count(1), notes_hook(&["sniff"]))
                }],
                ..Script::default()
            }),
        ),
        (
            lockdown.id.clone(),
            faces(Script {
                activations: vec![ActivationDecl {
                    cost: Some(ActivationCost {
                        tribute_self: Some(true),
                        ..ActivationCost::default()
                    }),
                    ..ability(
                        "leave",
                        "Tribute this",
                        ActivationUses::Count(1),
                        // TS reads the live `ctx.self`: where the card is now that its cost has moved it.
                        hook(|ctx| {
                            let zone = ctx.live_self().map_or("none", |card| card.zone.z().as_str());
                            vec![note(format!("left:{zone}"))]
                        }),
                    )
                }],
                ..Script::default()
            }),
        ),
        (
            merchant.id.clone(),
            faces(Script {
                activations: vec![ActivationDecl {
                    cost: Some(ActivationCost {
                        mana: Some(MERCHANT_PRICE),
                        ..ActivationCost::default()
                    }),
                    ..ability(
                        "buy",
                        "Spend (2): Draw 1",
                        ActivationUses::Count(1),
                        hook(|_ctx| vec![draw(json_as(json!({ "count": 1 })))]),
                    )
                }],
                ..Script::default()
            }),
        ),
        (punisher.id.clone(), faces_of(punish(false), punish(true))),
        (
            asker.id.clone(),
            faces(Script {
                static_flags: quickdraw(),
                activations: vec![ability(
                    "ask",
                    "Ask",
                    ActivationUses::Count(1),
                    hook(|_ctx| vec![note("ask"), ask_controller("asked"), note("ask:tail")]),
                )],
                resume: IndexMap::from([("asked", notes_hook(&["answered"]))]),
                ..Script::default()
            }),
        ),
        (
            scepter.id.clone(),
            faces(Script {
                activations: vec![ActivationDecl {
                    can_activate: Some(condition_hook(|c| c.self_.memory.contains_key(SCEPTER_KEY))),
                    ..ability(
                        "cast",
                        "Cast a copy of that Spell",
                        ActivationUses::Count(1),
                        notes_hook(&["cast"]),
                    )
                }],
                ..Script::default()
            }),
        ),
        (
            chooser.id.clone(),
            faces(Script {
                activations: vec![
                    ActivationDecl {
                        has: Some(read_hook(|a| a.self_.memory.get(PICK_KEY).and_then(Value::as_str) == Some("alpha"))),
                        ..ability("alpha", "Alpha", ActivationUses::Count(1), notes_hook(&["alpha"]))
                    },
                    ActivationDecl {
                        has: Some(read_hook(|a| a.self_.memory.get(PICK_KEY).and_then(Value::as_str) == Some("beta"))),
                        ..ability("beta", "Beta", ActivationUses::Count(1), notes_hook(&["beta"]))
                    },
                    ability("gamma", "Gamma", ActivationUses::Count(1), notes_hook(&["gamma"])),
                ],
                ..Script::default()
            }),
        ),
        (
            mourner.id.clone(),
            faces(Script {
                death: Some(hook(|_ctx| vec![note("death"), ask_controller("mourned"), note("death:tail")])),
                resume: IndexMap::from([("mourned", notes_hook(&["mourned"]))]),
                ..Script::default()
            }),
        ),
        (
            trapper.id.clone(),
            faces(Script {
                activations: vec![ability("spring", "Spring", ActivationUses::Count(1), notes_hook(&["spring"]))],
                ..Script::default()
            }),
        ),
        (
            endless.id.clone(),
            faces(Script {
                activations: vec![ability("again", "Again", ActivationUses::Unlimited, notes_hook(&["again"]))],
                ..Script::default()
            }),
        ),
        (heroic.id.clone(), faces_of(heroic_face(false), heroic_face(true))),
        (
            keeper.id.clone(),
            faces(Script {
                activations: vec![ActivationDecl {
                    // TS `recalled({ self, data: {} }, KEEPER_KEY) !== undefined`: with no part path in the
                    // data, the key is the plain one (`work.partMemoryKey`).
                    can_activate: Some(condition_hook(|c| {
                        c.self_.memory.contains_key(&part_memory_key(&IndexMap::new(), KEEPER_KEY))
                    })),
                    ..ability(
                        "recall",
                        "Note what this remembered",
                        ActivationUses::Count(1),
                        hook(|ctx| vec![note(format!("recall:{}", js_string(recalled(&*ctx, KEEPER_KEY))))]),
                    )
                }],
                ..Script::default()
            }),
        ),
        (
            zapper.id.clone(),
            faces(Script {
                activations: vec![ActivationDecl {
                    targets: vec![any_target()],
                    ..ability(
                        "zap",
                        "Deal {damage} damage",
                        ActivationUses::Count(1),
                        hook(|ctx| {
                            let amount = param(&*ctx, "damage");
                            vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))]
                        }),
                    )
                }],
                ..Script::default()
            }),
        ),
        (spark.id.clone(), faces(Script::default())),
        (low_teller.id.clone(), teller()),
        (high_teller.id.clone(), teller()),
    ])
});

/// Every Activate fixture on top of `base`. TS `activateCatalog(base = {})`: pass `CardDefs::new()`
/// for the default.
pub fn activate_catalog(base: CardDefs) -> CardDefs {
    let mut defs = base;
    for entry in ACTIVATE_DEFS.iter() {
        defs.insert(entry.id.clone(), entry.clone());
    }
    defs
}
