//! #1429: the search counts the panics it catches apart from the simulations the reducer only
//! refuses (`SearchStats.sim_panics`, a part of `sim_errors`), and keeps the first one's message. In
//! WebAssembly a panic is a trap no `catch_unwind` sees, so the gates fail on any.
//!
//! Each test swaps one card's script for one that panics, through the testkit's `register_scripts`,
//! which holds for its own thread only: The Coin's play, or Mr. Vanilla's end of turn.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, is_legal, random_decks};

/// The panic The Coin's swapped script raises when it is played.
pub const COIN_PANIC: &str = "ai-test #1429: The Coin's play panicked";
/// The panic Mr. Vanilla's swapped script raises at its controller's end of turn.
const END_PANIC: &str = "ai-test #1429: Mr. Vanilla's end of turn panicked";
/// #8 Mr. Vanilla, a vanilla Unit.
const VANILLA: &str = "core-008";
const LIBRARY: &[&str] = &["core-011", "core-020", "core-011", "core-020"];
const TURN: i32 = 9;
/// A small budget, so a whole match stays quick.
const MATCH_BUDGET: SearchBudget = SearchBudget {
    nodes: 120,
    lethal_nodes: 20,
    ..AI_BUDGET
};
const MATCH_ACTIONS: usize = 60;

/// `def_id`'s script, base and Radiant, is `script` on this thread.
fn install_with(def_id: &str, script: Script) {
    jackioh_cards::register_all();
    let mut scripts = registered_scripts();
    scripts.insert(
        def_id.to_string(),
        CardScripts {
            base: script.clone(),
            radiant: script,
        },
    );
    register_scripts(scripts);
}

/// The Coin panics when it resolves: a Spell's script is its `cry` (`t_coin.rs`).
pub fn install_panicking_coin() {
    install_with(
        COIN_DEF_ID,
        Script {
            cry: Some(hook(|_ctx| -> Vec<Effect> { panic!("{COIN_PANIC}") })),
            ..Script::default()
        },
    );
}

/// The AI's turn 9 with The Coin and Tempo Timmy in hand and 1 crystal: The Coin's play panics.
fn coin_in_hand(seed: &str) -> GameState {
    jackioh_cards::register_all();
    let state = scenario(json!({
        "seed": seed,
        "active": "p1",
        "turn": TURN,
        "p1": { "hand": [COIN_DEF_ID, "core-011"], "mana": 1, "library": LIBRARY },
        "p2": { "library": LIBRARY },
    }))
    .state()
    .clone();
    install_panicking_coin();
    state
}

fn coin_play(state: &GameState) -> ActionBody {
    let coin = state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == COIN_DEF_ID)
        .map(|card| card.id.clone())
        .expect("the AI holds The Coin");
    legal_actions(state, AI)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == coin))
        .expect("The Coin's play is legal")
}

#[test]
fn a_panicking_simulated_play_is_counted_apart_and_decide_still_answers_a_legal_action() {
    let seed = "sim-panics-play";
    let state = coin_in_hand(seed);

    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let step = simulate(&state, AI, &coin_play(&state), &counter);
    assert_eq!(step, Some(Err(COIN_PANIC.to_string())));
    assert_eq!(counter.sim_errors(), 1);
    assert_eq!(counter.sim_panics(), 1);
    assert_eq!(counter.first_sim_panic().as_deref(), Some(COIN_PANIC));

    let decision = decide(&state, AI, &mut AiOptions::new(create_rng(seed, 0))).expect("the AI acts");
    assert!(is_legal(&state, AI, &decision.action), "{decision:?}");
    assert!(decision.stats.sim_panics >= 1, "{:?}", decision.stats);
    assert!(
        decision.stats.sim_errors >= decision.stats.sim_panics,
        "{:?}",
        decision.stats
    );
    assert_eq!(decision.first_sim_panic.as_deref(), Some(COIN_PANIC));
}

#[test]
fn a_refused_simulation_counts_in_sim_errors_and_not_in_sim_panics() {
    let state = coin_in_hand("sim-panics-refused");
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    // p2 may not end p1's turn: the reducer refuses it, and nothing panics.
    let step = simulate(&state, HUMAN, &ActionBody::EndTurn, &counter);
    assert!(matches!(step, Some(Err(_))), "{step:?}");
    assert_eq!(counter.sim_errors(), 1);
    assert_eq!(counter.sim_panics(), 0);
    assert_eq!(counter.first_sim_panic(), None);
}

#[test]
fn a_panic_in_the_opponents_reply_is_counted_and_keeps_its_message() {
    jackioh_cards::register_all();
    let s = scenario(json!({
        "seed": "sim-panics-reply",
        "active": "p1",
        "turn": TURN,
        "p1": { "library": LIBRARY },
        "p2": { "field": [VANILLA], "library": LIBRARY },
    }));
    let handed = act(s.state(), AI, &ActionBody::EndTurn);
    assert_eq!(handed.active, HUMAN);
    install_with(
        VANILLA,
        Script {
            end_of_turn: Some(hook(|_ctx| -> Vec<Effect> { panic!("{END_PANIC}") })),
            ..Script::default()
        },
    );

    let counter = create_node_counter(20, None);
    let after = simulate_reply(&handed, AI, &counter, &hidden_card_ids(&handed, AI)).expect("a reply");
    // Mr. Vanilla attacks, then its end of turn panics inside the reply's endTurn: the reply stops
    // where it stands, still p2's turn.
    assert_eq!(after.active, HUMAN);
    assert_eq!(counter.sim_panics(), 1);
    assert_eq!(counter.sim_errors(), 1);
    assert_eq!(counter.first_sim_panic().as_deref(), Some(END_PANIC));
}

#[test]
fn a_match_record_sums_its_search_panics_and_names_the_seat_of_the_first() {
    install_panicking_coin();
    let seed = "sim-panics-match";
    let record = play_match(
        &MatchConfig {
            seed: seed.into(),
            decks: random_decks(seed),
            handicaps: None,
            controllers: PerPlayer {
                p1: SeatController::Random,
                p2: SeatController::Ai {
                    budget: Some(MATCH_BUDGET),
                    skill: None,
                },
            },
            max_actions: Some(MATCH_ACTIONS),
        },
        &mut MatchHooks::default(),
    );
    // p2 goes second, so it is dealt The Coin (R244), and its searches try playing it.
    assert!(record.thrown.is_empty(), "{:?}", record.thrown);
    assert!(
        record.sim_panics >= 1,
        "no search panic in {} decisions",
        record.decisions
    );
    assert_eq!(
        record.first_sim_panic,
        Some(SimPanic {
            seat: HUMAN,
            message: COIN_PANIC.to_string(),
        })
    );
}
