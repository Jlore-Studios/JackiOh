//! R2575000 (issue #2575): the practice AI's budget doubled, and the server's two other users of it
//! kept on the budget of before, each on a named constant: the ladder bots' middle strengths, whose
//! seed ratings were calibrated on it (R748000), and the Duos AI seats, so the server's work per AI
//! step does not double.

use jackioh_ai::SearchBudget;
use jackioh_server::config::{BOT_MID_STRENGTH_BUDGET, BOT_STRENGTHS, DUOS_AI_BUDGET};

/// The practice AI's `AI_BUDGET` before issue #2575.
const BEFORE: SearchBudget = SearchBudget {
    nodes: 600,
    lethal_nodes: 150,
    determinizations: 3,
    beam_width: 4,
    root_branching: 20,
    branching: 6,
    max_depth: 8,
    finalists: 3,
};

#[test]
fn r2575000_the_middle_ladder_strengths_keep_the_budget_their_ratings_were_calibrated_on() {
    assert_eq!(BOT_MID_STRENGTH_BUDGET, BEFORE);
    assert_eq!(BOT_STRENGTHS[2].budget, BOT_MID_STRENGTH_BUDGET);
    assert_eq!(BOT_STRENGTHS[3].budget, BOT_MID_STRENGTH_BUDGET);
    assert_ne!(
        jackioh_ai::AI_BUDGET,
        BEFORE,
        "the practice AI's own budget moved on"
    );
}

#[test]
fn r2575000_the_duos_ai_seats_keep_the_budget_of_before() {
    assert_eq!(DUOS_AI_BUDGET, BEFORE);
}
