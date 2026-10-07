//! Port of `packages/engine/test/fixtures/prompts.ts`.
//!
//! Test-only cards for the prompts-and-movement systems of patch v0.2.0 (docs/classic-sets.md B5 E13,
//! E16, E17, E18, E26). Each reproduces the engine half of a Classic or Classic+ card through the
//! effects library — the card itself arrives with its own script and test in the card waves — so the
//! engine is proved without `packages/cards` (CLAUDE.md: the engine never imports it).
//!
//! TS numbered the definitions from a module counter (`nextIndex = 3100`, one step per `def`); each
//! index is written out here in the order TS made them.

use std::sync::LazyLock;

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

/// TS's `{ ...a, ...b }` on two object literals: `b`'s keys replace `a`'s.
fn spread(mut base: Value, extra: Value) -> Value {
    if let (Some(into), Value::Object(from)) = (base.as_object_mut(), extra) {
        for (key, value) in from {
            into.insert(key, value);
        }
    }
    base
}

fn def(index: u32, name: &str, type_: &str, extra: Value) -> CardDef {
    json_as(spread(
        json!({
            "id": format!("pm-{name}"),
            "index": index.to_string(),
            "name": format!("{name} (prompts)"),
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
    ))
}

fn unit(index: u32, name: &str, attack: i32, health: i32, extra: Value) -> CardDef {
    let faces = json!({
        "base": { "attack": attack, "health": health, "keywords": [], "text": name },
        "radiant": { "attack": attack * 2, "health": health * 2, "keywords": [], "text": name },
    });
    def(index, name, "Unit", spread(faces, extra))
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// `list` followed by `more` (TS's `[...list, ...more]`).
fn then(mut list: Vec<Effect>, more: Vec<Effect>) -> Vec<Effect> {
    list.extend(more);
    list
}

fn damage_enemy_hero(amount: i32) -> Effect {
    effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))
}

fn damage_chosen(amount: i32) -> Effect {
    effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))
}

fn heal_self_hero(amount: i32) -> Effect {
    effects::heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": amount })))
}

fn exile_chosen() -> Effect {
    effects::exile(json_as(json!({ "target": { "of": "chosen" } })))
}

// ---------------------------------------------------------------------------------------------
// E18: the prompt kinds
// ---------------------------------------------------------------------------------------------

/// What every fixture hands out when its chain reached the end it should have.
pub fn prize() -> CardDef {
    unit(3101, "prize", 1, 1, json!({}))
}

/// Classic #8 Pickle's shape: the opponent chooses three times, their own hand pick for "discard".
pub fn pickle() -> CardDef {
    def(3102, "pickle", "Spell", json!({ "cost": 1 }))
}

pub const PICKLE_OPTIONS: &[&str] = &["discard", "exile", "draw"];
const PICKLE_CHOICES: i32 = 3;

fn pickle_ask(n: i32) -> Vec<Effect> {
    if n > PICKLE_CHOICES {
        return vec![];
    }
    vec![effects::choose_mode(json_as(json!({
        "by": "enemy",
        "options": PICKLE_OPTIONS,
        "step": "chosen",
        "prompt": format!("Pickle: choose ({n} of {PICKLE_CHOICES})"),
        "data": { "n": n },
    })))]
}

fn pickle_count(ctx: &EffectContext<'_>) -> i32 {
    match ctx.data.get("n") {
        Some(Value::Number(n)) => n.as_f64().map(|n| n as i32).unwrap_or(1),
        _ => 1,
    }
}

fn pickle_script() -> Script {
    Script {
        cry: Some(hook(|_ctx| pickle_ask(1))),
        resume: IndexMap::from([
            (
                "chosen",
                hook(|ctx| {
                    let n = pickle_count(ctx);
                    let chosen = effects::chosen_options(ctx);
                    match chosen.first().map(String::as_str) {
                        Some("discard") => vec![effects::choose_from_hand(json_as(json!({
                            "of": "enemy",
                            "by": "enemy",
                            "step": "discarded",
                            "prompt": "Pickle: discard",
                            "data": { "n": n },
                        })))],
                        Some("exile") => then(
                            vec![effects::exile_bottom_of_library(json_as(json!({ "player": "enemy" })))],
                            pickle_ask(n + 1),
                        ),
                        _ => then(vec![effects::draw(json_as(json!({ "count": 1 })))], pickle_ask(n + 1)),
                    }
                }),
            ),
            (
                "discarded",
                hook(|ctx| {
                    then(
                        vec![effects::discard(json_as(json!({ "target": { "of": "chosen" } })))],
                        pickle_ask(pickle_count(ctx) + 1),
                    )
                }),
            ),
        ]),
        ..Script::default()
    }
}

