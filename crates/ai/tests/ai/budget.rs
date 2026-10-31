//! R2575000 (issue #2575): the practice AI's budget, doubled to 1,200 nodes and 300 lethal nodes in
//! the search shape of before, and what follows it.

use jackioh_ai::{
    AI_BUDGET, AI_GATE_BUDGET, AI_SEARCH, LETHAL_HINT_DETERMINIZATIONS, LETHAL_HINT_NODES, SearchBudget,
};

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
