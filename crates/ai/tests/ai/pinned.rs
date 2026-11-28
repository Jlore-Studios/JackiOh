//! #1240: pinned slow positions from improve cycles (#1058, Meditative+ #15 #1337).
//! Tests costly steps (§10.7, `surcharge_nodes`), autopilot owed turns, determinism,
//! and node budgets while blind to hidden cards (R185).

use jackioh_ai::{
    AI_BUDGET, AI_SEARCH, AiOptions, Decision, DecisionReason, Prices, SearchBudget, create_node_counter,
    decide, hidden_instance_ids, redact, simulate, surcharge_nodes,
};
use jackioh_engine::testkit::{
    ActionBody, GameState, PlayerId, Value, create_rng, find_instance, hash_state, legal_actions, subsystems,
};

use super::support::{TEST_BUDGET, register_cards};

const IMPROVE_DEV_154: &str = include_str!("../fixtures/pinned-improve-dev-154.json");
const IMPROVE_DEV_200: &str = include_str!("../fixtures/pinned-improve-dev-200.json");
const AUTOPILOT_DRAW: &str = include_str!("../fixtures/pinned-autopilot-draw.json");
/// #5044: the enemy commander's decision on the 2026-11-28 Assault's level 9 (turn 16) that fell back.
const ASSAULT_ROOT_CUT: &str = include_str!("../fixtures/pinned-assault-root-cut.json");

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
        Prices::NONE,
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

/// R2575000: improve-dev:200's position is a fat one (a long hand, a full board): its allowance is
/// scaled from at most `AI_SEARCH.fat_budget_nodes`, so the doubled budget decides there exactly as
/// the 600 nodes of before did, at the same cost.
#[test]
fn r2575000_a_fat_state_decides_at_ai_budget_as_it_did_at_600_nodes() {
    let at = pinned(IMPROVE_DEV_200);
    let before = SearchBudget {
        nodes: 600,
        lethal_nodes: 150,
        ..AI_BUDGET
    };
    let mut options = AiOptions::with_budget(create_rng(&at.rng_seed, at.rng_cursor), before);
    let old = decide(&at.state, at.seat, &mut options).expect("the seat has a decision to make");
    let new = decide_on(&at.state, &at);
    assert_eq!(new.action, old.action);
    assert_eq!(
        new.stats.nodes, old.stats.nodes,
        "{:?} against {:?}",
        new.stats, old.stats
    );
}

/// R2575000: a position of ordinary width spends past the 600 nodes of before.
#[test]
fn r2575000_a_state_of_ordinary_width_spends_past_600_nodes() {
    let at = pinned(IMPROVE_DEV_154);
    let decision = decide_on(&at.state, &at);
    assert!(decision.stats.nodes > 600, "{:?}", decision.stats);
}

/// R5044000: #5044's position at the test budget. The lethal search leaves 72 of 150 nodes, and the
/// root holds two moves: a play of Meditative #21 whose simulation draws every one of them, then End turn.
/// The beam simulates End turn first, so the cut play still leaves it a line and the decision searches.
#[test]
fn r5044000_a_root_move_that_spends_the_last_node_leaves_the_beam_its_end_turn() {
    let mut at = pinned(ASSAULT_ROOT_CUT);
    // R5183000: the defender's Armor 1 wears off with the first hit since issue #5183, which put
    // the seat's attacks in lethal reach and ended the search before the beam; this much more health
    // keeps the position the budget case #5044 pinned, with no lethal to find.
    const HEALTH_PAST_THE_WEAR: i32 = 6;
    at.state.players.p1.hero.health += HEALTH_PAST_THE_WEAR;
    // R5236011, R5236021: a Nerf's menu now holds Fragile and Unlucky, which moves the draws of the
    // turns Meditative #21's play runs ahead, so the one world the pinned stream deals ends in the seat's
    // win there, a lethal played before the beam; the stream this many draws on deals a world that keeps
    // the budget case #5044 pinned (150 nodes spent, End turn's line the beam's one).
    const STREAM_PAST_THE_NERF_MENU: u32 = 4;
    let mut options = AiOptions::with_budget(
        create_rng(&at.rng_seed, at.rng_cursor + STREAM_PAST_THE_NERF_MENU),
        TEST_BUDGET,
    );
    let d = decide(&at.state, at.seat, &mut options).expect("a decision");
    assert_ne!(
        d.reason,
        DecisionReason::Fallback,
        "the beam was left no line: {:?} after {} nodes",
        d.action,
        d.stats.nodes
    );
    assert!(d.stats.lines > 0, "{:?}", d.stats);
}