/// Classic #18 Glitch in the System's shape: a `number` declared with the play (R81), 0 to 10.
pub fn glitch() -> CardDef {
    def(3103, "glitch", "Spell", json!({ "cost": 0 }))
}

pub static GLITCH_OPTIONS: LazyLock<Vec<String>> = LazyLock::new(|| (0..11).map(|n| n.to_string()).collect());

fn glitch_script() -> Script {
    Script {
        modes: vec![json_as::<ModeDecl>(json!({ "kind": "number", "options": &*GLITCH_OPTIONS }))],
        cry: Some(hook(|ctx| vec![damage_enemy_hero(effects::chosen_number(ctx).unwrap_or(0))])),
        ..Script::default()
    }
}

/// A `number` prompt asked at resolution, 1 to 3: the chosen number is the damage.
pub fn numberer() -> CardDef {
    def(3104, "numberer", "Spell", json!({ "cost": 0 }))
}

fn numberer_script() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![effects::choose_number(json_as(json!({ "from": 1, "to": 3, "step": "n", "prompt": "Pick 1 to 3" })))]
        })),
        resume: IndexMap::from([(
            "n",
            hook(|ctx| vec![damage_enemy_hero(effects::chosen_number(ctx).unwrap_or(0))]),
        )]),
        ..Script::default()
    }
}

/// Classic+ #42 KY's Test's answer half: right answer, the prize; wrong, nothing (R465).
pub fn quiz() -> CardDef {
    def(3105, "quiz", "Spell", json!({ "cost": 1 }))
}

/// TS's `QUIZ` literal.
pub struct Quiz {
    pub statement: &'static str,
    pub options: &'static [&'static str],
    pub correct: i32,
}

pub const QUIZ: Quiz = Quiz {
    statement: "2 + 2 = ?",
    options: &["4", "3", "5", "22"],
    correct: 0,
};

fn quiz_script() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![
                effects::choose_answer(json_as(json!({
                    "step": "answered",
                    "statement": QUIZ.statement,
                    "options": QUIZ.options,
                    "correct": QUIZ.correct,
                    "data": { "difficulty": "easy" },
                }))),
                // The tail after the question: it runs after the answer, and it reads no key.
                heal_self_hero(1),
            ]
        })),
        resume: IndexMap::from([(
            "answered",
            hook(|ctx| {
                if effects::answered_correctly(ctx) {
                    vec![effects::add_to_hand(json_as(json!({ "defId": prize().id })))]
                } else {
                    vec![]
                }
            }),
        )]),
        ..Script::default()
    }
}

/// Classic+ #62 KY's Papaya's cell half: cells one at a time, a lane each, "done" after the first.
pub fn papaya() -> CardDef {
    def(3106, "papaya", "Spell", json!({ "cost": 1 }))
}

pub const PAPAYA_CELLS: usize = 4;

fn cells_so_far(ctx: &EffectContext<'_>) -> Vec<ZoneRef> {
    let mut held: Vec<ZoneRef> = match ctx.data.get("cells") {
        Some(cells @ Value::Array(_)) => serde_json::from_value(cells.clone()).unwrap_or_default(),
        _ => Vec::new(),
    };
    held.extend(effects::chosen_cells(ctx));
    held
}

fn papaya_script() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![effects::choose_cell(json_as(json!({ "step": "cell", "prompt": "Papaya: a point", "data": { "cells": [] } })))]
        })),
        resume: IndexMap::from([(
            "cell",
            hook(|ctx| {
                let cells = cells_so_far(ctx);
                let done = ctx.targets.iter().any(|selection| matches!(selection, Selection::None))
                    || cells.len() >= PAPAYA_CELLS;
                if !done {
                    let lanes: Vec<i32> = cells.iter().map(|cell| cell.lane).collect();
                    return vec![effects::choose_cell(json_as(json!({
                        "step": "cell",
                        "done": true,
                        "cells": { "exceptLanes": lanes },
                        "prompt": "Papaya: another point",
                        "data": { "cells": cells },
                    })))];
                }
                // Exile whatever stands on the chosen cells: the engine half of "every card on the curve".
                cells
                    .iter()
                    .filter_map(|cell| {
                        let slot = ZoneRef {
                            player: cell.player,
                            row: if cell.row == Row::Units { Row::Units } else { Row::Backrow },
                            lane: cell.lane,
                        };
                        zones::card_at(&*ctx.state, &slot).map(|card| card.id.clone())
                    })
                    .map(|id| effects::exile(json_as(json!({ "target": { "of": "instance", "instanceId": id } }))))
                    .collect()
            }),
        )]),
        ..Script::default()
    }
}

