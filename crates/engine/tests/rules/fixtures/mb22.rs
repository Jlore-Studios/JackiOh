//! Fixture cards for Meditative batch 22 (issue #538): Mayor Medinamogger's random targets
//! (ME-RANDOMTARGETS, R1200, R1201), Windfast's and Windfurious Prime's attack summons
//! (ME-ATTACKSUMMON, R1202, R1203) and Unan's lethal guard (ME-LETHALGUARD, R1204). The engine
//! never imports `crates/cards`, so each system is proved through a card shaped like the one that
//! will use it. Ids are `mb22-…`, indexed from 9200, so they collide with no other test file.

#![allow(non_upper_case_globals)]

use std::cell::Cell;
use std::sync::LazyLock;

use jackioh_engine::effects::{damage, destroy};
use jackioh_engine::testkit::*;

use super::damage_combat::note;
use super::harness::new_game;

/// TS `def(name, type, extra = {})`: a Core Common at cost 0 whose faces print `name`, `extra` over it.
fn def(name: &str, index: i32, type_: &str, extra: Value) -> CardDef {
    let mut card = json!({
        "id": format!("mb22-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (meditative batch 22)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
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

/// TS `unit(name, attack, health, keywords = [], radiantKeywords = keywords)`: the Radiant face
/// doubles the stats. Pass `json!([])` for no keywords and the same list twice for TS's default.
fn unit(
    name: &str,
    index: i32,
    attack: i32,
    health: i32,
    keywords: Value,
    radiant_keywords: Value,
) -> CardDef {
    def(
        name,
        index,
        "Unit",
        json!({
            "base": { "attack": attack, "health": health, "keywords": keywords, "text": name },
            "radiant": { "attack": attack * 2, "health": health * 2, "keywords": radiant_keywords, "text": name },
        }),
    )
}

fn plain_unit(name: &str, index: i32, attack: i32, health: i32) -> CardDef {
    unit(name, index, attack, health, json!([]), json!([]))
}

/// TS `both(script, radiant = script)`.
fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn both_of(base: Script, radiant: Script) -> CardScripts {
    CardScripts { base, radiant }
}

/// A replacement as TS's literal `{ id, on, instead }`, with no `where`, `when`, `then` or `by`.
fn replacement(id: &str, on: ReplacementMoment, instead: Value) -> ReplacementDef {
    ReplacementDef {
        id: id.to_string(),
        on,
        where_: None,
        when: None,
        instead: json_as(instead),
        then: None,
        by: None,
    }
}

/// Any unit or hero, either side.
fn any_target() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "of": ["unit", "hero"] }))]
}

// ---------------------------------------------------------------------------
// ME-RANDOMTARGETS (R1200, R1201)
// ---------------------------------------------------------------------------

/// Meditative #86's shape: while this acts, every declared target, `target` prompt and attack
/// target is drawn at random. Radiant: Lucky 1.
pub static mayor: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "mayor",
        9200,
        2,
        2,
        json!([]),
        json!([{ "kind": "Lucky", "n": 1 }]),
    )
});

/// A Spell: deal 10 damage to a declared target.
pub static bolt: LazyLock<CardDef> = LazyLock::new(|| def("bolt", 9201, "Spell", json!({})));

/// A Spell: deal 30 damage to a declared hero.
pub static smite: LazyLock<CardDef> = LazyLock::new(|| def("smite", 9202, "Spell", json!({})));

/// A Unit whose Cry asks its controller a `target` question (one "nothing" answer) and notes it.
pub static asker: LazyLock<CardDef> = LazyLock::new(|| plain_unit("asker", 9203, 1, 1));

/// A Spell with a declared target, a hand pick and two modes: hand picks and modes stay choices
/// while a Mayor acts.
pub static tricky: LazyLock<CardDef> = LazyLock::new(|| def("tricky", 9204, "Spell", json!({})));

// ---------------------------------------------------------------------------
// ME-ATTACKSUMMON (R1202, R1203)
// ---------------------------------------------------------------------------

/// Meditative #91's shape: 1/1 Windfury; a Unit from hand makes each attack instead, bounced after.
pub static windfast: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "windfast",
        9205,
        1,
        1,
        json!([{ "kind": "Windfury" }]),
        json!([{ "kind": "Windfury" }]),
    )
});

/// Meditative #91.1's shape: 5/10; `joiners` Units from hand and deck attack first.
pub static prime: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "prime",
        9206,
        "Unit",
        json!({
            "params": [{ "key": "joiners", "base": 2, "radiant": 2, "better": "up", "step": 1, "min": 1 }],
            "base": { "attack": 5, "health": 10, "keywords": [], "text": "prime" },
            "radiant": { "attack": 10, "health": 20, "keywords": [], "text": "prime" },
        }),
    )
});

/// A Trap that destroys the declared attack's target inside the window, so it has left the field
/// before its combat.
pub static snapper: LazyLock<CardDef> = LazyLock::new(|| def("snapper", 9207, "Trap", json!({})));

// ---------------------------------------------------------------------------
// ME-LETHALGUARD (R1204)
// ---------------------------------------------------------------------------

