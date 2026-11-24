//! Audit (decide): exploratory.

use jackioh_ai::{
    AI_BUDGET, AiOptions, Decision, DecisionReason, Prices, SearchBudget, action_key, candidate_actions,
    decide, greedy_action,
};
use jackioh_engine::testkit::{
    Action, ActionBody, GameState, PlayerId, create_rng, json, legal_actions, reduce, scenario, subsystems,
};

const AI: PlayerId = PlayerId::P1;

fn register_cards() {
    jackioh_cards::register_all();
}

fn is_legal(state: &GameState, seat: PlayerId, action: &ActionBody) -> bool {
    let key = action_key(action);
    legal_actions(state, seat)
        .iter()
        .any(|legal| action_key(legal) == key)
}

fn mid_board() -> GameState {
    register_cards();
    scenario(json!({
        "seed": "audit-decide-mid",
        "active": AI,
        "turn": 9,
        "p1": {
            "hand": ["core-008", "core-011", "core-019", "core-044"],
            "field": ["core-008", "core-011"],
            "mana": 5,
            "library": ["core-008", "core-011", "core-020"],
        },
        "p2": {
            "hand": ["core-011", "core-008"],
            "field": ["core-008", "core-019"],
            "library": ["core-008", "core-011", "core-020"],
        },
    }))
    .state()
    .clone()
}



/// `BOT_STRENGTHS[0]`'s budget (crates/shared/src/config.rs), the weakest ladder bot's.
const WEAKEST: SearchBudget = SearchBudget {
    nodes: 150,
    lethal_nodes: 40,
    determinizations: 1,
    beam_width: 2,
    root_branching: 10,
    branching: 4,
    max_depth: 6,
    finalists: 3,
};

fn decided(state: &GameState, budget: SearchBudget) -> Decision {
    decide(state, AI, &mut AiOptions::with_budget(create_rng("audit-decide", 0), budget)).expect("a decision")
}

/// The nodes the lethal solver spent on `mid_board` (no lethal there) under `budget`: with `max_depth` 0
/// the beam takes none, so a decision's nodes are its prices' and the solver's, and the prices' are what
/// the same decision spends with `lethal_nodes` 0.
fn solver_nodes(budget: SearchBudget) -> usize {
    let state = mid_board();
    let no_beam = SearchBudget { max_depth: 0, ..budget };
    let prices = decided(&state, SearchBudget { lethal_nodes: 0, ..no_beam }).stats.nodes;
    decided(&state, no_beam).stats.nodes - prices
}

/// `SearchBudget.lethal_nodes` (types.rs): "Of `nodes`, the most the lethal solver may spend before the
/// beam starts." `AI_SEARCH.fat_min_lethal` (config.rs): "The least of the scaled budget the lethal
/// solver's allowance shrinks to": a floor for a fat state's shrunk budget. `mid_board` is not fat (12
/// cards, 7 candidates, `fat_reference` 30), so nothing is scaled, yet `scaled_budget` (decide.rs:289-295)
/// raises the allowance to 60.
#[test]
fn lethal_nodes_caps_the_solver_on_a_state_that_is_not_fat() {
    assert_eq!(solver_nodes(SearchBudget { lethal_nodes: 300, ..AI_BUDGET }), 300, "PREMISE: the solver spends its allowance here");
    let asked = SearchBudget { lethal_nodes: 10, ..AI_BUDGET };
    assert!(solver_nodes(asked) <= 10, "lethal_nodes 10 of 1200: the solver spent {}", solver_nodes(asked));
}

/// The same on the weakest ladder bot's shipped budget: 150 nodes, 40 of them the solver's.
#[test]
fn the_weakest_ladder_bots_solver_keeps_to_its_40_nodes() {
    assert!(solver_nodes(WEAKEST) <= WEAKEST.lethal_nodes, "the solver spent {}", solver_nodes(WEAKEST));
}

/// A budget of 60 nodes or fewer: the floor, capped at `nodes`, hands the solver the whole decision, so
/// the beam gets no node and the decision is a fallback endTurn with no line searched. With the same 30
/// nodes and no solver the beam finds lines (the premise).
#[test]
fn a_small_budget_still_leaves_the_beam_its_nodes() {
    let state = mid_board();
    let alone = decided(&state, SearchBudget { nodes: 30, lethal_nodes: 0, ..AI_BUDGET });
    assert_eq!(alone.reason, DecisionReason::Search, "PREMISE: 30 nodes search this board");
    let small = decided(&state, SearchBudget { nodes: 30, lethal_nodes: 7, ..AI_BUDGET });
    println!("small: {:?} lines={} nodes={}", small.reason, small.stats.lines, small.stats.nodes);
    assert!(small.stats.lines > 0, "7 of 30 nodes were the solver's, yet the beam searched no line");
    assert_ne!(small.reason, DecisionReason::Fallback);
}

/// A caller's budget is plain data (`SearchBudget` deserializes from the web's JSON, crates/wasm/core
/// `DecideOptions`); "no limit" written as the largest number overflows decide.rs:292, :616, :618, :626 and
/// search.rs:300. With overflow checks the panic is caught and the decision is a fallback endTurn counted
/// in simPanics (which the gates fail on); without them the sums wrap.
#[test]
fn a_budget_field_at_its_largest_value_does_not_overflow() {
    let state = mid_board();
    let big = usize::MAX;
    let cases = [
        ("max_depth", SearchBudget { max_depth: big, ..AI_BUDGET }),
        ("finalists", SearchBudget { finalists: big, ..AI_BUDGET }),
        ("root_branching", SearchBudget { root_branching: big, ..AI_BUDGET }),
        ("branching", SearchBudget { branching: big, ..AI_BUDGET }),
        ("lethal_nodes", SearchBudget { lethal_nodes: big, ..AI_BUDGET }),
    ];
    let mut failed: Vec<String> = Vec::new();
    for (name, budget) in cases {
        let d = decided(&state, budget);
        if d.stats.sim_panics > 0 {
            failed.push(format!("{name}: {:?}, {:?}", d.reason, d.first_sim_panic));
        }
    }
    assert!(failed.is_empty(), "{failed:#?}");
}