/// Classic #90 In Too Deep's reward half: asked of its controller whenever the other player draws.
pub fn quest() -> CardDef {
    def(3107, "quest", "Field Spell", json!({ "cost": 1 }))
}

/// One of `QUEST_REWARDS`: TS's `{ id, label }`.
pub struct RewardOption {
    pub id: &'static str,
    pub label: &'static str,
}

pub const QUEST_REWARDS: &[RewardOption] = &[
    RewardOption {
        id: "A",
        label: "Heal your hero 6",
    },
    RewardOption {
        id: "B",
        label: "Deal 3 damage to the enemy hero",
    },
];

fn quest_trigger() -> TriggerDef {
    TriggerDef::new("quest-done", &[GameEventType::Drawn], |ctx, event| {
        if let GameEvent::Drawn { player, .. } = event
            && *player == ctx.controller
        {
            return vec![];
        }
        let rewards: Vec<Value> = QUEST_REWARDS
            .iter()
            .map(|reward| json!({ "id": reward.id, "label": reward.label }))
            .collect();
        vec![effects::choose_reward(json_as(json!({
            "step": "reward",
            "rewards": rewards,
            "prompt": "Quest complete",
        })))]
    })
}

fn quest_script() -> Script {
    Script {
        triggers: vec![quest_trigger()],
        resume: IndexMap::from([(
            "reward",
            hook(|ctx| {
                if effects::chosen_options(ctx).first().map(String::as_str) == Some("A") {
                    vec![heal_self_hero(6)]
                } else {
                    vec![damage_enemy_hero(3)]
                }
            }),
        )]),
        ..Script::default()
    }
}

/// An up-to pick from a pile (C #34's shape before R684): up to 2 (Radiant 4, graveyard or exile) to hand.
pub fn acquire() -> CardDef {
    def(3108, "acquire", "Spell", json!({ "cost": 1 }))
}

fn return_picks(ctx: &EffectContext<'_>) -> Vec<Effect> {
    (0..ctx.targets.len())
        .map(|index| effects::add_to_hand(json_as(json!({ "instance": { "of": "chosen", "index": index } }))))
        .collect()
}

pub fn acquire_scripts() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| {
                vec![effects::choose_pick(json_as(json!({
                    "step": "picked",
                    "from": [{ "zone": "graveyard" }],
                    "max": 2,
                    "prompt": "Return 2",
                })))]
            })),
            resume: IndexMap::from([("picked", hook(|ctx| return_picks(ctx)))]),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| {
                vec![effects::choose_pick(json_as(json!({
                    "step": "picked",
                    "from": [{ "zone": "graveyard" }, { "zone": "exile" }],
                    "max": 4,
                    "prompt": "Return 4",
                })))]
            })),
            resume: IndexMap::from([("picked", hook(|ctx| return_picks(ctx)))]),
            ..Script::default()
        },
    }
}

/// Classic #44 Back from the GY's shape: graveyard Units costing (5) or less together, summoned.
pub fn back_from_gy() -> CardDef {
    def(3109, "back-from-gy", "Spell", json!({ "cost": 1 }))
}

pub const BACK_BUDGET: i32 = 5;

fn back_script() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![effects::choose_pick(json_as(json!({
                "step": "picked",
                "from": [{ "zone": "graveyard" }],
                "filter": { "type": "Unit" },
                "max": 5,
                "budget": BACK_BUDGET,
                "prompt": "Summon Units costing (5) or less",
            })))]
        })),
        resume: IndexMap::from([(
            "picked",
            hook(|ctx| {
                (0..ctx.targets.len())
                    .map(|index| effects::summon(json_as(json!({ "instance": { "of": "chosen", "index": index } }))))
                    .collect()
            }),
        )]),
        ..Script::default()
    }
}

// ---------------------------------------------------------------------------------------------
// E17: the opponent's hand as the options
// ---------------------------------------------------------------------------------------------