/// Meditative #92's shape: 1/5 Armor 3; a lethal hit on a friendly ally goes to this instead.
pub static guard: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "guard",
        9208,
        1,
        5,
        json!([{ "kind": "Armor", "n": 3 }]),
        json!([{ "kind": "Armor", "n": 3 }]),
    )
});

/// A second guard, so a hit that would kill the first goes to this instead.
pub static guard2: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "guard2",
        9209,
        1,
        5,
        json!([{ "kind": "Armor", "n": 3 }]),
        json!([{ "kind": "Armor", "n": 3 }]),
    )
});

/// A plain 1/4 ally to guard.
pub static ally: LazyLock<CardDef> = LazyLock::new(|| plain_unit("ally", 9210, 1, 4));
/// A plain 1/20 ally no 10-damage hit kills.
pub static wall: LazyLock<CardDef> = LazyLock::new(|| plain_unit("wall", 9211, 1, 20));
/// A 2/2 Divine Shield: the shield takes the hit first, so no window opens.
pub static screen: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "screen",
        9212,
        2,
        2,
        json!([{ "kind": "Divine Shield" }]),
        json!([{ "kind": "Divine Shield" }]),
    )
});
/// A 2/2 Indestructible: it takes nothing, so no window opens.
pub static bastion: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "bastion",
        9213,
        2,
        2,
        json!([{ "kind": "Indestructible" }]),
        json!([{ "kind": "Indestructible" }]),
    )
});
/// A plain 3/2 attacker.
pub static skirmisher: LazyLock<CardDef> = LazyLock::new(|| plain_unit("skirmisher", 9214, 3, 2));
/// A 2/6 Taunt: while a Mayor acts every drawn attack target is this.
pub static palisade: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "palisade",
        9215,
        2,
        6,
        json!([{ "kind": "Taunt" }]),
        json!([{ "kind": "Taunt" }]),
    )
});
/// A 10/10 Trample: its excess reaches the hero through whoever caught the hit.
pub static ram: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "ram",
        9216,
        10,
        10,
        json!([{ "kind": "Trample" }]),
        json!([{ "kind": "Trample" }]),
    )
});

pub static MB22_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        mayor.clone(),
        bolt.clone(),
        smite.clone(),
        asker.clone(),
        tricky.clone(),
        windfast.clone(),
        prime.clone(),
        snapper.clone(),
        guard.clone(),
        guard2.clone(),
        ally.clone(),
        wall.clone(),
        screen.clone(),
        bastion.clone(),
        skirmisher.clone(),
        palisade.clone(),
        ram.clone(),
    ]
});

/// A `target` prompt with one "nothing" answer: a prompt answered at once still runs the Cry to
/// its end, so the asker enters play with no prompt ever opening.
fn ask_then_note() -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let resume = resume_self(&*ctx, "asked", Default::default());
            let player = ctx.controller;
            vec![Effect::new("mb22:ask", move |ctx| {
                let resume = resume.clone();
                open_prompt(
                    ctx,
                    json_as(json!({
                        "player": player,
                        "kind": "target",
                        "prompt": "the card asks its controller",
                        "options": [{ "key": "none", "label": "nothing", "selection": { "pick": "none" } }],
                        "resume": resume,
                    })),
                );
            })]
        })),
        resume: IndexMap::from([(
            "asked",
            hook(|_| vec![Effect::new("mb22:asked", |_| {})]),
        )]),
        ..Script::default()
    }
}

pub static MB22_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            mayor.id.clone(),
            both_of(
                Script {
                    static_flags: Some(StaticFlags {
                        random_targets: Some(true),
                        ..Default::default()
                    }),
                    ..Script::default()
                },
                Script {
                    static_flags: Some(StaticFlags {
                        random_targets: Some(true),
                        ..Default::default()
                    }),
                    ..Script::default()
                },
            ),
        ),
        (
            bolt.id.clone(),
            both(Script {
                targets: any_target(),
                cry: Some(hook(|_| {
                    vec![damage(
                        json_as(json!({ "to": { "of": "chosen" }, "amount": 10 })),
                    )]
                })),
                ..Script::default()
            }),
        ),
        (
            smite.id.clone(),
            both(Script {
                targets: vec![TargetDecl::target(1, 1, json!({ "of": ["hero"] }))],
                cry: Some(hook(|_| {
                    vec![damage(
                        json_as(json!({ "to": { "of": "chosen" }, "amount": 30 })),
                    )]
                })),
                ..Script::default()
            }),
        ),
        (asker.id.clone(), both(ask_then_note())),
        (
            tricky.id.clone(),
            both(Script {
                targets: vec![
                    TargetDecl::target(1, 1, json!({ "of": ["unit", "hero"] })),
                    TargetDecl::hand(1, 1, json!({ "of": ["hand"] })),
                ],
                modes: vec![ModeDecl {
                    kind: PromptKind::Mode,
                    options: vec!["red".to_string(), "blue".to_string()],
                }],
                cry: Some(hook(|_| vec![note("tricky:ran")])),
                ..Script::default()
            }),
        ),
        (
            windfast.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    attack_from_hand: Some(true),
                    bounce_attacker: Some(true),
                    ..Default::default()
                }),
                ..Script::default()
            }),
        ),
        (
            prime.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    attack_joiners: Some(true),
                    ..Default::default()
                }),
                ..Script::default()
            }),
        ),
        (
            snapper.id.clone(),
            both(Script {
                triggers: vec![
                    TriggerDef::new(
                        "snap",
                        &[GameEventType::AttackDeclared],
                        |_ctx, event| match event {
                            GameEvent::AttackDeclared { target_id, .. } => {
                                vec![destroy(json_as(json!({
                                    "target": { "of": "instance", "instanceId": target_id },
                                })))]
                            }
                            _ => vec![],
                        },
                    )
                    .with_when(|_ctx, event| {
                        matches!(event, GameEvent::AttackDeclared { forced: false, .. })
                    }),
                ],
                ..Script::default()
            }),
        ),
        (
            guard.id.clone(),
            both(Script {
                replacements: vec![replacement(
                    "guard",
                    ReplacementMoment::LethalHit,
                    json!({ "redirect": "self" }),
                )],
                ..Script::default()
            }),
        ),
        (
            guard2.id.clone(),
            both(Script {
                replacements: vec![replacement(
                    "guard2",
                    ReplacementMoment::LethalHit,
                    json!({ "redirect": "self" }),
                )],
                ..Script::default()
            }),
        ),
    ])
});

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

