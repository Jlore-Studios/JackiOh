//! Meditative+ #20 KY's Competition's question sequence (SPEC §8.9 row 20, R1184000, R1184001), the
//! worked answer the AI gives (R1184002) and `summon`'s X (R1184003), through a fixture Competition and
//! a fixture Kenny (`fixtures/ky_competition.rs`). The real cards are proved again in `crates/cards`
//! (`scripts/meditative_plus/c020_kys_competition.rs`, `c020_1_lil_kenny.rs`).

use jackioh_engine::subsystems::ai_policy::choose_action;
use jackioh_engine::subsystems::ky_competition::{
    KY_COMPETITION_STREAK, competition_question, worked_answer,
};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{json_of, setup_catalog};
use crate::rules::fixtures::ky_competition::{
    KENNY_RUSH_AT, competition, competition_qd, kenny, register_ky_competition_fixtures,
};
use crate::rules::fixtures::prompt_harness::{act, answer_keys, board, cast_now, must, open_as, round_trip};

/// p1's main phase with the fixture Competition cast and its first question open.
fn asked(seed: &str, radiant: bool) -> GameState {
    let mut state = board(seed);
    register_ky_competition_fixtures();
    cast_now(&mut state, &competition().id, P1, radiant);
    state
}

/// The open question's right letter, read from where only the engine reads it.
fn key(state: &GameState) -> String {
    must(
        answer_key_of(&open_as(state, PromptKind::Answer, P1).resume.data),
        "the key",
    )
}

