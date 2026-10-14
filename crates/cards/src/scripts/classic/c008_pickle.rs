//! C #8 Pickle (SPEC §8.6 row 8, §6.3 Choose one, §10.6; R16, R79, R97, R113, R177). Spell, cost 1, Rare.
//!   Base:    "Your opponent chooses {choices|time|times}: they discard {discard|card|cards}, they exile
//!            the bottom {exile|card|cards} of their deck, or you draw {draw|card|cards}." (3; 1, 1, 1)
//!   Radiant: the same words with 3; 2, 2, 2.
//!   Engine:  "Three mode prompts held by the opponent (§10.6), one after another, repeats allowed;
//!            "discard" discards at random from their hand (R682). Only choices that would do something
//!            are offered, and "you draw" always is. The opponent answers during your turn, a non-active
//!            player's prompt on its own clock (R79); a timeout answers with the AI policy. Tunes:
//!            choices 3 ↑; discard 1 ↑; exile 1 ↑; draw 1 ↑."
//!
//! THE CHOICES are mode prompts the opponent holds (B5 E18, `chooseMode({ by: "enemy" })`): they read
//! the options and answer, and the answered step still runs as this card's controller, so "you draw"
//! draws for you. Each answer asks the next question, so the chain is a named step (`chosen`) and a
//! count carried in the prompt's data (§10.6): `n` is the question being asked, and the numbers the
//! card declares (`param`) are read once as the Spell resolves and carried with it, so every question
//! of one resolution reads the same numbers.
//!
//! ONLY CHOICES THAT WOULD DO SOMETHING are offered, read as each question is asked: "discard" while
//! their hand holds a card, "exile" while their deck does, and "you draw" always — a draw from an
//! empty deck is fatigue (§2.4), which still does something. The options name no card. R800: "discard"
//! is not offered against a guarded hand (Meditative #1 Disruptive Disruptor) — the guard would stop
//! the discard, so the choice would do nothing.
//!
//! "DISCARD" is random from their hand (R682: no "of your choice"), so no prompt opens and the next
//! question follows at once. You read only the discard events, never the cards (R177).
//!
//! THE CLOCK is not the card's: the engine runs a non-active player's prompt on its own clock, and a
//! timeout answers it with the AI policy (R79).


pub const ID: &str = "classic-008";

