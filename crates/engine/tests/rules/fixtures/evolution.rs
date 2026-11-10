//! Test-only cards for Evolve and Evolves (issue #3536, R3536000–R3536005): a pinnable Evolve text,
//! the `evolve` and `evolveDeck` verbs, the standing "Evolves when" conditions and the evolution
//! multiplier, through the engine's verbs and hooks, so the engine never imports `crates/cards`. Ids
//! are `ev-` from 35360, so they collide with no other fixture file's.

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

use super::harness::both;

/// What every Evolve text deals to the enemy hero, so runs are counted by health.
pub const EV_PING: i32 = 1;
/// How many zero-cost cards a hand must hold for `ev-zero` (Stinkyscape+ Selective Breeding's shape).
pub const EV_ZERO_NEEDED: i32 = 3;
/// Damage that leaves `ev-half` above half health, so it evolves nothing.
pub const EV_HALF: i32 = 2;
/// What `ev-bolt` deals: below half for `ev-half`, lethal for `ev-survivor`'s kin, never quite `ev-dear`.
pub const EV_BOLT: i32 = 5;
/// The extra Evolve runs `ev-twice` and `ev-thrice` give (High Evolutionary's Aura's shape).
pub const EV_TWICE: i32 = 1;
pub const EV_THRICE: i32 = 2;

/// A (1) 2/2 Unit: "Evolve: deal 1 damage to the enemy hero."
pub const PINGER: &str = "ev-pinger";
/// A Spell: "Evolve a Unit."
pub const EVOLVER: &str = "ev-evolver";
/// A Spell: "Summon a copy of a Unit."
pub const COPIER: &str = "ev-copier";
/// A Spell: "Return a Unit to its owner's hand."
pub const BOUNCER: &str = "ev-bounce";
/// A Spell: "Deal 5 damage to a Unit."
pub const BOLT: &str = "ev-bolt";
/// A Spell: "Evolve all cards in your deck."
pub const BREEDER: &str = "ev-breeder";
/// A (1) 2/2 Unit: "Evolves when your hand holds at least 3 cards that cost (0). Evolve: ping."
pub const ZERO: &str = "ev-zero";
/// A (1) 2/2 Unit: "Evolves when all your unit zones are full. Evolve: ping."
pub const FULL: &str = "ev-full";
/// A (2) 4/8 Unit: "Evolves when this falls below half health. Evolve: ping."
pub const HALF: &str = "ev-half";
/// A (1) 2/2 Unit with Stack: "Evolves when this is Stacked on something. Evolve: ping."
pub const STACKER: &str = "ev-stacker";
/// A (1) 2/2 Unit: "Evolves when your opponent plays a Unit. Evolve: ping."
pub const WATCHER: &str = "ev-watcher";
/// A (1) 2/2 Unit: "Evolves when this moves from Defense to Attack Position. Evolve: ping."
pub const RISER: &str = "ev-riser";
/// A (1) 2/6 Unit: "Evolves when this survives damage. Evolve: ping."
pub const SURVIVOR: &str = "ev-survivor";
/// A (1) 2/2 Unit: "Evolves when a friendly Unit dies. Evolve: ping."
pub const MOURNER: &str = "ev-mourner";
/// A (1) 5/5 Unit: "Evolves when this destroys a Unit. Evolve: ping."
pub const KILLER: &str = "ev-killer";
/// A (1) 2/2 Unit: "Evolves at the Start of your Turn. Evolve: ping."
pub const DAWN: &str = "ev-dawn";
/// A Field Spell with Temporary: "Evolves when this is discarded to Temporary. Evolve: ping."
pub const CHEESE: &str = "ev-cheese";
/// A Field Spell: "Your evolutions trigger twice."
pub const TWICE: &str = "ev-twice";
/// A Field Spell: "Your evolutions trigger thrice."
pub const THRICE: &str = "ev-thrice";
/// A (1) 2/2 Unit: "Evolves when you launch a Starship. Evolve: ping."
pub const SHIP: &str = "ev-ship";
/// A (0) 2/2 Unit with no text: what never evolves.
pub const PLAIN: &str = "ev-plain";
/// A (2) 8/8 Unit with no text: what kills an attacker without dying.
pub const DEAR: &str = "ev-dear";