/// Answer the open question rightly or wrongly.
fn answer(state: &mut GameState, right: bool) {
    let key = key(state);
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

fn stop(state: &mut GameState) {
    assert_eq!(answer_keys(state, &[ANSWER_STOP_KEY]).error, None);
}

/// The Kennies on p1's side.
fn kennies(state: &GameState) -> Vec<CardInstance> {
    active_units_of(state, P1)
        .into_iter()
        .filter(|card| card.def_id == kenny().id)
        .cloned()
        .collect()
}

/// The one Kenny on p1's side: its X, attack, health and whether it has Rush.
fn the_kenny(state: &GameState) -> (i32, i32, i32, bool) {
    let found = kennies(state);
    assert_eq!(found.len(), 1);
    let card = must(found.first(), "a Kenny");
    let view = unit_view(state, card);
    let rush = view
        .keywords
        .iter()
        .any(|keyword| keyword.kind() == KeywordKind::Rush);
    (x_of(card), view.attack, view.health, rush)
}

fn streak(state: &GameState) -> Option<i32> {
    state.pending.as_ref().and_then(|pending| pending.streak)
}

/// `a op b = ?` read back into its parts, by hand (no regex crate, SURFACE §8).
fn parts(statement: &str) -> (i32, String, i32) {
    let sum = must(statement.strip_suffix(" = ?"), "a ` = ?` statement");
    let pieces: Vec<&str> = sum.split(' ').collect();
    assert_eq!(pieces.len(), 3, "{statement}");
    (
        pieces[0].parse().expect("an operand"),
        pieces[1].to_string(),
        pieces[2].parse().expect("an operand"),
    )
}

mod m_plus_20_the_questions_r1184000 {
    use super::*;

    /// Every question of 300 seeds on one face holds to its operands and misses.
    fn holds(easy: bool, operands: IntRange, misses: &[i32]) -> IndexSet<String> {
        let mut operations: IndexSet<String> = IndexSet::new();
        for n in 0..300 {
            let question = competition_question(&mut Rng::new(&format!("kc-q-{easy}-{n}"), 0), easy);
            let (a, operation, b) = parts(&question.statement);
            for operand in [a, b] {
                assert!(
                    operand >= operands.min && operand <= operands.max,
                    "{}",
                    question.statement
                );
            }
            let answer = match operation.as_str() {
                "+" => a + b,
                "−" => {
                    assert!(
                        a >= b,
                        "a subtraction's larger operand first: {}",
                        question.statement
                    );
                    a - b
                }
                other => panic!("an operation {other}"),
            };
            operations.insert(operation);
            assert_eq!(question.options.len(), KY_TEST_OPTIONS as usize);
            assert_eq!(question.options.first(), Some(&answer.to_string()));
            assert_eq!(
                question.options.iter().collect::<IndexSet<_>>().len(),
                question.options.len()
            );
            for option in question.options.iter().skip(1) {
                let value: i32 = option.parse().expect("a number");
                assert!(value >= 0, "never below 0: {}", question.statement);
                assert!(misses.contains(&(value - answer)));
            }
        }
        operations
    }

    #[test]
    fn r1184000_a_base_question_adds_or_subtracts_two_three_digit_numbers_with_three_answers_a_column_off() {
        let operations = holds(false, KY_COMPETITION_OPERANDS, KY_COMPETITION_MISSES);
        assert_eq!(operations.len(), 2, "both operations come up");
    }

    #[test]
    fn r1184000_a_radiant_question_adds_or_subtracts_one_and_two_digit_numbers() {
        let operations = holds(true, KY_COMPETITION_EASY_OPERANDS, KY_COMPETITION_EASY_MISSES);
        assert_eq!(operations.len(), 2, "both operations come up");
        let one_digit = (0..200).any(|n| {
            let question = competition_question(&mut Rng::new(&format!("kc-digit-{n}"), 0), true);
            let (a, _, b) = parts(&question.statement);
            a < 10 || b < 10
        });
        assert!(one_digit);
    }

    #[test]
    fn r1184002_the_worked_answer_of_every_question_is_its_answer_and_of_no_other_statement() {
        for easy in [false, true] {
            for n in 0..300 {
                let question = competition_question(&mut Rng::new(&format!("kc-worked-{n}"), 0), easy);
                let answer: i32 = must(question.options.first(), "the answer")
                    .parse()
                    .expect("a number");
                assert_eq!(worked_answer(&question.statement), Some(answer));
            }
        }
        for other in ["∫₀¹ 2x dx = ?", "12 × 3 = ?", "12 + 3", "a + b = ?", ""] {
            assert_eq!(worked_answer(other), None, "{other}");
        }
    }
}

mod m_plus_20_the_sequence_r1184000 {
    use super::*;

    #[test]
    fn r1184000_casting_asks_a_question_four_lettered_answers_then_stop_its_streak_0_its_key_in_the_resume_data()
     {
        let state = asked("kc-ask", false);
        let pending = open_as(&state, PromptKind::Answer, P1);
        let keys: Vec<String> = pending.options.iter().map(|option| option.key.clone()).collect();
        assert_eq!(
            keys,
            vec!["mode:A", "mode:B", "mode:C", "mode:D", ANSWER_STOP_KEY]
        );
        assert_eq!(
            pending.options.last().map(|option| option.label.clone()),
            Some("Stop".to_string())
        );
        assert_eq!(
            pending.options.last().map(|option| option.selection.clone()),
            Some(Selection::None)
        );
        assert_eq!(pending.streak, Some(0));
        assert_eq!(pending.resume.data.get(KY_COMPETITION_STREAK), Some(&json!(0)));
        let (a, operation, b) = parts(&pending.prompt);
        let answer = if operation == "+" { a + b } else { a - b };
        let key = key(&state);
        assert_eq!(
            pending
                .options
                .iter()
                .find(|option| option.key == format!("mode:{key}"))
                .map(|option| option.label.clone()),
            Some(answer.to_string())
        );
    }

    #[test]
    fn r1184000_r90_legal_actions_lists_the_four_answers_and_stop_and_nothing_for_the_other_seat() {
        let state = asked("kc-legal", false);
        let answers = |player: PlayerId| {
            legal_actions(&state, player)
                .into_iter()
                .filter(|action| matches!(action, ActionBody::Answer { .. }))
                .count()
        };
        assert_eq!(answers(P1), 5);
        assert_eq!(answers(P2), 0);
    }

    #[test]
    fn r1184000_the_options_come_in_an_rng_shuffled_order_so_the_right_letter_moves() {
        let letters: IndexSet<String> = (0..40)
            .map(|n| key(&asked(&format!("kc-shuffle-{n}"), false)))
            .collect();
        assert!(letters.len() > 1);
    }

    #[test]
    fn r1184000_r44_the_random_policy_answers_each_question_with_one_of_its_five_answers_until_the_sequence_ends()
     {
        let state = asked("kc-random", false);
        let legal = legal_actions(&state, P1);
        let mut seen: IndexSet<String> = IndexSet::new();
        for n in 0..80 {
            let pick = must(
                choose_action(&state, P1, &mut Rng::new(&format!("kc-random-{n}"), 0)),
                "an answer",
            );
            assert!(legal.contains(&pick));
            seen.insert(serde_json::to_string(&pick).expect("an action serialises"));
        }
        assert_eq!(seen.len(), 5, "every answer and Stop come up");
        let mut playing = state;
        let mut rng = Rng::new("kc-random-run", 0);
        let mut answers = 0;
        while playing.pending.is_some() {
            let ActionBody::Answer { choice_id, selection } =
                must(choose_action(&playing, P1, &mut rng), "an answer")
            else {
                panic!("the policy answers the open question");
            };
            playing = act(
                &playing,
                json!({ "type": "answer", "playerId": "p1", "choiceId": choice_id, "selection": selection }),
                None,
            );
            answers += 1;
            assert!(answers <= KY_COMPETITION_MAX);
        }
    }

    #[test]
    fn r1184000_a_right_answer_asks_the_next_question_with_the_streak_one_higher() {
        let mut state = asked("kc-next", false);
        let first = must(state.pending.clone(), "the first question");
        answer(&mut state, true);
        let second = open_as(&state, PromptKind::Answer, P1);
        assert_ne!(second.id, first.id);
        assert_eq!(second.streak, Some(1));
        assert_eq!(second.resume.data.get(KY_COMPETITION_STREAK), Some(&json!(1)));
        assert!(kennies(&state).is_empty());
        answer(&mut state, true);
        assert_eq!(streak(&state), Some(2));
    }

    #[test]
    fn r1184000_three_right_then_a_miss_summons_a_kenny_with_x_3_a_3_3_with_no_rush() {
        let mut state = asked("kc-miss", false);
        for _ in 0..3 {
            answer(&mut state, true);
        }
        answer(&mut state, false);
        assert!(state.pending.is_none());
        assert_eq!(the_kenny(&state), (3, 3, 3, false));
    }

    #[test]
    fn r1184000_r1184003_stop_after_four_right_summons_a_kenny_with_x_4_its_rush_read_off_its_x() {
        let mut state = asked("kc-stop", false);
        for _ in 0..KENNY_RUSH_AT {
            answer(&mut state, true);
        }
        stop(&mut state);
        assert!(state.pending.is_none());
        assert_eq!(the_kenny(&state), (4, 4, 4, true));
    }

    #[test]
    fn r1184000_a_miss_or_stop_at_the_first_question_summons_nothing() {
        for quit_by_stop in [false, true] {
            let mut state = asked(&format!("kc-none-{quit_by_stop}"), false);
            if quit_by_stop {
                stop(&mut state);
            } else {
                answer(&mut state, false);
            }
            assert!(state.pending.is_none());
            assert!(kennies(&state).is_empty());
        }
    }

    #[test]
    fn r1184000_the_question_that_brings_the_streak_to_ky_competition_max_is_the_last_and_summons_its_kenny()
    {
        let mut state = asked("kc-cap", false);
        let mut asked_count = 0;
        while state.pending.is_some() {
            assert_eq!(streak(&state), Some(asked_count));
            answer(&mut state, true);
            asked_count += 1;
            assert!(asked_count <= KY_COMPETITION_MAX, "the sequence outran its cap");
        }
        assert_eq!(asked_count, KY_COMPETITION_MAX);
        assert_eq!(the_kenny(&state), (30, 30, 30, true));
    }

    #[test]
    fn r1184000_the_radiant_face_asks_easy_questions_and_summons_a_radiant_kenny_at_2x_2x() {
        let mut state = asked("kc-radiant", true);
        for _ in 0..2 {
            let (a, _, b) = parts(&open_as(&state, PromptKind::Answer, P1).prompt);
            assert!(a <= KY_COMPETITION_EASY_OPERANDS.max && b <= KY_COMPETITION_EASY_OPERANDS.max);
            answer(&mut state, true);
        }
        answer(&mut state, false);
        let found = kennies(&state);
        assert!(found.iter().all(|card| card.radiant));
        assert_eq!(the_kenny(&state), (2, 4, 4, false));
    }

    #[test]
    fn r1184000_r465_the_key_never_reaches_a_view_the_chooser_sees_the_question_its_answers_stop_and_the_streak()
     {
        let mut state = asked("kc-view", false);
        answer(&mut state, true);
        answer(&mut state, true);
        let pending = open_as(&state, PromptKind::Answer, P1);
        let Some(PendingView::ForYou(mine)) = view_for(&state, P1).pending else {
            panic!("p1's view shows the question as theirs");
        };
        assert_eq!(mine.prompt, pending.prompt);
        assert_eq!(mine.streak, Some(2));
        let shown: Vec<String> = mine.options.iter().map(|option| option.label.clone()).collect();
        let offered: Vec<String> = pending
            .options
            .iter()
            .map(|option| option.label.clone())
            .collect();
        assert_eq!(shown, offered);
        assert_eq!(shown.last().map(String::as_str), Some("Stop"));
        assert_eq!(
            json_of(view_for(&state, P2).pending),
            json!({ "forYou": false, "pendingFor": "p1" })
        );
        // Two states differing only in the key show both seats the same view.
        let mut twin = round_trip(&state);
        let key = key(&state);
        must(twin.pending.as_mut(), "the twin's question")
            .resume
            .data
            .insert(ANSWER_KEY.to_string(), json!(if key == "A" { "B" } else { "A" }));
        for viewer in [P1, P2] {
            assert_eq!(
                json_of(view_for(&twin, viewer)),
                json_of(view_for(&state, viewer))
            );
            let text = serde_json::to_string(&view_for(&state, viewer)).expect("a view serialises");
            assert!(!text.contains(ANSWER_KEY));
            assert!(!text.contains(KY_COMPETITION_STREAK));
        }
    }

    #[test]
    fn r1184000_9_3_paused_mid_sequence_the_state_survives_json_and_answers_to_the_same_game() {
        let mut state = asked("kc-json", false);
        answer(&mut state, true);
        answer(&mut state, true);
        let mut thawed = round_trip(&state);
        assert_eq!(hash_state(&thawed), hash_state(&state));
        let key = key(&state);
        answer_keys(&mut state, &[&format!("mode:{key}")]);
        answer_keys(&mut thawed, &[&format!("mode:{key}")]);
        assert_eq!(hash_state(&thawed), hash_state(&state));
        assert_eq!(streak(&state), Some(3));
        let mut thawed = round_trip(&state);
        stop(&mut state);
        stop(&mut thawed);
        assert_eq!(hash_state(&thawed), hash_state(&state));
        assert_eq!(the_kenny(&state), (3, 3, 3, false));
    }

    #[test]
    fn r1184000_a_game_with_the_competition_replays_from_its_log_to_the_same_hash() {
        setup_catalog();
        register_ky_competition_fixtures();
        let seed = "kc-replay";
        let mut first_deck = vanilla_deck(DECK_SIZE - 1, 1);
        first_deck.push(competition_qd().id);
        let decks = (first_deck, vanilla_deck(DECK_SIZE, 21));
        let mut log: Vec<Action> = Vec::new();
        let mut state = begin_game(&create_game(&CreateGameArgs {
            seed: seed.into(),
            decks: decks.clone(),
            ..Default::default()
        }))
        .state;
        for player in [P1, P2] {
            let keep: Vec<String> = state.players[player]
                .hand
                .iter()
                .map(|card| card.id.clone())
                .collect();
            state = act(
                &state,
                json!({ "type": "mulligan", "playerId": player, "keep": keep }),
                Some(&mut log),
            );
        }
        let card = must(
            state
                .players
                .p1
                .hand
                .iter()
                .find(|held| held.def_id == competition_qd().id),
            "the Competition in hand",
        )
        .clone();
        state = act(
            &state,
            json!({ "type": "play", "playerId": "p1", "instanceId": card.id }),
            Some(&mut log),
        );
        for _ in 0..2 {
            let question = open_as(&state, PromptKind::Answer, P1);
            let key = key(&state);
            state = act(
                &state,
                json!({
                    "type": "answer",
                    "playerId": "p1",
                    "choiceId": question.id,
                    "selection": [{ "pick": "mode", "option": key }],
                }),
                Some(&mut log),
            );
        }
        let question = open_as(&state, PromptKind::Answer, P1);
        state = act(
            &state,
            json!({ "type": "answer", "playerId": "p1", "choiceId": question.id, "selection": [{ "pick": "none" }] }),
            Some(&mut log),
        );
        assert!(state.pending.is_none());
        assert_eq!(the_kenny(&state), (2, 2, 2, false));
        assert!(
            state
                .players
                .p1
                .graveyard
                .iter()
                .any(|held| held.def_id == competition_qd().id)
        );
        let folded = fold(&FoldArgs {
            seed: seed.into(),
            decks,
            log,
            ..FoldArgs::default()
        });
        assert!(folded.errors.is_empty(), "the fold refused nothing");
        assert_eq!(hash_state(&folded.state), hash_state(&state));
    }
}

mod m_plus_20_a_timeout_stops_the_sequence_r1184001 {
    use super::*;

    #[test]
    fn r1184001_r79_the_turn_clock_running_out_stops_the_sequence_summons_the_kenny_so_far_and_ends_the_turn()
    {
        for n in 0..12 {
            let mut state = asked(&format!("kc-timeout-{n}"), false);
            answer(&mut state, true);
            answer(&mut state, true);
            let result = reduce(&state, &Action::new(ActionBody::Timeout, P1, "kc-timeout"));
            assert_eq!(result.error, None);
            assert!(result.state.pending.is_none());
            assert_eq!(the_kenny(&result.state), (2, 2, 2, false));
            assert_eq!(result.state.active, P2);
        }
    }

    #[test]
    fn r1184001_stop_answer_names_stop_only_on_a_question_of_a_sequence() {
        let state = asked("kc-stop-answer", false);
        let pending = open_as(&state, PromptKind::Answer, P1);
        assert_eq!(
            stop_answer(&state),
            Some(ActionBody::Answer {
                choice_id: pending.id.clone(),
                selection: vec![Selection::None],
            })
        );
        let mut single = round_trip(&state);
        must(single.pending.as_mut(), "the question").streak = None;
        assert_eq!(stop_answer(&single), None);
        let mut quiet = round_trip(&state);
        quiet.pending = None;
        assert_eq!(stop_answer(&quiet), None);
    }
}

mod summon_with_an_x_r1184003 {
    use super::*;

    /// One effect applied for p1 with no running card, as `effects_summon.rs` runs one.
    fn run(state: &mut GameState, effect: Effect) {
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
        {
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..HookOptions::default()
                },
            );
            (effect.apply)(&mut ctx);
        }
        state.rng_cursor = rng.cursor();
    }

    #[test]
    fn r1184003_a_card_summoned_with_an_x_arrives_at_its_x_stats_and_whatever_reads_its_x_reads_it() {
        for (radiant, stats) in [(false, 5), (true, 10)] {
            let mut state = board(&format!("kc-summon-{radiant}"));
            register_ky_competition_fixtures();
            run(
                &mut state,
                effects::summon(json_as(
                    json!({ "defId": kenny().id, "x": 5, "radiant": radiant }),
                )),
            );
            assert_eq!(the_kenny(&state), (5, stats, stats, true));
        }
    }

    #[test]
    fn r1184003_with_no_x_the_card_is_its_printed_0_0_as_b2_7_has_a_card_with_no_chosen_x() {
        let mut state = board("kc-summon-none");
        register_ky_competition_fixtures();
        run(
            &mut state,
            effects::summon(json_as(json!({ "defId": kenny().id }))),
        );
        let found = kennies(&state);
        assert_eq!(found.len(), 1);
        assert!(found.iter().all(|card| card.x.is_none()));
        assert!(found.iter().all(|card| x_of(card) == 0));
    }
}
