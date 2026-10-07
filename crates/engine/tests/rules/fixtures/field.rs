//! Fixtures for the field workstream's systems (docs/classic-sets.md B3.1 Animated, B5 E20 Lock variants
//! and Unlock, E21 backrow piles and carriers, E22 Flicker). Test-only scripts, since the engine never
//! imports `packages/cards` (CLAUDE.md): each is the shape of a Classic or Classic+ card that uses the
//! system, cut down to the part the engine test exercises. Defs are prefixed `fd-` and indexed from
//! 9400, so they collide with no other file's catalog (BUILD §0).
//!
//! Port of `packages/engine/test/fixtures/field.ts`. Each exported def is a `pub static` under TS's
//! name; TS's module counter (`nextIndex`, from 9400, one per `def` call in file order) is each def's
//! stated index. TS's module `let nonce` is a per-thread counter (each Rust test runs on its own
//! thread, as each TS test file ran in its own module), and an action body is the TS literal as JSON
//! (`act(&state, json!({ "type": "endTurn", "playerId": "p1" }))`) or any `ActionInput`.

#![allow(non_upper_case_globals)]

use std::cell::Cell;
use std::sync::LazyLock;

use jackioh_engine::effects::{
    animate, choose_target, damage, destroy_all, flicker, lock_lane, lock_own_zone, lock_played_zone, lock_random_zone,
    unlock_all,
};
use jackioh_engine::testkit::*;

use super::harness::new_game;

/// TS `def(name, type, extra = {})`: a Core Common at cost 1 whose faces print `name`, `extra` over it.
fn def(name: &str, index: i32, type_: &str, extra: Value) -> CardDef {
    let mut card = json!({
        "id": format!("fd-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (field fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    });
    if let (Some(target), Value::Object(extra)) = (card.as_object_mut(), extra) {
        for (key, value) in extra {
            target.insert(key, value);
        }
    }
    json_as(card)
}

/// TS `face(attack, health, keywords, text = "")`, as the JSON a def literal takes.
fn face(attack: i32, health: i32, keywords: Value) -> Value {
    json!({ "attack": attack, "health": health, "keywords": keywords, "text": "" })
}

fn animated() -> Value {
    json!({ "kind": "Animated" })
}

fn on_your_turn() -> Value {
    json!({ "kind": "Animated on your turn" })
}

/// Classic #5 Tesla's shape: a 1/4 Animated Field Trap that zaps each enemy Unit arriving, then animates in Defense.
pub static tesla: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "tesla",
        9401,
        "Field Trap",
        json!({
            "cost": 2,
            "base": face(1, 4, json!([animated(), { "kind": "Lifesteal" }])),
            "radiant": face(2, 8, json!([animated(), { "kind": "Lifesteal" }])),
        }),
    )
});

/// A plain Animated Trap (not a Field Trap): fires once on an enemy play, then animates.
pub static springer: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "springer",
        9402,
        "Trap",
        json!({ "base": face(3, 3, json!([animated()])), "radiant": face(6, 6, json!([animated()])) }),
    )
});

/// An Animated Field Trap whose list asks before it animates: `[chooseTarget, animate]` (R113).
pub static asker: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "asker",
        9403,
        "Field Trap",
        json!({ "base": face(2, 2, json!([animated()])), "radiant": face(4, 4, json!([animated()])) }),
    )
});

/// Classic+ #12.8 Frostspatula's shape: an "Animated on your turn" Field Spell with Rush.
pub static spatula: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "spatula",
        9404,
        "Field Spell",
        json!({
            "cost": 2,
            "base": face(10, 3, json!([on_your_turn(), { "kind": "Rush" }])),
            "radiant": face(20, 6, json!([on_your_turn(), { "kind": "Rush" }])),
        }),
    )
});

/// A plain Animated Field Spell, which animates as it enters the field (B3.1 rule 4).
pub static golem: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "golem",
        9405,
        "Field Spell",
        json!({ "base": face(3, 3, json!([animated()])), "radiant": face(6, 6, json!([animated()])) }),
    )
});

/// A stat-less Animated Field Spell (R657): incidentally animated, it fights as a 0/1.
pub static wisp: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "wisp",
        9406,
        "Field Spell",
        json!({
            "base": { "keywords": [animated()], "text": "wisp" },
            "radiant": { "keywords": [animated()], "text": "wisp" },
        }),
    )
});

/// A carrier whose aura gives its controller's cards Stack: Classic+ #33 Ivory Tower's shape before patch v0.2.3.
pub static tower: LazyLock<CardDef> = LazyLock::new(|| def("tower", 9407, "Field Spell", json!({ "cost": 2 })));

