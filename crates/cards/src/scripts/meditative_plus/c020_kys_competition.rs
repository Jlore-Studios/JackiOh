//! Meditative+ #20 KY's Competition (SPEC §8.9 row 20, R1184000, R1184001, R420, R465). (2) Spell,
//! KY, Rare.
//!
//!   Base:    "Answer three-digit sums one after another until you stop or miss one. Then summon a
//!             Lil' Kenny with X/X, X your correct answers."
//!   Radiant: "Answer one- and two-digit sums one after another until you stop or miss one. Then
//!             summon a Radiant Lil' Kenny with 2X/2X, X your correct answers."
//!
//! The whole card is the engine's question sequence (`subsystems/ky_competition.rs`): C+ #42's
//! `answer` prompt asked again after each right answer, with Stop, the streak in the prompt's resume
//! data, each question rolled from the match rng (three-digit addition or subtraction; the Radiant
//! face one- and two-digit), the key never leaving the engine (R465); a wrong answer, Stop or a timeout
//! (R1184001) ends it, and so does the `KY_COMPETITION_MAX`th right answer. Then M+ #20.1 Lil' Kenny is
//! summoned with X the right answers (R1184003), Radiant on the Radiant face, whose own face makes it
//! 2X/2X; none at X = 0. Both faces run the same script: the steps read the face that cast it.
//! Declares no numbers.
//!
//! Voice audio is owed until the set ships (R1420): the `card-audio.json5` entry is written, and the
//! release renders it (`[gen:voice]`, R1448).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-020";

/// The token an ended sequence summons (M+ #20.1).
const KENNY: &str = "meditativeplus-020-1";

