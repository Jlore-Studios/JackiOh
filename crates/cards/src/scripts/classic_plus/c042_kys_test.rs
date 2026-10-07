//! C+ #42 KY's Test (SPEC §8.7 row 42, R420, R465, R580). (1) Spell, KY, Legendary.
//!   Offer an Easy, a Medium and a Hard problem, each showing a random reward from its list; choose one
//!   and answer it; if you're right, gain its reward. Radiant: the same, and the reward cards are Radiant.
//!
//! The whole card is the engine's question-bank subsystem (`subsystems/kyTest.ts`, E31): the reward
//! rolls, the two prompts, the Easy generator (R580) and the grade. This file hands it the bank of
//! Medium and Hard problems, which is card data (`../../kyTestBank.ts`). Both faces run the same script:
//! the grade step reads the face that asked, so the Radiant face's reward cards are Radiant.

use jackioh_engine::prelude::*;

use crate::ky_test_bank::KY_TEST_BANK;

pub const ID: &str = "classicplus-042";

pub fn script() -> CardScripts {
    let ky_test = subsystems::ky_test_script(&KY_TEST_BANK);
    let base = Script {
        cry: Some(ky_test.cry),
        resume: ky_test.resume,
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C+ #42 KY's Test — SPEC §8.7 row 42, R420, R465, R580, BUILD M9 row C+ 42. The machinery is the
// engine's question bank (`subsystems/kyTest.ts`, proved with a fixture bank in
// packages/engine/test/kyTest.test.ts); this file proves the real card, the real bank and the real
// reward pools again.
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::Arc;

    use jackioh_engine::testkit::*;

    use crate::ky_test_bank::KY_TEST_BANK;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TEST: &str = "classicplus-042";
    const GIFT: &str = "classicplus-042-1";
    const COIN: &str = "core-t-coin";
    const CONJURE_KY: &str = "core-057";
    const FILLER: &str = "core-005";

    use crate::scenario;

    /// TS `type Difficulty = "Easy" | "Medium" | "Hard"`, read as the config's difficulty.
    fn difficulty_of(name: &str) -> KyTestDifficulty {
        match name {
            "Easy" => KyTestDifficulty::Easy,
            "Medium" => KyTestDifficulty::Medium,
            _ => KyTestDifficulty::Hard,
        }
    }

    /// `KY_TEST_REWARDS[difficulty]`.
    fn rewards(name: &str) -> &'static [KyTestReward] {
        KY_TEST_REWARDS.of(difficulty_of(name))
    }

    /// KY's Test cast from p1's hand, its first prompt open.
    fn cast(seed: &str, radiant_face: bool, hand: &[&str]) -> Scenario {
        let mut cards = vec![json!({ "def": TEST, "radiant": radiant_face })];
        cards.extend(hand.iter().map(|id| json!(id)));
        let mut s = scenario(json!({ "seed": seed, "p1": { "hand": cards }, "p2": { "hand": [FILLER] } }));
        s.play(TEST, json!({}));
        s
    }

    fn label_of(s: &Scenario, difficulty: &str) -> String {
        let key = format!("mode:{difficulty}");
        s.state()
            .pending
            .as_ref()
            .and_then(|prompt| prompt.options.iter().find(|option| option.key == key))
            .map(|option| option.label.clone())
            .unwrap_or_default()
    }

    /// The first seed whose roll for `difficulty` is `reward_id`; `salt` picks another run of seeds.
    fn rolled(difficulty: &str, reward_id: &str, radiant_face: bool, hand: &[&str], salt: &str) -> Scenario {
        let reward = rewards(difficulty).iter().find(|entry| entry.id == reward_id).expect("a reward of that list");
        for n in 0..200 {
            let s = cast(&format!("kys-test-{difficulty}-{reward_id}-{salt}{n}"), radiant_face, hand);
            if label_of(&s, difficulty) == format!("{difficulty}: {}", reward.label) {
                return s;
            }
        }
        panic!("no seed rolls {reward_id}");
    }

    fn key(s: &Scenario) -> String {
        let data = s.state().pending.as_ref().map(|prompt| prompt.resume.data.clone()).unwrap_or_default();
        match answer_key_of(&data) {
            Some(found) => found,
            None => panic!("an answer prompt with its key"),
        }
    }

    /// Choose the difficulty, then answer rightly or wrongly.
    fn take<'a>(s: &'a mut Scenario, difficulty: &str, right: bool) -> &'a mut Scenario {
        s.answer(json!(difficulty));
        let correct = key(s);
        let letter = if right {
            correct
        } else {
            ["A", "B", "C", "D"].into_iter().find(|letter| *letter != correct).unwrap_or_default().to_string()
        };
        s.answer(json!(format!("mode:{letter}")))
    }

    /// The cards the reward put in p1's hand (the filler excluded).
    fn gained(s: &Scenario) -> Vec<Value> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id != FILLER)
            .map(|card| {
                let mut entry = json!({ "defId": card.def_id, "radiant": card.radiant });
                if let Some(cost) = card.cost_override {
                    entry["costOverride"] = json!(cost);
                }
                entry
            })
            .collect()
    }

    fn def_id_of(entry: &Value) -> String {
        entry["defId"].as_str().unwrap_or_default().to_string()
    }

    fn view(s: &Scenario, seat: PlayerId) -> Value {
        serde_json::to_value(s.view(seat)).expect("a view is JSON")
    }

    fn events_json(s: &Scenario) -> Vec<Value> {
        s.events().iter().map(|event| serde_json::to_value(event).expect("an event is JSON")).collect()
    }

    fn answers_offered(s: &Scenario, seat: PlayerId) -> usize {
        legal_actions(s.state(), seat).iter().filter(|action| matches!(action, ActionBody::Answer { .. })).count()
    }

    fn pending_option_labels(s: &Scenario) -> Vec<String> {
        s.state()
            .pending
            .as_ref()
            .map(|prompt| prompt.options.iter().map(|option| option.label.clone()).collect())
            .unwrap_or_default()
    }

    fn pending_prompt(s: &Scenario) -> String {
        s.state().pending.as_ref().map(|prompt| prompt.prompt.clone()).unwrap_or_default()
    }

    #[test]
    fn is_the_engines_question_bank_on_both_faces_over_the_real_bank() {
        assert_eq!(crate::card_def(TEST).id, TEST);
        let scripts = super::script();
        let base_cry = scripts.base.cry.as_ref().expect("a Cry");
        let radiant_cry = scripts.radiant.cry.as_ref().expect("a Cry");
        assert!(Arc::ptr_eq(base_cry, radiant_cry));
        let mut steps: Vec<&str> = scripts.base.resume.keys().copied().collect();
        steps.sort();
        assert_eq!(steps, vec!["ask", "grade"]);
    }

    mod the_bank_r420 {
        use super::*;

        #[test]
        fn r420_holds_at_least_ky_test_min_problems_medium_and_hard_problems_easy_ones_being_generated_r580() {
            for difficulty in ["Medium", "Hard"] {
                let count = KY_TEST_BANK.iter().filter(|problem| problem.difficulty.to_string() == difficulty).count();
                assert!(count >= KY_TEST_MIN_PROBLEMS as usize);
            }
            assert!(!KY_TEST_BANK.iter().any(|problem| problem.difficulty.to_string() == "Easy"));
        }

        #[test]
        fn r420_every_id_is_unique_every_problem_has_four_distinct_options_one_of_them_its_answer() {
            let ids: IndexSet<String> = KY_TEST_BANK.iter().map(|problem| problem.id.to_string()).collect();
            assert_eq!(ids.len(), KY_TEST_BANK.len());
            for problem in KY_TEST_BANK.iter() {
                let options: Vec<String> = problem.options.iter().map(|option| option.to_string()).collect();
                assert_eq!(options.len(), KY_TEST_OPTIONS as usize, "{}", problem.id);
                assert_eq!(options.iter().collect::<IndexSet<_>>().len(), KY_TEST_OPTIONS as usize, "{}", problem.id);
                assert!(options.contains(&problem.answer.to_string()), "{}", problem.id);
                assert!(!problem.statement.is_empty(), "{}", problem.id);
            }
        }
    }

    mod the_prompts_r420 {
        use super::*;

        #[test]
        fn r420_as_it_resolves_it_offers_easy_medium_and_hard_each_labelled_with_the_reward_rolled_from_its_list() {
            let s = cast("kys-test-offer", false, &[FILLER]);
            let pending = s.state().pending.clone().expect("the first prompt");
            assert_eq!(pending.kind, PromptKind::Mode);
            assert_eq!(pending.player_id, P1);
            let keys: Vec<String> = pending.options.iter().map(|option| option.key.clone()).collect();
            assert_eq!(keys, vec!["mode:Easy", "mode:Medium", "mode:Hard"]);
            for difficulty in ["Easy", "Medium"] {
                let labels: Vec<String> =
                    rewards(difficulty).iter().map(|reward| format!("{difficulty}: {}", reward.label)).collect();
                assert!(labels.contains(&label_of(&s, difficulty)));
            }
            assert_eq!(label_of(&s, "Hard"), "Hard: KY's Gift, which costs (0)");
        }

        #[test]
        fn r420_the_cards_text_names_no_reward_the_rewards_live_in_the_discover_menu() {
            let text = crate::card_def(TEST).base.text;
            assert_eq!(text, "Offer an easy, a medium, and a hard problem, each with a random reward.");
            for difficulty in ["Easy", "Medium", "Hard"] {
                for reward in rewards(difficulty) {
                    assert!(!text.contains(reward.label));
                }
            }
        }

        #[test]
        fn r420_the_chosen_difficultys_problem_opens_as_an_answer_prompt_the_statement_and_four_options_under_letters() {
            let mut s = cast("kys-test-medium", false, &[FILLER]);
            s.answer(json!("Medium"));
            let pending = s.state().pending.clone().expect("the answer prompt");
            assert_eq!(pending.kind, PromptKind::Answer);
            let problem = KY_TEST_BANK
                .iter()
                .find(|entry| entry.statement == pending.prompt)
                .expect("a problem of the bank");
            assert_eq!(problem.difficulty.to_string(), "Medium");
            let keys: Vec<String> = pending.options.iter().map(|option| option.key.clone()).collect();
            assert_eq!(keys, vec!["mode:A", "mode:B", "mode:C", "mode:D"]);
            let mut labels: Vec<String> = pending.options.iter().map(|option| option.label.clone()).collect();
            labels.sort();
            let mut options: Vec<String> = problem.options.iter().map(|option| option.to_string()).collect();
            options.sort();
            assert_eq!(labels, options);
            let right = format!("mode:{}", key(&s));
            let label = pending.options.iter().find(|option| option.key == right).map(|option| option.label.clone());
            assert_eq!(label, Some(problem.answer.to_string()));
        }

        #[test]
        fn r420_the_options_come_in_an_rng_shuffled_order_the_right_letter_is_not_always_the_same() {
            let mut letters: BTreeSet<String> = BTreeSet::new();
            for n in 0..12 {
                let mut s = cast(&format!("kys-test-order-{n}"), false, &[FILLER]);
                s.answer(json!("Medium"));
                letters.insert(key(&s));
            }
            assert!(letters.len() > 1);
        }

        #[test]
        fn r420_a_hard_choice_asks_a_hard_problem_of_the_bank() {
            let mut s = cast("kys-test-hard", false, &[FILLER]);
            s.answer(json!("Hard"));
            let prompt = pending_prompt(&s);
            let problem = KY_TEST_BANK.iter().find(|entry| entry.statement == prompt);
            assert_eq!(problem.map(|entry| entry.difficulty.to_string()), Some("Hard".to_string()));
        }

        #[test]
        fn r580_an_easy_choice_asks_a_generated_a_b_with_both_addends_from_10_to_99() {
            let mut s = cast("kys-test-easy", false, &[FILLER]);
            s.answer(json!("Easy"));
            // TS /^(\d+) \+ (\d+) = \?$/, by hand (no regex crate in the pure crates).
            let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
            let prompt = pending_prompt(&s);
            let parsed = prompt
                .strip_suffix(" = ?")
                .and_then(|sum| sum.split_once(" + "))
                .filter(|(a, b)| digits(a) && digits(b))
                .map(|(a, b)| (a.parse::<i64>().unwrap_or(-1), b.parse::<i64>().unwrap_or(-1)));
            assert!(parsed.is_some());
            let (a, b) = parsed.unwrap_or((-1, -1));
            for addend in [a, b] {
                assert!((10..=99).contains(&addend));
            }
            let right = format!("mode:{}", key(&s));
            let label = s
                .state()
                .pending
                .as_ref()
                .and_then(|prompt| prompt.options.iter().find(|option| option.key == right))
                .map(|option| option.label.clone());
            assert_eq!(label, Some((a + b).to_string()));
        }

        #[test]
        fn r420_legalactions_lists_the_three_difficulties_then_all_four_answers_so_the_ai_and_the_fuzz_reach_every_one() {
            let mut s = cast("kys-test-legal", false, &[FILLER]);
            assert_eq!(answers_offered(&s, P1), 3);
            s.answer(json!("Medium"));
            assert_eq!(answers_offered(&s, P1), 4);
            assert_eq!(answers_offered(&s, P2), 0);
        }

        #[test]
        fn r420_a_wrong_answer_gives_nothing_and_the_spell_was_still_played() {
            let mut s = rolled("Easy", "coins", false, &[FILLER], "");
            take(&mut s, "Easy", false);
            assert!(s.state().pending.is_none());
            assert_eq!(gained(&s), Vec::<Value>::new());
            s.expect_in_zone(TEST, "graveyard").expect_events(json!(["cardPlayed", "cardResolved"]));
        }
    }

    mod hidden_information_r97_r177_r465 {
        use super::*;

        #[test]
        fn r465_the_key_never_reaches_a_view_the_chooser_sees_the_labels_the_statement_and_the_options_the_opponent_only_that_a_prompt_is_open() {
            let mut s = cast("kys-test-view", false, &[FILLER]);
            assert_eq!(view(&s, P2)["pending"], json!({ "forYou": false, "pendingFor": "p1" }));
            let offer = view(&s, P1)["pending"].clone();
            let offered: Vec<String> = if offer["forYou"] == true {
                offer["options"]
                    .as_array()
                    .map(|options| {
                        options.iter().map(|option| option["label"].as_str().unwrap_or_default().to_string()).collect()
                    })
                    .unwrap_or_default()
            } else {
                vec![]
            };
            assert_eq!(offered, pending_option_labels(&s));
            s.answer(json!("Hard"));
            let mine = view(&s, P1)["pending"].clone();
            let shown = if mine["forYou"] == true { mine["prompt"].as_str().unwrap_or_default().to_string() } else { String::new() };
            assert_eq!(shown, pending_prompt(&s));
            assert_eq!(view(&s, P2)["pending"], json!({ "forYou": false, "pendingFor": "p1" }));
            for viewer in [P1, P2] {
                let text = serde_json::to_string(&s.view(viewer)).expect("a view is JSON");
                assert!(!text.contains("__answerKey"));
                assert!(!text.contains("kyTestProblem"));
            }
        }

        #[test]
        fn r97_the_rewards_cards_reach_p1s_hand_under_the_sentinel_for_the_opponent() {
            let mut s = cast("kys-test-hidden", false, &[FILLER]);
            take(&mut s, "Hard", true);
            let added = |seat: PlayerId| -> Vec<String> {
                view(&s, seat)["events"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .filter(|event| event["type"] == "addedToHand")
                    .map(def_id_of)
                    .collect()
            };
            assert_eq!(added(P1), vec![GIFT]);
            assert_eq!(added(P2), vec!["hidden"]);
        }
    }

    mod the_rewards_r420_pools_of_non_token_cards_of_every_set_never_ky_s_test_r387 {
        use super::*;

        #[test]
        fn r420_easy_3_the_coins() {
            let mut s = rolled("Easy", "coins", false, &[FILLER], "");
            take(&mut s, "Easy", true);
            let coin = json!({ "defId": COIN, "radiant": false });
            assert_eq!(gained(&s), vec![coin.clone(), coin.clone(), coin]);
        }

        #[test]
        fn r420_easy_a_random_2_cost_ky_card_which_is_conjure_ky() {
            let mut s = rolled("Easy", "ky", false, &[FILLER], "");
            take(&mut s, "Easy", true);
            assert_eq!(gained(&s), vec![json!({ "defId": CONJURE_KY, "radiant": false })]);
        }

        #[test]
        fn r420_r387_easy_a_random_legendary_card_which_costs_0_never_kys_test() {
            let mut seen: BTreeSet<String> = BTreeSet::new();
            for n in 0..8 {
                let mut s = rolled("Easy", "legendary", false, &[FILLER], &format!("run{n}-"));
                take(&mut s, "Easy", true);
                let cards = gained(&s);
                let card = cards.first().cloned().unwrap_or(Value::Null);
                let def_id = def_id_of(&card);
                seen.insert(def_id.clone());
                assert_eq!(crate::card_def(&def_id).rarity, Rarity::Legendary);
                assert!(!crate::card_def(&def_id).token);
                assert_ne!(def_id, TEST);
                assert_eq!(card["costOverride"], 0);
            }
            // Not one card eight times over: the seeds differ, so the pool is really sampled.
            assert!(seen.len() > 1);
        }

        #[test]
        fn r420_easy_2_random_books() {
            let mut s = rolled("Easy", "books", false, &[FILLER], "");
            take(&mut s, "Easy", true);
            let cards = gained(&s);
            assert_eq!(cards.len(), 2);
            for card in &cards {
                assert!(crate::card_def(&def_id_of(card)).tags.contains(&Tag::Book));
            }
        }

        #[test]
        fn r420_medium_2_random_4_cost_cards_which_cost_1() {
            let mut s = rolled("Medium", "fours", false, &[FILLER], "");
            take(&mut s, "Medium", true);
            let cards = gained(&s);
            assert_eq!(cards.len(), 2);
            for card in &cards {
                let def = crate::card_def(&def_id_of(card));
                assert_eq!(query_cost(&def), 4);
                assert!(!def.token);
                assert_eq!(card["costOverride"], 1);
            }
        }

        #[test]
        fn r420_medium_5_random_books() {
            let mut s = rolled("Medium", "books", false, &[FILLER], "");
            take(&mut s, "Medium", true);
            let cards = gained(&s);
            assert_eq!(cards.len(), 5);
            for card in &cards {
                assert!(crate::card_def(&def_id_of(card)).tags.contains(&Tag::Book));
            }
        }

        #[test]
        fn r420_r387_medium_5_random_ky_cards_never_kys_test_and_never_a_token() {
            // Four runs: twenty picks from the six other KY cards, so a pool that held KY's Test would show it.
            for salt in ["a-", "b-", "c-", "d-"] {
                let mut s = rolled("Medium", "ky", false, &[FILLER], salt);
                take(&mut s, "Medium", true);
                let cards = gained(&s);
                assert_eq!(cards.len(), 5);
                for card in &cards {
                    let def_id = def_id_of(card);
                    assert!(crate::card_def(&def_id).tags.contains(&Tag::Ky));
                    assert!(!crate::card_def(&def_id).token);
                    assert_ne!(def_id, TEST);
                }
            }
        }

        #[test]
        fn r420_medium_fill_your_hand_with_random_books_until_it_holds_10() {
            let mut s = rolled("Medium", "fill", false, &[FILLER], "");
            take(&mut s, "Medium", true);
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            for card in gained(&s) {
                assert!(crate::card_def(&def_id_of(&card)).tags.contains(&Tag::Book));
            }
        }

        #[test]
        fn r420_hard_kys_gift_which_costs_0() {
            let mut s = cast("kys-test-gift", false, &[FILLER]);
            take(&mut s, "Hard", true);
            assert_eq!(gained(&s), vec![json!({ "defId": GIFT, "radiant": false, "costOverride": 0 })]);
        }

        #[test]
        fn s2_4_a_full_hand_burns_the_reward_cards_that_do_not_fit() {
            let hand: Vec<&str> = (0..8).map(|_| FILLER).collect();
            let mut s = rolled("Easy", "coins", false, &hand, "");
            take(&mut s, "Easy", true);
            // 8 fillers in hand: two Coins fit, the third is burned.
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            assert_eq!(events_json(&s).iter().filter(|event| event["type"] == "burned").count(), 1);
        }

        #[test]
        fn r420_radiant_every_reward_card_is_radiant() {
            let mut gift = cast("kys-test-radiant-gift", true, &[FILLER]);
            take(&mut gift, "Hard", true);
            assert_eq!(gained(&gift), vec![json!({ "defId": GIFT, "radiant": true, "costOverride": 0 })]);
            let mut coins = rolled("Easy", "coins", true, &[FILLER], "");
            take(&mut coins, "Easy", true);
            let radiant: Vec<Value> = gained(&coins).iter().map(|card| card["radiant"].clone()).collect();
            assert_eq!(radiant, vec![json!(true), json!(true), json!(true)]);
            let mut books = rolled("Medium", "books", true, &[FILLER], "");
            take(&mut books, "Medium", true);
            let radiant: Vec<Value> = gained(&books).iter().map(|card| card["radiant"].clone()).collect();
            assert_eq!(radiant, vec![json!(true); 5]);
        }
    }

    mod pauses_and_replay_9_3 {
        use super::*;

        /// One `answer` action on the open prompt, with a fresh nonce.
        fn answer_with(state: &GameState, option: &str, nonce: &mut u32) -> GameState {
            *nonce += 1;
            let action = Action::new(
                ActionBody::Answer {
                    choice_id: state.pending.as_ref().map(|prompt| prompt.id.clone()).unwrap_or_default(),
                    selection: vec![Selection::Mode { option: option.to_string() }],
                },
                P1,
                format!("kys-json-{nonce}"),
            );
            let result = reduce(state, &action);
            if let Some(error) = result.error {
                panic!("{error}");
            }
            result.state
        }

        fn thaw(state: &GameState) -> GameState {
            serde_json::from_str(&serde_json::to_string(state).expect("the state is JSON")).expect("and back")
        }

        fn key_of(state: &GameState) -> Option<String> {
            let data = state.pending.as_ref().map(|prompt| prompt.resume.data.clone()).unwrap_or_default();
            answer_key_of(&data)
        }

        /// One action of the replayed game, logged as it is applied.
        fn act(state: &GameState, body: ActionBody, player: PlayerId, log: &mut Vec<Action>) -> GameState {
            let action = Action::new(body, player, format!("kys-replay-{}", log.len()));
            let result = reduce(state, &action);
            if let Some(error) = result.error {
                panic!("{error}");
            }
            log.push(action);
            result.state
        }

        #[test]
        fn r420_paused_at_either_prompt_the_state_survives_json_key_included_and_answers_to_the_same_hash() {
            let s = rolled("Medium", "ky", false, &[FILLER], "");
            let mut nonce = 0;
            let thawed = thaw(s.state());
            assert_eq!(hash_state(&thawed), hash_state(s.state()));
            let live = answer_with(s.state(), "Medium", &mut nonce);
            let frozen = answer_with(&thawed, "Medium", &mut nonce);
            assert_eq!(hash_state(&frozen), hash_state(&live));
            let right = key_of(&live).unwrap_or_default();
            let again = thaw(&live);
            assert_eq!(key_of(&again), Some(right.clone()));
            let from_again = hash_state(&answer_with(&again, &right, &mut nonce));
            let from_live = hash_state(&answer_with(&live, &right, &mut nonce));
            assert_eq!(from_again, from_live);
        }

        #[test]
        fn r420_r79_a_timeout_answers_the_open_problem_as_the_turn_clock_does_any_prompt() {
            let mut s = cast("kys-test-timeout", false, &[FILLER]);
            s.answer(json!("Easy"));
            let result = reduce(s.state(), &Action::new(ActionBody::Timeout, P1, "kys-timeout"));
            assert_eq!(result.error, None);
            assert!(result.state.pending.is_none());
            assert!(result.state.players.p1.graveyard.iter().any(|card| card.def_id == TEST));
        }

        #[test]
        fn r420_a_game_with_kys_test_replays_from_its_log_to_the_same_hash() {
            crate::register_all();
            let deck = |first: &str| -> Vec<String> {
                let mut cards = vec![first.to_string()];
                for id in [
                    "core-001", "core-002", "core-003", "core-004", "core-005", "core-006", "core-007", "core-008", "core-009",
                    "core-010", "core-011", "core-012", "core-013", "core-014", "core-016", "core-017", "core-019", "core-020",
                    "core-025",
                ] {
                    cards.push(id.to_string());
                }
                cards
            };
            let decks = (deck(TEST), deck("core-026"));
            let mut found: Option<(String, GameState)> = None;
            for n in 0..300 {
                let seed = format!("kys-test-replay-{n}");
                let options: CreateGameOptions = json_as(json!({ "seed": seed, "decks": [decks.0, decks.1] }));
                let begun = begin_game(&create_game(&options)).state;
                if begun.players.p1.hand.iter().any(|card| card.def_id == TEST) {
                    found = Some((seed, begun));
                    break;
                }
            }
            let Some((seed, begun)) = found else {
                panic!("no seed deals KY's Test to p1");
            };
            let mut log: Vec<Action> = Vec::new();
            let mut state = begun;
            for player in [P1, P2] {
                let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
                state = act(&state, ActionBody::Mulligan { keep }, player, &mut log);
            }
            let card = state.players.p1.hand.iter().find(|held| held.def_id == TEST).map(|held| held.id.clone());
            let play: ActionBody = json_as(json!({ "type": "play", "instanceId": card.unwrap_or_default() }));
            state = act(&state, play, P1, &mut log);
            let hard = ActionBody::Answer {
                choice_id: state.pending.as_ref().map(|prompt| prompt.id.clone()).unwrap_or_default(),
                selection: vec![Selection::Mode { option: "Hard".to_string() }],
            };
            state = act(&state, hard, P1, &mut log);
            let right = key_of(&state).unwrap_or_default();
            let answer = ActionBody::Answer {
                choice_id: state.pending.as_ref().map(|prompt| prompt.id.clone()).unwrap_or_default(),
                selection: vec![Selection::Mode { option: right }],
            };
            state = act(&state, answer, P1, &mut log);
            assert!(state.players.p1.hand.iter().any(|held| held.def_id == GIFT));
            let args: FoldArgs = json_as(json!({ "seed": seed, "decks": [decks.0, decks.1], "log": log }));
            let replayed = fold(&args);
            assert!(replayed.errors.is_empty());
            assert_eq!(hash_state(&replayed.state), hash_state(&state));
        }
    }
}
