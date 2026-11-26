//! R4798000, #4798: the body a Reborn unit comes back as, in the AI's evaluation (`reborn_return`, so
//! `kill_worth`, the Taunt walk and `afflicted_before_swing`), is the engine's own (`reborn_body_of`): a
//! unit token summoned X/X keeps its X/X (R175), and a stolen unit comes back on the side it died on, its
//! keeper's card (R2618000), under that side's auras. Each board is destroyed through the sandbox's
//! destroy, the engine's own Reborn return, and the body it stands up is the read. `GREEDY_EVAL` reads
//! every body as before.
//!
//! The cards: a Bread Token (Core T-bread), printed 0/0; Group leader (TG #3), "Aura: Your other Units
//! have +1 Attack"; and the testkit's `stand-in` bodies, so a balance patch cannot move them.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, close, register_cards, unit};

/// The body as it read before #4798.
const OFF: EvalWeights = EvalWeights {
    reborn_body_read: false,
    ..AI_EVAL
};

/// The Bread Token, a unit token printed 0/0 whose X/X is its summoned face.
const BREAD: &str = "core-t-bread";
/// Group leader: "Aura: Your other Units have +1 Attack."
const GROUP_LEADER: &str = "traininggrounds-003";
/// A Unit printing Reborn.
const REBORN: &str = "stand-in (4) 5/5 Reborn";
/// What the AI's side holds, so it has a board.
const VANILLA: &str = "stand-in (1) 4/4";

/// The AI's turn with `human` the human's field.
fn board(seed: &str, human: Value) -> GameState {
    register_cards();
    scenario(json!({
        "seed": seed,
        "active": AI,
        "p1": { "field": [VANILLA], "hand": [VANILLA], "library": [VANILLA, VANILLA] },
        "p2": { "field": human, "hand": [VANILLA], "library": [VANILLA, VANILLA] },
    }))
    .state()
    .clone()
}

/// `state` after the sandbox destroys `id` (R923000) and its Reborn brings it back, and the body the
/// engine stood up in its place.
fn destroyed(state: &GameState, id: &str) -> (GameState, CardInstance) {
    let mut sandbox = state.clone();
    sandbox.sandbox = Some(true);
    for player in PLAYER_IDS {
        sandbox.players[player].auto_end_turn = Some(false);
    }
    let destroy = ActionBody::Sandbox {
        op: SandboxOp::Destroy {
            instance_id: id.to_string(),
        },
    };
    let after = simulate(
        &sandbox,
        AI,
        &destroy,
        &create_node_counter(AI_BUDGET.nodes, None),
        Prices::NONE,
    )
    .expect("a node")
    .expect("the destroy applies");
    let body = find_instance(&after, id)
        .filter(|card| card.zone.z() == ZoneName::Field)
        .expect("Reborn brings the unit back")
        .clone();
    (after, body)
}

/// What the AI reads a kill of `unit` to remove, by `w`: `kill_worth`.
fn kill(state: &GameState, unit: &CardInstance, w: &EvalWeights) -> f64 {
    kill_worth(state, unit, w, Prices::NONE)
}

/// What the engine's own body is worth on `state`, the board the AI priced the kill on.
fn body_worth(state: &GameState, body: &CardInstance) -> f64 {
    unit_worth(state, body, &AI_EVAL, Prices::NONE)
}

#[test]
fn r4798000_an_xx_tokens_body_keeps_its_summoned_xx() {
    let mut state = board(
        "ai-r4798000-token",
        json!([{ "def": BREAD, "statsOverride": { "attack": 6, "health": 6 } }]),
    );
    let id = unit(&state, HUMAN, BREAD).id.clone();
    find_instance_mut(&mut state, &id)
        .expect("the token")
        .granted_keywords
        .push(Keyword::Reborn);
    let token = unit(&state, HUMAN, BREAD).clone();
    let (after, body) = destroyed(&state, &id);
    let view = unit_view(&after, &body);
    assert_eq!(
        (view.attack, view.health),
        (6, 1),
        "the engine's body is the token's 6/6 at 1 health (R175)"
    );
    // The AI's body is the engine's: the kill removes the 6/6 less a 6/1.
    close(
        kill(&state, &token, &AI_EVAL),
        unit_worth(&state, &token, &AI_EVAL, Prices::NONE) - body_worth(&state, &body),
    );
    assert!(
        kill(&state, &token, &OFF) > kill(&state, &token, &AI_EVAL),
        "read off the printed 0/0, the old body came back as nothing and the kill removed the whole 6/6"
    );
}

#[test]
fn r4798000_a_stolen_units_body_comes_back_to_the_thief_under_its_aura() {
    let mut state = board("ai-r4798000-stolen", json!([REBORN, GROUP_LEADER]));
    let id = unit(&state, HUMAN, REBORN).id.clone();
    // The human stole the AI's unit: the AI owns it, the human controls it.
    find_instance_mut(&mut state, &id).expect("the unit").owner = AI;
    let stolen = unit(&state, HUMAN, REBORN).clone();
    assert_eq!(
        unit_view(&state, &stolen).attack,
        6,
        "5 printed, +1 from Group leader"
    );
    let (after, body) = destroyed(&state, &id);
    assert_eq!(
        (body.controller, body.owner),
        (HUMAN, HUMAN),
        "it comes back as its keeper's card, on the side it died on (R2618000)"
    );
    let view = unit_view(&after, &body);
    assert_eq!(
        (view.attack, view.health),
        (6, 1),
        "the thief's Group leader gives the body its +1"
    );
    close(
        kill(&state, &stolen, &AI_EVAL),
        unit_worth(&state, &stolen, &AI_EVAL, Prices::NONE) - body_worth(&state, &body),
    );
    let mine = reborn_body_of(&state, &stolen);
    assert_eq!(
        (mine.controller, mine.owner, unit_view(&state, &mine).attack),
        (HUMAN, HUMAN, 6),
        "the engine's read ahead agrees with its return"
    );
    assert!(
        kill(&state, &stolen, &OFF) > kill(&state, &stolen, &AI_EVAL),
        "the old body went back to the AI, out of the thief's aura, so the kill read it a point weaker"
    );
}

#[test]
fn r4798000_a_printed_reborn_units_read_is_unchanged() {
    let state = board("ai-r4798000-printed", json!([REBORN, GROUP_LEADER]));
    let printed = unit(&state, HUMAN, REBORN).clone();
    let (after, body) = destroyed(&state, &printed.id);
    let view = unit_view(&after, &body);
    assert_eq!((body.controller, view.attack, view.health), (HUMAN, 6, 1));
    close(kill(&state, &printed, &AI_EVAL), kill(&state, &printed, &OFF));
    close(
        kill(&state, &printed, &AI_EVAL),
        unit_worth(&state, &printed, &AI_EVAL, Prices::NONE) - body_worth(&state, &body),
    );
}

#[test]
fn r4798000_the_frozen_baseline_reads_the_body_as_before() {
    const { assert!(AI_EVAL.reborn_body_read) };
    const { assert!(!GREEDY_EVAL.reborn_body_read) };
}