/// Classic+ #33 Ivory Tower's shape since patch v0.2.3: a carrier that takes one Unit a stay (R653).
pub static fuser: LazyLock<CardDef> = LazyLock::new(|| def("fuser", 9408, "Field Spell", json!({ "cost": 2 })));

/// A Field Spell that prints Stack, so it tops an occupied backrow zone without any aura (B5 E21).
pub static cover: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "cover",
        9409,
        "Field Spell",
        json!({
            "base": { "keywords": [{ "kind": "Stack" }], "text": "Stack" },
            "radiant": { "keywords": [{ "kind": "Stack" }], "text": "Stack" },
        }),
    )
});

/// A Field Spell whose aura gives its controller's units +2 attack: off while it lies under a pile.
pub static banner: LazyLock<CardDef> = LazyLock::new(|| def("banner", 9410, "Field Spell", json!({})));

/// A face-down Trap that notes every enemy play it answers: silent while it lies under a pile.
pub static watcher: LazyLock<CardDef> = LazyLock::new(|| def("watcher", 9411, "Trap", json!({})));

/// A Field Trap (not Animated) that notes every enemy play, fired again and again.
pub static listener: LazyLock<CardDef> = LazyLock::new(|| def("listener", 9412, "Field Trap", json!({})));

/// An Animated Field Trap that notes every enemy play and animates: a turret on `cardPlayed`.
pub static ears: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "ears",
        9413,
        "Field Trap",
        json!({ "base": face(1, 5, json!([animated()])), "radiant": face(2, 10, json!([animated()])) }),
    )
});

/// A Spell that destroys its caster's backrow, then asks (a pause after a carrier has gone, R446).
pub static wrecker: LazyLock<CardDef> = LazyLock::new(|| def("wrecker", 9414, "Spell", json!({})));

/// Classic+ #34's first mode as a Spell: Lock a random zone on the opponent's side.
pub static leak: LazyLock<CardDef> = LazyLock::new(|| def("leak", 9415, "Spell", json!({})));

/// Classic #84 Lockdown's shape: after a permanent is played, Lock its zone (either player's play).
pub static lockdown: LazyLock<CardDef> = LazyLock::new(|| def("lockdown", 9416, "Field Spell", json!({ "cost": 2 })));

/// Classic+ #1 Doom Shroom's shape: fires on an enemy play and Locks its own zone.
pub static doom: LazyLock<CardDef> = LazyLock::new(|| def("doom", 9417, "Trap", json!({})));

/// Classic #71 Lane Eater's shape: Cry: Lock this lane (Radiant: the enemy side of it).
pub static eater: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "eater",
        9418,
        "Unit",
        json!({ "cost": 3, "base": face(4, 4, json!([])), "radiant": face(8, 8, json!([])) }),
    )
});

/// A Field Spell that notes every death it answers: silent for the death that uncovers it (R212).
pub static mourner: LazyLock<CardDef> = LazyLock::new(|| def("mourner", 9419, "Field Spell", json!({})));

/// Classic+ #77's Unlock half: Unlock every zone.
pub static unlocker: LazyLock<CardDef> = LazyLock::new(|| def("unlocker", 9420, "Spell", json!({})));

/// Classic #14's Radiant move as a Spell: flicker a chosen Unit, then ask (a pause after the flicker).
pub static blink: LazyLock<CardDef> = LazyLock::new(|| def("blink", 9421, "Spell", json!({})));

pub static FIELD_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        tesla.clone(),
        springer.clone(),
        asker.clone(),
        spatula.clone(),
        golem.clone(),
        wisp.clone(),
        tower.clone(),
        fuser.clone(),
        cover.clone(),
        banner.clone(),
        watcher.clone(),
        listener.clone(),
        ears.clone(),
        wrecker.clone(),
        leak.clone(),
        lockdown.clone(),
        doom.clone(),
        eater.clone(),
        unlocker.clone(),
        blink.clone(),
        mourner.clone(),
    ]
});

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// The memory key the note effects write, on the card whose script runs.
pub const NOTES: &str = "fdNotes";

/// An effect that appends `name` to the running card's own notes (`memory.fdNotes`): on the card as it
/// stands in the state (TS wrote through the live `ctx.self`), and on the context's copy of it.
pub fn note(name: impl Into<String>) -> Effect {
    let name: String = name.into();
    Effect::new("fd:note", move |ctx| {
        let Some(id) = ctx.self_.as_ref().map(|card| card.id.clone()) else {
            return;
        };
        let mut written = match find_instance(ctx.state, &id) {
            Some(live) => notes_of(live),
            None => notes_of(ctx.self_.as_ref()),
        };
        written.push(name.clone());
        if let Some(live) = find_instance_mut(ctx.state, &id) {
            live.memory.insert(NOTES.to_string(), json!(written));
        }
        if let Some(own) = ctx.self_.as_mut() {
            own.memory.insert(NOTES.to_string(), json!(written));
        }
    })
}

