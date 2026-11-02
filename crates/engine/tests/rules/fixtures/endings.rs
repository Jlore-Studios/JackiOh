//! Test-only cards for issue #1843's two endings: a vote to remove a player (`vote_out`, R1843000,
//! R1843001) and a flight from the game (`flee`, R1843002, R1843003), each through the engine's own
//! verbs, as a card script would, since the engine never imports `crates/cards`. The Stinkyscape cards
//! using them (#18.1 MELCHIOR, #18.2 BALTHASAR, #18.3 CASPER, #148.1 Edge of the Universe) sit behind
//! the set's gate (R1420) and test the same cases again. Ids are prefixed `en-` and indexed from 18430,
//! so they collide with no other fixture file's.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects;
// The verbs as a card script names them, through the prelude (CLAUDE.md rule 5).
use jackioh_engine::prelude::{FleeArgs, PlayerSpec, choose_mode, flee, lose_health, vote_out};
use jackioh_engine::testkit::*;

use super::harness::both;

/// The health loss that kills a hero outright, whatever it stands at (losing health is not damage, R18).
pub const LETHAL: i32 = 100 * HERO_HEALTH;

/// Edge of the Universe's Radiant choice: the options of its `mode` prompt and the step it resumes at.
pub const LEAVE_A_DRAW: &str = "draw";
pub const LEAVE_A_LOSS: &str = "loss";
pub const FLEE_STEP: &str = "flee";