// C #8 Pickle — SPEC §8.6 row 8, BUILD M9 Classic row C 8: "Three mode prompts held by the opponent
// during your turn, one after another, repeats allowed: they discard a card at random (R682),
// they exile the bottom card of their deck, or you draw 1; a mode that would do nothing is not
// offered (discard with an empty hand, exile with an empty deck), and "you draw" always is, fatigue
// included; each prompt runs its own clock and a timeout answers it with the AI policy (R79); your
// view names none of their remaining hand, their view never names your drawn card, and the mode
// options name no card; the paused prompts survive a JSON round trip; radiant: discard 2, exile the
// bottom 2, or draw 2; its tuned numbers (choices, discard, exile, draw) read through `param()`
// (R386)".

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PICKLE: &str = "classic-008";
    const FILLER: &str = "core-005"; // (1) Spell, p1's spare card (§2.5).
    // p1's library: distinct definitions, top first.
    const MY_DECK: [&str; 4] = ["core-019", "core-025", "core-012", "core-020"];
    // p2's hand and library: definitions p1 holds nowhere.
    const THEIR_HAND: [&str; 3] = ["core-008", "core-011", "core-015"];
    const THEIR_DECK: [&str; 4] = ["core-006", "core-035", "core-048", "core-039"];

    use crate::js;

    /// TS `JSON.parse(JSON.stringify(state))`.
    fn round_trip(state: &GameState) -> GameState {
        serde_json::from_value(js(state)).expect("the state round-trips")
    }

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(value) => value,
            None => panic!("the scenario has no {what}"),
        }
    }

    fn open(s: &Scenario) -> PendingChoice {
        must(s.state().pending.clone(), "open prompt")
    }

    fn modes(s: &Scenario) -> Vec<String> {
        open(s)
            .options
            .iter()
            .map(|option| match &option.selection {
                Selection::Mode { option } => option.clone(),
                _ => "?".to_string(),
            })
            .collect()
    }

    fn labels(s: &Scenario) -> Vec<String> {
        open(s).options.iter().map(|option| option.label.clone()).collect()
    }

    #[derive(Default)]
    struct Piles {
        my_deck: Option<Vec<&'static str>>,
        their_hand: Option<Vec<&'static str>>,
        their_deck: Option<Vec<&'static str>>,
    }

    fn pickle(radiant_face: bool, piles: Piles) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": PICKLE, "radiant": radiant_face }, FILLER],
                "library": piles.my_deck.unwrap_or_else(|| MY_DECK.to_vec()),
            },
            "p2": {
                "hand": piles.their_hand.unwrap_or_else(|| THEIR_HAND.to_vec()),
                "library": piles.their_deck.unwrap_or_else(|| THEIR_DECK.to_vec()),
            },
        }))
    }

    fn defs(s: &Scenario, player: PlayerId, zone: &str) -> Vec<String> {
        s.pile(player, zone).into_iter().map(|card| card.def_id).collect()
    }

    fn reduce_json(state: &GameState, action: Value) -> ReduceResult {
        reduce(state, &json_as::<Action>(action))
    }

    mod c_n8_pickle {
        use super::*;
        use std::sync::Arc;

        #[test]
        fn declares_its_four_numbers_r386_choices_3_discard_exile_draw_1_radiant_2() {
            crate::register_all();
            assert_eq!(
                js(&crate::card_def(PICKLE).params),
                json!([
                    { "key": "choices", "base": 3, "radiant": 3, "better": "up", "step": 1, "min": 1 },
                    { "key": "discard", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 },
                    { "key": "exile", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 },
                    { "key": "draw", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 },
                ])
            );
            // The same script on both faces: only the numbers differ.
            let scripts = crate::scripts_of()[ID].clone();
            let (Some(base_cry), Some(radiant_cry)) = (&scripts.base.cry, &scripts.radiant.cry) else {
                panic!("both faces have a Cry");
            };
            assert!(Arc::ptr_eq(base_cry, radiant_cry));
            let (Some(base_step), Some(radiant_step)) =
                (scripts.base.resume.get("chosen"), scripts.radiant.resume.get("chosen"))
            else {
                panic!("both faces have the chosen step");
            };
            assert!(Arc::ptr_eq(base_step, radiant_step));
        }

        mod base {
            use super::*;

            #[test]
            fn e18_the_first_question_is_a_mode_prompt_your_opponent_holds_during_your_turn_offering_all_three() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                let pending = open(&s);
                assert_eq!(pending.player_id, P2);
                assert_eq!(pending.kind, PromptKind::Mode);
                assert_eq!(modes(&s), vec!["discard", "exile", "draw"]);
                assert_eq!(s.state().active, P1);
            }

            #[test]
            fn three_questions_one_after_another_repeats_allowed_three_draws_draw_you_three_cards() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                for _question in 0..3 {
                    assert_eq!(open(&s).player_id, P2);
                    s.answer(json!("draw"));
                }
                assert!(s.state().pending.is_none());
                let mut expected = vec![FILLER];
                expected.extend_from_slice(&MY_DECK[..3]);
                assert_eq!(defs(&s, P1, "hand"), expected);
                s.expect_in_zone(PICKLE, "graveyard");
            }

            #[test]
            fn r682_discard_discards_a_random_card_of_theirs_at_once_with_no_hand_pick() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                s.answer(json!("discard"));
                // No follow-up pick: the random discard landed and the next question is open.
                assert_eq!(open(&s).kind, PromptKind::Mode);
                let grave = defs(&s, P2, "graveyard");
                assert_eq!(grave.len(), 1);
                assert!(THEIR_HAND.contains(&grave[0].as_str()));
                s.expect_events(json!(["discarded"]));
            }

            #[test]
            fn exile_takes_the_bottom_card_of_their_deck() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                s.answer(json!("exile"));
                assert_eq!(defs(&s, P2, "exile"), vec![THEIR_DECK[3]]);
                assert_eq!(defs(&s, P2, "library"), THEIR_DECK[..3].to_vec());
            }

            #[test]
            fn a_mix_discard_exile_draw_in_any_order() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                s.answer(json!("exile"));
                s.answer(json!("discard"));
                s.answer(json!("draw"));
                assert!(s.state().pending.is_none());
                assert_eq!(defs(&s, P2, "exile"), vec![THEIR_DECK[3]]);
                let grave = defs(&s, P2, "graveyard");
                assert_eq!(grave.len(), 1);
                assert!(THEIR_HAND.contains(&grave[0].as_str()));
                assert_eq!(defs(&s, P1, "hand"), vec![FILLER, MY_DECK[0]]);
            }

            #[test]
            fn discard_is_not_offered_while_their_hand_is_empty_nor_exile_while_their_deck_is() {
                crate::register_all();
                let mut s = pickle(
                    false,
                    Piles {
                        their_hand: Some(vec![]),
                        their_deck: Some(vec![]),
                        ..Piles::default()
                    },
                );
                s.play(PICKLE, json!({}));
                assert_eq!(modes(&s), vec!["draw"]);
            }

            #[test]
            fn each_question_reads_their_piles_as_it_is_asked_an_exile_that_empties_their_deck_takes_exile_off_the_next() {
                crate::register_all();
                let mut s = pickle(false, Piles { their_deck: Some(vec!["core-006"]), ..Piles::default() });
                s.play(PICKLE, json!({}));
                assert_eq!(modes(&s), vec!["discard", "exile", "draw"]);
                s.answer(json!("exile"));
                assert_eq!(modes(&s), vec!["discard", "draw"]);
            }

            #[test]
            fn a_discard_that_empties_their_hand_takes_discard_off_the_next_question() {
                crate::register_all();
                let mut s = pickle(false, Piles { their_hand: Some(vec!["core-008"]), ..Piles::default() });
                s.play(PICKLE, json!({}));
                s.answer(json!("discard"));
                assert_eq!(defs(&s, P2, "graveyard"), vec!["core-008"]);
                assert_eq!(modes(&s), vec!["exile", "draw"]);
            }

            #[test]
            fn s2_4_you_draw_is_always_offered_and_from_an_empty_deck_it_is_fatigue() {
                crate::register_all();
                let mut s = pickle(
                    false,
                    Piles {
                        my_deck: Some(vec![]),
                        their_hand: Some(vec![]),
                        their_deck: Some(vec![]),
                    },
                );
                s.play(PICKLE, json!({}));
                s.answer(json!("draw"));
                s.answer(json!("draw"));
                s.answer(json!("draw"));
                assert!(s.state().pending.is_none());
                // Three fatigue draws: 1 + 2 + 3.
                s.expect_health(P1, 24);
            }

            #[test]
            fn r79_each_question_runs_its_opponent_s_own_clock_a_timeout_answers_the_one_open_with_the_ai_policy() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                let first = open(&s);
                let result = reduce_json(s.state(), json!({ "type": "timeout", "playerId": "p2", "nonce": "pickle-timeout" }));
                assert!(result.error.is_none());
                // The first question was answered; the next is open for p2 (or its discard pick), p1 still active.
                let next = must(result.state.pending.clone(), "a following prompt");
                assert_ne!(next.id, first.id);
                assert_eq!(next.player_id, P2);
                assert_eq!(result.state.active, P1);
                assert_eq!(
                    result
                        .events
                        .iter()
                        .filter(|event| event.event_type() == GameEventType::PromptAnswered)
                        .count(),
                    1
                );
            }

            #[test]
            fn r177_your_view_names_none_of_their_remaining_hand() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                s.answer(json!("discard"));
                let mine = js(&s.view(P1)).to_string();
                for card in s.hand(P2) {
                    assert!(!mine.contains(&format!("\"{}\"", card.id)));
                    assert!(!mine.contains(&card.def_id));
                }
                // The random discard itself is public: the graveyard names it.
                for card in s.pile(P2, "graveyard") {
                    assert!(mine.contains(&card.id));
                }
            }

            #[test]
            fn r97_their_view_never_names_the_card_you_draw() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                s.answer(json!("draw"));
                let drawn = s.card(MY_DECK[0]).clone();
                s.expect_in_zone(&drawn, "hand");
                let theirs = js(&s.view(P2)).to_string();
                assert!(!theirs.contains(&format!("\"{}\"", drawn.id)));
                assert!(!theirs.contains(MY_DECK[0]));
            }

            #[test]
            fn the_mode_options_name_no_card_only_what_each_would_do() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                let view = js(&s.view(P2));
                let theirs = must(Some(&view["pending"]).filter(|pending| !pending.is_null()), "p2's view of the question");
                assert_eq!(theirs["forYou"], json!(true));
                assert_eq!(
                    labels(&s),
                    vec!["Discard 1 card", "Exile the bottom 1 card of your deck", "Your opponent draws 1 card"]
                );
                // p1 reads only that p2 is choosing.
                assert_eq!(js(&s.view(P1))["pending"], json!({ "forYou": false, "pendingFor": "p2" }));
            }

            #[test]
            fn s9_3_the_paused_questions_survive_a_json_round_trip_and_resume_through_reduce() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                s.play(PICKLE, json!({}));
                s.answer(json!("draw"));
                let revived = round_trip(s.state());
                assert_eq!(&revived, s.state());
                let second = must(revived.pending.clone(), "the second question");
                let answered = reduce_json(
                    &revived,
                    json!({
                        "type": "answer",
                        "playerId": "p2",
                        "choiceId": second.id,
                        "selection": [{ "pick": "mode", "option": "exile" }],
                        "nonce": "pickle-round-trip-1",
                    }),
                );
                assert!(answered.error.is_none());
                let third = must(answered.state.pending.clone(), "the third question");
                assert_eq!(third.player_id, P2);
                // And the discard pick, paused in turn, round-trips too.
                let again = round_trip(&answered.state);
                let picked = reduce_json(
                    &again,
                    json!({
                        "type": "answer",
                        "playerId": "p2",
                        "choiceId": third.id,
                        "selection": [{ "pick": "mode", "option": "draw" }],
                        "nonce": "pickle-round-trip-2",
                    }),
                );
                assert!(picked.error.is_none());
                assert!(picked.state.pending.is_none());
                assert!(picked.state.work.is_empty());
            }

            #[test]
            fn r682_the_random_discard_comes_from_the_match_rng_the_same_game_discards_the_same_card() {
                crate::register_all();
                let mut first = pickle(false, Piles::default());
                first.play(PICKLE, json!({}));
                first.answer(json!("discard"));
                let mut second = pickle(false, Piles::default());
                second.play(PICKLE, json!({}));
                second.answer(json!("discard"));
                let ids = |s: &Scenario| -> Vec<String> {
                    s.events()
                        .iter()
                        .filter_map(|event| match event {
                            GameEvent::Discarded { instance_id, .. } => Some(instance_id.clone()),
                            _ => None,
                        })
                        .collect()
                };
                assert_eq!(ids(&first), ids(&second));
            }

            #[test]
            fn r386_an_upgrade_of_choices_asks_a_fourth_question() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                step_param(s.card_mut(PICKLE), "choices", 1);
                s.play(PICKLE, json!({}));
                for _question in 0..4 {
                    s.answer(json!("draw"));
                }
                assert!(s.state().pending.is_none());
                let mut expected = vec![FILLER];
                expected.extend_from_slice(&MY_DECK);
                assert_eq!(defs(&s, P1, "hand"), expected);
            }

            #[test]
            fn r386_an_upgrade_of_draw_makes_you_draw_2() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                step_param(s.card_mut(PICKLE), "draw", 1);
                s.play(PICKLE, json!({}));
                s.answer(json!("draw"));
                assert_eq!(defs(&s, P1, "hand"), vec![FILLER, MY_DECK[0], MY_DECK[1]]);
            }

            #[test]
            fn r386_an_upgrade_of_discard_makes_them_discard_2_and_of_exile_exiles_2() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                step_param(s.card_mut(PICKLE), "discard", 1);
                step_param(s.card_mut(PICKLE), "exile", 1);
                s.play(PICKLE, json!({}));
                s.answer(json!("discard"));
                let grave = defs(&s, P2, "graveyard");
                assert_eq!(grave.len(), 2);
                for def_id in &grave {
                    assert!(THEIR_HAND.contains(&def_id.as_str()));
                }
                s.answer(json!("exile"));
                assert_eq!(defs(&s, P2, "exile"), vec![THEIR_DECK[3], THEIR_DECK[2]]);
            }

            #[test]
            fn r386_a_degrade_of_choices_asks_only_two_questions() {
                crate::register_all();
                let mut s = pickle(false, Piles::default());
                step_param(s.card_mut(PICKLE), "choices", -1);
                s.play(PICKLE, json!({}));
                s.answer(json!("draw"));
                s.answer(json!("draw"));
                assert!(s.state().pending.is_none());
            }

            #[test]
            fn r800_offers_no_discard_against_a_guarded_hand() {
                crate::register_all();
                // p2's Disruptor guards p2's side while p1 is active: the discard would do nothing.
                let mut s = scenario(json!({
                    "p1": { "hand": [PICKLE, FILLER], "library": MY_DECK.to_vec() },
                    "p2": {
                        "hand": THEIR_HAND.to_vec(),
                        "library": THEIR_DECK.to_vec(),
                        "backrow": [{ "def": "meditative-001", "faceUp": true, "lane": 1 }],
                    },
                }));
                s.play(PICKLE, json!({}));
                assert_eq!(modes(&s), vec!["exile", "draw"]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r682_they_discard_2_cards_at_random() {
                crate::register_all();
                let mut s = pickle(true, Piles::default());
                s.play(PICKLE, json!({}));
                assert_eq!(
                    labels(&s),
                    vec!["Discard 2 cards", "Exile the bottom 2 cards of your deck", "Your opponent draws 2 cards"]
                );
                s.answer(json!("discard"));
                assert_eq!(open(&s).kind, PromptKind::Mode);
                let grave = defs(&s, P2, "graveyard");
                assert_eq!(grave.len(), 2);
                for def_id in &grave {
                    assert!(THEIR_HAND.contains(&def_id.as_str()));
                }
            }

            #[test]
            fn a_hand_of_one_card_discards_that_one() {
                crate::register_all();
                let mut s = pickle(true, Piles { their_hand: Some(vec!["core-008"]), ..Piles::default() });
                s.play(PICKLE, json!({}));
                s.answer(json!("discard"));
                assert!(s.hand(P2).is_empty());
                assert_eq!(defs(&s, P2, "graveyard"), vec!["core-008"]);
            }

            #[test]
            fn they_exile_the_bottom_2_cards_of_their_deck() {
                crate::register_all();
                let mut s = pickle(true, Piles::default());
                s.play(PICKLE, json!({}));
                s.answer(json!("exile"));
                assert_eq!(defs(&s, P2, "exile"), vec![THEIR_DECK[3], THEIR_DECK[2]]);
            }

            #[test]
            fn you_draw_2() {
                crate::register_all();
                let mut s = pickle(true, Piles::default());
                s.play(PICKLE, json!({}));
                s.answer(json!("draw"));
                assert_eq!(defs(&s, P1, "hand"), vec![FILLER, MY_DECK[0], MY_DECK[1]]);
            }

            #[test]
            fn an_exile_of_2_that_empties_their_deck_takes_exile_off_the_next_question() {
                crate::register_all();
                let mut s = pickle(true, Piles { their_deck: Some(vec!["core-006", "core-035"]), ..Piles::default() });
                s.play(PICKLE, json!({}));
                s.answer(json!("exile"));
                assert_eq!(modes(&s), vec!["discard", "draw"]);
            }
        }
    }
}
