//! The match harness and the two baselines (docs/polish/3-ai.md B26, B27).
//!
//! B26: `play_match` is deterministic, stops with `result: null` at `max_actions`, and every record's
//! log folds with its handicaps back to `record.hash` with no errors (R180's replay, through the
//! harness the gates use). B27: `random_action` is §10.7's policy under the same rng, and
//! `greedy_action` takes the candidate whose one-ply `evaluate` is best, or ends the turn when
//! nothing beats standing still. The greedy oracle here uses the real `evaluate` on states with no
//! hidden card and no randomness, so a determinization cannot change any number it compares.
//!
//! Port of `packages/ai/test/match.test.ts`. TS's per-test timeouts are dropped.

use std::panic::catch_unwind;

use indexmap::IndexMap;
use jackioh_ai::{
    AI_GATE, AI_GATE_BUDGET, AI_EVAL, AI_MULLIGAN, MatchConfig, MatchHooks, MatchRecord, Matchup, NextSwing,
    SeatController, action_key, candidate_actions, evaluate, game_config, greedy_action, play_match, random_action,
};
use jackioh_engine::testkit::{
    AI_DIFFICULTY, Action, ActionBody, ActionType, FoldArgs, GameState, PerPlayer, PerPlayerOpt, PlayerId, Value,
    create_rng, def_of, fold, hash_state, json, json_as, query_cost, reduce, seat_to_act, subsystems,
};

use super::support::{
    AI, HUMAN, act, dealt_game, is_legal, random_decks, random_decks_sized, random_policy_states, register_cards,
    scenario,
};

fn random_vs_random(seed: &str) -> MatchConfig {
    MatchConfig {
        seed: seed.to_string(),
        decks: random_decks(seed),
        handicaps: None,
        controllers: PerPlayer::new(SeatController::Random, SeatController::Random),
        max_actions: None,
    }
}

fn fold_args(seed: &str, config: &MatchConfig, record: &MatchRecord, with_handicaps: bool) -> FoldArgs {
    let mut args = json!({ "seed": seed, "decks": config.decks, "log": record.log });
    if with_handicaps && let Some(handicaps) = &config.handicaps {
        args["handicaps"] = serde_json::to_value(handicaps).expect("handicaps serialise");
    }
    json_as(args)
}

fn expect_folds(config: &MatchConfig, record: &MatchRecord) {
    let replayed = fold(&fold_args(&config.seed, config, record, true));
    assert!(replayed.errors.is_empty(), "{}", config.seed);
    assert_eq!(hash_state(&replayed.state), record.hash, "{}", config.seed);
    assert_eq!(replayed.state.result, record.result, "{}", config.seed);
}

// ---------------------------------------------------------------------------------------------
// B26
// ---------------------------------------------------------------------------------------------

mod play_match_b26 {
    use super::*;

    #[test]
    fn b26_a_random_vs_random_match_is_deterministic_finishes_and_folds_back_to_its_hash() {
        let config = random_vs_random("match-rr");
        let first = play_match(&config, &mut MatchHooks::default());
        let second = play_match(&config, &mut MatchHooks::default());

        assert_eq!(second.log, first.log);
        assert_eq!(second.hash, first.hash);
        assert_eq!(second.result, first.result);
        assert!(first.result.is_some());
        assert!(first.rejected.is_empty());
        assert!(first.thrown.is_empty());
        assert!(!first.log.is_empty());
        expect_folds(&config, &first);
    }

    #[test]
    fn b26_a_records_log_does_not_fold_to_its_hash_under_another_seed() {
        let config = random_vs_random("match-other-seed");
        let record = play_match(&config, &mut MatchHooks::default());
        let replayed = fold(&fold_args("match-other-seed-not", &config, &record, false));
        let same = replayed.errors.is_empty() && hash_state(&replayed.state) == record.hash;
        assert!(!same);
    }

