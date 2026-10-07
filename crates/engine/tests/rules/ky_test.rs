//! Classic+ #42 KY's Test's question bank (docs/classic-sets.md B5 E31, SPEC §8.7 row 42, R420, R465,
//! R580), through a fixture KY's Test and a fixture bank (`fixtures/kyTest.ts`). The real card and the
//! real bank are proved again in packages/cards (test/classic-plus/042-kys-test.test.ts).
//!
//! Port of `packages/engine/test/kyTest.test.ts`.

use serde::Serialize;

use jackioh_engine::subsystems::ky_test::easy_problem;
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{in_hand, setup_catalog};
use crate::rules::fixtures::ky_test::{
    FIXTURE_BANK, book, coin, four, gift, ky_test, ky_test_qd, ky_two, legend, register_ky_test_fixtures,
};
use crate::rules::fixtures::prompt_harness::{act, answer_keys, board, cast_now, must, open_as, round_trip};

use jackioh_engine::config::KyTestDifficulty::{Easy, Hard, Medium};

/// p1's main phase with KY's Test cast and its first prompt open.
fn offered(seed: &str, radiant: bool) -> GameState {
    let mut state = board(seed);
    register_ky_test_fixtures();
    cast_now(&mut state, &ky_test().id, P1, radiant);
    state
}

/// The label the first prompt shows for a difficulty.
fn label_of(state: &GameState, difficulty: &str) -> String {
    let key = format!("mode:{difficulty}");
    must(
        state
            .pending
            .as_ref()
            .and_then(|pending| pending.options.iter().find(|option| option.key == key)),
        difficulty,
    )
    .label
    .clone()
}

/// The first seed of `prefix-<n>` whose roll for `difficulty` is `reward_id`.
fn rolled(prefix: &str, difficulty: KyTestDifficulty, reward_id: &str, radiant: bool) -> GameState {
    let label = must(
        KY_TEST_REWARDS
            .of(difficulty)
            .iter()
            .find(|reward| reward.id == reward_id),
        reward_id,
    )
    .label;
    for n in 0..200 {
        let state = offered(&format!("{prefix}-{n}"), radiant);
        if label_of(&state, difficulty.as_str()) == format!("{difficulty}: {label}") {
            return state;
        }
    }
    panic!("no seed rolls {reward_id} for {difficulty}");
}

/// Answer the difficulty, then the problem rightly or wrongly.
fn take(state: &mut GameState, difficulty: &str, right: bool) {
    assert_eq!(answer_keys(state, &[&format!("mode:{difficulty}")]).error, None);
    let pending = open_as(state, PromptKind::Answer, P1);
    let key = must(answer_key_of(&pending.resume.data), "the key").to_string();
    let pick = if right {
        key
    } else {
        must(
            ["A", "B", "C", "D"].into_iter().find(|letter| *letter != key),
            "a wrong letter",
        )
        .to_string()
    };
    assert_eq!(answer_keys(state, &[&format!("mode:{pick}")]).error, None);
}

fn hand_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player].hand.iter().map(|card| card.def_id.clone()).collect()
}

fn json_of<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

fn input(body: Value) -> ActionInput {
    json_as(body)
}

fn answers(state: &GameState, player: PlayerId) -> usize {
    legal_actions(state, player)
        .iter()
        .filter(|action| matches!(action, ActionBody::Answer { .. }))
        .count()
}

/// `/^\d\d \+ \d\d = \?$/`, by hand (no regex crate, SURFACE §8).
fn is_easy_statement(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 11
        && b[0].is_ascii_digit()
        && b[1].is_ascii_digit()
        && &b[2..5] == b" + "
        && b[5].is_ascii_digit()
        && b[6].is_ascii_digit()
        && &b[7..] == b" = ?"
}