/// Classic #11 Mind Melt's shape (SPEC §8.6): base, a `pick` of one card of their hand, exiled; Radiant,
/// a `mode` prompt of the costs in their hand, and every card of the chosen cost exiled.
pub fn mind_melt() -> CardDef {
    def(3110, "mind-melt", "Spell", json!({ "cost": 1 }))
}

pub fn mind_melt_scripts() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| {
                vec![effects::choose_pick(json_as(json!({
                    "step": "exile",
                    "from": [{ "zone": "hand", "player": "enemy" }],
                    "min": 1,
                    "max": 1,
                    "prompt": "Exile a card from their hand",
                })))]
            })),
            resume: IndexMap::from([("exile", hook(|_ctx| vec![exile_chosen()]))]),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| {
                vec![effects::choose_cost_in_hand(json_as(json!({ "step": "cost", "prompt": "Choose a cost" })))]
            })),
            resume: IndexMap::from([(
                "cost",
                hook(|ctx| match effects::chosen_number(ctx) {
                    None => vec![],
                    Some(cost) => vec![effects::exile_matching(json_as(json!({
                        "zones": ["hand"],
                        "player": "enemy",
                        "cost": cost,
                    })))],
                }),
            )]),
            ..Script::default()
        },
    }
}

// ---------------------------------------------------------------------------------------------
// E16, E2: cards between the players' piles
// ---------------------------------------------------------------------------------------------

/// Classic #9 Income Tax's shape: when the opponent draws, they keep one card and give you the rest.
pub fn income_tax() -> CardDef {
    def(3111, "income-tax", "Trap", json!({ "cost": 2 }))
}

fn income_tax_script(radiant: bool) -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("tax", &[GameEventType::Drawn], |_ctx, _event| {
                vec![effects::choose_from_hand(json_as(json!({
                    "of": "enemy",
                    "by": "enemy",
                    "step": "keep",
                    "prompt": "Keep one card",
                })))]
            })
            .with_when(|ctx, event| {
                matches!(event, GameEvent::Drawn { player, .. } if *player == opponent_of(ctx.controller))
            }),
        ],
        resume: IndexMap::from([(
            "keep",
            hook(move |_ctx| {
                let mut args = json!({ "from": "enemy", "cards": "unchosen" });
                if radiant {
                    args["costMod"] = json!(-1);
                }
                vec![effects::give_from_hand(json_as(args))]
            }),
        )]),
        ..Script::default()
    }
}

/// Classic+ #12.3 Fluffy Grip's shape: a random Unit of their deck to your hand, costing (0).
pub fn fluffy_grip() -> CardDef {
    def(3112, "fluffy-grip", "Spell", json!({ "cost": 1 }))
}

pub fn fluffy_grip_scripts() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| {
                vec![effects::take_from_library(json_as(json!({
                    "from": "enemy",
                    "filter": { "type": "Unit" },
                    "costOverride": 0,
                })))]
            })),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| {
                vec![effects::take_from_library(json_as(json!({
                    "from": "enemy",
                    "filter": { "type": "Unit" },
                    "costOverride": 0,
                    "radiant": true,
                })))]
            })),
            ..Script::default()
        },
    }
}

/// Classic #58 Common Resources' shape: start of turn, draw the bottom card of their deck.
pub fn common_resources() -> CardDef {
    def(3113, "common-resources", "Field Spell", json!({ "cost": 2 }))
}

fn common_resources_script() -> Script {
    Script {
        start_of_turn: Some(hook(|_ctx| vec![effects::draw_from_opponent(Default::default())])),
        ..Script::default()
    }
}

/// A card that casts itself on draw and says so: it proves a stolen draw casts for the drawer.
pub fn cast_on_draw_marker() -> CardDef {
    def(3114, "cast-marker", "Spell", json!({ "cost": 0 }))
}

fn cast_on_draw_marker_script() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|_ctx| vec![damage_enemy_hero(2)])),
        ..Script::default()
    }
}

// ---------------------------------------------------------------------------------------------
// E13: trigger a Cry
// ---------------------------------------------------------------------------------------------

/// A Cry with no choices: 2 to the enemy hero, and +1/+1 on "this".
pub fn crier() -> CardDef {
    unit(3115, "crier", 2, 3, json!({ "cost": 1 }))
}