    #[test]
    fn b26_a_handicapped_match_folds_only_with_its_handicaps() {
        let seed = "match-rr-medium";
        let mut config = random_vs_random(seed);
        config.decks = random_decks_sized(seed, (20, AI_DIFFICULTY.medium.deck_size));
        config.handicaps = Some(PerPlayerOpt {
            p1: None,
            p2: Some(AI_DIFFICULTY.medium),
        });
        let record = play_match(&config, &mut MatchHooks::default());
        assert!(record.result.is_some());
        expect_folds(&config, &record);
        let unhandicapped = fold_args(seed, &config, &record, false);
        assert!(catch_unwind(|| fold(&unhandicapped)).is_err());
    }

    #[test]
    fn b26_a_match_cut_off_at_max_actions_has_no_result_and_its_partial_log_still_folds_to_its_hash() {
        let mut config = random_vs_random("match-cut");
        config.max_actions = Some(7);
        let record = play_match(&config, &mut MatchHooks::default());
        assert!(record.result.is_none());
        assert!(!record.log.is_empty());
        assert!(record.log.len() <= 7);
        expect_folds(&config, &record);
    }

    #[test]
    fn b26_an_ai_vs_greedy_gate_game_is_deterministic_and_folds_to_its_hash_with_nothing_rejected() {
        register_cards();
        let config = game_config(Matchup::AiVsGreedy, 1, AI_GATE_BUDGET, AI_GATE.seed_series);
        let first = play_match(&config, &mut MatchHooks::default());
        let second = play_match(&config, &mut MatchHooks::default());
        assert_eq!(second.log, first.log);
        assert_eq!(second.hash, first.hash);
        assert!(first.rejected.is_empty());
        assert!(first.thrown.is_empty());
        assert!(first.decisions > 0);
        assert!(first.nodes > 0);
        expect_folds(&config, &first);
    }

    #[test]
    fn b26_a_match_cut_off_after_one_action_holds_exactly_that_action_and_no_result() {
        let mut config = random_vs_random("match-cut-one");
        config.max_actions = Some(1);
        let record = play_match(&config, &mut MatchHooks::default());
        assert!(record.result.is_none());
        assert!(record.log.len() <= 1);
        expect_folds(&config, &record);
    }

    #[test]
    fn b26_after_action_sees_every_accepted_action_with_the_true_states_either_side_of_it() {
        let config = random_vs_random("match-hooks");
        let mut seen: Vec<(GameState, GameState, PlayerId, ActionBody)> = Vec::new();
        let record = {
            let mut hooks = MatchHooks::default();
            hooks.after_action = Some(Box::new(
                |before: &GameState, after: &GameState, seat: PlayerId, action: &ActionBody| {
                    seen.push((before.clone(), after.clone(), seat, action.clone()));
                },
            ));
            play_match(&config, &mut hooks)
        };
        assert_eq!(seen.len(), record.log.len());
        for (at, (before, _after, seat, action)) in seen.iter().enumerate() {
            let logged = &record.log[at];
            assert_eq!(*seat, logged.player_id, "action {at}");
            assert_eq!(action.action_type(), logged.action_type(), "action {at}");
            if at > 0 {
                assert_eq!(hash_state(before), hash_state(&seen[at - 1].1), "action {at}");
            }
        }
        assert_eq!(seen.last().map(|call| hash_state(&call.1)), Some(record.hash.clone()));
    }

    #[test]
    fn b26_time_decision_wraps_the_controller_calls_without_changing_the_match() {
        let config = random_vs_random("match-timing");
        let mut calls = 0;
        let timed = {
            let mut hooks = MatchHooks::default();
            hooks.time_decision = Some(Box::new(|_seat: PlayerId, run: &mut dyn FnMut()| {
                calls += 1;
                run();
            }));
            play_match(&config, &mut hooks)
        };
        assert!(calls > 0);
        assert_eq!(timed.hash, play_match(&config, &mut MatchHooks::default()).hash);
    }
}

// ---------------------------------------------------------------------------------------------
// B27
// ---------------------------------------------------------------------------------------------

/// p1 to act, nothing hidden anywhere and no card with a random effect.
fn open_board(p1: Value, p2: Value) -> GameState {
    register_cards();
    scenario(json!({ "seed": "match-greedy", "p1": p1, "p2": p2 })).state().clone()
}

fn eval(state: &GameState) -> f64 {
    evaluate(state, AI, NextSwing::Enemy, &AI_EVAL)
}