/// `/^easy:\d\d\+\d\d$/`, by hand.
fn is_easy_id(text: &str) -> bool {
    text.strip_prefix("easy:").is_some_and(|rest| {
        let b = rest.as_bytes();
        b.len() == 5
            && b[0].is_ascii_digit()
            && b[1].is_ascii_digit()
            && b[2] == b'+'
            && b[3].is_ascii_digit()
            && b[4].is_ascii_digit()
    })
}

mod c_42_kys_test_the_easy_generator_r580 {
    use super::*;

    #[test]
    fn r580_a_b_with_both_addends_from_10_to_99_and_three_wrong_sums_each_moved_by_a_different_miss() {
        for n in 0..300 {
            let problem = easy_problem(&mut Rng::new(&format!("easy-{n}"), 0));
            let addends: Vec<i32> = problem.id["easy:".len()..]
                .split('+')
                .map(|part| part.parse::<i32>().expect("an addend"))
                .collect();
            let (a, b) = (addends[0], addends[1]);
            assert!(a >= 10);
            assert!(a <= 99);
            assert!(b >= 10);
            assert!(b <= 99);
            let sum = a + b;
            assert_eq!(problem.statement, format!("{a} + {b} = ?"));
            assert_eq!(problem.answer, sum.to_string());
            assert_eq!(problem.options.len(), 4);
            assert_eq!(problem.options.iter().collect::<IndexSet<_>>().len(), 4);
            assert!(problem.options.contains(&problem.answer));
            let misses: Vec<i32> = problem
                .options
                .iter()
                .filter(|option| **option != problem.answer)
                .map(|option| option.parse::<i32>().expect("a sum") - sum)
                .collect();
            assert_eq!(misses.iter().collect::<IndexSet<_>>().len(), 3);
            for miss in &misses {
                assert!(KY_TEST_EASY_MISSES.contains(miss));
            }
        }
    }
}

mod c_42_kys_test_the_two_prompts_r420 {
    use super::*;

    #[test]
    fn r420_as_it_resolves_it_rolls_a_reward_per_difficulty_and_offers_the_three_each_labelled_with_its_reward() {
        let mut before = board("kt-offer");
        register_ky_test_fixtures();
        let cursor = before.rng_cursor;
        cast_now(&mut before, &ky_test().id, P1, false);
        let pending = open_as(&before, PromptKind::Mode, P1);
        let keys: Vec<String> = pending.options.iter().map(|option| option.key.clone()).collect();
        assert_eq!(keys, vec!["mode:Easy", "mode:Medium", "mode:Hard"]);
        for difficulty in [Easy, Medium, Hard] {
            let labels: Vec<String> = KY_TEST_REWARDS
                .of(difficulty)
                .iter()
                .map(|reward| format!("{difficulty}: {}", reward.label))
                .collect();
            assert!(labels.contains(&label_of(&before, difficulty.as_str())));
        }
        assert_eq!(label_of(&before, "Hard"), "Hard: KY's Gift, which costs (0)");
        // R129: Easy and Medium roll one draw each; Hard's list has one entry, so nothing is drawn for it.
        assert_eq!(before.rng_cursor - cursor, 2);
    }

    #[test]
    fn r420_each_reward_of_a_list_comes_up_over_enough_seeds() {
        for difficulty in [Easy, Medium] {
            let mut seen: IndexSet<String> = IndexSet::new();
            for n in 0..80 {
                seen.insert(label_of(&offered(&format!("kt-spread-{n}"), false), difficulty.as_str()));
            }
            assert_eq!(seen.len(), KY_TEST_REWARDS.of(difficulty).len());
        }
    }