pub fn script() -> CardScripts {
    let made = subsystems::ky_competition_script(KENNY);
    let base = Script {
        cry: Some(made.cry),
        resume: made.resume,
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Meditative+ #20 KY's Competition — SPEC §8.9 row 20, R1184000, R1184001, R420, R465, BUILD M11 row
// M+ 20: "asks a three-digit sum, then another after each right answer, the key in no view or event
// (R420); a wrong answer ends it, a Stop ends it, a timeout ends it; after 3 right then a miss, summons
// Lil' Kenny 3/3; after 0, nothing; at the cap, 30/30; the paused state survives a JSON round trip and
// replays; the AI answers within its budget; radiant the problems are one- or two-digit and the Kenny
// is Radiant (2X/2X)". The AI's answer is `crates/ai/tests/ai/ky_competition.rs`'s (R1184002); the
// engine proves the machinery with fixtures (`crates/engine/tests/rules/ky_competition.rs`).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005";

    /// The Competition cast from p1's hand, its first question open.
    fn cast(seed: &str, radiant: bool) -> Scenario {
        let mut s = crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(ID, json!({}));
        s
    }

    fn key(s: &Scenario) -> String {
        let data = s
            .state()
            .pending
            .as_ref()
            .map(|prompt| prompt.resume.data.clone())
            .unwrap_or_default();
        match answer_key_of(&data) {
            Some(found) => found,
            None => panic!("a question with its key"),
        }
    }

    /// Answer the open question rightly or wrongly.
    fn answer(s: &mut Scenario, right: bool) {
        let correct = key(s);
        let letter = if right {
            correct
        } else {
            ["A", "B", "C", "D"]
                .into_iter()
                .find(|letter| *letter != correct)
                .unwrap_or_default()
                .to_string()
        };
        s.answer(json!(format!("mode:{letter}")));
    }

    fn statement(s: &Scenario) -> String {
        s.state()
            .pending
            .as_ref()
            .map(|prompt| prompt.prompt.clone())
            .unwrap_or_default()
    }

    fn streak(s: &Scenario) -> Option<i32> {
        s.state().pending.as_ref().and_then(|prompt| prompt.streak)
    }

    /// `a op b = ?`'s two operands.
    fn operands(text: &str) -> (i32, i32) {
        let pieces: Vec<&str> = text.trim_end_matches(" = ?").split(' ').collect();
        let number = |at: usize| {
            pieces
                .get(at)
                .and_then(|piece| piece.parse::<i32>().ok())
                .unwrap_or(-1)
        };
        (number(0), number(2))
    }

    /// The Lil' Kennies on p1's field: (X, attack, health, Radiant).
    fn kennies(s: &Scenario) -> Vec<(i32, i32, i32, bool)> {
        active_units_of(s.state(), P1)
            .into_iter()
            .filter(|card| card.def_id == KENNY)
            .map(|card| {
                let view = unit_view(s.state(), card);
                (x_of(card), view.attack, view.health, card.radiant)
            })
            .collect()
    }

    fn spent(s: &Scenario) -> bool {
        s.state().pending.is_none()
            && s.state()
                .players
                .p1
                .graveyard
                .iter()
                .any(|card| card.def_id == ID)
    }

    #[test]
    fn declares_no_numbers() {
        crate::register_all();
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r1184000_asks_a_three_digit_sum_under_four_letters_then_stop_with_no_answer_right_yet() {
            let s = cast("kc-card-ask", false);
            let prompt = s.state().pending.clone().unwrap_or_else(|| panic!("a question"));
            assert_eq!(prompt.kind, PromptKind::Answer);
            let keys: Vec<String> = prompt.options.iter().map(|option| option.key.clone()).collect();
            assert_eq!(
                keys,
                vec!["mode:A", "mode:B", "mode:C", "mode:D", ANSWER_STOP_KEY]
            );
            assert_eq!(prompt.streak, Some(0));
            let (a, b) = operands(&prompt.prompt);
            for operand in [a, b] {
                assert!((100..=999).contains(&operand), "{}", prompt.prompt);
            }
        }

        #[test]
        fn r1184000_a_right_answer_asks_another_three_digit_sum_the_streak_one_higher() {
            let mut s = cast("kc-card-next", false);
            answer(&mut s, true);
            assert_eq!(streak(&s), Some(1));
            answer(&mut s, true);
            assert_eq!(streak(&s), Some(2));
            let (a, b) = operands(&statement(&s));
            assert!((100..=999).contains(&a) && (100..=999).contains(&b));
            assert!(kennies(&s).is_empty());
        }

        #[test]
        fn r1184000_three_right_then_a_miss_summons_lil_kenny_3_3() {
            let mut s = cast("kc-card-miss", false);
            for _ in 0..3 {
                answer(&mut s, true);
            }
            answer(&mut s, false);
            assert!(spent(&s));
            assert_eq!(kennies(&s), vec![(3, 3, 3, false)]);
        }

        #[test]
        fn r1184000_stop_ends_it_with_the_right_answers_so_far() {
            let mut s = cast("kc-card-stop", false);
            for _ in 0..5 {
                answer(&mut s, true);
            }
            s.answer(json!(ANSWER_STOP_KEY));
            assert!(spent(&s));
            assert_eq!(kennies(&s), vec![(5, 5, 5, false)]);
        }

        #[test]
        fn r1184000_a_miss_or_a_stop_at_the_first_question_summons_nothing_and_the_spell_was_still_played() {
            for by_stop in [false, true] {
                let mut s = cast(&format!("kc-card-zero-{by_stop}"), false);
                if by_stop {
                    s.answer(json!(ANSWER_STOP_KEY));
                } else {
                    answer(&mut s, false);
                }
                assert!(spent(&s));
                assert!(kennies(&s).is_empty());
                s.expect_events(json!(["cardPlayed", "cardResolved"]));
            }
        }

        #[test]
        fn r1184000_the_thirtieth_right_answer_ends_it_with_a_30_30() {
            let mut s = cast("kc-card-cap", false);
            for _ in 0..KY_COMPETITION_MAX {
                answer(&mut s, true);
            }
            assert!(spent(&s));
            assert_eq!(
                kennies(&s),
                vec![(KY_COMPETITION_MAX, KY_COMPETITION_MAX, KY_COMPETITION_MAX, false)]
            );
        }

        #[test]
        fn r1184001_r79_a_timeout_ends_it_as_a_stop_and_ends_the_turn() {
            let mut s = cast("kc-card-timeout", false);
            answer(&mut s, true);
            answer(&mut s, true);
            let result = reduce(
                s.state(),
                &Action::new(ActionBody::Timeout, P1, "kc-card-timeout"),
            );
            assert_eq!(result.error, None);
            assert!(result.state.pending.is_none());
            assert_eq!(result.state.active, P2);
            let made: Vec<(i32, bool)> = active_units_of(&result.state, P1)
                .into_iter()
                .filter(|card| card.def_id == KENNY)
                .map(|card| (x_of(card), card.radiant))
                .collect();
            assert_eq!(made, vec![(2, false)]);
        }

        #[test]
        fn r465_r1184000_the_key_reaches_no_view_and_no_event_the_chooser_sees_the_streak_the_opponent_only_a_prompt()
         {
            let mut s = cast("kc-card-view", false);
            answer(&mut s, true);
            for viewer in [P1, P2] {
                let text = serde_json::to_string(&s.view(viewer)).expect("a view is JSON");
                assert!(!text.contains(ANSWER_KEY));
            }
            let events = serde_json::to_string(s.events()).expect("the events are JSON");
            assert!(!events.contains(ANSWER_KEY));
            let mine = serde_json::to_value(s.view(P1)).expect("a view is JSON");
            assert_eq!(mine["pending"]["streak"], json!(1));
            assert_eq!(mine["pending"]["prompt"], json!(statement(&s)));
            let theirs = serde_json::to_value(s.view(P2)).expect("a view is JSON");
            assert_eq!(theirs["pending"], json!({ "forYou": false, "pendingFor": "p1" }));
        }

        #[test]
        fn r1184000_paused_mid_sequence_the_state_survives_json_and_answers_to_the_same_hash() {
            let mut s = cast("kc-card-json", false);
            answer(&mut s, true);
            let live = s.state().clone();
            let thawed: GameState =
                serde_json::from_str(&serde_json::to_string(&live).expect("the state is JSON"))
                    .expect("and back");
            assert_eq!(hash_state(&thawed), hash_state(&live));
            let key = key(&s);
            let pick = |state: &GameState| {
                let action = Action::new(
                    ActionBody::Answer {
                        choice_id: state
                            .pending
                            .as_ref()
                            .map(|prompt| prompt.id.clone())
                            .unwrap_or_default(),
                        selection: vec![Selection::Mode { option: key.clone() }],
                    },
                    P1,
                    "kc-card-json",
                );
                reduce(state, &action).state
            };
            assert_eq!(hash_state(&pick(&thawed)), hash_state(&pick(&live)));
        }

        #[test]
        fn r1184000_a_game_with_the_competition_replays_from_its_log_to_the_same_hash() {
            crate::register_all();
            let _preview = preview_sets(&[SetName::MeditativePlus]);
            let mut deck = vec![ID.to_string()];
            for id in [
                "core-001", "core-002", "core-003", "core-004", "core-005", "core-006", "core-007",
                "core-008", "core-009", "core-010", "core-011", "core-012", "core-013", "core-014",
                "core-016", "core-017", "core-019", "core-020", "core-025",
            ] {
                deck.push(id.to_string());
            }
            let other: Vec<String> = deck
                .iter()
                .skip(1)
                .cloned()
                .chain(["core-026".to_string()])
                .collect();
            let mut log: Vec<Action> = Vec::new();
            let act =
                |state: &GameState, body: ActionBody, player: PlayerId, log: &mut Vec<Action>| -> GameState {
                    let action = Action::new(body, player, format!("kc-replay-{}", log.len()));
                    let result = reduce(state, &action);
                    if let Some(error) = result.error {
                        panic!("{error}");
                    }
                    log.push(action);
                    result.state
                };
            let mut found: Option<(String, GameState)> = None;
            for n in 0..300 {
                let seed = format!("kc-card-replay-{n}");
                let options: CreateGameOptions = json_as(json!({ "seed": seed, "decks": [deck, other] }));
                let begun = begin_game(&create_game(&options)).state;
                if begun.players.p1.hand.iter().any(|card| card.def_id == ID) {
                    found = Some((seed, begun));
                    break;
                }
            }
            let Some((seed, mut state)) = found else {
                panic!("no seed deals KY's Competition to p1");
            };
            for player in [P1, P2] {
                let keep: Vec<String> = state.players[player]
                    .hand
                    .iter()
                    .map(|card| card.id.clone())
                    .collect();
                state = act(&state, ActionBody::Mulligan { keep }, player, &mut log);
            }
            // p1's first turn has 1 mana: pass to p1's second turn, at 2.
            state = act(&state, ActionBody::EndTurn, P1, &mut log);
            state = act(&state, ActionBody::EndTurn, P2, &mut log);
            let card = state
                .players
                .p1
                .hand
                .iter()
                .find(|held| held.def_id == ID)
                .map(|held| held.id.clone());
            let play: ActionBody = json_as(json!({ "type": "play", "instanceId": card.unwrap_or_default() }));
            state = act(&state, play, P1, &mut log);
            for _ in 0..2 {
                let right = answer_key_of(
                    &state
                        .pending
                        .as_ref()
                        .map(|p| p.resume.data.clone())
                        .unwrap_or_default(),
                );
                let body = ActionBody::Answer {
                    choice_id: state
                        .pending
                        .as_ref()
                        .map(|prompt| prompt.id.clone())
                        .unwrap_or_default(),
                    selection: vec![Selection::Mode {
                        option: right.unwrap_or_default(),
                    }],
                };
                state = act(&state, body, P1, &mut log);
            }
            let stop = ActionBody::Answer {
                choice_id: state
                    .pending
                    .as_ref()
                    .map(|prompt| prompt.id.clone())
                    .unwrap_or_default(),
                selection: vec![Selection::None],
            };
            state = act(&state, stop, P1, &mut log);
            assert!(
                active_units_of(&state, P1)
                    .iter()
                    .any(|card| card.def_id == KENNY && x_of(card) == 2)
            );
            let args: FoldArgs = json_as(json!({ "seed": seed, "decks": [deck, other], "log": log }));
            let folded = fold(&args);
            assert!(
                folded.errors.is_empty(),
                "the fold refused nothing: {:?}",
                folded.errors
            );
            assert_eq!(hash_state(&folded.state), hash_state(&state));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1184000_asks_one_and_two_digit_sums() {
            let mut s = cast("kc-card-easy", true);
            let mut one_digit = false;
            for _ in 0..KY_COMPETITION_MAX - 1 {
                let (a, b) = operands(&statement(&s));
                for operand in [a, b] {
                    assert!((1..=99).contains(&operand), "{}", statement(&s));
                }
                one_digit |= a < 10 || b < 10;
                answer(&mut s, true);
            }
            assert!(one_digit);
        }

        #[test]
        fn r1184000_three_right_then_a_miss_summons_a_radiant_lil_kenny_6_6() {
            let mut s = cast("kc-card-radiant", true);
            for _ in 0..3 {
                answer(&mut s, true);
            }
            answer(&mut s, false);
            assert!(spent(&s));
            assert_eq!(kennies(&s), vec![(3, 6, 6, true)]);
        }

        #[test]
        fn r1184000_stop_at_the_first_question_summons_nothing() {
            let mut s = cast("kc-card-radiant-zero", true);
            s.answer(json!(ANSWER_STOP_KEY));
            assert!(spent(&s));
            assert!(kennies(&s).is_empty());
        }

        #[test]
        fn r1184000_the_thirtieth_right_answer_ends_it_with_a_radiant_60_60() {
            let mut s = cast("kc-card-radiant-cap", true);
            for _ in 0..KY_COMPETITION_MAX {
                answer(&mut s, true);
            }
            assert!(spent(&s));
            assert_eq!(
                kennies(&s),
                vec![(
                    KY_COMPETITION_MAX,
                    2 * KY_COMPETITION_MAX,
                    2 * KY_COMPETITION_MAX,
                    true
                )]
            );
        }
    }
}
