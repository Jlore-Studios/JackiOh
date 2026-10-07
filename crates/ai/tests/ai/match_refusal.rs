//! What `play_match` does when a controller misbehaves (docs/polish/3-ai.md §Surface, match.rs): "A
//! refused action is recorded in `rejected` and replaced by endTurn (or the first legal answer); a
//! throw is recorded and ends the match", and a controller that returns nothing is counted in
//! `fallbacks` and replaced by the random policy. No real controller does any of this (the gates
//! insist on zero of each), so this file scripts the baselines: `random_action` or `greedy_action`
//! misbehaves once, at the first main phase the scripted seat owns, and is the real one otherwise.
//!
//! Port of `packages/ai/test/match-refusal.test.ts`. TS scripted the baselines with `vi.mock`, which
//! Rust has no twin of: the controller's answer is rewritten through `MatchHooks.override_choice`
//! instead (called with each controller call's answer, inside the call, before a null is replaced),
//! and a scripted throw is a panic there. That seam is a GAP for `match_.rs` (see the notes).

use std::cell::OnceCell;

use jackioh_ai::{MatchConfig, MatchHooks, MatchRecord, SeatController, play_match};
use jackioh_engine::testkit::{
    ActionBody, FoldArgs, GameState, PerPlayer, Phase, PlayerId, fold, hash_state, json, json_as, opponent_of,
};

use super::support::random_decks;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Misbehaviour {
    Refuse,
    Throw,
    Null,
}

fn config(seed: &str, p1: SeatController) -> MatchConfig {
    MatchConfig {
        seed: seed.to_string(),
        decks: random_decks(seed),
        handicaps: None,
        controllers: PerPlayer::new(p1, SeatController::Random),
        max_actions: None,
    }
}

fn expect_folds(cfg: &MatchConfig, record: &MatchRecord) {
    let replayed = fold(&json_as::<FoldArgs>(
        json!({ "seed": cfg.seed, "decks": cfg.decks, "log": record.log }),
    ));
    assert!(replayed.errors.is_empty());
    assert_eq!(hash_state(&replayed.state), record.hash);
}

/// TS's `due`: the scripted controller has not misbehaved yet and owns a main phase with no prompt.
fn due(fired_on: &OnceCell<GameState>, state: &GameState, seat: PlayerId) -> bool {
    fired_on.get().is_none() && state.phase == Phase::Main && state.pending.is_none() && state.active == seat
}

/// TS's mocked baselines: `random_action` refuses or throws once (`Refuse`, `Throw`), `greedy_action`
/// answers null once (`Null`), each at the first state `due` holds for; every other call is the real one.
fn scripted<'a>(
    mode: Misbehaviour,
    cfg: &'a MatchConfig,
    fired_on: &'a OnceCell<GameState>,
) -> impl FnMut(&GameState, PlayerId, Option<ActionBody>) -> Option<ActionBody> + 'a {
    move |state: &GameState, seat: PlayerId, action: Option<ActionBody>| {
        let controller = &cfg.controllers[seat];
        let random = matches!(controller, SeatController::Random);
        let greedy = matches!(controller, SeatController::Greedy);
        if random && matches!(mode, Misbehaviour::Refuse | Misbehaviour::Throw) && due(fired_on, state, seat)
        {
            let _ = fired_on.set(state.clone());
            if mode == Misbehaviour::Throw {
                panic!("scripted controller failure");
            }
            return Some(ActionBody::Attack {
                attacker_id: "no-such-unit".to_string(),
                target_id: format!("hero-{}", opponent_of(seat)),
            });
        }
        if greedy && mode == Misbehaviour::Null && due(fired_on, state, seat) {
            let _ = fired_on.set(state.clone());
            return None;
        }
        action
    }
}

mod surface_play_match_when_a_controller_misbehaves {
    use super::*;

    #[test]
    fn a_refused_action_is_recorded_in_rejected_and_replaced_by_end_turn_and_the_match_plays_on_and_replays()
    {
        let cfg = config("refusal-endturn", SeatController::Random);
        let fired_on: OnceCell<GameState> = OnceCell::new();
        let mut replacement: Option<ActionBody> = None;
        let record = {
            let mut hooks = MatchHooks {
                override_choice: Some(Box::new(scripted(Misbehaviour::Refuse, &cfg, &fired_on))),
                after_action: Some(Box::new(
                    |before: &GameState, _after: &GameState, seat: PlayerId, action: &ActionBody| {
                        if fired_on.get() == Some(before) && seat == PlayerId::P1 {
                            replacement = Some(action.clone());
                        }
                    },
                )),
                ..MatchHooks::default()
            };
            play_match(&cfg, &mut hooks)
        };

        assert!(fired_on.get().is_some());
        assert_eq!(record.rejected.len(), 1);
        assert_eq!(record.rejected[0].seat, PlayerId::P1);
        assert_eq!(
            record.rejected[0].action,
            ActionBody::Attack {
                attacker_id: "no-such-unit".to_string(),
                target_id: "hero-p2".to_string(),
            }
        );
        assert!(!record.rejected[0].error.is_empty());
        assert_eq!(replacement, Some(ActionBody::EndTurn));
        assert!(record.thrown.is_empty());
        assert!(record.result.is_some());
        expect_folds(&cfg, &record);
    }

    #[test]
    fn a_controller_that_throws_is_recorded_in_thrown_and_ends_the_match_with_no_result() {
        let cfg = config("refusal-throw", SeatController::Random);
        let fired_on: OnceCell<GameState> = OnceCell::new();
        let record = {
            let mut hooks = MatchHooks {
                override_choice: Some(Box::new(scripted(Misbehaviour::Throw, &cfg, &fired_on))),
                ..MatchHooks::default()
            };
            play_match(&cfg, &mut hooks)
        };

        assert_eq!(record.thrown.len(), 1);
        assert_eq!(record.thrown[0].seat, PlayerId::P1);
        assert!(record.thrown[0].message.contains("scripted controller failure"));
        assert!(record.result.is_none());
        assert!(record.rejected.is_empty());
        // The log stops at the throw, and what it holds still replays to the record's hash.
        expect_folds(&cfg, &record);
    }

    #[test]
    fn a_controller_that_returns_nothing_counts_one_fallback_and_the_random_policy_moves_for_it() {
        let cfg = config("refusal-null", SeatController::Greedy);
        let fired_on: OnceCell<GameState> = OnceCell::new();
        let record = {
            let mut hooks = MatchHooks {
                override_choice: Some(Box::new(scripted(Misbehaviour::Null, &cfg, &fired_on))),
                ..MatchHooks::default()
            };
            play_match(&cfg, &mut hooks)
        };

        assert!(fired_on.get().is_some());
        assert_eq!(record.fallbacks as i64, 1);
        assert!(record.rejected.is_empty());
        assert!(record.thrown.is_empty());
        assert!(record.result.is_some());
        expect_folds(&cfg, &record);
    }
}
