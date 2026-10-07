//! R448 and R185: a card the opponent is setting face-down waits in their resolving zone while its
//! announce window is open (docs/classic-sets.md B5 E1), and the AI's seat may not read it there any
//! more than in the backrow (R33) — `redact` hides it, and `determinize` fills its placeholder from
//! the trap pool, as it fills a face-down backrow card.
//!
//! Port of `packages/ai/test/redact-announce.test.ts`.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, card_by_id, trap_pool};

/// `determinize` with TS's default options (`{}`).
fn det(public: &GameState, seat: PlayerId, rng: &mut Rng) -> GameState {
    determinize(public, seat, rng, DeterminizeOptions::default())
}

fn setting(face_down: bool) -> (GameState, String) {
    jackioh_cards::register_all();
    let mut state = scenario(json!({
        "seed": "redact-announce",
        "p1": { "hand": ["core-008"], "field": ["core-011"], "library": ["core-020", "core-053"] },
        "p2": { "hand": ["core-002"], "field": ["core-019"], "library": ["core-005", "core-016"] },
    }))
    .state()
    .clone();
    let card = new_instance(
        &mut state,
        if face_down { "core-041" } else { "core-002" },
        HUMAN,
        Zone::Resolving { player: HUMAN },
    );
    let id = card.id.clone();
    state.players[HUMAN].resolving.push(card);
    begin_announce(
        &mut state,
        AnnounceRecord {
            instance_id: id.clone(),
            player: HUMAN,
            face_down: if face_down { Some(true) } else { None },
            countered: None,
        },
    );
    (state, id)
}

mod r448_the_ai_never_reads_a_card_being_set_face_down {
    use super::*;

    /// R448 redact hides the opponent's card waiting to be set face-down, and determinize makes it a trap
    #[test]
    fn r448_redact_hides_the_opponents_card_waiting_to_be_set_face_down_and_determinize_makes_it_a_trap() {
        let (state, id) = setting(true);
        assert!(hidden_instance_ids(&state, AI).contains(&id));
        let seen = redact(&state, AI);
        assert_eq!(card_by_id(&seen, &id).map(|card| card.def_id.clone()), Some(HIDDEN_DEF_ID.to_string()));
        let world = det(&seen, AI, &mut create_rng("redact-announce-world", 0));
        let sampled = card_by_id(&world, &id).map(|card| card.def_id.clone()).unwrap_or_default();
        assert!(trap_pool().contains(&sampled), "{sampled}");
    }

    /// R448 the player setting it reads it, and a face-up play waiting in the window is public
    #[test]
    fn r448_the_player_setting_it_reads_it_and_a_face_up_play_waiting_in_the_window_is_public() {
        let (own_state, own_id) = setting(true);
        assert!(!hidden_instance_ids(&own_state, HUMAN).contains(&own_id));
        let (open_state, open_id) = setting(false);
        assert!(!hidden_instance_ids(&open_state, AI).contains(&open_id));
        assert_eq!(
            card_by_id(&redact(&open_state, AI), &open_id).map(|card| card.def_id.clone()),
            Some("core-002".to_string())
        );
    }
}