/// One-ply values of every non-endTurn candidate on the true state, by actionKey.
fn one_ply(state: &GameState) -> IndexMap<String, f64> {
    let mut values = IndexMap::new();
    for (at, action) in candidate_actions(state, AI).into_iter().enumerate() {
        if action.action_type() == ActionType::EndTurn {
            continue;
        }
        let key = action_key(&action);
        let kind = action.action_type();
        let result = reduce(state, &Action::new(action, AI, format!("greedy-oracle-{at}")));
        if let Some(error) = result.error {
            panic!("{kind} refused: {error}");
        }
        values.insert(key, eval(&result.state));
    }
    values
}

fn expect_greedy(state: &GameState, label: &str) -> ActionBody {
    let chosen = greedy_action(state, AI, &mut create_rng(&format!("match-greedy:{label}"), 0))
        .unwrap_or_else(|| panic!("{label}: greedyAction returned null"));
    assert!(is_legal(state, AI, &chosen), "{label}");

    let values = one_ply(state);
    let standing = eval(state);
    // TS `Math.max(...[])` is -Infinity.
    let best = values.values().copied().fold(f64::NEG_INFINITY, f64::max);
    if values.is_empty() || best <= standing {
        assert_eq!(chosen, ActionBody::EndTurn, "{label}");
    } else {
        assert_ne!(chosen.action_type(), ActionType::EndTurn, "{label}");
        assert_eq!(
            values.get(&action_key(&chosen)).copied(),
            Some(best),
            "{label}: {}",
            serde_json::to_string(&chosen).unwrap()
        );
    }
    chosen
}

mod the_baselines_b27 {
    use super::*;

    #[test]
    fn b27_greedy_action_takes_the_candidate_with_the_best_one_ply_evaluate_on_a_busy_board() {
        let state = open_board(
            json!({ "hand": ["core-008", "core-020"], "field": ["core-011"] }),
            json!({ "field": ["core-008"] }),
        );
        let chosen = expect_greedy(&state, "busy");
        assert_ne!(chosen.action_type(), ActionType::EndTurn);
    }

    #[test]
    fn b27_greedy_action_takes_a_winning_attack() {
        let state = open_board(json!({ "field": ["core-011"] }), json!({ "health": 3 }));
        let chosen = expect_greedy(&state, "winning");
        assert_eq!(chosen.action_type(), ActionType::Attack);
        let result = reduce(&state, &Action::new(chosen, AI, "greedy-win"));
        assert_eq!(result.state.result.and_then(|result| result.winner.player()), Some(AI));
    }

    #[test]
    fn b27_greedy_action_ends_the_turn_when_no_candidate_beats_standing_still() {
        // Mr. Vanilla facing Midrange Menace's 9/9 Taunt: attacking only loses the Vanilla.
        let state = open_board(json!({ "field": ["core-008"] }), json!({ "field": ["core-019"] }));
        expect_greedy(&state, "stand-still");
    }

    #[test]
    fn b27_greedy_action_with_only_end_turn_available_ends_the_turn() {
        let state = open_board(json!({}), json!({ "field": ["core-008"] }));
        assert_eq!(
            greedy_action(&state, AI, &mut create_rng("match-greedy-only", 0)),
            Some(ActionBody::EndTurn)
        );
    }

    #[test]
    fn b27_greedy_action_keeps_the_cheap_cards_at_the_mulligan_and_declines_a_draw_offer() {
        let dealt = dealt_game("match-greedy-mulligan");
        let mulligan = greedy_action(&dealt, PlayerId::P1, &mut create_rng("match-greedy-mulligan", 0));
        assert_eq!(
            mulligan.as_ref().map(ActionBody::action_type),
            Some(ActionType::Mulligan)
        );
        let mut keep: Vec<String> = dealt
            .players
            .p1
            .hand
            .iter()
            .filter(|card| query_cost(def_of(Some(&dealt), &card.def_id)) <= AI_MULLIGAN.keep_max_cost)
            .map(|card| card.id.clone())
            .collect();
        keep.sort();
        let kept = match &mulligan {
            Some(ActionBody::Mulligan { keep }) => {
                let mut sorted = keep.clone();
                sorted.sort();
                Some(sorted)
            }
            _ => None,
        };
        assert_eq!(kept, Some(keep));

        let humans_turn = scenario(json!({
            "seed": "match-greedy-offer",
            "active": HUMAN,
            "turn": 10,
            "p1": { "hand": ["core-008"] },
            "p2": { "hand": ["core-011"], "field": ["core-008"] },
        }))
        .state()
        .clone();
        let offered = act(&humans_turn, HUMAN, ActionBody::OfferDraw);
        assert_eq!(
            greedy_action(&offered, AI, &mut create_rng("match-greedy-offer", 0)),
            Some(ActionBody::AnswerDraw { accept: false })
        );
    }

