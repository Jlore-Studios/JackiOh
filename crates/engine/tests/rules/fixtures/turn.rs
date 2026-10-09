//! Test-only cards for the turn systems (docs/classic-sets.md B5): ending a turn from an effect (E10),
//! draw limits and counts (E3, E4), cast on draw (E39), the delayed kinds (E27), the rest-of-game
//! effect (E28), and the start-of-turn and cleanup stages for Brittle and Animated (B3.3, B3.1). Each
//! reproduces one shape through the engine's own verbs; the engine never imports `crates/cards`.
//!
//! Ids are prefixed `tn-` and indexed from 4300, so they cannot collide with another fixture file's.
use std::sync::LazyLock;

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

fn def(index: u32, name: &str, type_: &str, extra: Value) -> CardDef {
    let mut rest = match extra {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    let attack = rest.remove("attack").and_then(|v| v.as_i64()).unwrap_or(2);
    let health = rest.remove("health").and_then(|v| v.as_i64()).unwrap_or(2);
    let keywords = rest.remove("keywords").unwrap_or_else(|| json!([]));
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
        "id": format!("tn-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (turn)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face,
        "radiant": radiant_face,
    });
    if let Some(into) = card.as_object_mut() {
        for (key, value) in rest {
            into.insert(key, value);
        }
    }
    json_as(card)
}

// The note log: a Field Spell in p2's backrow lane 5 whose memory records what ran, in order.

pub fn log_card() -> CardDef {
    def(4301, "log", "Field Spell", json!({}))
}

pub const LOG_LANE: i32 = 5;

fn log_of(state: &GameState) -> Option<&CardInstance> {
    state
        .players
        .p2
        .backrow
        .get((LOG_LANE - 1) as usize)
        .and_then(|slot| slot.as_ref())
}

fn log_of_mut(state: &mut GameState) -> Option<&mut CardInstance> {
    state
        .players
        .p2
        .backrow
        .get_mut((LOG_LANE - 1) as usize)
        .and_then(|slot| slot.as_mut())
}

/// The steps the log card's memory holds, in order (empty without a log card).
pub fn notes(state: &GameState) -> Vec<String> {
    match log_of(state).and_then(|log| log.memory.get("steps")) {
        Some(Value::Array(steps)) => steps
            .iter()
            .map(|step| step.as_str().unwrap_or_default().to_string())
            .collect(),
        _ => Vec::new(),
    }
}

/// Append to the log directly, for a test double standing in for another module's body.
pub fn write(state: &mut GameState, entry: &str) {
    let Some(log) = log_of_mut(state) else {
        return;
    };
    let mut steps = match log.memory.get("steps") {
        Some(Value::Array(steps)) => steps.clone(),
        _ => Vec::new(),
    };
    steps.push(json!(entry));
    log.memory.insert("steps".to_string(), Value::Array(steps));
}

pub fn note(entry: impl Into<String>) -> Effect {
    let entry = entry.into();
    Effect::new("tn:note", move |ctx| {
        write(&mut *ctx.state, &entry);
    })
}

/// §10.6: a prompt for the controller with one answer, whose answer re-enters `step`.
pub fn ask_controller(step: &str) -> Effect {
    let step = step.to_string();
    Effect::new("tn:ask", move |ctx| {
        let resume = prompts::resume_self(ctx, &step, IndexMap::new());
        let player = ctx.controller;
        let _ = prompts::open_prompt(
            ctx,
            prompts::OpenPromptArgs {
                player,
                kind: PromptKind::Target,
                aim: None,
                prompt: "the card asks its controller".into(),
                options: vec![PromptOption {
                    key: "none".into(),
                    label: "nothing".into(),
                    selection: Selection::None,
                    cost: None,
                    radiant: None,
                }],
                min: None,
                max: None,
                budget: None,
                owner: None,
                resume,
            },
        );
    })
}

