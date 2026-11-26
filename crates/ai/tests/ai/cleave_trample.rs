//! R4805000, #4805: the AI's Taunt walk (`damage_past_taunts_with`, so `face_threat` and `ready_gap`) lands a
//! Cleave splash on every unit it strikes (§4.4 step 10): a unit without Taunt, and a Taunt the walk has
//! passed, are met as its earlier hits and splashes left them, gone once destroyed or back as their Reborn
//! body (§4.5 step 4). Each splash spends the defender's damage pools in the engine's order (R4064000), and a
//! Trample attacker's sends what it deals beyond the unit's health to the hero (step 9, R63). Each board is
//! played out through `reduce` in the order the walk reads it, and the hero damage the engine deals is the
//! read. `GREEDY_EVAL` reads every board as before.
//!
//! The cards are the testkit's `stand-in` bodies, so a balance patch cannot move them, Mr. Vanilla (Core #8)
//! to draw, and Force Projector (S+ #42), face up in the AI's backrow: the first 5 damage its side takes each
//! turn is nullified.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, register_cards, unit};

/// The walk as it read before #4805.
const OFF: EvalWeights = EvalWeights {
    splash_trample_read: false,
    ..AI_EVAL
};

/// Mr. Vanilla, 4/4: the cards each side draws.
const VANILLA: &str = "core-008";
/// The issue's attacker: a 5/4 with Cleave and Trample.
const CLEAVER: &str = "stand-in (1) 5/4 Cleave, Trample";
/// A second such attacker, a card of its own so each is found by its id.
const SECOND_CLEAVER: &str = "stand-in (2) 5/4 Cleave, Trample";
/// Force Projector: the first 5 damage the AI's side takes each turn is nullified.
const PROJECTOR: &str = "stinkyscapeplus-042";

/// p1 (the AI, on turn at turn 9) as `p1` says, against p2's (the human's) `p2_field`, each with plain cards
/// to draw. Neither turn ends by itself (R82), so a play-out stays on the human's turn.
fn board(seed: &str, p1: Value, p2_field: Value) -> GameState {
    register_cards();
    let mut p1 = p1;
    p1["library"] = json!([VANILLA, VANILLA]);
    let mut state = scenario(json!({
        "seed": seed,
        "active": "p1",
        "turn": 9,
        "p1": p1,
        "p2": { "library": [VANILLA, VANILLA], "field": p2_field },
    }))
    .state()
    .clone();
    for player in [AI, HUMAN] {
        state.players[player].auto_end_turn = Some(false);
    }
    state
}

/// The AI passes, then each of the human's `attackers`, in turn, attacks the AI's `taunt`: the state after
/// the pass, and after the attacks.
fn play_out(state: &GameState, attackers: &[&str], taunt: &str) -> (GameState, GameState) {
    let passed = act(state, AI, ActionBody::EndTurn);
    let target_id = unit(&passed, AI, taunt).id.clone();
    let mut after = passed.clone();
    for def in attackers {
        let attacker_id = unit(&after, HUMAN, def).id.clone();
        after = act(
            &after,
            HUMAN,
            ActionBody::Attack {
                attacker_id,
                target_id: target_id.clone(),
            },
        );
    }
    (passed, after)
}

/// What the play-out dealt the AI's hero, on the human's turn.
fn dealt(passed: &GameState, after: &GameState) -> i32 {
    assert_eq!(
        (after.active, after.turn),
        (HUMAN, passed.turn),
        "the play-out stays on the human's turn"
    );
    passed.players[AI].hero.health - after.players[AI].hero.health
}

/// The 0/9 Taunt of boards 1 and 3.
const WALL: &str = "stand-in (1) 0/9 Taunt";

/// Board 1, the issue's: the AI at 8, a 0/9 Taunt between two 1/1s, against the Cleave Trample 5/4.
fn between_two() -> GameState {
    board(
        "ai-cleave-trample-between-two",
        json!({ "health": 8, "field": [
            { "def": "stand-in (1) 1/1", "lane": 2 },
            { "def": WALL, "lane": 3 },
            { "def": "stand-in (1) 1/1", "lane": 4 },
        ] }),
        json!([{ "def": CLEAVER, "lane": 3 }]),
    )
}

/// The 0/20 Taunt of board 2.
const BIG_WALL: &str = "stand-in (1) 0/20 Taunt";
/// The Reborn 2/3 of board 2.
const REBORN: &str = "stand-in (1) 2/3 Reborn";

/// Board 2: the AI at 20, a 0/20 Taunt between a 1/1 and a 2/3 Reborn, against two Cleave Trample 5/4s.
fn reborn_beside() -> GameState {
    board(
        "ai-cleave-trample-reborn-beside",
        json!({ "health": 20, "field": [
            { "def": "stand-in (1) 1/1", "lane": 2 },
            { "def": BIG_WALL, "lane": 3 },
            { "def": REBORN, "lane": 4 },
        ] }),
        json!([{ "def": CLEAVER, "lane": 2 }, { "def": SECOND_CLEAVER, "lane": 3 }]),
    )
}

/// The Flying Taunt of board 3.
const FLYING_WALL: &str = "stand-in (1) 0/2 Flying, Taunt";

/// Board 3: the AI at 20, a 0/2 Flying Taunt beside the 0/9 Taunt, against the Cleave Trample 5/4.
fn flying_beside() -> GameState {
    board(
        "ai-cleave-trample-flying-beside",
        json!({ "health": 20, "field": [
            { "def": FLYING_WALL, "lane": 2, "position": "ATK" },
            { "def": WALL, "lane": 3 },
        ] }),
        json!([{ "def": CLEAVER, "lane": 3 }]),
    )
}