fn crier_script() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![
                damage_enemy_hero(2),
                effects::buff(json_as(json!({ "target": { "of": "self" }, "attack": 1, "health": 1 }))),
            ]
        })),
        ..Script::default()
    }
}

/// A Cry with a declared target (R81): 3 damage to it.
pub fn aimer() -> CardDef {
    unit(3116, "aimer", 1, 1, json!({ "cost": 1 }))
}

fn aimer_script() -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit", "hero"] }))],
        cry: Some(hook(|_ctx| vec![damage_chosen(3)])),
        ..Script::default()
    }
}

/// A Cry with a mode, and a target that belongs to one mode only (`forModes`, R90).
pub fn moder() -> CardDef {
    unit(3117, "moder", 1, 1, json!({ "cost": 1 }))
}

fn moder_script() -> Script {
    Script {
        modes: vec![json_as::<ModeDecl>(json!({ "kind": "mode", "options": ["hit", "heal"] }))],
        targets: vec![json_as::<TargetDecl>(json!({
            "kind": "target",
            "min": 1,
            "max": 1,
            "filter": { "side": "enemy", "of": ["unit", "hero"] },
            "forModes": ["hit"],
        }))],
        cry: Some(hook(|ctx| {
            if ctx.modes.first().map(String::as_str) == Some("hit") {
                vec![damage_chosen(4)]
            } else {
                vec![heal_self_hero(4)]
            }
        })),
        ..Script::default()
    }
}

/// A Cry that asks in the middle of its list, so its own tail is parked (R113).
pub fn asker() -> CardDef {
    unit(3118, "asker", 1, 1, json!({ "cost": 1 }))
}

fn asker_script() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![
                damage_enemy_hero(1),
                effects::choose_mode(json_as(json!({ "options": ["left", "right"], "step": "after", "prompt": "asker" }))),
                damage_enemy_hero(2),
            ]
        })),
        resume: IndexMap::from([("after", hook(|_ctx| vec![damage_enemy_hero(4)]))]),
        ..Script::default()
    }
}

/// A Cry whose first declaration is a Tribute (a play's price), then a target (R90, R123).
pub fn tribute_crier() -> CardDef {
    unit(3119, "tribute-crier", 3, 3, json!({ "cost": 2 }))
}

fn tribute_crier_script() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(1),
            ..StaticFlags::default()
        }),
        targets: vec![
            json_as::<TargetDecl>(json!({
                "kind": "tribute",
                "min": 1,
                "max": 1,
                "filter": { "side": "ally", "of": ["unit"] },
                "amount": 1,
            })),
            TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["hero"] })),
        ],
        cry: Some(hook(|ctx| {
            let amount = if matches!(ctx.targets.first(), Some(Selection::None)) { 5 } else { 1 };
            vec![effects::damage(json_as(json!({ "to": { "of": "chosen", "index": 1 }, "amount": amount })))]
        })),
        ..Script::default()
    }
}

/// Classic #54 Rewind's shape. Base: a Unit of yours on the field, declared with the play. The graveyard
/// form picks the Unit out of the graveyard at resolution, as a card with `TargetFilter.of: "graveyard"`
/// will declare it once the play pipeline offers graveyard targets. Radiant: twice, each run its own.
pub fn rewind() -> CardDef {
    def(3120, "rewind", "Spell", json!({ "cost": 0 }))
}

fn trigger_cry_chosen() -> Effect {
    effects::trigger_cry(json_as(json!({ "target": { "of": "chosen" } })))
}

pub fn rewind_scripts() -> CardScripts {
    CardScripts {
        base: Script {
            targets: vec![TargetDecl::target(1, 1, json!({ "side": "ally", "of": ["unit"] }))],
            cry: Some(hook(|_ctx| vec![trigger_cry_chosen(), heal_self_hero(1)])),
            ..Script::default()
        },
        radiant: Script {
            targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
            cry: Some(hook(|_ctx| vec![trigger_cry_chosen(), trigger_cry_chosen(), heal_self_hero(1)])),
            ..Script::default()
        },
    }
}

pub fn grave_rewind() -> CardDef {
    def(3121, "grave-rewind", "Spell", json!({ "cost": 1 }))
}

fn grave_rewind_script() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![effects::choose_pick(json_as(json!({
                "step": "picked",
                "from": [{ "zone": "graveyard" }],
                "filter": { "type": "Unit" },
                "min": 1,
                "max": 1,
            })))]
        })),
        resume: IndexMap::from([("picked", hook(|_ctx| vec![trigger_cry_chosen()]))]),
        ..Script::default()
    }
}