/// A card's notes, or none (TS `notesOf(card: CardInstance | undefined | null)`: pass `&card`,
/// `Some(&card)` or `None`).
pub fn notes_of<'a>(card: impl Into<Option<&'a CardInstance>>) -> Vec<String> {
    match card.into().and_then(|card| card.memory.get(NOTES)).and_then(Value::as_array) {
        Some(entries) => entries.iter().filter_map(|entry| entry.as_str().map(str::to_string)).collect(),
        None => vec![],
    }
}

fn enemy_play(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    matches!(event, GameEvent::CardPlayed { player, .. } if *player != ctx.controller)
}

fn tesla_face(amount: i32) -> Script {
    Script {
        triggers: vec![TriggerDef::new("fd-tesla-zap", &[GameEventType::Summoned], move |_ctx, event| {
            match event {
                GameEvent::Summoned { instance_id, .. } => vec![
                    damage(json_as(json!({ "to": { "of": "instance", "instanceId": instance_id }, "amount": amount }))),
                    animate(json_as(json!({ "position": "DEF" }))),
                ],
                _ => vec![],
            }
        })
        .with_when(|ctx, event| {
            matches!(event, GameEvent::Summoned { player, row: Row::Units, .. } if *player != ctx.controller)
        })],
        ..Script::default()
    }
}

/// A trigger on an enemy play whose list is `notes` then, when `animates`, `animate()`.
fn enemy_play_noter(id: &'static str, entry: &'static str, animates: bool) -> Script {
    Script {
        triggers: vec![TriggerDef::new(id, &[GameEventType::CardPlayed], move |_ctx, _event| {
            let mut effects = vec![note(entry)];
            if animates {
                effects.push(animate(Default::default()));
            }
            effects
        })
        .with_when(enemy_play)],
        ..Script::default()
    }
}