fn faces(base: Script) -> CardScripts {
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

fn faces_with(base: Script, radiant: Script) -> CardScripts {
    CardScripts { base, radiant }
}

// E10: ending the turn from an effect

/// "End your turn", then more of the same list: the rest resolves first (R456).
pub fn cutter() -> CardDef {
    def(4302, "cutter", "Spell", json!({}))
}

/// "End your turn", then a question: the turn waits for the answer (R456).
pub fn cut_asker() -> CardDef {
    def(4303, "cut-asker", "Spell", json!({ "tags": ["Quickdraw"] }))
}

/// Classic+ #26's base: a Unit with Cast on draw: End your turn. Radiant: one more action.
pub fn tempo() -> CardDef {
    def(
        4304,
        "tempo",
        "Unit",
        json!({ "attack": 9, "health": 9, "keywords": [{ "kind": "Taunt" }] }),
    )
}

/// "You may take N more actions, then your turn ends" as a Spell, for the count's own tests.
pub fn one_more() -> CardDef {
    def(4305, "one-more", "Spell", json!({}))
}

pub fn two_more() -> CardDef {
    def(4306, "two-more", "Spell", json!({}))
}

/// The AI card Rate Limit's shape: the opponent's play sets it off, and their turn ends after it.
pub fn rate_limit() -> CardDef {
    def(4307, "rate-limit", "Trap", json!({}))
}

/// A plain Spell that notes its own resolution.
pub fn marker() -> CardDef {
    def(4308, "marker", "Spell", json!({}))
}

/// A Spell that asks its caster, and notes the answer.
pub fn questioner() -> CardDef {
    def(4309, "questioner", "Spell", json!({}))
}

// E3, E4, E39: draws

/// Classic #4's aura: "Your opponent can't draw more than 1 card each turn."
pub fn palantir() -> CardDef {
    def(4310, "palantir", "Field Spell", json!({ "tags": ["Quickdraw"] }))
}

/// Classic #49's aura, loosened to 2 so the lowest-holds rule shows: "Players can't draw more than 2".
pub fn anti_greed() -> CardDef {
    def(4311, "anti-greed", "Unit", json!({ "attack": 9, "health": 9 }))
}

/// "Draw 2", for a second and third draw inside one turn.
pub fn draw_two() -> CardDef {
    def(4312, "draw-two", "Spell", json!({ "tags": ["Quickdraw"] }))
}

/// A cast-on-draw Spell and a cast-on-draw Unit that note being cast.
pub fn cast_spell() -> CardDef {
    def(4313, "cast-spell", "Spell", json!({}))
}

pub fn cast_unit() -> CardDef {
    def(4314, "cast-unit", "Unit", json!({ "attack": 1, "health": 1 }))
}

/// A plain card, drawn to hand.
pub fn plain() -> CardDef {
    def(4315, "plain", "Spell", json!({}))
}

/// Classic #9's trigger, as a Field Spell that notes each of the opponent's 2nd draws in a turn.
pub fn taxman() -> CardDef {
    def(4316, "taxman", "Field Spell", json!({}))
}

// E27, E28: delayed kinds and the rest of the game

/// Classic #20's third mode: a chosen Unit is destroyed at the start of your next turn.
pub fn doom() -> CardDef {
    def(4317, "doom", "Spell", json!({}))
}

/// Classic #20 Radiant's: all enemy Units then.
pub fn doom_all() -> CardDef {
    def(4318, "doom-all", "Spell", json!({}))
}

/// Classic #37's clause: discard your hand at the end of this turn (Radiant: of your next turn).
pub fn hurrah() -> CardDef {
    def(4319, "hurrah", "Spell", json!({}))
}

/// `delay` with `next`: a card step at the end of the controller's next turn.
pub fn later() -> CardDef {
    def(4320, "later", "Spell", json!({}))
}

/// Classic+ #52's shape: for the rest of the game, at the start of your turn, a note (a card, in #52).
pub fn contract() -> CardDef {
    def(4321, "contract", "Spell", json!({}))
}

/// The same, whose start-of-turn effect asks.
pub fn contract_ask() -> CardDef {
    def(4322, "contract-ask", "Spell", json!({ "tags": ["Quickdraw"] }))
}

// The start-of-turn stages: a delayed effect and a start-of-turn hook to order them against.

/// A Spell that schedules a note for the start of its controller's next turn (R62's delayed stage).
pub fn reminder() -> CardDef {
    def(4323, "reminder", "Spell", json!({}))
}

/// A Field Spell with a start-of-turn hook (R62's triggers stage) and an end-of-turn one.
pub fn clock() -> CardDef {
    def(4324, "clock", "Field Spell", json!({}))
}

/// A Field Spell whose trigger on `crumbled` asks: a start-of-turn stage's own events can pause it.
pub fn crumble_watcher() -> CardDef {
    def(4325, "crumble-watcher", "Field Spell", json!({}))
}

fn limits(list: Vec<DrawLimit>) -> Option<DrawLimitHook> {
    Some(read_hook(move |_args| list.clone()))
}

fn second_draw(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    matches!(
        event,
        GameEvent::Drawn { player, turn_draw: Some(2), .. } if *player == opponent_of(ctx.controller)
    )
}

/// A data bag entry as text: a string as itself, `undefined` for a missing key.
fn js_string(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
    }
}

