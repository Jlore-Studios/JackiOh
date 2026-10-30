//! R2433000: the tutorial opponent plays only Core cards, whatever reaches its hand (SPEC §9.10). The
//! bar is on `AI_TUTORIAL`'s seat and the engine keeps it in its legal moves, so the AI, which chooses
//! among them, never plays a card of another set: not a Classic card, and not a Training Grounds
//! snapshot of a Core card (#43's Mr. Vanilla).

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, register_cards, scenario, trace};

const CORE_VANILLA: &str = "core-008";
const SNAPSHOT_VANILLA: &str = "traininggrounds-043";
const CLASSIC_UNIT: &str = "classic-041";

/// p1, the AI with 3 crystals, holding a Classic Unit, a Training Grounds snapshot and a Core Unit,
/// each costing 1, under AI_TUTORIAL or AI_TUTORIAL without the bar.
fn board(bar: bool) -> GameState {
    register_cards();
    let mut s = scenario(json!({
        "seed": "r2433000-ai",
        "active": AI,
        "turn": 9,
        "p1": { "hand": [CLASSIC_UNIT, SNAPSHOT_VANILLA, CORE_VANILLA], "mana": 3 },
        "p2": { "field": ["core-019"] },
    }));
    let state = s.state_mut();
    state.players[AI].handicap = Some(Handicap {
        core_only: if bar { Some(true) } else { None },
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

mod r2433000_the_tutorial_opponent_plays_only_core_cards {
    use super::*;

    #[test]
    fn r2433000_the_ai_under_ai_tutorial_holds_a_classic_card_and_a_snapshot_and_plays_neither() {
        let barred = board(true);
        let mut ai = AiOptions::with_budget(create_rng("r2433000-ai-barred", 0), AI_BUDGET);
        let turn = play_ai_turn(&barred, AI, &mut ai);
        let plays = played(&turn, &barred);
        assert!(!plays.iter().any(|id| id == CLASSIC_UNIT), "{}", trace(&turn));
        assert!(!plays.iter().any(|id| id == SNAPSHOT_VANILLA), "{}", trace(&turn));
        assert!(
            plays.iter().any(|id| id == CORE_VANILLA),
            "the Core card is still played: {}",
            trace(&turn)
        );
        let hand: Vec<&str> = turn.state.players[AI]
            .hand
            .iter()
            .map(|card| card.def_id.as_str())
            .collect();
        assert!(
            hand.contains(&CLASSIC_UNIT) && hand.contains(&SNAPSHOT_VANILLA),
            "{hand:?}"
        );

        // Without the bar, the same AI plays them: the bar, not the AI's taste, keeps them in hand.
        let open = board(false);
        let mut ai = AiOptions::with_budget(create_rng("r2433000-ai-open", 0), AI_BUDGET);
        let turn = play_ai_turn(&open, AI, &mut ai);
        let plays = played(&turn, &open);
        assert!(
            plays
                .iter()
                .any(|id| id == CLASSIC_UNIT || id == SNAPSHOT_VANILLA),
            "unbarred, a non-Core card is played: {}",
            trace(&turn)
        );
    }
}
