//! R2201000: the tutorial opponent against a human who cannot lose (SPEC §9.10). A lesson's human
//! seat plays under `TUTORIAL_HUMAN`, whose hero never falls below `TUTORIAL_HERO_FLOOR`, so no hit on
//! it is lethal and the AI, which plays every tier alike, must see that: a face hit on a floored hero
//! at its floor does nothing, and the solver looks for no lethal there.
//!
//! The full game, the AI against a floored human who only ends turns, is `tutorial_tier`'s (the slow
//! binary).

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards, scenario, trace};

/// p1, the AI under AI_TUTORIAL, with a ready Midrange Menace 9/9; p2, the human at 1 health, with a
/// Mr. Vanilla 4/4, both libraries empty. Floored, the face is no win; the Menace kills the Vanilla
/// and lives.
fn board(floored: bool) -> GameState {
    register_cards();
    let mut s = scenario(json!({
        "seed": "r2201000-ai-floor",
        "active": AI,
        "turn": 9,
        "p1": { "field": ["core-019"], "mana": 0 },
        "p2": { "field": ["core-008"], "health": 1 },
    }));
    let state = s.state_mut();
    state.players[AI].handicap = Some(AI_TUTORIAL);
    if floored {
        state.players[HUMAN].handicap = Some(TUTORIAL_HUMAN);
    }
    state.clone()
}

mod r2201000_the_ai_against_a_human_who_cannot_lose {
    use super::*;

    #[test]
    fn r2201000_the_solver_finds_no_lethal_on_a_floored_hero_and_the_ai_trades_instead_of_hitting_its_floor()
    {
        // Unfloored, the Menace's swing is the win, and the AI takes it.
        let open = board(false);
        assert!(lethal_hint(&open, AI, None), "a 9/9 into a hero at 1 is lethal");
        let mut ai = AiOptions::with_budget(create_rng("r2201000-ai-open", 0), AI_BUDGET);
        let turn = play_ai_turn(&open, AI, &mut ai);
        let result = turn.state.result.as_ref().expect("the AI wins");
        assert_eq!(result.winner, Winner::from(AI), "{}", trace(&turn));

        // Floored, there is no lethal: the AI trades with the Vanilla, and the hero stays at 1.
        let floored = board(true);
        assert!(!lethal_hint(&floored, AI, None), "no lethal on a floored hero");
        assert!(ready_gap(&floored, AI).is_infinite());
        let id_of = |seat: PlayerId| active_units_of(&floored, seat)[0].id.clone();
        let (vanilla, menace) = (id_of(HUMAN), id_of(AI));
        let mut ai = AiOptions::with_budget(create_rng("r2201000-ai-floored", 0), AI_BUDGET);
        let first = decide(&floored, AI, &mut ai).expect("the AI decides");
        assert_eq!(
            first.action,
            ActionBody::Attack {
                attacker_id: menace,
                target_id: vanilla
            },
            "the Menace kills the Vanilla rather than hit a hero at its floor"
        );
        // Played on, the AI's turns (and the human's, which R82 ends for want of anything to do)
        // never take the hero below 1, and both libraries are empty: the AI fatigues to death.
        let mut ai = AiOptions::with_budget(create_rng("r2201000-ai-floored", 0), AI_BUDGET);
        let turn = play_ai_turn(&floored, AI, &mut ai);
        assert_eq!(turn.state.players[HUMAN].hero.health, 1, "{}", trace(&turn));
        if let Some(result) = &turn.state.result {
            assert_eq!(
                result.winner,
                Winner::from(HUMAN),
                "only the human can win: {}",
                trace(&turn)
            );
        }
    }
}
