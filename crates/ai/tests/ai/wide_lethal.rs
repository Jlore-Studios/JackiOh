//! R4788000 (#4788): a wide decision's lethal. `decide` scales a wide state's allowance down
//! (AI_SEARCH.fat_*, R2575000), the lethal solver's share with it, and a lethal of four to six actions
//! then needs more nodes than the solver is given. Once its scaled allowance is spent, the best-first walk
//! goes on only from a position whose ready attacks already reach lethal (`ready_gap` at or under 0), and
//! finishes the proof of a line that won on determinization 0, up to the allowance its caller gave.
//!
//! The pinned position is turn 8 of `audit:hard-vs-easy:2`, p2 to move with 32 root candidates: the
//! solver proves its six-action lethal in 213 nodes, and the scaled allowance was 140.

use jackioh_ai::{
    AI_BUDGET, AI_SEARCH, AiOptions, DecisionReason, DeterminizeOptions, NodeCounter, Prices, SearchBudget,
    candidate_actions, create_node_counter, decide, det_ground, determinize_on, find_lethal,
    find_lethal_reaching, probe_world, ready_gap, redact,
};
use jackioh_engine::testkit::{GameState, PlayerId, Value, create_rng, json};

use super::support::{AI, decision_prices_at, register_cards, scenario};

const WIDE_LETHAL: &str = include_str!("../fixtures/pinned-wide-lethal.json");

/// The pinned position: the true state, the deciding seat and its stream.
struct Pinned {
    state: GameState,
    seat: PlayerId,
    rng_seed: String,
    rng_cursor: u32,
}