/// #97 Zephrys: (1) Spell, a Play of which `simulate` surcharges `AI_SEARCH.expensive_def_nodes` (100).
const ZEPHRYS: &str = "core-097";

fn one_card(def: &str) -> GameState {
    register_cards();
    scenario(json!({
        "seed": "audit-wrap",
        "active": AI,
        "turn": 9,
        "p1": { "hand": [def], "mana": 3, "library": ["core-008", "core-011", "core-020"] },
        "p2": { "hand": ["core-011"], "field": ["core-008"], "library": ["core-008", "core-011", "core-020"] },
    }))
    .state()
    .clone()
}

/// R3498000 (README, Budgets): a wrap-up "stops the beam's slice of the counter ...: the beam's open lines
/// end their turn and the reply and the other determinizations run on". decide.rs:620-623: the wrap-up "is
/// first polled once the root has been expanded (root_branching and endTurn): a wrap-up that fired before
/// the beam would otherwise leave it no line and the decision would fall back." The floor is counted in
/// nodes (`root_branching + 1`), so a first root move that costs more nodes than that (a Zephrys play, 101)
/// is refused inside its own surcharge and the beam has no line.
#[test]
fn a_wrap_up_fired_before_the_beam_still_leaves_a_line_when_the_first_root_move_is_surcharged() {
    let fired = || true;
    let wrapped = |state: &GameState| {
        let mut options = AiOptions::with_budget(create_rng("audit-wrap", 0), AI_BUDGET);
        options.wrap_up = Some(&fired);
        decide(state, AI, &mut options).expect("a decision")
    };
    let plain = wrapped(&one_card("core-008"));
    println!("plain: {:?} lines={} stop={:?}", plain.reason, plain.stats.lines, plain.stats.stopped_by);
    assert!(plain.stats.lines > 0, "PREMISE: an ordinary root is expanded under a fired wrap-up");
    let state = one_card(ZEPHRYS);
    let cands = candidate_actions(&state, AI, Prices::NONE);
    println!("cands: {:?}", cands.iter().map(action_key).collect::<Vec<_>>());
    let unclocked = decided(&state, AI_BUDGET);
    println!("unclocked: {:?} {} lines={}", unclocked.reason, action_key(&unclocked.action), unclocked.stats.lines);
    let d = wrapped(&state);
    println!("wrapped: {:?} {} lines={} nodes={} stop={:?}", d.reason, action_key(&d.action), d.stats.lines, d.stats.nodes, d.stats.stopped_by);
    assert!(d.stats.lines > 0, "the beam was left no line");
    assert_ne!(d.reason, DecisionReason::Fallback);
}

// ---- exploration, to delete ----

#[test]
fn explore_greedy_pareto() {
    register_cards();
    for hand in [
        json!(["core-017", "core-088", "core-094", "core-044"]),
        json!(["core-017", "core-088", "core-094", "core-017", "core-044"]),
        json!(["core-017", "core-088", "core-094", "core-088", "core-044"]),
        json!(["core-017", "core-088", "core-094", "core-094", "core-044"]),
    ] {
        let state = scenario(json!({
            "seed": "audit-pareto",
            "active": AI,
            "turn": 9,
            "p1": { "hand": hand, "mana": 4, "library": ["core-008", "core-011", "core-020"] },
            "p2": { "health": 4, "backrow": [{ "def": "meditative-071", "faceUp": true }], "library": ["core-008", "core-011", "core-020"] },
        }))
        .state()
        .clone();
        let cands = candidate_actions(&state, AI, Prices::NONE);
        let g = greedy_action(&state, AI, &mut create_rng("audit-pareto", 0));
        let won = g.as_ref().map(|a| {
            let r = reduce(&state, &Action::new(a.clone(), AI, "g"));
            r.state.result.is_some()
        });
        println!("hand {hand}: watching={} cands={} greedy={:?} won={won:?}", subsystems::watching(&state, AI), cands.len(), g.as_ref().map(action_key));
        for c in &cands { println!("    {}", action_key(c)); }
    }
}

#[test]
fn explore_stop_at_prompt() {
    register_cards();
    let mut s = scenario(json!({
        "seed": "audit-prompt",
        "p1": { "hand": ["core-072"], "graveyard": ["core-008", "core-011"], "library": ["core-011"] },
        "p2": { "hand": ["core-005"] },
    }));
    s.play("core-072", json!({}));
    let state = s.state().clone();
    println!("pending: {:?}", state.pending.as_ref().map(|p| (p.player_id, p.options.len())));
    let stop = || true;
    for (name, st, wr) in [("stop", true, false), ("wrap", false, true), ("none", false, false)] {
        let mut options = AiOptions::with_budget(create_rng("audit-prompt", 0), AI_BUDGET);
        if st { options.should_stop = Some(&stop); }
        if wr { options.wrap_up = Some(&stop); }
        let d = decide(&state, AI, &mut options).expect("d");
        println!("{name}: {:?} {} legal={} lines={} nodes={} stop={:?}", d.reason, action_key(&d.action), is_legal(&state, AI, &d.action), d.stats.lines, d.stats.nodes, d.stats.stopped_by);
    }
}
