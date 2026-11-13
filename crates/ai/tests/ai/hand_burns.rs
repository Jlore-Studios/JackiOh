//! R3575800 (issue #3575): a draw a side's coming start of turn will burn (§2.4, R4) costs that side an
//! unseen hand card's price in the evaluation (`coming_burns`, `AI_EVAL.burned_draw_share`), read for
//! the side whose turn is not running. Its draws are DRAWS_PER_TURN plus its handicap's
//! `extraDrawsPerTurn`, read off the state (R180), never a difficulty.
//!
//! Every board is a `scenario()` with p1 to act and libraries of uncastable 9-cost stand-ins.

use jackioh_ai::*;
use jackioh_engine::config::{AI_DIFFICULTY, HAND_CAP};
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, close};

/// A card nobody can cast: the libraries' draws change nothing.
const INERT: &str = "stand-in (9) 1/1";

/// p1 to act on turn 9, p1 holding `own` cards and p2 `enemy`, each library `library` cards deep.
fn board(own: usize, enemy: usize, library: usize) -> GameState {
    jackioh_cards::register_all();
    scenario(json!({
        "seed": "ai-hand-burns",
        "active": "p1",
        "turn": 9,
        "p1": { "hand": vec![INERT; own], "library": vec![INERT; library] },
        "p2": { "hand": vec![INERT; enemy], "library": vec![INERT; library] },
    }))
    .state()
    .clone()
}

/// `state` with `player` on Hard's handicap (one extra draw a turn).
fn on_hard(mut state: GameState, player: PlayerId) -> GameState {
    state.players[player].handicap = Some(AI_DIFFICULTY.hard);
    state
}

/// What one burned draw costs: an unseen hand card's price.
fn burn_price() -> f64 {
    AI_EVAL.burned_draw_share * (AI_EVAL.hand_card + AI_EVAL.hand_per_cost * AI_EVAL.opponent_hand_cost)
}

#[test]
fn r3575800_coming_burns_counts_the_draws_past_the_hand_cap() {
    let cap = HAND_CAP as usize;
    // A human's one draw a turn: a full hand burns it, one short of full does not.
    assert_eq!(coming_burns(&board(0, cap, 4), HUMAN), 1);
    assert_eq!(coming_burns(&board(0, cap - 1, 4), HUMAN), 0);
    // Hard's extra draw, read off the seat's handicap: one short of full burns the second.
    assert_eq!(coming_burns(&on_hard(board(0, cap - 1, 4), HUMAN), HUMAN), 1);
    assert_eq!(coming_burns(&on_hard(board(0, cap, 4), HUMAN), HUMAN), 2);
    assert_eq!(coming_burns(&on_hard(board(0, cap - 2, 4), HUMAN), HUMAN), 0);
    // A draw from an empty library is fatigue, not a burn.
    assert_eq!(coming_burns(&on_hard(board(0, cap, 1), HUMAN), HUMAN), 1);
    assert_eq!(coming_burns(&board(0, cap, 0), HUMAN), 0);
}

#[test]
fn r3575800_a_burn_coming_costs_an_unseen_hand_card() {
    let cap = HAND_CAP as usize;
    let off = EvalWeights {
        burned_draw_share: 0.0,
        ..AI_EVAL
    };
    let score = |state: &GameState, w: &EvalWeights| evaluate(state, AI, NextSwing::Enemy, w);
    // The enemy, whose turn is next, will burn one draw of a full hand: the seat gains its price.
    let full = board(0, cap, 4);
    close(score(&full, &AI_EVAL) - score(&full, &off), burn_price());
    // On Hard's handicap one short of full burns one too.
    let hard = on_hard(board(0, cap - 1, 4), HUMAN);
    close(score(&hard, &AI_EVAL) - score(&hard, &off), burn_price());
    // The side whose turn is running may still empty its hand: its own full hand costs nothing yet.
    let own = board(cap, 0, 4);
    close(score(&own, &AI_EVAL), score(&own, &off));
    // A hand with room costs nothing.
    let roomy = board(0, cap - 1, 4);
    close(score(&roomy, &AI_EVAL), score(&roomy, &off));
}

#[test]
fn r3575800_the_frozen_greedy_baseline_reads_no_burn() {
    assert_eq!(GREEDY_EVAL.burned_draw_share, 0.0);
    let full = board(0, HAND_CAP as usize, 4);
    let off = EvalWeights {
        burned_draw_share: 0.0,
        ..GREEDY_EVAL
    };
    close(
        evaluate(&full, AI, NextSwing::Enemy, &GREEDY_EVAL),
        evaluate(&full, AI, NextSwing::Enemy, &off),
    );
}