// ---------------------------------------------------------------------------------------------
// E26: deck and graveyard triggers, "summon this"
// ---------------------------------------------------------------------------------------------

/// The fields of a `cardResolved` the triggers below read.
struct Played<'a> {
    player: PlayerId,
    instance_id: &'a str,
    def_id: &'a str,
}

/// "After you play …": the play's `cardResolved` (§10.5 step 7), once the played card has resolved.
fn played_event(event: &GameEvent) -> Option<Played<'_>> {
    match event {
        GameEvent::CardResolved {
            player,
            instance_id,
            def_id,
            ..
        } => Some(Played {
            player: *player,
            instance_id,
            def_id,
        }),
        _ => None,
    }
}

fn type_of(ctx: &EffectContext<'_>, def_id: &str) -> CardType {
    catalog::def_of(Some(&*ctx.state), def_id).type_
}

/// Classic #66 EU Striker's shape: from your hand, after you play a Unit, summon this (its Cry would ping).
pub fn striker() -> CardDef {
    unit(3122, "striker", 5, 4, json!({ "cost": 2 }))
}

fn striker_trigger() -> TriggerDef {
    TriggerDef::new("striker-arrive", &[GameEventType::CardResolved], |ctx, event| {
        let Some(played) = played_event(event) else {
            return vec![];
        };
        if played.player != ctx.controller
            || ctx.self_.as_ref().map(|card| card.id.as_str()) == Some(played.instance_id)
        {
            return vec![];
        }
        if type_of(ctx, played.def_id) == CardType::Unit {
            vec![effects::summon_this()]
        } else {
            vec![]
        }
    })
}

fn striker_script() -> Script {
    Script {
        hand_triggers: vec![striker_trigger()],
        cry: Some(hook(|_ctx| vec![damage_enemy_hero(9)])),
        ..Script::default()
    }
}

/// Classic+ #37 Wardrum's shape: from your hand or deck, after you play a Spell, summon this.
pub fn wardrum() -> CardDef {
    unit(3123, "wardrum", 5, 5, json!({ "cost": 5 }))
}

fn wardrum_trigger() -> TriggerDef {
    TriggerDef::new("wardrum-arrive", &[GameEventType::CardResolved], |ctx, event| {
        let Some(played) = played_event(event) else {
            return vec![];
        };
        if played.player != ctx.controller {
            return vec![];
        }
        if type_of(ctx, played.def_id) == CardType::Spell {
            vec![effects::summon_this()]
        } else {
            vec![]
        }
    })
}

fn wardrum_script() -> Script {
    Script {
        hand_triggers: vec![wardrum_trigger()],
        deck_triggers: vec![wardrum_trigger()],
        ..Script::default()
    }
}

/// A deck trigger that asks, so its list parks under its id and resumes (R113, `work.scriptStepFor`).
pub fn deck_asker() -> CardDef {
    unit(3124, "deck-asker", 1, 1, json!({ "cost": 1 }))
}

fn deck_asker_script() -> Script {
    Script {
        deck_triggers: vec![TriggerDef::new("deck-ask", &[GameEventType::CardResolved], |ctx, event| {
            let Some(played) = played_event(event) else {
                return vec![];
            };
            if played.player != ctx.controller || type_of(ctx, played.def_id) != CardType::Spell {
                return vec![];
            }
            vec![
                effects::choose_mode(json_as(json!({ "options": ["stay", "come"], "step": "decided", "prompt": "Deck asker" }))),
                damage_enemy_hero(1),
            ]
        })],
        resume: IndexMap::from([(
            "decided",
            hook(|ctx| {
                if effects::chosen_options(ctx).first().map(String::as_str) == Some("come") {
                    vec![effects::summon_this()]
                } else {
                    vec![]
                }
            }),
        )]),
        ..Script::default()
    }
}

/// A deck trigger that answers every play and does nothing: the hidden card that must stay hidden.
pub fn deck_watcher() -> CardDef {
    unit(3125, "deck-watcher", 1, 1, json!({ "cost": 1 }))
}

fn deck_watcher_script() -> Script {
    Script {
        deck_triggers: vec![TriggerDef::new(
            "deck-watch",
            &[GameEventType::CardPlayed, GameEventType::CardResolved],
            |_ctx, _event| vec![],
        )],
        ..Script::default()
    }
}

