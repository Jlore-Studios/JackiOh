//! R2575000 (issue #2575): the practice AI's budget, doubled to 1,200 nodes and 300 lethal nodes in
//! the search shape of before, and what follows it.
//! R4802000 (issue #4802): a decision keeps to its caller's own budget on a state that is not fat, and a
//! budget field at any value overflows nothing.

use jackioh_ai::{
    AI_BUDGET, AI_GATE_BUDGET, AI_SEARCH, Decision, DecisionReason, LETHAL_HINT_DETERMINIZATIONS,
    LETHAL_HINT_NODES, SearchBudget, decide,
};
use jackioh_engine::testkit::{GameState, json};

use super::support::{AI, TEST_BUDGET, ai_options, register_cards, scenario};

#[test]
fn r2575000_the_practice_budget_is_1200_nodes_300_of_them_lethal_in_the_search_shape_of_before() {
    assert_eq!(
        AI_BUDGET,
        SearchBudget {
            nodes: 1200,
            lethal_nodes: 300,
            determinizations: 3,
            beam_width: 4,
            root_branching: 20,
            branching: 6,
            max_depth: 8,
            finalists: 3,
        }
    );
}

#[test]
fn r2575000_the_gates_the_sweep_and_the_lethal_hint_follow_the_practice_budget() {
    assert_eq!(AI_GATE_BUDGET, AI_BUDGET);
    assert_eq!(LETHAL_HINT_NODES, AI_BUDGET.lethal_nodes);
    assert_eq!(LETHAL_HINT_DETERMINIZATIONS, AI_BUDGET.determinizations);
}

#[test]
fn r2575000_a_fat_state_scales_from_the_600_nodes_of_before() {
    assert_eq!(AI_SEARCH.fat_budget_nodes, 600);
    assert!((AI_SEARCH.fat_budget_nodes as usize) < AI_BUDGET.nodes);
}

/// R4802000: a mid-game board well under `AI_SEARCH.fat_reference`'s width (12 cards, 7 candidates), with
/// no lethal on it.
fn narrow_board() -> GameState {
    register_cards();
    scenario(json!({
        "seed": "audit-decide-mid", "active": AI, "turn": 9,
        "p1": { "hand": ["core-008", "core-011", "core-019", "core-044"], "field": ["core-008", "core-011"],
                "mana": 5, "library": ["core-008", "core-011", "core-020"] },
        "p2": { "hand": ["core-011", "core-008"], "field": ["core-008", "core-019"],
                "library": ["core-008", "core-011", "core-020"] },
    }))
    .state()
    .clone()
}

fn decided(state: &GameState, budget: SearchBudget) -> Decision {
    decide(state, AI, &mut ai_options("audit-decide", budget)).expect("a decision")
}

/// The nodes the lethal solver spent on `narrow_board` under `budget`: with `max_depth` 0 the beam takes
/// none, so a decision's nodes are its prices' and the solver's, and the prices' are what the same
/// decision spends with `lethal_nodes` 0.
fn solver_nodes(budget: SearchBudget) -> usize {
    let state = narrow_board();
    let no_beam = SearchBudget {
        max_depth: 0,
        ..budget
    };
    let prices = decided(
        &state,
        SearchBudget {
            lethal_nodes: 0,
            ..no_beam
        },
    )
    .stats
    .nodes;
    decided(&state, no_beam).stats.nodes - prices
}

#[test]
fn r4802000_lethal_nodes_caps_the_solver_on_a_state_that_is_not_fat() {
    assert_eq!(
        solver_nodes(SearchBudget {
            lethal_nodes: 300,
            ..AI_BUDGET
        }),
        300,
        "PREMISE: the solver spends its allowance here"
    );
    let spent = solver_nodes(SearchBudget {
        lethal_nodes: 10,
        ..AI_BUDGET
    });
    assert!(spent <= 10, "lethal_nodes 10 of 1200: the solver spent {spent}");
}

#[test]
fn r4802000_a_150_node_budget_keeps_its_solver_to_its_40_nodes() {
    let spent = solver_nodes(TEST_BUDGET);
    assert!(spent <= TEST_BUDGET.lethal_nodes, "the solver spent {spent}");
}

#[test]
fn r4802000_a_small_budget_still_leaves_the_beam_its_nodes() {
    let state = narrow_board();
    let alone = decided(
        &state,
        SearchBudget {
            nodes: 30,
            lethal_nodes: 0,
            ..AI_BUDGET
        },
    );
    assert_eq!(
        alone.reason,
        DecisionReason::Search,
        "PREMISE: 30 nodes search this board"
    );
    let small = decided(
        &state,
        SearchBudget {
            nodes: 30,
            lethal_nodes: 7,
            ..AI_BUDGET
        },
    );
    assert!(
        small.stats.lines > 0,
        "7 of 30 nodes were the solver's, yet the beam searched no line"
    );
    assert_ne!(small.reason, DecisionReason::Fallback);
}

#[test]
fn r4802000_a_budget_field_at_its_largest_value_does_not_overflow() {
    let state = narrow_board();
    let big = usize::MAX;
    let cases = [
        (
            "max_depth",
            SearchBudget {
                max_depth: big,
                ..AI_BUDGET
            },
        ),
        (
            "finalists",
            SearchBudget {
                finalists: big,
                ..AI_BUDGET
            },
        ),
        (
            "root_branching",
            SearchBudget {
                root_branching: big,
                ..AI_BUDGET
            },
        ),
        (
            "branching",
            SearchBudget {
                branching: big,
                ..AI_BUDGET
            },
        ),
        (
            "lethal_nodes",
            SearchBudget {
                lethal_nodes: big,
                ..AI_BUDGET
            },
        ),
    ];
    let mut failed: Vec<String> = Vec::new();
    for (name, budget) in cases {
        let decision = decided(&state, budget);
        if decision.stats.sim_panics > 0 {
            failed.push(format!(
                "{name}: {:?}, {:?}",
                decision.reason, decision.first_sim_panic
            ));
        }
    }
    assert!(failed.is_empty(), "{failed:#?}");
}
