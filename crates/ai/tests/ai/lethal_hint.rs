//! R2102000: the Easy practice tier's lethal hint (`lethal_hint`, SPEC §9.9): whether the human has a
//! sure win this turn by what they can see, and the same answer whatever the AI's hidden cards are.
//!
//! The human sits in p2 with Tempo Timmy 3/3 and Mr. Vanilla 4/4 ready on the field, 7 to the face;
//! the AI sits in p1 with nothing on its field. The AI's hidden cards come in the two sets the
//! observe tests' fixed pair uses (#2 Bigot and #11 Tempo Timmy against #53 Reno and #20
//! Pointmaster in hand, different libraries, a face-down #41 Sheepish against a face-down #60 Bear
//! Honeypot, which show the same cost). `observe.rs` holds the property over real states.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{HUMAN, register_cards};

/// The AI's hidden cards: its hand, its library and the face-down trap it may have set.
struct Hidden {
    hand: [&'static str; 2],
    library: [&'static str; 3],
    trap: &'static str,
}

const SET_A: Hidden = Hidden {
    hand: ["core-002", "core-011"],
    library: ["core-005", "core-016", "core-047"],
    trap: "core-041",
};

const SET_B: Hidden = Hidden {
    hand: ["core-053", "core-020"],
    library: ["core-063", "core-044", "core-072"],
    trap: "core-060",
};

/// p2's 7 attack facing p1 at `health`, p1 holding `hidden` (its trap set face down when `trapped`),
/// with `active` the seat whose turn it is.
fn board(seed: &str, active: PlayerId, hidden: &Hidden, trapped: bool, health: i32) -> GameState {
    register_cards();
    let backrow = if trapped {
        json!([{ "def": hidden.trap, "faceUp": false }])
    } else {
        json!([])
    };
    scenario(json!({
        "seed": seed,
        "active": active,
        "p1": { "hand": hidden.hand, "library": hidden.library, "backrow": backrow, "health": health },
        "p2": { "field": ["core-011", "core-008"] },
    }))
    .state()
    .clone()
}

/// R2102000: two swings for 7 at an AI on 6, with nothing in the way, light the hint
#[test]
fn r2102000_a_board_the_human_wins_this_turn_lights_the_hint() {
    let state = board("lethal-hint", PlayerId::P2, &SET_A, false, 6);
    assert!(lethal_hint(&state, HUMAN, None));
}

/// R2102000: 7 to the face of an AI on 20 is no win, so the hint stays dark
#[test]
fn r2102000_a_board_the_human_cannot_win_this_turn_lights_nothing() {
    let state = board("lethal-hint", PlayerId::P2, &SET_A, false, 20);
    assert!(!lethal_hint(&state, HUMAN, None));
}

/// R2102000: a clock that has run out (the browser's cap) stops the search before it proves the
/// lethal, and a search the clock stopped lights nothing
#[test]
fn r2102000_a_search_the_clock_stops_lights_nothing() {
    let state = board("lethal-hint", PlayerId::P2, &SET_A, false, 6);
    let stopped = || true;
    assert!(!lethal_hint(&state, HUMAN, Some(&stopped)));
    let running = || false;
    assert!(lethal_hint(&state, HUMAN, Some(&running)));
}

/// R2102000: on the AI's turn the human can win nothing this turn, whatever their board would deal
#[test]
fn r2102000_off_the_humans_own_turn_the_hint_is_dark() {
    let state = board("lethal-hint", PlayerId::P1, &SET_A, false, 6);
    assert!(!lethal_hint(&state, HUMAN, None));
}

/// R2102000, R185: states that differ only in the AI's hand, library, face-down trap and the seed
/// redact alike and give one answer, with and without a trap and with and without a lethal
#[test]
fn r2102000_two_states_that_differ_only_in_the_ais_hidden_cards_give_one_answer() {
    for trapped in [false, true] {
        for health in [6, 20] {
            let a = board("lethal-hint-a", PlayerId::P2, &SET_A, trapped, health);
            let b = board("lethal-hint-b", PlayerId::P2, &SET_B, trapped, health);
            let case = format!("trapped {trapped}, health {health}");
            assert_ne!(hash_state(&a), hash_state(&b), "{case}");
            assert_eq!(
                hash_state(&redact(&a, HUMAN)),
                hash_state(&redact(&b, HUMAN)),
                "{case}"
            );
            let hint = lethal_hint(&a, HUMAN, None);
            assert_eq!(lethal_hint(&b, HUMAN, None), hint, "{case}");
            if !trapped {
                assert_eq!(hint, health == 6, "{case}");
            }
        }
    }
}