fn card(index: u32, id: &str, type_: &str, cost: i32, stats: Option<(i32, i32)>, keywords: Value) -> CardDef {
    let (base, radiant) = match stats {
        Some((attack, health)) => (
            json!({ "attack": attack, "health": health, "keywords": keywords, "text": id }),
            json!({ "attack": 2 * attack, "health": 2 * health, "keywords": keywords, "text": id }),
        ),
        None => (
            json!({ "keywords": keywords, "text": id }),
            json!({ "keywords": keywords, "text": id }),
        ),
    };
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (evolution fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": base,
        "radiant": radiant,
    }))
}

fn pick(filter: Value) -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, filter)]
}

fn chosen() -> Value {
    json!({ "of": "chosen" })
}

fn ping() -> Effect {
    effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": EV_PING })))
}

fn evolve_self() -> Effect {
    effects::evolve(json_as(json!({ "target": { "of": "self" } })))
}

fn evolve_hook() -> Option<Hook> {
    Some(hook(|_ctx| vec![ping()]))
}

/// This card's own id, so a trigger answers only its own card.
fn me(ctx: &EffectContext<'_>) -> String {
    ctx.self_.as_ref().map(|card| card.id.clone()).unwrap_or_default()
}

pub fn scripts() -> IndexMap<String, CardScripts> {
    let entries: Vec<(&str, Script)> = vec![
        (
            PINGER,
            Script {
                evolve: evolve_hook(),
                ..Script::default()
            },
        ),
        (
            EVOLVER,
            Script {
                targets: pick(json!({ "of": ["unit"] })),
                cry: Some(hook(|_ctx| {
                    vec![effects::evolve(json_as(json!({ "target": chosen() })))]
                })),
                ..Script::default()
            },
        ),
        (
            COPIER,
            Script {
                targets: pick(json!({ "of": ["unit"] })),
                cry: Some(hook(|_ctx| {
                    vec![effects::summon_copy(json_as(json!({ "of": chosen() })))]
                })),
                ..Script::default()
            },
        ),
        (
            BOUNCER,
            Script {
                targets: pick(json!({ "of": ["unit"] })),
                cry: Some(hook(|_ctx| {
                    vec![effects::bounce(json_as(json!({ "target": chosen() })))]
                })),
                ..Script::default()
            },
        ),
        (
            BOLT,
            Script {
                targets: pick(json!({ "of": ["unit"] })),
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "chosen" }, "amount": EV_BOLT }),
                    ))]
                })),
                ..Script::default()
            },
        ),
        (
            BREEDER,
            Script {
                cry: Some(hook(|_ctx| vec![effects::evolve_deck(json_as(json!({})))])),
                ..Script::default()
            },
        ),
        // R3536001: standing conditions, read at the settled state check.
        (
            ZERO,
            Script {
                evolve: evolve_hook(),
                evolves_when: Some(read_hook(|a| {
                    a.state.players[a.self_.controller]
                        .hand
                        .iter()
                        .filter(|card| {
                            jackioh_engine::mana::effective_cost(a.state, card, Default::default()) == 0
                        })
                        .count()
                        >= EV_ZERO_NEEDED as usize
                })),
                ..Script::default()
            },
        ),
        (
            FULL,
            Script {
                evolve: evolve_hook(),
                evolves_when: Some(read_hook(|a| {
                    a.state.players[a.self_.controller]
                        .units
                        .iter()
                        .filter(|pile| pile.as_ref().is_some_and(|pile| !pile.is_empty()))
                        .count()
                        == jackioh_engine::config::UNIT_ZONES as usize
                })),
                ..Script::default()
            },
        ),
        (
            HALF,
            Script {
                evolve: evolve_hook(),
                evolves_when: Some(read_hook(|a| {
                    let view = jackioh_engine::layers::unit_view(a.state, a.self_);
                    view.health * 2 < view.max_health
                })),
                ..Script::default()
            },
        ),
        (
            STACKER,
            Script {
                evolve: evolve_hook(),
                evolves_when: Some(read_hook(|a| {
                    jackioh_engine::zones::slot_of(a.state, a.self_)
                        .is_some_and(|slot| !jackioh_engine::zones::beneath_at(a.state, slot).is_empty())
                })),
                ..Script::default()
            },
        ),
        // R3536001: event conditions, each the card's own trigger evolving itself.
        (
            WATCHER,
            Script {
                evolve: evolve_hook(),
                triggers: vec![TriggerDef::new(
                    "watch-plays",
                    &[GameEventType::CardPlayed],
                    move |ctx, event| {
                        let GameEvent::CardPlayed { player, def_id, .. } = event else {
                            return vec![];
                        };
                        if *player == ctx.controller {
                            return vec![];
                        }
                        let played_is_unit = jackioh_engine::catalog::find_def(Some(&*ctx.state), def_id)
                            .is_some_and(|def| def.type_ == jackioh_engine::wire::CardType::Unit);
                        if played_is_unit {
                            vec![evolve_self()]
                        } else {
                            vec![]
                        }
                    },
                )],
                ..Script::default()
            },
        ),
        (
            RISER,
            Script {
                evolve: evolve_hook(),
                triggers: vec![TriggerDef::new(
                    "watch-rise",
                    &[GameEventType::PositionSwitched],
                    move |ctx, event| {
                        let GameEvent::PositionSwitched {
                            instance_id,
                            position,
                        } = event
                        else {
                            return vec![];
                        };
                        if *instance_id == me(ctx) && *position == jackioh_engine::wire::Position::Atk {
                            vec![evolve_self()]
                        } else {
                            vec![]
                        }
                    },
                )],
                ..Script::default()
            },
        ),
        (
            SURVIVOR,
            Script {
                evolve: evolve_hook(),
                triggers: vec![TriggerDef::new(
                    "watch-hurt",
                    &[GameEventType::Damage],
                    move |ctx, event| {
                        let GameEvent::Damage { target_id, .. } = event else {
                            return vec![];
                        };
                        if *target_id != me(ctx) {
                            return vec![];
                        }
                        // "Survives": a lethal hit leaves the field, so nothing evolves.
                        let alive = ctx
                            .live_self()
                            .is_some_and(|card| jackioh_engine::zones::acts_on_field(ctx.state, card));
                        if alive { vec![evolve_self()] } else { vec![] }
                    },
                )],
                ..Script::default()
            },
        ),
        (
            MOURNER,
            Script {
                evolve: evolve_hook(),
                triggers: vec![TriggerDef::new(
                    "watch-deaths",
                    &[GameEventType::Destroyed],
                    move |ctx, event| {
                        let GameEvent::Destroyed {
                            instance_id,
                            controller,
                            ..
                        } = event
                        else {
                            return vec![];
                        };
                        if *controller == ctx.controller && *instance_id != me(ctx) {
                            vec![evolve_self()]
                        } else {
                            vec![]
                        }
                    },
                )],
                ..Script::default()
            },
        ),
        (
            KILLER,
            Script {
                evolve: evolve_hook(),
                triggers: vec![TriggerDef::new(
                    "watch-kills",
                    &[GameEventType::Destroyed],
                    move |ctx, event| {
                        let GameEvent::Destroyed { killer_id, .. } = event else {
                            return vec![];
                        };
                        if killer_id.as_deref() == Some(me(ctx).as_str()) {
                            vec![evolve_self()]
                        } else {
                            vec![]
                        }
                    },
                )],
                ..Script::default()
            },
        ),
        (
            DAWN,
            Script {
                evolve: evolve_hook(),
                start_of_turn: Some(hook(|_ctx| vec![evolve_self()])),
                ..Script::default()
            },
        ),
        (
            CHEESE,
            Script {
                evolve: evolve_hook(),
                graveyard_triggers: vec![TriggerDef::new(
                    "watch-temporary",
                    &[GameEventType::Discarded],
                    move |ctx, event| {
                        let GameEvent::Discarded {
                            instance_id,
                            by_temporary,
                            ..
                        } = event
                        else {
                            return vec![];
                        };
                        if *instance_id == me(ctx) && *by_temporary == Some(true) {
                            vec![evolve_self()]
                        } else {
                            vec![]
                        }
                    },
                )],
                ..Script::default()
            },
        ),
        (
            SHIP,
            Script {
                evolve: evolve_hook(),
                triggers: vec![TriggerDef::new(
                    "watch-launch",
                    &[GameEventType::StarshipLaunched],
                    move |ctx, event| {
                        let GameEvent::StarshipLaunched { player, .. } = event else {
                            return vec![];
                        };
                        if *player == ctx.controller {
                            vec![evolve_self()]
                        } else {
                            vec![]
                        }
                    },
                )],
                ..Script::default()
            },
        ),
        // R3536003: the evolution multiplier.
        (
            TWICE,
            Script {
                evolve_extra: Some(read_hook(|_| EV_TWICE)),
                ..Script::default()
            },
        ),
        (
            THRICE,
            Script {
                evolve_extra: Some(read_hook(|_| EV_THRICE)),
                ..Script::default()
            },
        ),
    ];
    entries
        .into_iter()
        .map(|(id, script)| (id.to_string(), both(script)))
        .collect()
}

