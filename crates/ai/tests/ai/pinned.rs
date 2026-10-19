//! #1240: the pinned positions, decisions found slow: two by the improve screen of 2026-10-16
//! (`training/history/improve-cycle-2026-10-16.md`) and one since Meditative+ shipped (#1337). Each
//! fixture holds the true state, the deciding seat and the seat's stream (`rngSeed`, `rngCursor`) at
//! the decision, beside where it came from:
//!
//! - `pinned-improve-dev-154.json`: game 307 (`improve-dev:154`), turn 15. The other side's Pareto
//!   Optimality judges every play the seat makes, running §10.7's scorer once per playable card.
//! - `pinned-improve-dev-200.json`: game 399 (`improve-dev:200`), turn 23. The seat holds a fused
//!   Zephyrs, whose resolve ranks every shipped card as Zephyrs' does.
//! - `pinned-autopilot-draw.json`: the gate's hard-vs-easy deal 3 (`lagforce:hard-vs-easy:3`, with
//!   Meditative+ #15 Book of Potions put in both decks), turn 7. The other side's library holds
//!   Cognitive Offloading, hidden from the seat. When its search ended the turn into a draw of it,
//!   `reduce` played that whole turn by §10.7's scorer inside the one step, and the decision took
//!   2.9 s with 97% of its work there; a search's step now leaves that turn owed
//!   (`reduce_unpiloted`) and the decision takes about 0.16 s.
//!
//! The first two were captured from that screen's candidate (#1058's second reply, switched on over
//! `f04c284`) against its parent, in an `arena` run over `improve-dev` that wrote the state before the
//! decision once game 307's log held 78 actions and game 399's 101 (the run's games 307 and 399 ended
//! with hashes `7e73ab3a` and `a0b48401`); the third, on `f72f2af`, from a hard-vs-easy game whose
//! decisions were timed. Today's deals deal other decks, so no game reaches them now; the perf gate
//! times them (`cargo jackioh gate`, `AI_GATE.pinned_decision_ms`), and these tests hold what `cargo
//! test` can: the charges `surcharge_nodes` makes for their costly steps, the autopilot's turn left
//! owed, and a decision on each that is deterministic, legal, within the node budget and blind to the
//! hidden cards (R185).

use jackioh_ai::{
    AI_BUDGET, AI_SEARCH, AiOptions, Decision, create_node_counter, decide, hidden_instance_ids, redact,
    simulate, surcharge_nodes,
};
use jackioh_engine::testkit::{
    ActionBody, GameState, PlayerId, Value, create_rng, find_instance, hash_state, legal_actions, subsystems,
};

use super::support::register_cards;

const IMPROVE_DEV_154: &str = include_str!("../fixtures/pinned-improve-dev-154.json");
const IMPROVE_DEV_200: &str = include_str!("../fixtures/pinned-improve-dev-200.json");
const AUTOPILOT_DRAW: &str = include_str!("../fixtures/pinned-autopilot-draw.json");

/// The fused Zephyrs the seat holds on improve-dev:200's position (R77, R179).
const FUSED_ZEPHYRS: &str = "t-10:core-097+core-068";
/// What the twin's hidden cards become: any card the true state's hidden ones are not all.
const STAND_IN: &str = "core-005";

/// The cards improve-dev:154's seat holds.
const IMPROVE_DEV_154_HAND: usize = 5;

/// Meditative+ #18 Cognitive Offloading, in the other side's library on the autopilot-draw position.
const OFFLOADING: &str = "meditativeplus-018";

/// A pinned position: the true state, the deciding seat and its stream.
struct Pinned {
    state: GameState,
    seat: PlayerId,
    rng_seed: String,
    rng_cursor: u32,
}

fn pinned(text: &str) -> Pinned {
    register_cards();
    let fixture: Value = serde_json::from_str(text).expect("a pinned fixture is JSON");
    Pinned {
        state: serde_json::from_value(fixture["state"].clone()).expect("the fixture's state"),
        seat: serde_json::from_value(fixture["seat"].clone()).expect("the fixture's seat"),
        rng_seed: fixture["rngSeed"]
            .as_str()
            .expect("the fixture's rngSeed")
            .to_string(),
        rng_cursor: fixture["rngCursor"]
            .as_u64()
            .and_then(|cursor| u32::try_from(cursor).ok())
            .expect("the fixture's rngCursor"),
    }
}

fn all() -> [Pinned; 3] {
    [
        pinned(IMPROVE_DEV_154),
        pinned(IMPROVE_DEV_200),
        pinned(AUTOPILOT_DRAW),
    ]
}

/// The seat's decision on `state` at AI_BUDGET, on the position's stream.
fn decide_on(state: &GameState, at: &Pinned) -> Decision {
    let mut options = AiOptions::with_budget(create_rng(&at.rng_seed, at.rng_cursor), AI_BUDGET);
    decide(state, at.seat, &mut options).expect("the seat has a decision to make")
}

