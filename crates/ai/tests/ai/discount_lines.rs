//! R4595002: a child whose step made the seat's other hand cards cheaper (Infinite Assault #72
//! Overclock to Infinity's "Your cards cost (0) this turn") is played on beside the beam's best, so
//! the plays the discount pays for are searched, though a play that adds a unit outranks it a step in.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, register_cards, scenario};

const OVERCLOCK: &str = "infiniteassault-072";
const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt
const VANILLA: &str = "core-008"; // (1) Unit 4/4
const DECK: [&str; 3] = ["core-008", "core-011", "core-020"];

#[test]
fn r4595002_a_beam_one_line_wide_still_plays_on_from_a_discount() {
    register_cards();
    let s = scenario(json!({
        "seed": "r4595002", "active": AI, "turn": 9,
        "p1": { "hand": [{ "def": OVERCLOCK, "radiant": true }, MENACE, VANILLA], "mana": 4, "library": DECK },
        "p2": { "hand": ["core-011"], "library": DECK },
    }));
    let state = s.state();
    let overclock = s.card(OVERCLOCK).id.clone();
    let narrow = SearchBudget {
        beam_width: 1,
        ..AI_BUDGET
    };
    let lines = beam_search(
        state,
        AI,
        &create_node_counter(AI_BUDGET.nodes, None),
        narrow,
        Prices::NONE,
    );
    let opens_with_overclock = |line: &Line| matches!(line.actions.first(), Some(ActionBody::Play { instance_id, .. }) if *instance_id == overclock);
    let from_overclock: Vec<&Line> = lines.iter().filter(|line| opens_with_overclock(line)).collect();
    assert!(
        from_overclock.iter().any(|line| line.actions.len() >= 3),
        "a line plays the Menace and the 4/4 for (0) after the Overclock: {:?}",
        lines
            .iter()
            .map(|line| line.actions.iter().map(action_key).collect::<Vec<_>>())
            .collect::<Vec<_>>()
    );
}