    #[test]
    fn r420_a_medium_choice_asks_a_bank_problem_of_that_difficulty_its_options_shuffled_the_key_in_the_resume_data() {
        let mut state = offered("kt-medium", false);
        answer_keys(&mut state, &["mode:Medium"]);
        let pending = open_as(&state, PromptKind::Answer, P1);
        let problem = must(
            FIXTURE_BANK.iter().find(|entry| entry.statement == pending.prompt),
            "a bank problem",
        );
        assert_eq!(problem.difficulty, Medium);
        let keys: Vec<String> = pending.options.iter().map(|option| option.key.clone()).collect();
        assert_eq!(keys, vec!["mode:A", "mode:B", "mode:C", "mode:D"]);
        let mut labels: Vec<String> = pending.options.iter().map(|option| option.label.clone()).collect();
        labels.sort();
        let mut expected: Vec<String> = problem.options.iter().map(|option| option.to_string()).collect();
        expected.sort();
        assert_eq!(labels, expected);
        let key = must(answer_key_of(&pending.resume.data), "the key").to_string();
        assert_eq!(
            pending
                .options
                .iter()
                .find(|option| option.key == format!("mode:{key}"))
                .map(|option| option.label.clone()),
            Some(problem.answer.to_string())
        );
        assert_eq!(pending.resume.data.get("kyTestProblem"), Some(&json!(problem.id)));
    }

    #[test]
    fn r420_r129_hards_bank_holds_one_problem_here_only_the_option_shuffle_draws() {
        let mut state = offered("kt-hard", false);
        let cursor = state.rng_cursor;
        answer_keys(&mut state, &["mode:Hard"]);
        assert_eq!(
            open_as(&state, PromptKind::Answer, P1).prompt,
            FIXTURE_BANK[2].statement.to_string()
        );
        assert_eq!(state.rng_cursor - cursor, 3); // a shuffle of four options is three draws
    }

    #[test]
    fn r420_an_easy_choice_asks_a_generated_a_b_r580() {
        let mut state = offered("kt-easy", false);
        answer_keys(&mut state, &["mode:Easy"]);
        let pending = open_as(&state, PromptKind::Answer, P1);
        assert!(is_easy_statement(&pending.prompt));
        assert!(
            pending
                .resume
                .data
                .get("kyTestProblem")
                .and_then(Value::as_str)
                .is_some_and(is_easy_id)
        );
    }

    #[test]
    fn r420_a_right_answer_gains_the_reward_a_wrong_one_gives_nothing() {
        for right in [true, false] {
            let mut state = rolled(&format!("kt-grade-{right}"), Easy, "coins", false);
            take(&mut state, "Easy", right);
            assert!(state.pending.is_none());
            assert_eq!(
                hand_ids(&state, P1).iter().filter(|id| **id == coin().id).count(),
                if right { 3 } else { 0 }
            );
        }
    }

    #[test]
    fn r420_legal_actions_lists_the_three_difficulties_then_the_four_answers() {
        let mut state = offered("kt-legal", false);
        assert_eq!(answers(&state, P1), 3);
        answer_keys(&mut state, &["mode:Medium"]);
        assert_eq!(answers(&state, P1), 4);
        assert_eq!(answers(&state, P2), 0);
    }

    #[test]
    fn r465_the_key_never_reaches_a_view_the_chooser_sees_labels_statement_and_options_the_opponent_only_that_a_prompt_is_open()
     {
        let mut state = offered("kt-view", false);
        answer_keys(&mut state, &["mode:Medium"]);
        let pending = open_as(&state, PromptKind::Answer, P1);
        let Some(PendingView::ForYou(mine)) = view_for(&state, P1).pending else {
            panic!("p1's view shows the prompt as theirs");
        };
        assert!(mine.for_you);
        assert_eq!(mine.prompt, pending.prompt);
        let shown: Vec<String> = mine.options.iter().map(|option| option.label.clone()).collect();
        let offered_labels: Vec<String> = pending.options.iter().map(|option| option.label.clone()).collect();
        assert_eq!(shown, offered_labels);
        assert_eq!(
            json_of(view_for(&state, P2).pending),
            json!({ "forYou": false, "pendingFor": "p1" })
        );
        // Two states differing only in the key show both seats the same view.
        let mut twin = round_trip(&state);
        let key = must(answer_key_of(&pending.resume.data), "the key").to_string();
        must(twin.pending.as_mut(), "the twin's prompt")
            .resume
            .data
            .insert(ANSWER_KEY.to_string(), json!(if key == "A" { "B" } else { "A" }));
        for viewer in [P1, P2] {
            assert_eq!(json_of(view_for(&twin, viewer)), json_of(view_for(&state, viewer)));
            assert!(!serde_json::to_string(&view_for(&state, viewer))
                .expect("a view serialises")
                .contains(ANSWER_KEY));
        }
    }

