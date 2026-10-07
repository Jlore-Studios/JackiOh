//! `redact` and last boards (R185, R417): a seat's last board is that player's own, as C+ #29's options
//! reach only their chooser (§10.8), so the AI keeps its own seat's and never reads the other's.
//!
//! Port of `packages/ai/test/redact-last-boards.test.ts`.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN};

fn mine() -> Value {
    json!([{ "defId": "core-012", "radiant": true }])
}

fn theirs() -> Value {
    json!([{ "defId": "core-025", "radiant": false }])
}

/// A value as its JSON, for comparisons that pin the wire shape rather than a Rust type's name.
fn js<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

fn with_last_boards(boards: Value) -> GameState {
    jackioh_cards::register_all();
    scenario(json!({ "active": "p1", "lastBoards": boards }))
        .state()
        .clone()
}

mod redact_keeps_only_the_seats_own_last_board_r185_r417 {
    use super::*;

    /// R417 the other seat's board is gone, the seat's own stays
    #[test]
    fn r417_the_other_seats_board_is_gone_the_seats_own_stays() {
        let state = with_last_boards(json!([mine(), theirs()]));
        // `{ [AI]: MINE }` and `{ [HUMAN]: THEIRS }`: the AI is p1 and the human p2 (support.rs).
        assert_eq!(js(&redact(&state, AI).last_boards), json!({ "p1": mine() }));
        assert_eq!(js(&redact(&state, HUMAN).last_boards), json!({ "p2": theirs() }));
    }

    /// R185 two states that differ only in the other seat's board redact to the same hash
    #[test]
    fn r185_two_states_that_differ_only_in_the_other_seats_board_redact_to_the_same_hash() {
        let one = with_last_boards(json!([mine(), theirs()]));
        let other = with_last_boards(json!([mine(), []]));
        assert_eq!(hash_state(&redact(&one, AI)), hash_state(&redact(&other, AI)));
        assert!(
            redact(&with_last_boards(json!([[], theirs()])), AI)
                .last_boards
                .is_none()
        );
    }
}
