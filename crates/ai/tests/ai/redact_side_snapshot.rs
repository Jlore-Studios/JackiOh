//! R3550007, Stinkyscape+ #84 Bogos Binted's side snapshot: `@sideSnapshot` dropped from the
//! opponent's cards so the seat never reads what their Cry saw.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, clone, dealt_game, trap_pool};

fn snapshot_of(state: &GameState, id: &str) -> Option<Value> {
    find_instance(state, id).and_then(|card| {
        card.memory
            .get(jackioh_engine::subsystems::board_history::SIDE_SNAPSHOT_KEY)
            .cloned()
    })
}

mod r3550007_redact_drops_the_opponents_side_snapshot {
    use super::*;

    /// R3550007 redact drops the opponent's side snapshot, so what their Cry saw never reaches the
    /// seat
    #[test]
    fn r3550007_redact_drops_the_opponents_side_snapshot() {
        let mut state = dealt_game("ai-side-snapshot");
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
        let key = jackioh_engine::subsystems::board_history::SIDE_SNAPSHOT_KEY;
        if let Some(card) = find_instance_mut(&mut state, &trap.id) {
            card.memory
                .insert(key.to_string(), json!({ "turn": 1, "side": "snapshot" }));
        }
        assert!(snapshot_of(&state, &trap.id).is_some());

        let seen = redact(&state, AI);
        assert!(snapshot_of(&seen, &trap.id).is_none());

        // What the snapshot holds changes nothing the seat reads.
        let mut other = clone(&state);
        if let Some(card) = find_instance_mut(&mut other, &trap.id) {
            card.memory
                .insert(key.to_string(), json!({ "turn": 2, "side": "other" }));
        }
        assert_ne!(hash_state(&other), hash_state(&state));
        assert_eq!(hash_state(&redact(&other, AI)), hash_state(&seen));
    }
}
