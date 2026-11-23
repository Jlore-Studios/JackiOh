//! R3695000: the engine's read of what a Float would take off each of a player's hand cards
//! (`float::float_cuts_in_hand`), which the AI's `float_credit` prices. It reads the "Float reduces
//! cost" hand triggers the cards' scripts register (R464, R1563000), the mark `float_reduces_cost`
//! puts on them, and no other trigger: not a deck card's, not the opponent's, not a Tribute Float's
//! (R1841003).

use jackioh_engine::float::{float_cuts_in_hand, float_lowers_tribute, float_reduces_cost};
use jackioh_engine::testkit::{PlayerId, json};

use super::scenario;

/// Drone Core IX and X (S #35, #36) and GIGA Glowy Jelly Bean (#29) carry "Float reduces cost" in hand;
/// Mr. Vanilla (#8) carries nothing.
#[test]
fn r3695000_a_float_of_n_cuts_n_off_each_hand_card_that_reads_it_and_no_other() {
    let s = scenario(json!({
        "seed": "float-cuts",
        "active": "p1",
        "p1": {
            "hand": ["stinkyscape-035", "core-029", "stinkyscape-036", "core-008"],
            "library": ["stinkyscape-036", "core-008"],
        },
        "p2": { "hand": ["stinkyscape-035", "core-008"] },
    }));
    let state = s.state();
    let cuts = |player, float| -> Vec<(String, i32)> {
        float_cuts_in_hand(state, player, float)
            .into_iter()
            .map(|(card, cut)| (card.def_id.clone(), cut))
            .collect()
    };
    let named = |cuts: &[(&str, i32)]| -> Vec<(String, i32)> {
        cuts.iter().map(|(id, cut)| ((*id).to_string(), *cut)).collect()
    };

    assert_eq!(
        cuts(PlayerId::P1, 3),
        named(&[("stinkyscape-035", 3), ("core-029", 3), ("stinkyscape-036", 3)]),
        "the hand's Float cards, in hand order; the deck's copy is no hand card"
    );
    assert_eq!(cuts(PlayerId::P2, 2), named(&[("stinkyscape-035", 2)]));
    assert!(cuts(PlayerId::P1, 0).is_empty(), "no Float, no cut");
    assert!(cuts(PlayerId::P1, -1).is_empty());
}

#[test]
fn r3695000_only_float_reduces_cost_is_marked() {
    assert!(float_reduces_cost("x").lowers_cost_on_float);
    assert!(
        !float_lowers_tribute("x").lowers_cost_on_float,
        "a Tribute Float (R1841003) lowers no cost"
    );
}