pub static FIELD_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            tesla.id.clone(),
            CardScripts {
                base: tesla_face(4),
                radiant: tesla_face(8),
            },
        ),
        (springer.id.clone(), both(enemy_play_noter("fd-spring", "sprang", true))),
        (
            asker.id.clone(),
            both(Script {
                triggers: vec![TriggerDef::new("fd-ask", &[GameEventType::CardPlayed], |_ctx, _event| {
                    vec![
                        choose_target(json_as(json!({ "step": "hit", "scope": { "side": "enemy", "of": ["unit", "hero"] } }))),
                        note("tail"),
                        animate(Default::default()),
                    ]
                })
                .with_when(enemy_play)],
                resume: IndexMap::from([(
                    "hit",
                    hook(|_ctx| {
                        vec![
                            damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 1 }))),
                            note("answered"),
                        ]
                    }),
                )]),
                ..Script::default()
            }),
        ),
        (
            spatula.id.clone(),
            both(Script {
                end_of_turn: Some(hook(|ctx| {
                    let where_ = match ctx.self_.as_ref().map(|card| &card.zone) {
                        Some(Zone::Field { row, .. }) => row.as_str(),
                        _ => "gone",
                    };
                    vec![note(where_)]
                })),
                ..Script::default()
            }),
        ),
        (
            tower.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    carrier: Some(true),
                    ..StaticFlags::default()
                }),
                aura: Some(aura_hook(|a| {
                    let controller = a.self_.controller;
                    vec![AuraEntry {
                        applies: Box::new(move |card: &CardInstance| card.controller == controller),
                        mod_: StatMod {
                            keywords: Some(vec![Keyword::Stack]),
                            ..StatMod::default()
                        },
                    }]
                })),
                ..Script::default()
            }),
        ),
        (
            fuser.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    fuses_carried: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            }),
        ),
        (wisp.id.clone(), both(Script::default())),
        (
            banner.id.clone(),
            both(Script {
                aura: Some(aura_hook(|a| {
                    let controller = a.self_.controller;
                    vec![AuraEntry {
                        applies: Box::new(move |card: &CardInstance| {
                            card.controller == controller && card.zone.z() == ZoneName::Field
                        }),
                        mod_: StatMod {
                            attack: Some(2),
                            ..StatMod::default()
                        },
                    }]
                })),
                ..Script::default()
            }),
        ),
        (watcher.id.clone(), both(enemy_play_noter("fd-watch", "watched", false))),
        (listener.id.clone(), both(enemy_play_noter("fd-listen", "heard", false))),
        (ears.id.clone(), both(enemy_play_noter("fd-ears", "heard", true))),
        (
            wrecker.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![
                        destroy_all(json_as(json!({ "side": "self", "rows": ["backrow"] }))),
                        choose_target(json_as(json!({ "step": "after", "scope": { "side": "enemy", "of": ["hero"] } }))),
                    ]
                })),
                resume: IndexMap::from([("after", hook(|_ctx| vec![]))]),
                ..Script::default()
            }),
        ),
        (
            leak.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![lock_random_zone(json_as(json!({ "side": "enemy" })))])),
                ..Script::default()
            }),
        ),
        (
            lockdown.id.clone(),
            both(Script {
                triggers: vec![TriggerDef::new("fd-lockdown", &[GameEventType::CardPlayed], |_ctx, _event| {
                    vec![lock_played_zone(Default::default())]
                })],
                ..Script::default()
            }),
        ),
        (
            doom.id.clone(),
            both(Script {
                triggers: vec![TriggerDef::new("fd-doom", &[GameEventType::CardPlayed], |_ctx, _event| {
                    vec![note("doomed"), lock_own_zone()]
                })
                .with_when(enemy_play)],
                ..Script::default()
            }),
        ),
        (
            eater.id.clone(),
            CardScripts {
                base: Script {
                    cry: Some(hook(|_ctx| vec![lock_lane(Default::default())])),
                    ..Script::default()
                },
                radiant: Script {
                    cry: Some(hook(|_ctx| vec![lock_lane(json_as(json!({ "side": "enemy" })))])),
                    ..Script::default()
                },
            },
        ),
        (
            unlocker.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![unlock_all(Default::default())])),
                ..Script::default()
            }),
        ),
        (
            mourner.id.clone(),
            both(Script {
                triggers: vec![TriggerDef::new("fd-mourn", &[GameEventType::Destroyed], |_ctx, _event| {
                    vec![note("mourned")]
                })],
                ..Script::default()
            }),
        ),
        (
            blink.id.clone(),
            both(Script {
                targets: vec![TargetDecl::target(1, 1, Value::Null)],
                cry: Some(hook(|_ctx| {
                    vec![
                        flicker(json_as(json!({ "target": { "of": "chosen" } }))),
                        choose_target(json_as(json!({ "step": "then", "scope": { "side": "enemy", "of": ["hero"] } }))),
                    ]
                })),
                resume: IndexMap::from([("then", hook(|_ctx| vec![]))]),
                ..Script::default()
            }),
        ),
    ])
});

/// A game with the field fixtures registered beside the engine's own.
pub fn field_game(seed: &str) -> GameState {
    let state = new_game(seed, None);
    let mut defs: CardDefs = registered_catalog().clone();
    for card in FIELD_DEFS.iter() {
        defs.insert(card.id.clone(), card.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(FIELD_SCRIPTS.clone());
    register_scripts(merged);
    state
}

thread_local! {
    /// TS's module `let nonce = 0`.
    static NONCE: Cell<u32> = const { Cell::new(0) };
}

fn next_nonce() -> u32 {
    NONCE.with(|nonce| {
        let next = nonce.get() + 1;
        nonce.set(next);
        next
    })
}

/// `reduce` with the next `fd<n>` nonce. `body` is an `ActionInput` or its JSON.
pub fn act_result(state: &GameState, body: impl serde::Serialize) -> ReduceResult {
    let n = next_nonce();
    let input: ActionInput = json_as(serde_json::to_value(&body).expect("an action body is JSON"));
    reduce(state, &input.with_nonce(format!("fd{n}")))
}

/// `act_result`'s state; panics with the refusal's text.
pub fn act(state: &GameState, body: impl serde::Serialize) -> GameState {
    let result = act_result(state, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

/// Past both mulligans, in p1's main phase of turn 1, with every card of the opening hands kept.
pub fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&field_game(seed)).state;
    let keep: Vec<String> = state.players.p1.hand.iter().map(|card| card.id.clone()).collect();
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p1" }));
    let keep: Vec<String> = state.players.p2.hand.iter().map(|card| card.id.clone()).collect();
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }));
    state
}

/// Gives a player plenty of mana for a test's plays. TS `flush(state, player, mana = 10)`.
pub fn flush(state: &mut GameState, player: PlayerId, mana: i32) {
    state.players[player].mana.current = mana;
    state.players[player].mana.max = mana;
}