fn quickdraw() -> Option<StaticFlags> {
    Some(StaticFlags {
        quickdraw: Some(true),
        ..StaticFlags::default()
    })
}

fn cast_on_draw() -> Option<StaticFlags> {
    Some(StaticFlags {
        cast_on_draw: Some(true),
        ..StaticFlags::default()
    })
}

pub static TURN_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        log_card(),
        cutter(),
        cut_asker(),
        tempo(),
        one_more(),
        two_more(),
        rate_limit(),
        marker(),
        questioner(),
        palantir(),
        anti_greed(),
        draw_two(),
        cast_spell(),
        cast_unit(),
        plain(),
        taxman(),
        doom(),
        doom_all(),
        hurrah(),
        later(),
        contract(),
        contract_ask(),
        reminder(),
        clock(),
        crumble_watcher(),
    ]
});

pub static TURN_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let mut table: IndexMap<String, CardScripts> = IndexMap::new();
    table.insert(log_card().id, faces(Script::default()));
    table.insert(
        cutter().id,
        faces(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::end_turn(Default::default()), note("after the cut")]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        cut_asker().id,
        faces(Script {
            static_flags: quickdraw(),
            cry: Some(hook(|_ctx| {
                vec![
                    effects::end_turn(Default::default()),
                    ask_controller("asked"),
                    note("cut:tail"),
                ]
            })),
            resume: IndexMap::from([("asked", hook(|_ctx| vec![note("cut:answered")]))]),
            ..Script::default()
        }),
    );
    table.insert(
        tempo().id,
        faces_with(
            Script {
                static_flags: cast_on_draw(),
                cry: Some(hook(|_ctx| vec![effects::end_turn(Default::default())])),
                ..Script::default()
            },
            Script {
                static_flags: cast_on_draw(),
                cry: Some(hook(|_ctx| {
                    vec![effects::end_turn_after_actions(json_as(json!({ "actions": 1 })))]
                })),
                ..Script::default()
            },
        ),
    );
    table.insert(
        one_more().id,
        faces(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::end_turn_after_actions(json_as(json!({ "actions": 1 })))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        two_more().id,
        faces(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::end_turn_after_actions(json_as(json!({ "actions": 2 })))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        rate_limit().id,
        faces(Script {
            triggers: vec![
                TriggerDef::new("rate", &[GameEventType::CardPlayed], |_ctx, _event| {
                    vec![effects::end_turn(json_as(json!({ "player": "enemy" })))]
                })
                .with_when(|ctx, event| {
                    matches!(event, GameEvent::CardPlayed { player, .. } if *player != ctx.controller)
                }),
            ],
            ..Script::default()
        }),
    );
    table.insert(
        marker().id,
        faces(Script {
            cry: Some(hook(|ctx| vec![note(format!("marker:{}", ctx.controller))])),
            ..Script::default()
        }),
    );
    table.insert(
        questioner().id,
        faces(Script {
            cry: Some(hook(|_ctx| vec![ask_controller("asked")])),
            resume: IndexMap::from([("asked", hook(|_ctx| vec![note("questioner:answered")]))]),
            ..Script::default()
        }),
    );
    table.insert(
        palantir().id,
        faces(Script {
            static_flags: quickdraw(),
            draw_limit: limits(vec![DrawLimit {
                player: DrawLimitPlayer::Enemy,
                count: 1,
            }]),
            ..Script::default()
        }),
    );
    table.insert(
        anti_greed().id,
        faces(Script {
            draw_limit: limits(vec![DrawLimit {
                player: DrawLimitPlayer::Both,
                count: 2,
            }]),
            ..Script::default()
        }),
    );
    table.insert(
        draw_two().id,
        faces(Script {
            static_flags: quickdraw(),
            cry: Some(hook(|_ctx| vec![effects::draw(json_as(json!({ "count": 2 })))])),
            ..Script::default()
        }),
    );
    table.insert(
        cast_spell().id,
        faces(Script {
            static_flags: cast_on_draw(),
            cry: Some(hook(|_ctx| vec![note("cast-spell")])),
            ..Script::default()
        }),
    );
    table.insert(
        cast_unit().id,
        faces(Script {
            static_flags: cast_on_draw(),
            cry: Some(hook(|_ctx| vec![note("cast-unit")])),
            ..Script::default()
        }),
    );
    table.insert(
        plain().id,
        faces(Script {
            cry: Some(hook(|_ctx| vec![note("plain")])),
            ..Script::default()
        }),
    );
    table.insert(
        taxman().id,
        faces(Script {
            // A Field Spell's trigger reads its condition in `run` (`when` is a trap's, R99).
            triggers: vec![TriggerDef::new("tax", &[GameEventType::Drawn], |ctx, event| {
                if second_draw(ctx, event) {
                    vec![note("taxed")]
                } else {
                    vec![]
                }
            })],
            ..Script::default()
        }),
    );
    table.insert(
        doom().id,
        faces(Script {
            targets: vec![TargetDecl::target(1, 1, json!({ "of": ["unit"] }))],
            cry: Some(hook(|_ctx| {
                vec![effects::destroy_at_next_turn_start(json_as(
                    json!({ "target": { "of": "chosen" } }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        doom_all().id,
        faces(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::destroy_at_next_turn_start(json_as(
                    json!({ "scope": { "side": "enemy" } }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        hurrah().id,
        faces_with(
            Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::discard_hand_at_turn_end(json_as(
                        json!({ "turn": "this" }),
                    ))]
                })),
                ..Script::default()
            },
            Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::discard_hand_at_turn_end(json_as(
                        json!({ "turn": "next" }),
                    ))]
                })),
                ..Script::default()
            },
        ),
    );
    table.insert(
        later().id,
        faces(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::delay(json_as(json!({
                    "at": { "phase": "end", "player": "self" },
                    "step": "later",
                    "next": true,
                })))]
            })),
            delayed: Some(hook(|ctx| vec![note(format!("later:{}", ctx.state.turn))])),
            ..Script::default()
        }),
    );
    table.insert(
        contract().id,
        faces(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::for_rest_of_game(json_as(json!({
                    "step": "tick",
                    "label": "At the start of your turn, take a note",
                    "data": { "n": 1 },
                })))]
            })),
            delayed: Some(hook(|ctx| {
                vec![note(format!(
                    "tick:{}:{}:{}:{}",
                    ctx.controller,
                    ctx.state.turn,
                    js_string(ctx.data.get("n")),
                    if ctx.self_.is_none() { "no-self" } else { "self" },
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        contract_ask().id,
        faces(Script {
            static_flags: quickdraw(),
            cry: Some(hook(|_ctx| {
                vec![effects::for_rest_of_game(json_as(json!({
                    "step": "tick",
                    "label": "At the start of your turn, answer",
                })))]
            })),
            delayed: Some(hook(|_ctx| {
                vec![note("ask-tick"), ask_controller("ticked"), note("ask-tick:tail")]
            })),
            resume: IndexMap::from([("ticked", hook(|_ctx| vec![note("ask-tick:answered")]))]),
            ..Script::default()
        }),
    );
    table.insert(
        reminder().id,
        faces(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::delay(json_as(json!({
                    "at": { "phase": "start", "player": "self" },
                    "step": "remind",
                })))]
            })),
            delayed: Some(hook(|_ctx| vec![note("delayed")])),
            ..Script::default()
        }),
    );
    table.insert(
        clock().id,
        faces(Script {
            start_of_turn: Some(hook(|ctx| {
                vec![note(format!("start-of-turn:{}", ctx.controller))]
            })),
            end_of_turn: Some(hook(|ctx| vec![note(format!("end-of-turn:{}", ctx.controller))])),
            ..Script::default()
        }),
    );
    table.insert(
        crumble_watcher().id,
        faces(Script {
            triggers: vec![TriggerDef::new(
                "watch",
                &[GameEventType::Crumbled],
                |_ctx, _event| vec![note("crumble-seen"), ask_controller("seen")],
            )],
            resume: IndexMap::from([("seen", hook(|_ctx| vec![note("crumble-answered")]))]),
            ..Script::default()
        }),
    );
    table
});

/// `base` with every turn fixture over it.
pub fn turn_catalog(base: CardDefs) -> CardDefs {
    let mut defs = base;
    for entry in TURN_DEFS.iter() {
        defs.insert(entry.id.clone(), entry.clone());
    }
    defs
}

/// This file's scripts, by id: `TURN_SCRIPTS`.
pub fn scripts() -> IndexMap<String, CardScripts> {
    TURN_SCRIPTS.clone()
}
