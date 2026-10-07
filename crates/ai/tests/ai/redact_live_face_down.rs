//! R602 and R403: a face-down trap with no reveal condition is live (C #88 Siphon Squad's aura
//! shrinks the enemy's Attack from the moment it is set), and the units it shrinks show it to both
//! players. `redact` hides the card (R185, R33), so without more the AI's determinizations would give
//! its units their Attack back and it would declare attacks the engine refuses. It keeps every unit's
//! shown Attack and Health instead, and the card stays a placeholder.
//!
//! Port of `packages/ai/test/redact-live-face-down.test.ts`.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, card_by_id, is_legal, trap_pool};

/// `determinize` with TS's default options (`{}`).
fn det(public: &GameState, seat: PlayerId, rng: &mut Rng) -> GameState {
    determinize(public, seat, rng, DeterminizeOptions::default())
}

/// TS's `{ rng: createRng(seed) }`: the AI's own stream at AI_BUDGET, with no clock.
fn ai_options(seed: &str) -> AiOptions<'static> {
    AiOptions {
        rng: create_rng(seed, 0),
        budget: AI_BUDGET,
        should_stop: None,
    }
}

/// The card at the bottom of every unit pile of `player`'s (TS `pile[pile.length - 1]`).
fn pile_bottoms(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    state.players[player]
        .units
        .iter()
        .flatten()
        .filter_map(|pile| pile.last().cloned())
        .collect()
}

/// p1 (the AI) swings into p2, whose face-down Siphon Squad sets p1's units' Attack to 0 (Radiant).
fn siphoned(radiant: bool) -> GameState {
    jackioh_cards::register_all();
    scenario(json!({
        "seed": "redact-live-face-down",
        "active": "p1",
        "turn": 9,
        "p1": { "field": ["core-019", "core-011"], "hand": ["core-008"], "mana": 3, "library": ["core-020", "core-053"] },
        "p2": { "backrow": [{ "def": "classic-088", "faceUp": false, "radiant": radiant }], "library": ["core-005", "core-016"] },
    }))
    .state()
    .clone()
}

mod r602_the_ais_view_keeps_what_a_live_face_down_card_visibly_does {
    use super::*;

    /// R602 R403 redact keeps each unit's shown Attack and Health and still hides the card
    #[test]
    fn r602_r403_redact_keeps_each_units_shown_attack_and_health_and_still_hides_the_card() {
        for radiant in [false, true] {
            let state = siphoned(radiant);
            let trap = state.players[HUMAN]
                .backrow
                .iter()
                .flatten()
                .next()
                .cloned()
                .expect("a face-down trap");
            let seen = redact(&state, AI);
            assert_eq!(
                card_by_id(&seen, &trap.id).map(|card| card.def_id.clone()),
                Some(HIDDEN_DEF_ID.to_string())
            );
            for unit in pile_bottoms(&state, AI) {
                let truth = unit_view(&state, &unit);
                let seen_unit = card_by_id(&seen, &unit.id).cloned().expect("the unit is public");
                let shown = unit_view(&seen, &seen_unit);
                assert_eq!(
                    (shown.attack, shown.health),
                    (truth.attack, truth.health),
                    "{}, radiant {}",
                    unit.def_id,
                    radiant
                );
            }
        }
        // The Radiant face's "0 Attack" leaves the AI's units nothing to swing with, and its view says so.
        let zero = siphoned(true);
        for unit in zero.players[AI].units.iter().flatten().flatten() {
            assert_eq!(unit_view(&zero, unit).attack, 0);
        }
    }

    /// R602 every move the AI's determinizations offer is legal on the true board, and so is its decision
    #[test]
    fn r602_every_move_the_ais_determinizations_offer_is_legal_on_the_true_board_and_so_is_its_decision() {
        let state = siphoned(true);
        let legal: IndexSet<String> = legal_actions(&state, AI).iter().map(action_key).collect();
        assert!(!legal.iter().any(|key| key.contains("\"attack\"")));
        let seen = redact(&state, AI);
        for k in 0..8 {
            let world = det(
                &seen,
                AI,
                &mut create_rng(&format!("redact-live-face-down:{k}"), 0),
            );
            for candidate in candidate_actions(&world, AI) {
                let key = action_key(&candidate);
                assert!(legal.contains(&key), "{key}");
            }
        }
        let decision = decide(&state, AI, &mut ai_options("redact-live-face-down-decide"));
        assert!(decision.is_some());
        let decision = decision.expect("a decision");
        assert_ne!(decision.reason, DecisionReason::Fallback);
        assert!(is_legal(&state, AI, &decision.action));
    }
}

mod r602_a_determinization_agrees_with_the_board_it_was_dealt_from {
    use super::*;

    /// The same board as `siphoned`, but p2's face-down trap is a plain (2) one, Counterspell: no unit is shrunk. It shows (2), as Siphon Squad does, so R762 leaves Siphon Squad in the pool and R602 alone keeps it out.
    fn plain() -> GameState {
        jackioh_cards::register_all();
        scenario(json!({
            "seed": "determinize-live-face-down",
            "active": "p1",
            "turn": 9,
            "p1": { "field": ["core-019", "core-011"], "hand": ["core-008"], "mana": 3, "library": ["core-020", "core-053"] },
            "p2": { "backrow": [{ "def": "classic-017", "faceUp": false }], "library": ["core-005", "core-016"] },
        }))
        .state()
        .clone()
    }

    fn shown_by(state: &GameState) -> Vec<String> {
        pile_bottoms(state, AI)
            .iter()
            .map(|unit| {
                let view = unit_view(state, unit);
                format!("{}:{}/{}", unit.id, view.attack, view.max_health)
            })
            .collect()
    }

    /// R602 never puts a hidden trap with a live aura where the board shows none
    #[test]
    fn r602_never_puts_a_hidden_trap_with_a_live_aura_where_the_board_shows_none() {
        let state = plain();
        let scripts = registered_scripts();
        let auras: Vec<String> = trap_pool()
            .into_iter()
            .filter(|id| {
                scripts
                    .get(id)
                    .is_some_and(|card| card.base.aura.is_some() || card.radiant.aura.is_some())
            })
            .collect();
        assert!(
            auras.contains(&"classic-088".to_string()),
            "the pool has a live-aura trap to rule out"
        );

        let seen = redact(&state, AI);
        let truth = shown_by(&state);
        let mut picked: IndexSet<String> = IndexSet::new();
        for k in 0..300 {
            let world = det(
                &seen,
                AI,
                &mut create_rng(&format!("determinize-live-face-down:{k}"), 0),
            );
            let hidden = world.players[HUMAN]
                .backrow
                .iter()
                .flatten()
                .next()
                .cloned()
                .expect("the hidden trap");
            picked.insert(hidden.def_id.clone());
            assert_eq!(shown_by(&world), truth, "seed {k}, {}", hidden.def_id);
        }
        for id in &auras {
            assert!(!picked.contains(id), "{id}");
        }
        // R762 keeps every sample at the shown (2); of those, only the ones R602 rules out are gone.
        let mut sorted: Vec<String> = picked.into_iter().collect();
        sorted.sort();
        let expected: Vec<String> = trap_pool()
            .into_iter()
            .filter(|id| def_of(Some(&state), id).cost == CardCost::Fixed(2) && !auras.contains(id))
            .collect();
        assert_eq!(sorted, expected);
    }
}
