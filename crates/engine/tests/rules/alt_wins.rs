//! ME-WIN's held wins (SPEC §2.5, R848–R850): Meditative #8 and #20's systems, proved here
//! through fixture-free effects so the engine owns them.
//!
//! Port of nothing shipped: no shipped card holds a win.

use jackioh_engine::effects::{alt_win, win_game};
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::{Effect, EngineSink};
use jackioh_engine::testkit::*;
use jackioh_engine::triggers::{SettleOptions, settle};

use crate::rules::fixtures::harness::{new_game, put, slot};

/// Apply effects the way `resolve.ts` does, then settle: prompts drain, the board settles and the
/// game-end point of the state check runs.
fn run_settled(state: &mut GameState, effects: &[Effect], controller: PlayerId) {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        {
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(controller),
                    ..Default::default()
                },
            );
            apply_effects(effects, &mut ctx);
        }
        settle(&mut sink, SettleOptions::default());
    }
    state.rng_cursor = rng.cursor();
}

fn hold(state: &mut GameState, condition: &str, threshold: i32, controller: PlayerId) {
    run_settled(
        state,
        &[alt_win(json_as(
            json!({ "condition": condition, "threshold": threshold }),
        ))],
        controller,
    );
}

fn labels(state: &GameState, viewer: PlayerId, side: PlayerId) -> Vec<String> {
    let view = view_for(state, viewer);
    let side = if side == viewer { &view.you } else { &view.opponent };
    side.modifiers.iter().map(|badge| badge.label.clone()).collect()
}

mod r848_held_conditions {
    use super::*;

    #[test]
    fn r848_a_condition_lasts_and_any_one_kept_wins() {
        let mut state = new_game("alt-win-basic", None);
        // Health 30 meets a threshold of 30 at once.
        hold(&mut state, "health", 30, PlayerId::P1);
        let kept: Vec<String> = state.players.p1.mods.iter().map(|m| m.id.clone()).collect();
        assert_eq!(kept.len(), 1, "the condition is kept");
        let result = state.result.expect("a met condition wins");
        assert_eq!(result.winner, Winner::P1);
        assert_eq!(result.reason, GameOverReason::AltWin);

        // Several kept conditions win on any one of them: an unmet graveyard need beside a met one.
        let mut state = new_game("alt-win-two", None);
        hold(&mut state, "graveyard", 100, PlayerId::P1);
        assert!(state.result.is_none(), "an unmet condition wins nothing");
        hold(&mut state, "health", 30, PlayerId::P1);
        assert_eq!(state.players.p1.mods.len(), 2, "both conditions are kept");
        let result = state.result.expect("any one kept win wins");
        assert_eq!(result.winner, Winner::P1);
        assert_eq!(result.reason, GameOverReason::AltWin);
    }

    #[test]
    fn r848_the_badge_shows_progress_on_both_seats() {
        let mut state = new_game("alt-win-badge", None);
        hold(&mut state, "health", 100, PlayerId::P1);
        assert!(state.result.is_none());
        for viewer in [PlayerId::P1, PlayerId::P2] {
            let seen = labels(&state, viewer, PlayerId::P1);
            assert!(
                seen.contains(&"You win at 100 hero Health (30/100)".to_string()),
                "progress on p1's side for {viewer}: {seen:?}"
            );
        }
    }
}

mod r849_board_condition {
    use super::*;

    #[test]
    fn r849_the_board_needs_both_totals() {
        // One 2/2: attack short of 3 and health short of 3.
        let mut attack_short = new_game("alt-win-board-attack", None);
        put(
            &mut attack_short,
            "fx-1",
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        hold(&mut attack_short, "board", 3, PlayerId::P1);
        assert!(attack_short.result.is_none(), "2 attack is short of 3");

        // Two 2/2s: both totals at 4 meet a threshold of 4.
        let mut both_met = new_game("alt-win-board-both", None);
        for lane in [1, 2] {
            put(
                &mut both_met,
                "fx-1",
                slot(PlayerId::P1, Row::Units, lane),
                Default::default(),
            );
        }
        hold(&mut both_met, "board", 4, PlayerId::P1);
        let result = both_met.result.expect("4/4 meets 4");
        assert_eq!(result.winner, Winner::P1);
        assert_eq!(result.reason, GameOverReason::AltWin);
    }
}

mod r850_the_game_end_point {
    use super::*;

    #[test]
    fn r850_a_hero_at_0_loses_even_holding_a_win() {
        let mut state = new_game("alt-win-loss-first", None);
        state.players.p1.hero.health = 0;
        // A held win (an outright one, so it holds however low the hero is) still loses to the
        // hero check, which runs first.
        run_settled(&mut state, &[win_game(json_as(json!({})))], PlayerId::P1);
        assert_eq!(state.players.p1.won_by_effect, Some(true), "the win is held");
        let result = state.result.expect("a hero at 0 ends the game");
        assert_eq!(result.winner, Winner::P2);
        assert_eq!(result.reason, GameOverReason::HeroDeath);
    }

    #[test]
    fn r850_two_winners_draw() {
        // Both players hold a met win before the same check: one settle, a draw. The two grants
        // run back to back with no check between them, as a single effect list would.
        let mut state = new_game("alt-win-draw-both", None);
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut events = Vec::new();
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            for controller in [PlayerId::P1, PlayerId::P2] {
                let mut ctx = make_context(
                    &mut sink,
                    None,
                    HookOptions {
                        controller: Some(controller),
                        ..Default::default()
                    },
                );
                apply_effects(
                    &[alt_win(json_as(
                        json!({ "condition": "health", "threshold": 30 }),
                    ))],
                    &mut ctx,
                );
            }
            settle(&mut sink, SettleOptions::default());
        }
        state.rng_cursor = rng.cursor();
        let result = state.result.expect("two winners draw");
        assert_eq!(result.winner, Winner::Draw);
        assert_eq!(result.reason, GameOverReason::AltWin);
    }

    #[test]
    fn r850_win_game_wins_at_the_closing_check_with_won_by_effect() {
        let mut state = new_game("won-by-effect", None);
        run_settled(&mut state, &[win_game(json_as(json!({})))], PlayerId::P1);
        let result = state.result.expect("win_game wins");
        assert_eq!(result.winner, Winner::P1);
        assert_eq!(result.reason, GameOverReason::WonByEffect);
        assert_eq!(state.players.p1.won_by_effect, Some(true));
    }

    #[test]
    fn r850_no_condition_leaves_the_state_json_unchanged() {
        let mut state = new_game("alt-win-quiet", None);
        let before = serde_json::to_value(&state).expect("a state serialises");
        run_settled(&mut state, &[], PlayerId::P1);
        assert_eq!(before, serde_json::to_value(&state).expect("a state serialises"));
    }
}