    #[test]
    fn r420_9_3_paused_at_either_prompt_the_state_survives_json_and_answers_to_the_same_game() {
        let mut state = rolled("kt-json", Medium, "books", false);
        let mut first = round_trip(&state);
        assert_eq!(hash_state(&first), hash_state(&state));
        answer_keys(&mut state, &["mode:Medium"]);
        answer_keys(&mut first, &["mode:Medium"]);
        assert_eq!(hash_state(&first), hash_state(&state));
        let mut second = round_trip(&state);
        let key = must(
            answer_key_of(&must(state.pending.as_ref(), "the problem").resume.data),
            "the key",
        )
        .to_string();
        answer_keys(&mut state, &[&format!("mode:{key}")]);
        answer_keys(&mut second, &[&format!("mode:{key}")]);
        assert_eq!(hash_state(&second), hash_state(&state));
        assert_eq!(hand_ids(&state, P1).iter().filter(|id| **id == book().id).count(), 5);
    }

    #[test]
    fn r420_a_game_with_kys_test_replays_from_its_log_to_the_same_hash() {
        setup_catalog();
        register_ky_test_fixtures();
        let seed = "kt-replay";
        let mut first_deck = vanilla_deck(DECK_SIZE - 1, 1);
        first_deck.push(ky_test_qd().id);
        let decks = (first_deck, vanilla_deck(DECK_SIZE, 21));
        let mut log: Vec<Action> = Vec::new();
        let mut state = begin_game(&create_game(&CreateGameArgs {
            seed: seed.into(),
            decks: decks.clone(),
            ..Default::default()
        }))
        .state;
        for player in [P1, P2] {
            let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
            state = act(
                &state,
                input(json!({ "type": "mulligan", "playerId": player, "keep": keep })),
                Some(&mut log),
            );
        }
        let card = must(
            state.players.p1.hand.iter().find(|held| held.def_id == ky_test_qd().id),
            "KY's Test in hand",
        )
        .clone();
        state = act(
            &state,
            input(json!({ "type": "play", "playerId": "p1", "instanceId": card.id })),
            Some(&mut log),
        );
        let offer = open_as(&state, PromptKind::Mode, P1);
        state = act(
            &state,
            input(json!({
                "type": "answer",
                "playerId": "p1",
                "choiceId": offer.id,
                "selection": [{ "pick": "mode", "option": "Hard" }],
            })),
            Some(&mut log),
        );
        let problem = open_as(&state, PromptKind::Answer, P1);
        let key = must(answer_key_of(&problem.resume.data), "the key").to_string();
        state = act(
            &state,
            input(json!({
                "type": "answer",
                "playerId": "p1",
                "choiceId": problem.id,
                "selection": [{ "pick": "mode", "option": key }],
            })),
            Some(&mut log),
        );
        assert!(hand_ids(&state, P1).contains(&gift().id));
        let folded = fold(&FoldArgs {
            seed: seed.into(),
            decks,
            log,
            ..FoldArgs::default()
        });
        assert!(folded.errors.is_empty());
        assert_eq!(hash_state(&folded.state), hash_state(&state));
    }
}

mod c_42_kys_test_the_rewards_r420 {
    use super::*;

    #[test]
    fn r420_easy_3_the_coins() {
        let mut state = rolled("kt-coins", Easy, "coins", false);
        take(&mut state, "Easy", true);
        assert_eq!(hand_ids(&state, P1), vec![coin().id; 3]);
    }