fn card(index: u32, name: &str, type_: &str, cost: i32) -> CardDef {
    json_as(json!({
        "id": format!("en-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (endings fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    }))
}

/// MELCHIOR's shape: "Start of Turn: votes to remove you if the enemy has more Units than you".
pub static melchior: LazyLock<CardDef> = LazyLock::new(|| card(18430, "melchior", "Field Spell", 1));
/// BALTHASAR's shape: "Start of Turn: votes to remove you if you have less than half your starting life".
pub static balthasar: LazyLock<CardDef> = LazyLock::new(|| card(18431, "balthasar", "Field Spell", 2));
/// CASPER's shape, its condition left to the card part: "Start of Turn: votes to remove you".
pub static casper: LazyLock<CardDef> = LazyLock::new(|| card(18432, "casper", "Field Spell", 3));
/// A face-down voter: a Trap whose start-of-turn hook votes to remove its controller.
pub static masked: LazyLock<CardDef> = LazyLock::new(|| card(18433, "masked", "Trap", 1));
/// A Field Spell whose start-of-turn hook loses its controller the game outright, before any Brain to
/// its right votes (R68's lane order).
pub static doom: LazyLock<CardDef> = LazyLock::new(|| card(18434, "doom", "Field Spell", 0));
/// Edge of the Universe's shape, a (0) Spell: "Escape the Universe and Win the Game", a draw for your
/// opponent; on the Radiant face you choose a draw or a loss for them (a `mode` prompt).
pub static escape: LazyLock<CardDef> = LazyLock::new(|| card(18435, "escape", "Spell", 0));
/// A (0) Spell that loses its caster the game and then flees: the flight meets a hero at 0 (R850).
pub static doomed_escape: LazyLock<CardDef> = LazyLock::new(|| card(18436, "doomed-escape", "Spell", 0));
/// A (0) Spell that kills the enemy hero and then flees: the hero check still comes first (R850).
pub static parting_shot: LazyLock<CardDef> = LazyLock::new(|| card(18437, "parting-shot", "Spell", 0));
/// A (0) Spell that wins outright (`win_game`) and then flees: the win beats the flight (R1843002).
pub static summit_escape: LazyLock<CardDef> = LazyLock::new(|| card(18438, "summit-escape", "Spell", 0));
/// A (0) Spell that makes both players flee at once.
pub static exodus: LazyLock<CardDef> = LazyLock::new(|| card(18439, "exodus", "Spell", 0));

fn vote_if(holds: bool) -> Vec<Effect> {
    if holds {
        vec![vote_out(Default::default())]
    } else {
        Vec::new()
    }
}

fn units(state: &GameState, player: PlayerId) -> usize {
    active_units_of(state, player).len()
}

fn flee_leaving(opponent: FleeOpponent) -> Effect {
    flee(FleeArgs {
        player: None,
        opponent: Some(opponent),
    })
}

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let lethal_to = |player: &str| lose_health(json_as(json!({ "player": player, "amount": LETHAL })));
    let mut escape_radiant = Script {
        // The fleer chooses as it resolves: a draw or a loss for the other player (R1843003).
        cry: Some(hook(|_ctx| {
            vec![choose_mode(json_as(json!({
                "options": [LEAVE_A_DRAW, LEAVE_A_LOSS],
                "step": FLEE_STEP,
                "prompt": "Leave your opponent",
                "labels": { "draw": "A draw", "loss": "A loss" },
            })))]
        })),
        ..Script::default()
    };
    escape_radiant.resume.insert(
        FLEE_STEP,
        hook(|ctx| {
            let loss = effects::chosen_options(ctx).first().map(String::as_str) == Some(LEAVE_A_LOSS);
            vec![flee_leaving(if loss {
                FleeOpponent::Loss
            } else {
                FleeOpponent::Draw
            })]
        }),
    );
    IndexMap::from([
        (
            melchior.id.clone(),
            both(Script {
                start_of_turn: Some(hook(|ctx| {
                    let me = ctx.controller;
                    let state: &GameState = &*ctx.state;
                    vote_if(units(state, opponent_of(me)) > units(state, me))
                })),
                ..Script::default()
            }),
        ),
        (
            balthasar.id.clone(),
            both(Script {
                start_of_turn: Some(hook(|ctx| {
                    let state: &GameState = &*ctx.state;
                    vote_if(2 * hero_of(state, ctx.controller).health < HERO_HEALTH)
                })),
                ..Script::default()
            }),
        ),
        (
            casper.id.clone(),
            both(Script {
                start_of_turn: Some(hook(|_ctx| vote_if(true))),
                ..Script::default()
            }),
        ),
        (
            masked.id.clone(),
            both(Script {
                start_of_turn: Some(hook(|_ctx| vote_if(true))),
                ..Script::default()
            }),
        ),
        (
            doom.id.clone(),
            both(Script {
                start_of_turn: Some(hook(move |_ctx| vec![lethal_to("self")])),
                ..Script::default()
            }),
        ),
        (
            escape.id.clone(),
            CardScripts {
                base: Script {
                    cry: Some(hook(|_ctx| vec![flee(Default::default())])),
                    ..Script::default()
                },
                radiant: escape_radiant,
            },
        ),
        (
            doomed_escape.id.clone(),
            both(Script {
                cry: Some(hook(move |_ctx| {
                    vec![lethal_to("self"), flee(Default::default())]
                })),
                ..Script::default()
            }),
        ),
        (
            parting_shot.id.clone(),
            both(Script {
                cry: Some(hook(move |_ctx| {
                    vec![lethal_to("enemy"), flee(Default::default())]
                })),
                ..Script::default()
            }),
        ),
        (
            summit_escape.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::win_game(json_as(json!({}))), flee(Default::default())]
                })),
                ..Script::default()
            }),
        ),
        (
            exodus.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![
                        flee(Default::default()),
                        flee(FleeArgs {
                            player: Some(PlayerSpec::Enemy),
                            opponent: None,
                        }),
                    ]
                })),
                ..Script::default()
            }),
        ),
    ])
});

static ENDINGS_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        melchior.clone(),
        balthasar.clone(),
        casper.clone(),
        masked.clone(),
        doom.clone(),
        escape.clone(),
        doomed_escape.clone(),
        parting_shot.clone(),
        summit_escape.clone(),
        exodus.clone(),
    ]
});

pub fn register_endings_fixtures() {
    super::harness::setup_catalog();
    let mut defs: CardDefs = registered_catalog().clone();
    for def in ENDINGS_DEFS.iter() {
        defs.insert(def.id.clone(), def.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}