/// Every legal Play of `seat` on `state`, with the id of the card it plays.
fn plays(state: &GameState, seat: PlayerId) -> Vec<(String, ActionBody)> {
    legal_actions(state, seat)
        .into_iter()
        .filter_map(|action| {
            let ActionBody::Play { instance_id, .. } = &action else {
                return None;
            };
            let id = instance_id.clone();
            Some((id, action))
        })
        .collect()
}

#[test]
fn a_play_judged_by_pareto_optimality_on_improve_dev_154_pays_per_card_held() {
    let Pinned { state, seat, .. } = pinned(IMPROVE_DEV_154);
    assert!(
        subsystems::watching(&state, seat),
        "the other side's Pareto Optimality judges the seat"
    );
    let held = state.players[seat].hand.len();
    assert_eq!(held, IMPROVE_DEV_154_HAND);
    let plays = plays(&state, seat);
    assert!(!plays.is_empty(), "the seat has a play to judge");
    for (_, play) in plays {
        assert_eq!(
            surcharge_nodes(&state, seat, &play),
            AI_SEARCH.judged_card_nodes as usize * held,
            "{play:?}"
        );
    }
}

#[test]
fn a_fused_zephyrs_on_improve_dev_200_pays_what_zephyrs_does() {
    let Pinned { state, seat, .. } = pinned(IMPROVE_DEV_200);
    assert!(
        state.players[seat]
            .hand
            .iter()
            .any(|card| card.def_id == FUSED_ZEPHYRS),
        "the seat holds the fused Zephyrs"
    );
    let fused: Vec<ActionBody> = plays(&state, seat)
        .into_iter()
        .filter(|(id, _)| find_instance(&state, id).is_some_and(|card| card.def_id == FUSED_ZEPHYRS))
        .map(|(_, play)| play)
        .collect();
    assert!(!fused.is_empty(), "the fused Zephyrs can be played");
    for play in fused {
        assert_eq!(
            surcharge_nodes(&state, seat, &play),
            AI_SEARCH.expensive_def_nodes as usize,
            "{play:?}"
        );
    }
}

#[test]
fn an_end_turn_into_the_hidden_cognitive_offloading_leaves_its_autopilot_turn_owed() {
    let Pinned { mut state, seat, .. } = pinned(AUTOPILOT_DRAW);
    let opponent = seat.opponent();
    let at = state.players[opponent]
        .library
        .iter()
        .position(|card| card.def_id == OFFLOADING)
        .expect("the other side's library holds Cognitive Offloading");
    assert!(
        hidden_instance_ids(&state, seat).contains(&state.players[opponent].library[at].id),
        "hidden from the seat"
    );
    // The world where it is the next card the other side draws, as a determinization can deal it.
    let library = &mut state.players[opponent].library;
    let card = library.remove(at);
    library.insert(0, card);
    let held: Vec<String> = state.players[opponent]
        .hand
        .iter()
        .map(|card| card.id.clone())
        .collect();
    let line = simulate(
        &state,
        seat,
        &ActionBody::EndTurn,
        &create_node_counter(AI_BUDGET.nodes, None),
    )
    .expect("the counter gives the step a node")
    .expect("End turn is accepted");
    assert_eq!(line.active, opponent, "the line passed the turn");
    assert_eq!(
        subsystems::autopilot::due(&line),
        Some(opponent),
        "the other side's turn is owed to the autopilot, not played inside the step"
    );
    assert!(
        held.iter()
            .all(|id| line.players[opponent].hand.iter().any(|card| card.id == *id)),
        "nothing was played from the other side's hand"
    );
}

#[test]
fn every_pinned_decision_is_deterministic_legal_and_within_the_budget() {
    for at in all() {
        let first = decide_on(&at.state, &at);
        assert_eq!(
            decide_on(&at.state, &at),
            first,
            "the same state and stream decide alike"
        );
        assert!(first.stats.nodes <= AI_BUDGET.nodes, "{:?}", first.stats);
        assert!(
            legal_actions(&at.state, at.seat).contains(&first.action),
            "{:?} is legal on the true state",
            first.action
        );
    }
}

#[test]
fn equivalent_visible_states_of_every_pinned_position_choose_alike() {
    for at in all() {
        let hidden = hidden_instance_ids(&at.state, at.seat);
        let mut twin = at.state.clone();
        let opponent = &mut twin.players[at.seat.opponent()];
        for card in opponent.hand.iter_mut().chain(opponent.library.iter_mut()) {
            if hidden.contains(&card.id) {
                card.def_id = STAND_IN.to_string();
            }
        }
        assert_ne!(twin, at.state, "the twin's hidden cards differ");
        assert_eq!(
            hash_state(&redact(&twin, at.seat)),
            hash_state(&redact(&at.state, at.seat)),
            "the seat sees the same state"
        );
        assert_eq!(decide_on(&twin, &at), decide_on(&at.state, &at));
    }
}