/// Classic #47 Recurring Felinor's shape: in your graveyard, when one of your Traps fires, return this.
pub fn recurring() -> CardDef {
    unit(3126, "recurring", 3, 2, json!({ "cost": 2 }))
}

fn recurring_script(radiant: bool) -> Script {
    Script {
        graveyard_triggers: vec![TriggerDef::new("recur", &[GameEventType::TrapFired], move |ctx, event| {
            // TS read `event.controller` off any event: one that is not a `trapFired` has none.
            if !matches!(event, GameEvent::TrapFired { controller, .. } if *controller == ctx.controller) {
                return vec![];
            }
            let mut args = json!({ "instance": { "of": "self" } });
            if radiant {
                args["costOverride"] = json!(0);
            }
            vec![effects::add_to_hand(json_as(args))]
        })],
        ..Script::default()
    }
}

/// A graveyard trigger that answers every play and does nothing: R464's last place in a side's order.
pub fn grave_watcher() -> CardDef {
    unit(3127, "grave-watcher", 1, 1, json!({ "cost": 1 }))
}

fn grave_watcher_script() -> Script {
    Script {
        graveyard_triggers: vec![TriggerDef::new(
            "grave-watch",
            &[GameEventType::CardPlayed, GameEventType::CardResolved],
            |_ctx, _event| vec![],
        )],
        ..Script::default()
    }
}

/// Classic #78 Radiant's pick across zones: one Unit of yours on the field, in your hand or in your deck.
pub fn cross_pick() -> CardDef {
    def(3128, "cross-pick", "Spell", json!({ "cost": 0 }))
}

fn cross_pick_script() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![effects::choose_pick(json_as(json!({
                "step": "picked",
                "from": [{ "zone": "field" }, { "zone": "hand" }, { "zone": "library" }],
                "filter": { "type": "Unit" },
                "min": 1,
                "max": 1,
                "prompt": "A Unit of yours",
            })))]
        })),
        resume: IndexMap::from([("picked", hook(|_ctx| vec![exile_chosen()]))]),
        ..Script::default()
    }
}

/// A Trap that fires when the other player plays a card, and does nothing else.
pub fn snare() -> CardDef {
    def(3129, "snare", "Trap", json!({ "cost": 1 }))
}

fn snare_script() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("snare", &[GameEventType::CardPlayed], |_ctx, _event| vec![]).with_when(
                |ctx, event| !matches!(event, GameEvent::CardPlayed { player, .. } if *player == ctx.controller),
            ),
        ],
        ..Script::default()
    }
}

/// Mills the top three cards of its controller's library into their graveyard, for a pile to pick from.
pub fn mill() -> CardDef {
    def(3130, "mill", "Spell", json!({ "cost": 0 }))
}

const MILL_COUNT: usize = 3;

fn mill_script() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![Effect::new("fixture:mill", |ctx| {
                let controller = ctx.controller;
                let cards: Vec<CardInstance> =
                    ctx.state.players[controller].library.iter().take(MILL_COUNT).cloned().collect();
                for mut card in cards {
                    let _ = zones::move_to_zone(&mut *ctx.state, &mut card, OffFieldZone::Graveyard, Default::default());
                }
            })]
        })),
        ..Script::default()
    }
}

/// A plain Spell and a plain Unit to play.
pub fn spark() -> CardDef {
    def(3131, "spark", "Spell", json!({ "cost": 0 }))
}

fn spark_script() -> Script {
    Script {
        cry: Some(hook(|_ctx| vec![damage_enemy_hero(1)])),
        ..Script::default()
    }
}

pub fn grunt() -> CardDef {
    unit(3132, "grunt", 2, 2, json!({ "cost": 0 }))
}

// ---------------------------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------------------------

/// A Quickdraw copy of a fixture, so a replay test's opening hand holds it whatever the shuffle (§2.1, R225).
pub fn quickdraw_of(card: &CardDef) -> CardDef {
    CardDef {
        id: format!("{}-qd", card.id),
        index: format!("{}-qd", card.index),
        name: format!("{} (quickdraw)", card.name),
        ..card.clone()
    }
}

fn quickdraw_sources() -> Vec<CardDef> {
    vec![
        pickle(),
        quiz(),
        papaya(),
        back_from_gy(),
        rewind(),
        mind_melt(),
        grave_rewind(),
        spark(),
        grunt(),
        mill(),
        crier(),
        aimer(),
        fluffy_grip(),
        striker(),
        wardrum(),
        quest(),
    ]
}

