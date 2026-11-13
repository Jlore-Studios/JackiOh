//! A GIGA card from the AI's hand in the root's probes (R3575202): a GIGA Field Spell or Unit fills its
//! anchor's lane and the next in both rows (R3546000), so the probes that set or place a hand card first
//! (R923001: the turn hooks', the Deaths', the awakenings') put it at the leftmost anchor its four zones
//! fit, not in the first open zone, whose other-row lane may be taken.
//!
//! The cards: Laser Drill (S+ #53), a (4) GIGA Field Spell, "End of turn: Add three random Minerals (that
//! cost (0)) or Resources to your hand"; Mr. Vanilla (core-008), a (1) 1/1.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, close, register_cards};

const LASER_DRILL: &str = "stinkyscapeplus-053";
const VANILLA: &str = "core-008";
const LIBRARY: &[&str] = &[VANILLA, VANILLA, VANILLA, VANILLA];

/// The AI's turn with a Laser Drill in hand and Mr. Vanillas in its unit `lanes`.
fn drill_in_hand(lanes: &[i32]) -> GameState {
    register_cards();
    let field: Vec<Value> = lanes
        .iter()
        .map(|lane| json!({ "def": VANILLA, "lane": lane }))
        .collect();
    scenario(json!({
        "seed": "ai-giga-probe",
        "p1": { "hand": [LASER_DRILL], "mana": 4, "field": field, "library": LIBRARY },
        "p2": { "hand": [VANILLA], "library": LIBRARY },
    }))
    .state()
    .clone()
}

fn drill_price(state: &GameState) -> Option<f64> {
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    turn_hook_prices(state, AI, &counter)
        .into_iter()
        .find(|entry| entry.def_id == LASER_DRILL)
        .map(|entry| entry.price)
}

#[test]
fn r3575202_a_hand_giga_field_spell_is_priced_where_its_four_zones_fit() {
    let open = drill_price(&drill_in_hand(&[])).expect("priced on an open board");
    assert!(open > 0.0, "three finds are worth something: {open}");
    // The first open backrow zone, lane 1, has its unit lane taken: the anchor is lane 2.
    let crowded = drill_price(&drill_in_hand(&[1])).expect("priced beside a unit in lane 1");
    close(crowded, open);
}

#[test]
fn r3575202_a_hand_giga_field_spell_with_no_fitting_anchor_is_not_priced() {
    // Units in lanes 1, 3 and 5 leave no two neighbouring unit zones open, so no play is offered and
    // nothing is priced.
    let state = drill_in_hand(&[1, 3, 5]);
    assert!(
        !legal_actions(&state, AI)
            .iter()
            .any(|action| matches!(action, ActionBody::Play { .. })),
        "the drill fits nowhere"
    );
    assert_eq!(drill_price(&state), None);
}
