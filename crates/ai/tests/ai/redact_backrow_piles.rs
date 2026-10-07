//! R185 and B5 E21 (R447): a card dormant under a backrow pile is on the board, and a face-down one there
//! is as hidden from the other seat as a face-down trap on top — `redact` makes it a placeholder,
//! `determinize` fills it from the trap pool, and a carrier's Unit (R446) stays a public card. Real
//! catalog, real dealt game; the AI is p1 as in every AI test.
//!
//! Port of `packages/ai/test/redact-backrow-piles.test.ts`.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{dealt_game, trap_pool};

/// `determinize` with TS's default options (`{}`).
fn det(public: &GameState, seat: PlayerId, rng: &mut Rng) -> GameState {
    determinize(public, seat, rng, DeterminizeOptions::default())
}

/// TS's module constant `FIELD_SPELL`: the first Core Field Spell, in `query`'s order.
fn field_spell() -> String {
    jackioh_cards::register_all();
    query(&json_as(json!({ "set": "Core", "type": ["Field Spell"] })))
        .first()
        .map(|def| def.id.clone())
        .unwrap_or_default()
}

/// The defId of the first card dormant under `player`'s first backrow pile.
fn buried_def(state: &GameState, player: PlayerId) -> Option<String> {
    state.players[player]
        .backrow_piles
        .as_ref()
        .and_then(|piles| piles.first())
        .and_then(|pile| pile.first())
        .map(|card| card.def_id.clone())
}

/// p2 sets `trap_def` in backrow lane 1 and tops it with a public Field Spell (a Stack card would).
fn pile_game(seed: &str, trap_def: &str) -> (GameState, String, String) {
    let mut state = dealt_game(seed);
    let slot = ZoneSlot { player: PlayerId::P2, row: Row::Backrow, lane: 1 };
    let mut trap = new_instance(&mut state, trap_def, PlayerId::P2, Zone::Hand { player: PlayerId::P2 });
    if !place_on_field(&mut state, &mut trap, &slot, PlaceOnFieldOptions::default()) {
        panic!("no zone");
    }
    let mut top = new_instance(&mut state, &field_spell(), PlayerId::P2, Zone::Hand { player: PlayerId::P2 });
    if !place_on_field(&mut state, &mut top, &slot, PlaceOnFieldOptions { stack: Some(true) }) {
        panic!("no stack");
    }
    (state, trap.id.clone(), top.id.clone())
}

mod r447_the_ais_seat_and_a_backrow_pile {
    use super::*;

    /// R447 a face-down trap under the opponent's pile is hidden, placeholdered and resampled from the trap pool
    #[test]
    fn r447_a_face_down_trap_under_the_opponents_pile_is_hidden_placeholdered_and_resampled_from_the_trap_pool() {
        let traps = trap_pool();
        let first = traps.first().cloned().unwrap_or_default();
        let (state, trap, top) = pile_game("ai-backrow-pile", &first);
        let hidden = hidden_instance_ids(&state, PlayerId::P1);
        assert!(hidden.contains(&trap));
        assert!(!hidden.contains(&top));

        let seen = redact(&state, PlayerId::P1);
        assert_eq!(buried_def(&seen, PlayerId::P2), Some(HIDDEN_DEF_ID.to_string()));
        assert_eq!(
            seen.players[PlayerId::P2].backrow.first().and_then(|card| card.as_ref()).map(|card| card.def_id.clone()),
            Some(field_spell())
        );

        // Two true states that differ only in the buried trap look the same to the seat.
        let (other, _, _) = pile_game("ai-backrow-pile", traps.get(1).unwrap_or(&first));
        assert_eq!(hash_state(&redact(&other, PlayerId::P1)), hash_state(&seen));

        let world = det(&seen, PlayerId::P1, &mut create_rng("ai-backrow-pile-world", 0));
        let sampled = buried_def(&world, PlayerId::P2).unwrap_or_default();
        assert!(traps.contains(&sampled), "{sampled}");
    }

    /// R447 the seat's own buried trap stays readable to it
    #[test]
    fn r447_the_seats_own_buried_trap_stays_readable_to_it() {
        let first = trap_pool().first().cloned().unwrap_or_default();
        let (state, trap, _) = pile_game("ai-backrow-pile-own", &first);
        assert!(!hidden_instance_ids(&state, PlayerId::P2).contains(&trap));
        assert_eq!(buried_def(&redact(&state, PlayerId::P2), PlayerId::P2), trap_pool().first().cloned());
    }
}