pub static PROMPT_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    let mut defs = vec![
        prize(),
        pickle(),
        glitch(),
        numberer(),
        quiz(),
        papaya(),
        quest(),
        acquire(),
        back_from_gy(),
        mind_melt(),
        income_tax(),
        fluffy_grip(),
        common_resources(),
        cast_on_draw_marker(),
        crier(),
        aimer(),
        moder(),
        asker(),
        tribute_crier(),
        rewind(),
        grave_rewind(),
        striker(),
        wardrum(),
        deck_asker(),
        deck_watcher(),
        recurring(),
        snare(),
        spark(),
        grunt(),
        mill(),
        grave_watcher(),
        cross_pick(),
    ];
    defs.extend(quickdraw_sources().iter().map(quickdraw_of));
    defs
});

fn quickdraw(scripts: &CardScripts) -> CardScripts {
    let flag = |script: &Script| {
        let mut flags = script.flags();
        flags.quickdraw = Some(true);
        Script {
            static_flags: Some(flags),
            ..script.clone()
        }
    };
    CardScripts {
        base: flag(&scripts.base),
        radiant: flag(&scripts.radiant),
    }
}

fn base_scripts() -> IndexMap<String, CardScripts> {
    let mut table: IndexMap<String, CardScripts> = IndexMap::new();
    table.insert(pickle().id, both(pickle_script()));
    table.insert(glitch().id, both(glitch_script()));
    table.insert(numberer().id, both(numberer_script()));
    table.insert(quiz().id, both(quiz_script()));
    table.insert(papaya().id, both(papaya_script()));
    table.insert(quest().id, both(quest_script()));
    table.insert(acquire().id, acquire_scripts());
    table.insert(back_from_gy().id, both(back_script()));
    table.insert(mind_melt().id, mind_melt_scripts());
    table.insert(
        income_tax().id,
        CardScripts {
            base: income_tax_script(false),
            radiant: income_tax_script(true),
        },
    );
    table.insert(fluffy_grip().id, fluffy_grip_scripts());
    table.insert(common_resources().id, both(common_resources_script()));
    table.insert(cast_on_draw_marker().id, both(cast_on_draw_marker_script()));
    table.insert(crier().id, both(crier_script()));
    table.insert(aimer().id, both(aimer_script()));
    table.insert(moder().id, both(moder_script()));
    table.insert(asker().id, both(asker_script()));
    table.insert(tribute_crier().id, both(tribute_crier_script()));
    table.insert(rewind().id, rewind_scripts());
    table.insert(grave_rewind().id, both(grave_rewind_script()));
    table.insert(striker().id, both(striker_script()));
    table.insert(wardrum().id, both(wardrum_script()));
    table.insert(deck_asker().id, both(deck_asker_script()));
    table.insert(deck_watcher().id, both(deck_watcher_script()));
    table.insert(
        recurring().id,
        CardScripts {
            base: recurring_script(false),
            radiant: recurring_script(true),
        },
    );
    table.insert(snare().id, both(snare_script()));
    table.insert(spark().id, both(spark_script()));
    table.insert(mill().id, both(mill_script()));
    table.insert(grave_watcher().id, both(grave_watcher_script()));
    table.insert(cross_pick().id, both(cross_pick_script()));
    table
}

pub static PROMPT_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let base = base_scripts();
    let mut table = base.clone();
    for card in quickdraw_sources() {
        let scripts = base.get(&card.id).cloned().unwrap_or_else(|| both(Script::default()));
        table.insert(quickdraw_of(&card).id, quickdraw(&scripts));
    }
    table
});

/// This file's definitions, by id (the brief's `catalog()`).
pub fn catalog() -> CardDefs {
    PROMPT_DEFS.iter().map(|card| (card.id.clone(), card.clone())).collect()
}

/// This file's scripts, by id (the brief's `scripts()`).
pub fn scripts() -> IndexMap<String, CardScripts> {
    PROMPT_SCRIPTS.clone()
}

/// Add these fixtures to whatever the harness registered (`newGame` registers its own first).
pub fn register_prompt_fixtures() {
    let mut all_defs = registered_catalog().clone();
    all_defs.extend(catalog());
    register_catalog(all_defs);
    let mut all_scripts = registered_scripts().clone();
    all_scripts.extend(scripts());
    register_scripts(all_scripts);
}
