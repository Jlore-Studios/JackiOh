//! Infinite mana and a lowered cost in the AI's evaluation (issue #3575).
//!
//! R3575200: a side acting with infinite mana (R3548004), or holding a Dormant card on the field whose
//! face carries it, counts what one turn of it frees: its hand's cost past its crystals, each crystal at
//! `unspent_mana`, at the Dormant card's slumber share (R2380000) while it sleeps; the enemy's hand,
//! unseen, at `opponent_hand_cost` a card. R3575201: a hand card's `costOverride` moves its price from
//! its printed cost as an Upgrade's `costMod` does (`hand_cost_delta`).
//!
//! The cards: Impact Reactor (S+ #56), a (3) GIGA Field Spell, Dormant for 2 turns, "When awoken: Gain
//! infinite mana"; 4-mana 7/7 (core-025); Mr. Vanilla (core-008); Metaglass (S+ #49), a (1) Mineral.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, close, register_cards};

const REACTOR: &str = "stinkyscapeplus-056";
const DEAR: &str = "core-025";
const VANILLA: &str = "core-008";
const MINERAL: &str = "stinkyscapeplus-049";
const LIBRARY: &[&str] = &[VANILLA, VANILLA, VANILLA, VANILLA];

/// The evaluation with R3575200's term off, everything else as the AI's.
const NO_INFINITE: EvalWeights = EvalWeights {
    infinite_mana: 0.0,
    ..AI_EVAL
};

/// `player` plays the Reactor on three crystals, keeping `hand` (its turn: p1's, or p2's after p1's).
fn reactor_played(player: PlayerId, hand: &[&str]) -> GameState {
    register_cards();
    let mut held = vec![REACTOR];
    held.extend_from_slice(hand);
    let side = json!({ "hand": held, "mana": 3, "library": LIBRARY });
    let other = json!({ "hand": [VANILLA], "mana": 3, "library": LIBRARY });
    let (p1, p2) = if player == AI {
        (side, other)
    } else {
        (other, side)
    };
    let mut s = scenario(json!({ "seed": "ai-infinite-mana", "p1": p1, "p2": p2 }));
    if player == HUMAN {
        s.end_turn();
    }
    s.play(REACTOR, json!({ "zone": 1 }));
    s.state().clone()
}

fn reactor(state: &GameState) -> &CardInstance {
    every_card_on_field(state)
        .into_iter()
        .find(|card| card.def_id == REACTOR)
        .expect("the Reactor stands")
}

fn every_card_on_field(state: &GameState) -> Vec<&CardInstance> {
    PLAYER_IDS
        .into_iter()
        .flat_map(|player| state.players[player].backrow.iter().flatten())
        .collect()
}

/// What R3575200 adds to the AI's evaluation of `state`.
fn term(state: &GameState) -> f64 {
    evaluate(state, AI, NextSwing::Enemy, &AI_EVAL) - evaluate(state, AI, NextSwing::Enemy, &NO_INFINITE)
}

#[test]
fn r3575200_a_dormant_reactor_counts_its_hand_past_its_crystals_at_its_slumber_share() {
    let state = reactor_played(AI, &[DEAR, DEAR, DEAR]);
    let slumber = reactor(&state).slumber.expect("the Reactor sleeps");
    let turns = slumber.turns.expect("a countdown");
    let crystals = state.players[AI].mana.max;
    let freed = f64::from(3 * 4 - crystals);
    assert!(
        freed > 0.0,
        "three 4-cost cards cost more than {crystals} crystals"
    );
    close(
        term(&state),
        AI_EVAL.slumber_turn_share.powi(turns) * AI_EVAL.unspent_mana * freed,
    );
}

#[test]
fn r3575200_an_awake_reactor_counts_all_of_it() {
    let mut state = reactor_played(AI, &[DEAR, DEAR, DEAR]);
    let id = reactor(&state).id.clone();
    find_instance_mut(&mut state, &id).expect("the Reactor").slumber = None;
    assert!(
        has_infinite_mana(&state, AI),
        "an awake Reactor gives infinite mana"
    );
    let freed = f64::from(3 * 4 - state.players[AI].mana.max);
    close(term(&state), AI_EVAL.unspent_mana * freed);
}

#[test]
fn r3575200_a_hand_its_crystals_already_pay_for_counts_nothing() {
    let state = reactor_played(AI, &[VANILLA]);
    close(term(&state), 0.0);
}

#[test]
fn r3575200_the_enemys_reactor_counts_against_the_seat_by_its_unseen_hand() {
    let state = reactor_played(HUMAN, &[DEAR, DEAR, DEAR]);
    let slumber = reactor(&state).slumber.expect("the Reactor sleeps");
    let side = &state.players[HUMAN];
    let unseen = side.hand.len() as f64 * AI_EVAL.opponent_hand_cost;
    let freed = (unseen - f64::from(side.mana.max)).max(0.0);
    assert!(freed > 0.0);
    close(
        term(&state),
        -AI_EVAL
            .slumber_turn_share
            .powi(slumber.turns.expect("a countdown"))
            * AI_EVAL.unspent_mana
            * freed,
    );
}

#[test]
fn r3575200_the_frozen_baseline_reads_none() {
    assert_eq!(GREEDY_EVAL.infinite_mana, 0.0);
}

#[test]
fn r3575201_a_card_added_at_0_over_its_printed_1_counts_as_an_upgrade() {
    register_cards();
    let hand = |entry: Value| {
        scenario(json!({
            "seed": "ai-cost-override",
            "p1": { "hand": [entry], "library": LIBRARY },
            "p2": { "hand": [VANILLA], "library": LIBRARY },
        }))
        .state()
        .clone()
    };
    let printed = hand(json!(MINERAL));
    let free = hand(json!({ "def": MINERAL, "costOverride": 0 }));
    let upgraded = hand(json!({ "def": MINERAL, "costMod": -1 }));
    let eval = |state: &GameState| evaluate(state, AI, NextSwing::Enemy, &AI_EVAL);
    close(eval(&free) - eval(&printed), AI_EVAL.hand_cost_delta);
    close(eval(&free), eval(&upgraded));
}
