//! R185 and C+ #35 Rollback's history (R419): `state.boardHistory` holds whole instances of the field as
//! each recent turn began — the opponent's face-down traps among them, and cards since gone to its hand —
//! so `redact` drops it and the seat never reads a hidden card through it. Two true states that differ
//! only in a snapshot's hidden card redact to the same hash. Real catalog, real dealt game; the AI is p1.
//!
//! Port of `packages/ai/test/redact-board-history.test.ts`.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, clone, dealt_game, trap_pool};

/// The defId of the card in p2's backrow lane 2 in the newest board snapshot.
fn snapshot_lane_two(state: &GameState) -> Option<String> {
    state
        .board_history
        .as_ref()
        .and_then(|history| history.last())
        .and_then(|snapshot| snapshot.sides.p2.backrow.get(1).cloned().flatten())
        .map(|card| card.def_id)
}

mod r185_the_ais_seat_and_the_board_history_r419 {
    use super::*;

    /// R185 redact drops the board history, so a snapshot's face-down trap never reaches the seat
    #[test]
    fn r185_redact_drops_the_board_history_so_a_snapshots_face_down_trap_never_reaches_the_seat() {
        let mut state = dealt_game("ai-board-history");
        let traps = trap_pool();
        let mut trap = new_instance(
            &mut state,
            traps.first().map(String::as_str).unwrap_or(""),
            PlayerId::P2,
            Zone::Hand { player: PlayerId::P2 },
        );
        let slot = ZoneSlot {
            player: PlayerId::P2,
            row: Row::Backrow,
            lane: 2,
        };
        if !place_on_field(&mut state, &mut trap, slot, PlaceOnFieldOptions::default()) {
            panic!("no zone");
        }
        subsystems::board_history::record_board_snapshot(&mut state);
        assert_eq!(snapshot_lane_two(&state), Some(trap.def_id.clone()));

        let seen = redact(&state, AI);
        assert!(seen.board_history.is_none());
        assert!(state.board_history.is_some());

        let mut other = clone(&state);
        {
            let copy = other
                .board_history
                .as_mut()
                .and_then(|history| history.last_mut())
                .and_then(|snapshot| snapshot.sides.p2.backrow.get_mut(1))
                .and_then(|card| card.as_mut())
                .unwrap_or_else(|| panic!("no snapshot copy"));
            if let Some(next) = traps.get(1) {
                copy.def_id = next.clone();
            }
        }
        assert_ne!(hash_state(&other), hash_state(&state));
        assert_eq!(hash_state(&redact(&other, AI)), hash_state(&seen));
    }
}