pub fn defs() -> Vec<CardDef> {
    vec![
        card(35360, PINGER, "Unit", 1, Some((2, 2)), json!([])),
        card(35361, EVOLVER, "Spell", 0, None, json!([])),
        card(35362, COPIER, "Spell", 0, None, json!([])),
        card(35363, BOUNCER, "Spell", 0, None, json!([])),
        card(35364, BOLT, "Spell", 0, None, json!([])),
        card(35365, BREEDER, "Spell", 0, None, json!([])),
        card(35366, ZERO, "Unit", 1, Some((2, 2)), json!([])),
        card(35367, FULL, "Unit", 1, Some((2, 2)), json!([])),
        card(35368, HALF, "Unit", 2, Some((4, 8)), json!([])),
        card(
            35369,
            STACKER,
            "Unit",
            1,
            Some((2, 2)),
            json!([{ "kind": "Stack" }]),
        ),
        card(35370, WATCHER, "Unit", 1, Some((2, 2)), json!([])),
        card(35371, RISER, "Unit", 1, Some((2, 2)), json!([])),
        card(35372, SURVIVOR, "Unit", 1, Some((2, 6)), json!([])),
        card(35373, MOURNER, "Unit", 1, Some((2, 2)), json!([])),
        card(35374, KILLER, "Unit", 5, Some((5, 5)), json!([])),
        card(35375, DAWN, "Unit", 1, Some((2, 2)), json!([])),
        card(
            35376,
            CHEESE,
            "Field Spell",
            0,
            None,
            json!([{ "kind": "Temporary" }]),
        ),
        card(35377, TWICE, "Field Spell", 1, None, json!([])),
        card(35378, THRICE, "Field Spell", 1, None, json!([])),
        card(35379, SHIP, "Unit", 1, Some((2, 2)), json!([])),
        card(35380, PLAIN, "Unit", 0, Some((2, 2)), json!([])),
        card(35381, DEAR, "Unit", 2, Some((8, 8)), json!([])),
    ]
}

/// The engine's fixture catalog (`harness::setup_catalog`) with these cards and scripts on top.
pub fn register_evolution_fixtures() {
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