fn pinned() -> Pinned {
    register_cards();
    let fixture: Value = serde_json::from_str(WIDE_LETHAL).expect("the fixture is JSON");
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

/// The worlds `decide` samples for the position, in its order, on its stream.
fn worlds(at: &Pinned) -> Vec<GameState> {
    let public = redact(&at.state, at.seat);
    let ground = det_ground(&public, at.seat);
    let mut rng = create_rng(&at.rng_seed, at.rng_cursor);
    (0..AI_BUDGET.determinizations)
        .map(|_| determinize_on(&public, at.seat, &mut rng, DeterminizeOptions::default(), &ground))
        .collect()
}

/// The position's root candidates, as `decide` lists them.
fn width(at: &Pinned) -> usize {
    let public = redact(&at.state, at.seat);
    let probe = probe_world(&public, at.seat, &det_ground(&public, at.seat));
    candidate_actions(&probe, at.seat, Prices::NONE).len()
}

/// The lethal allowance `decide` scales a state of `fat` width down to at AI_BUDGET (decide.rs
/// `scaled_budget`).
fn scaled_allowance(fat: usize) -> usize {
    let reference = AI_SEARCH.fat_reference as usize;
    let nodes = AI_BUDGET.nodes.min(AI_SEARCH.fat_budget_nodes as usize) * reference / fat;
    AI_BUDGET.lethal_nodes * nodes / AI_BUDGET.nodes
}

#[test]
fn r4788000_a_wide_decision_plays_the_lethal_its_scaled_allowance_cut_short() {
    let at = pinned();
    let fat = width(&at);
    assert!(
        fat > AI_SEARCH.fat_reference as usize,
        "{fat} candidates is a wide state"
    );
    let scaled = scaled_allowance(fat);
    assert!(
        scaled < AI_BUDGET.lethal_nodes,
        "the allowance is scaled down to {scaled}"
    );

    // The premise: the solver proves the lethal within the caller's allowance and not within the scaled one.
    let worlds = worlds(&at);
    let prices = decision_prices_at(&at.state, at.seat);
    let short = create_node_counter(scaled, None);
    assert_eq!(find_lethal(&worlds, at.seat, &short, scaled, &prices), None);
    let whole = create_node_counter(AI_BUDGET.lethal_nodes, None);
    let proved = find_lethal(&worlds, at.seat, &whole, AI_BUDGET.lethal_nodes, &prices);
    assert!(proved.is_some(), "the solver proves it in {} nodes", whole.used());
    assert!(whole.used() > scaled, "{} nodes against {scaled}", whole.used());

    let mut options = AiOptions::with_budget(create_rng(&at.rng_seed, at.rng_cursor), AI_BUDGET);
    let decision = decide(&at.state, at.seat, &mut options).expect("the seat has a decision to make");
    assert_eq!(decision.reason, DecisionReason::Lethal, "{:?}", decision.stats);
    assert!(decision.stats.nodes <= AI_BUDGET.nodes, "{:?}", decision.stats);
}

#[test]
fn r4788000_past_its_allowance_the_walk_goes_on_from_a_position_that_reads_as_lethal() {
    let at = pinned();
    let scaled = scaled_allowance(width(&at));
    let worlds = worlds(&at);
    let prices = decision_prices_at(&at.state, at.seat);
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let reach = AI_BUDGET.lethal_nodes;
    let proved = find_lethal_reaching(&worlds, at.seat, &counter, scaled, &counter, reach, &prices);
    assert!(
        proved.is_some(),
        "the walk reaches it in {} nodes",
        counter.used()
    );
    assert!(
        counter.used() > scaled,
        "{} nodes, past the {scaled} it was given",
        counter.used()
    );
    assert!(
        counter.used() <= reach,
        "{} nodes, within its reach of {reach}",
        counter.used()
    );
}

#[test]
fn r4788000_the_pinned_wide_decision_keeps_to_a_callers_own_figures() {
    let at = pinned();
    // R4802000's edges: a solver given nothing, and one given more than the whole budget.
    let none = SearchBudget {
        lethal_nodes: 0,
        ..AI_BUDGET
    };
    let past = SearchBudget {
        nodes: 200,
        lethal_nodes: 400,
        ..AI_BUDGET
    };
    for budget in [none, past] {
        let mut options = AiOptions::with_budget(create_rng(&at.rng_seed, at.rng_cursor), budget);
        let decision = decide(&at.state, at.seat, &mut options).expect("the seat has a decision to make");
        assert!(
            decision.stats.nodes <= budget.nodes,
            "{budget:?}: {:?}",
            decision.stats
        );
        if budget.lethal_nodes == 0 {
            assert_ne!(
                decision.reason,
                DecisionReason::Lethal,
                "a solver given nothing has no reach"
            );
        }
    }
}

/// The lethal tests' wide board (lethal.rs) with the enemy hero at full health: a tree the solver cannot
/// search out, and no position of it reads as lethal.
fn far_from_lethal() -> GameState {
    register_cards();
    scenario(json!({
        "seed": "wide-lethal-far",
        "active": AI,
        "turn": 9,
        "p1": {
            "hand": [{ "def": "core-055", "radiant": true }, "core-035", "core-031"],
            "mana": 4,
            "field": ["core-025", "core-008", "core-011"],
        },
        "p2": { "field": ["core-019", "core-055"], "health": 30 },
    }))
    .state()
    .clone()
}

#[test]
fn r4788000_a_walk_that_never_reads_as_lethal_stops_at_its_allowance() {
    let state = far_from_lethal();
    assert!(ready_gap(&state, AI) > 0.0);
    let public = redact(&state, AI);
    let ground = det_ground(&public, AI);
    let mut rng = create_rng("wide-lethal-far:worlds", 0);
    let worlds: Vec<GameState> = (0..AI_BUDGET.determinizations)
        .map(|_| determinize_on(&public, AI, &mut rng, DeterminizeOptions::default(), &ground))
        .collect();
    let allowance = AI_SEARCH.fat_min_lethal as usize;

    let plain = create_node_counter(AI_BUDGET.nodes, None);
    assert_eq!(find_lethal(&worlds, AI, &plain, allowance, Prices::NONE), None);
    assert_eq!(plain.used(), allowance, "the walk is cut, not searched out");

    let reaching = create_node_counter(AI_BUDGET.nodes, None);
    let reach = AI_BUDGET.lethal_nodes;
    assert_eq!(
        find_lethal_reaching(&worlds, AI, &reaching, allowance, &reaching, reach, Prices::NONE),
        None
    );
    assert_eq!(reaching.used(), plain.used(), "no node past the allowance");
}