    #[test]
    fn b27_greedy_action_returns_null_when_the_seat_owes_nothing() {
        register_cards();
        let humans_turn = scenario(json!({
            "seed": "match-greedy-null",
            "active": HUMAN,
            "turn": 10,
            "p2": { "hand": ["core-011"] },
        }))
        .state()
        .clone();
        assert_eq!(greedy_action(&humans_turn, AI, &mut create_rng("match-greedy-null", 0)), None);
    }

    #[test]
    fn b27_random_action_returns_null_for_a_seat_that_owes_nothing() {
        register_cards();
        let humans_turn = scenario(json!({
            "seed": "match-random-null",
            "active": HUMAN,
            "turn": 10,
            "p2": { "hand": ["core-011"] },
        }))
        .state()
        .clone();
        assert_eq!(random_action(&humans_turn, AI, &mut create_rng("match-random-null", 0)), None);
        // R265: both seats owe a mulligan at once, so p2 owes nothing only once it has answered its own.
        let dealt = dealt_game("match-random-null-mulligan");
        assert_eq!(
            random_action(&dealt, PlayerId::P2, &mut create_rng("match-random-null-mulligan", 0))
                .map(|action| action.action_type()),
            Some(ActionType::Mulligan)
        );
        let answered = act(&dealt, PlayerId::P2, ActionBody::Mulligan { keep: vec![] });
        assert_eq!(
            random_action(&answered, PlayerId::P2, &mut create_rng("match-random-null-mulligan", 0)),
            None
        );
    }

    #[test]
    fn b27_greedy_action_is_always_legal_and_never_concedes_offers_or_accepts_a_draw_across_real_states() {
        let states: Vec<GameState> = random_policy_states("match-greedy-real", 7, 500)
            .into_iter()
            .filter(|state| state.result.is_none())
            .collect();
        assert!(states.len() > 5);
        for (at, state) in states.iter().enumerate() {
            let seat = seat_to_act(state).expect("a live game has a seat to act");
            let chosen = greedy_action(state, seat, &mut create_rng(&format!("match-greedy-real:{at}"), 0));
            let action = chosen.unwrap_or_else(|| panic!("state {at}"));
            assert!(
                is_legal(state, seat, &action),
                "state {at}: {}",
                serde_json::to_string(&action).unwrap()
            );
            assert!(!matches!(action.action_type(), ActionType::Concede | ActionType::OfferDraw));
            if let ActionBody::AnswerDraw { accept } = action {
                assert!(!accept);
            }
        }
    }

    #[test]
    fn b27_random_action_is_subsystems_choose_action_under_the_same_rng_from_both_seats_across_real_states() {
        let mut states = vec![
            open_board(
                json!({ "hand": ["core-008", "core-020"], "field": ["core-011"] }),
                json!({ "field": ["core-008"] }),
            ),
            dealt_game("match-random-dealt"),
        ];
        states.extend(random_policy_states("match-random-real", 19, 400));
        for (at, state) in states.iter().enumerate() {
            for seat in [PlayerId::P1, PlayerId::P2] {
                let mut ours = create_rng(&format!("match-random:{at}:{seat}"), 0);
                let mut theirs = create_rng(&format!("match-random:{at}:{seat}"), 0);
                assert_eq!(
                    random_action(state, seat, &mut ours),
                    subsystems::choose_action(state, seat, &mut theirs),
                    "state {at} {seat}"
                );
                assert_eq!(ours.cursor(), theirs.cursor(), "state {at} {seat}");
            }
        }
    }
}