pub fn register() {
    let mut defs: CardDefs = registered_catalog().clone();
    for card in MB22_DEFS.iter() {
        defs.insert(card.id.clone(), card.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(MB22_SCRIPTS.clone());
    register_scripts(merged);
}

thread_local! {
    /// TS's module `let nonce = 0`, shared by `act_result` and every `recorder`.
    static NONCE: Cell<u32> = const { Cell::new(0) };
}

fn next_nonce() -> u32 {
    NONCE.with(|nonce| {
        let next = nonce.get() + 1;
        nonce.set(next);
        next
    })
}

fn action_of(body: impl serde::Serialize, nonce: String) -> Action {
    let input: ActionInput = json_as(serde_json::to_value(&body).expect("an action body is JSON"));
    input.with_nonce(nonce)
}

/// `reduce` with the next `mb<n>` nonce. `body` is an `ActionInput` or its JSON.
pub fn act_result(state: &GameState, body: impl serde::Serialize) -> ReduceResult {
    let n = next_nonce();
    reduce(state, &action_of(body, format!("mb{n}")))
}

/// `act_result`'s state; panics with the refusal's text.
pub fn act(state: &GameState, body: impl serde::Serialize) -> GameState {
    let result = act_result(state, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

/// A game past both mulligans, in p1's first main phase.
pub fn playing(seed: &str) -> GameState {
    let created = new_game(seed, None);
    register();
    let mut state = begin_game(&created).state;
    let keep: Vec<String> = state.players.p1.hand.iter().map(|card| card.id.clone()).collect();
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": keep, "playerId": "p1" }),
    );
    let keep: Vec<String> = state.players.p2.hand.iter().map(|card| card.id.clone()).collect();
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }),
    );
    state
}

/// Answer the one open prompt with these selections, whoever it belongs to.
pub fn answer_with(state: &GameState, selection: Value) -> ReduceResult {
    let Some(pending) = state.pending.as_ref() else {
        panic!("expected a prompt to be open");
    };
    let result = act_result(
        state,
        json!({
            "type": "answer",
            "choiceId": pending.id,
            "selection": selection,
            "playerId": pending.player_id,
        }),
    );
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

/// §9.3: the live game and its replay agree. `start` is the state the log was played from; the log is
/// folded again from a clone of it, action by action through `reduce`, and the two end states hash
/// the same (`replay.hashState`).
pub fn replays_to(start: &GameState, log: &[Action], live: &GameState) -> bool {
    let mut state = clone_state(start);
    for action in log {
        let result = reduce(&state, action);
        if let Some(error) = &result.error {
            panic!("replay rejected {}: {error}", action.action_type());
        }
        state = result.state;
    }
    hash_state(&state) == hash_state(live)
}

/// A recorder: every action it plays goes into its log, so a test can replay what it played.
#[derive(Clone, Debug)]
pub struct Recorder {
    current: GameState,
    pub log: Vec<Action>,
    pub start: GameState,
}

impl Recorder {
    /// The state after every action played so far.
    pub fn state(&self) -> &GameState {
        &self.current
    }

    /// Plays one action with the next `mbr<n>` nonce and records it; panics with a refusal's text.
    pub fn play(&mut self, body: impl serde::Serialize) -> ReduceResult {
        let n = next_nonce();
        let action = action_of(body, format!("mbr{n}"));
        let result = reduce(&self.current, &action);
        if let Some(error) = &result.error {
            panic!("{error}");
        }
        self.log.push(action);
        self.current = result.state.clone();
        result
    }
}

pub fn recorder(start: &GameState) -> Recorder {
    Recorder {
        current: start.clone(),
        log: Vec::new(),
        start: clone_state(start),
    }
}