    #[test]
    fn r420_easy_a_random_2_cost_ky_card() {
        let mut state = rolled("kt-ky", Easy, "ky", false);
        take(&mut state, "Easy", true);
        assert_eq!(hand_ids(&state, P1), vec![ky_two().id]);
    }

    #[test]
    fn r420_r387_easy_a_random_legendary_card_which_costs_0_never_kys_test_itself() {
        let mut state = rolled("kt-legend", Easy, "legendary", false);
        take(&mut state, "Easy", true);
        assert_eq!(hand_ids(&state, P1), vec![legend().id]);
        assert_eq!(state.players.p1.hand.first().and_then(|card| card.cost_override), Some(0));
    }

    #[test]
    fn r420_easy_2_random_books() {
        let mut state = rolled("kt-books2", Easy, "books", false);
        take(&mut state, "Easy", true);
        assert_eq!(hand_ids(&state, P1), vec![book().id, book().id]);
    }

    #[test]
    fn r420_medium_2_random_4_cost_cards_which_cost_1() {
        let mut state = rolled("kt-fours", Medium, "fours", false);
        take(&mut state, "Medium", true);
        // Every (4) Cost fixture of the catalog is in the pool; each one drawn is priced (1).
        let costs: Vec<i32> = state
            .players
            .p1
            .hand
            .iter()
            .map(|card| query_cost(def_of(Some(&state), &card.def_id)))
            .collect();
        assert_eq!(costs, vec![4, 4]);
        let overrides: Vec<Option<i32>> = state.players.p1.hand.iter().map(|card| card.cost_override).collect();
        assert_eq!(overrides, vec![Some(1), Some(1)]);
    }

    #[test]
    fn r420_r387_medium_5_random_ky_cards_never_kys_test_itself() {
        let mut state = rolled("kt-ky5", Medium, "ky", false);
        take(&mut state, "Medium", true);
        assert_eq!(hand_ids(&state, P1), vec![ky_two().id; 5]);
    }

    #[test]
    fn r420_medium_fill_your_hand_with_random_books_up_to_the_hand_cap() {
        let mut state = rolled("kt-fill", Medium, "fill", false);
        take(&mut state, "Medium", true);
        assert_eq!(hand_ids(&state, P1), vec![book().id; 10]);
    }

    #[test]
    fn r420_r129_medium_a_full_hand_has_no_room_to_fill_and_nothing_is_drawn_for_it() {
        let mut state = rolled("kt-fill-full", Medium, "fill", false);
        answer_keys(&mut state, &["mode:Medium"]);
        let pending = open_as(&state, PromptKind::Answer, P1);
        let key = must(answer_key_of(&pending.resume.data), "the key").to_string();
        in_hand(&mut state, &four().id, P1, 10);
        let cursor = state.rng_cursor;
        answer_keys(&mut state, &[&format!("mode:{key}")]);
        assert_eq!(state.rng_cursor, cursor);
        assert_eq!(hand_ids(&state, P1), vec![four().id; 10]);
    }

    #[test]
    fn r420_hard_kys_gift_which_costs_0() {
        let mut state = offered("kt-gift", false);
        take(&mut state, "Hard", true);
        assert_eq!(hand_ids(&state, P1), vec![gift().id]);
        assert_eq!(state.players.p1.hand.first().and_then(|card| card.cost_override), Some(0));
    }

    #[test]
    fn r420_the_radiant_faces_reward_cards_are_radiant() {
        let mut state = offered("kt-radiant", true);
        take(&mut state, "Hard", true);
        let radiants: Vec<bool> = state.players.p1.hand.iter().map(|card| card.radiant).collect();
        assert_eq!(radiants, vec![true]);
        let mut coins = rolled("kt-radiant-coins", Easy, "coins", true);
        take(&mut coins, "Easy", true);
        let radiants: Vec<bool> = coins.players.p1.hand.iter().map(|card| card.radiant).collect();
        assert_eq!(radiants, vec![true, true, true]);
    }
}
