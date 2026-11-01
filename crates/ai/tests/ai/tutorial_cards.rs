//! R2533000 (replacing R2433000's Core-only bar): the tutorial opponent plays only Training Grounds
//! cards that are not Legendary or Mythic, whatever reaches its hand (SPEC §9.10). The bar is on
//! `AI_TUTORIAL`'s seat and the engine keeps it in its legal moves, so the AI, which chooses among
//! them, never plays another card: not a Core card (#8 Mr. Vanilla), and not the set's Legendary or
//! Mythic cards (TG #37 Seance, TG #40 Goetzmann Game Design).

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, register_cards, scenario, trace};

const CORE_VANILLA: &str = "core-008";
const LEGENDARY: &str = "traininggrounds-037";
const MYTHIC: &str = "traininggrounds-040";
const TG_VANILLA: &str = "traininggrounds-043";

/// p1, the AI with 3 crystals, holding a Core Unit, a Legendary and a Mythic Training Grounds Spell
/// and a Common Training Grounds Unit, each castable, under AI_TUTORIAL or AI_TUTORIAL without the bar.
fn board(bar: bool) -> GameState {
    register_cards();
    let mut s = scenario(json!({
        "seed": "r2533000-ai",
        "active": AI,
        "turn": 9,
        "p1": { "hand": [CORE_VANILLA, LEGENDARY, MYTHIC, TG_VANILLA], "mana": 3 },
        "p2": { "field": ["core-019"] },
    }));
    let state = s.state_mut();
    state.players[AI].handicap = Some(Handicap {
        tutorial_cards_only: if bar { Some(true) } else { None },
        ..AI_TUTORIAL
    });
    state.clone()
}

/// The def ids the AI's turn played.
fn played(turn: &AiTurnResult, start: &GameState) -> Vec<String> {
    turn.actions
        .iter()
        .filter_map(|action| match &action.body {
            ActionBody::Play { instance_id, .. } => {
                find_instance(start, instance_id).map(|card| card.def_id.clone())
            }
            _ => None,
        })
        .collect()
}

mod r2533000_the_tutorial_opponent_plays_only_the_tutorials_cards {
    use super::*;

    #[test]
    fn r2533000_the_ai_under_ai_tutorial_holds_a_core_a_legendary_and_a_mythic_card_and_plays_none() {
        let barred = board(true);
        let mut ai = AiOptions::with_budget(create_rng("r2533000-ai-barred", 0), AI_BUDGET);
        let turn = play_ai_turn(&barred, AI, &mut ai);
        let plays = played(&turn, &barred);
        for id in [CORE_VANILLA, LEGENDARY, MYTHIC] {
            assert!(!plays.iter().any(|played| played == id), "{id}: {}", trace(&turn));
        }
        assert!(
            plays.iter().any(|id| id == TG_VANILLA),
            "the Training Grounds Common is still played: {}",
            trace(&turn)
        );
        let hand: Vec<&str> = turn.state.players[AI]
            .hand
            .iter()
            .map(|card| card.def_id.as_str())
            .collect();
        for id in [CORE_VANILLA, LEGENDARY, MYTHIC] {
            assert!(hand.contains(&id), "{id} stays in hand: {hand:?}");
        }

        // Without the bar, the same AI plays one: the bar, not the AI's taste, keeps them in hand.
        let open = board(false);
        let mut ai = AiOptions::with_budget(create_rng("r2533000-ai-open", 0), AI_BUDGET);
        let turn = play_ai_turn(&open, AI, &mut ai);
        let plays = played(&turn, &open);
        assert!(
            plays
                .iter()
                .any(|id| [CORE_VANILLA, LEGENDARY, MYTHIC].contains(&id.as_str())),
            "unbarred, a barred card is played: {}",
            trace(&turn)
        );
    }
}