/// The 0/3 Taunt of board 4.
const SMALL_WALL: &str = "stand-in (1) 0/3 Taunt";
/// The Cleave 2/4 of board 4.
const SMALL_CLEAVER: &str = "stand-in (1) 2/4 Cleave";
/// The Trample 6/6 of board 4.
const TRAMPLER: &str = "stand-in (1) 6/6 Trample";

/// Board 4: the AI at 20 behind its Force Projector, a 0/3 Taunt beside a 1/1, against a Cleave 2/4 and a
/// Trample 6/6.
fn pooled() -> GameState {
    board(
        "ai-cleave-trample-pooled",
        json!({
            "health": 20,
            "backrow": [{ "def": PROJECTOR, "faceUp": true }],
            "field": [
                { "def": "stand-in (1) 1/1", "lane": 2 },
                { "def": SMALL_WALL, "lane": 3 },
            ],
        }),
        json!([{ "def": SMALL_CLEAVER, "lane": 1 }, { "def": TRAMPLER, "lane": 2 }]),
    )
}

/// R4805000: the 5/4's hit on the 0/9 deals it 5, and each splash deals a 1/1 1 and tramples 4 onto the AI's
/// hero: 8, the AI's whole life, where the walk read 0. The engine ends the game with the one attack.
#[test]
fn r4805000_a_splash_tramples_over_a_neighbour_without_taunt() {
    let state = between_two();
    assert_eq!(face_threat(&state, HUMAN), 8);
    assert_eq!(face_threat_with(&state, HUMAN, &OFF), 0);

    let (passed, after) = play_out(&state, &[CLEAVER], WALL);
    assert_eq!(
        after.result.as_ref().and_then(|result| result.winner.player()),
        Some(HUMAN),
        "the attack is lethal in the engine"
    );
    assert_eq!(ready_gap(&passed, HUMAN), 0.0);
}

/// R4805000: the first 5/4's splashes trample 4 over the 1/1 and 2 over the 2/3 Reborn, which comes back at 1
/// health; the second's finds lane 2 empty and tramples 4 over the Reborn body: 10, where the walk read 0. A
/// splash read fresh from the state at each swing would read 12.
#[test]
fn r4805000_a_later_splash_meets_what_an_earlier_one_left() {
    let state = reborn_beside();
    assert_eq!(face_threat(&state, HUMAN), 10);
    assert_eq!(face_threat_with(&state, HUMAN, &OFF), 0);

    let (passed, first) = play_out(&state, &[CLEAVER], BIG_WALL);
    assert_eq!(dealt(&passed, &first), 6, "4 over the 1/1 and 2 over the 2/3");
    let body = unit(&first, AI, REBORN);
    assert_eq!(
        body.reborn_spent,
        Some(true),
        "the 2/3 is back as its Reborn body"
    );
    assert_eq!(unit_view(&first, body).health, 1);

    let (passed, after) = play_out(&state, &[CLEAVER, SECOND_CLEAVER], BIG_WALL);
    assert_eq!(active_units_of(&after, AI).len(), 1, "only the 0/20 still stands");
    assert_eq!(dealt(&passed, &after), 10);
    assert_eq!(ready_gap(&passed, HUMAN), 10.0);
}

/// R4805000: the 0/2 Flying Taunt binds no ground attacker (R783002), so the walk passes it first; the 5/4's
/// splash from the 0/9 then deals it 2 and tramples 3: 3, where the walk read 0.
#[test]
fn r4805000_a_splash_tramples_over_a_taunt_the_walk_has_passed() {
    let state = flying_beside();
    assert_eq!(face_threat(&state, HUMAN), 3);
    assert_eq!(face_threat_with(&state, HUMAN, &OFF), 0);

    let (passed, after) = play_out(&state, &[CLEAVER], WALL);
    assert_eq!(dealt(&passed, &after), 3);
    assert!(
        active_units_of(&after, AI)
            .iter()
            .all(|unit| unit.def_id != FLYING_WALL),
        "the splash destroys the 0/2"
    );
    assert_eq!(ready_gap(&passed, HUMAN), 17.0);
}

/// R4805000: the Force Projector's 5 soaks the 2/4's 2 on the 0/3 and its splash's 2 on the 1/1, leaving 1;
/// the 6/6 then deals the 0/3 5 and tramples 2 onto the hero: 2, where the walk, splashing nothing on the
/// 1/1, left 3 of the pool for the 6/6 and read 0.
#[test]
fn r4805000_a_splash_on_a_unit_without_taunt_spends_the_pool_first() {
    let state = pooled();
    assert_eq!(face_threat(&state, HUMAN), 2);
    assert_eq!(face_threat_with(&state, HUMAN, &OFF), 0);

    let (passed, after) = play_out(&state, &[SMALL_CLEAVER, TRAMPLER], SMALL_WALL);
    assert_eq!(dealt(&passed, &after), 2);
    assert_eq!(ready_gap(&passed, HUMAN), 18.0);
}

#[test]
fn r4805000_greedy_eval_reads_every_board_as_before() {
    const { assert!(AI_EVAL.splash_trample_read) };
    const { assert!(!GREEDY_EVAL.splash_trample_read) };
    let boards: [(GameState, i32); 4] = [
        (between_two(), 0),
        (reborn_beside(), 0),
        (flying_beside(), 0),
        (pooled(), 5),
    ];
    for (index, (state, read)) in boards.iter().enumerate() {
        assert_eq!(
            face_threat_with(state, HUMAN, &GREEDY_EVAL),
            *read,
            "board {}",
            index + 1
        );
    }
}
