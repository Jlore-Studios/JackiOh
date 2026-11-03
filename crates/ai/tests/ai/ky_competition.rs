//! R1184002: the AI's answer to Meditative+ #20 KY's Competition's question sequence (R1184000). The
//! AI works each question out from the statement its redacted prompt shows, at once and with no
//! search, and never stops: thirty questions cost it what one redaction does, inside its decision
//! budget, and it never reads the key (R465). R1563005: it answers right 85% of the time on the base
//! face and 93% on the Radiant one, a miss one of the wrong answers, drawn from its own stream.

use jackioh_ai::{
    AI_BUDGET, AI_KY_COMPETITION_ACCURACY, AI_KY_COMPETITION_EXPECTED_STREAK,
    AI_KY_COMPETITION_RADIANT_ACCURACY, AI_KY_COMPETITION_RADIANT_EXPECTED_STREAK, AiOptions, DecisionReason,
    NodeCounter, Prices, create_node_counter, decide, own_prompt_score, play_ai_turn, redact, simulate,
    simulate_first_answers,
};
use jackioh_engine::subsystems::ky_competition::worked_answer;
use jackioh_engine::testkit::{
    ANSWER_KEY, ANSWER_STOP_KEY, ActionBody, GameState, KY_COMPETITION_MAX, PromptKind, Selection,
    active_units_of, answer_key_of, create_rng, json, legal_actions, x_of,
};

use super::support::{AI, act, register_cards, scenario};

const COMPETITION: &str = "meditativeplus-020";
const KENNY: &str = "meditativeplus-020-1";
const FILLER: &str = "core-005";
const VANILLA: &str = "core-008";

/// The AI's seat, p1, mid-turn with the Competition cast and its first question open.
fn asked(seed: &str, radiant: bool) -> GameState {
    register_cards();
    let mut s = scenario(json!({
        "seed": seed,
        "active": AI,
        "turn": 9,
        "p1": { "hand": [{ "def": COMPETITION, "radiant": radiant }, FILLER] },
        "p2": { "hand": [FILLER] },
    }));
    s.play(COMPETITION, json!({}));
    s.state().clone()
}

/// The right option's selection, read where only the engine reads it.
fn right_selection(state: &GameState) -> Selection {
    let pending = state.pending.as_ref().unwrap_or_else(|| panic!("a question"));
    let key = answer_key_of(&pending.resume.data).unwrap_or_else(|| panic!("its key"));
    Selection::Mode { option: key }
}

fn kenny_x(state: &GameState) -> Vec<i32> {
    active_units_of(state, AI)
        .into_iter()
        .filter(|card| card.def_id == KENNY)
        .map(x_of)
        .collect()
}

/// The AI's seat, p1, in its main phase with the Competition (`radiant` or not) and a filler in hand
/// and the 2 mana to cast either, each side a library to draw from.
fn holding(seed: &str, radiant: bool) -> GameState {
    register_cards();
    let library = [VANILLA; 6];
    scenario(json!({
        "seed": seed,
        "p1": { "hand": [{ "def": COMPETITION, "radiant": radiant }, FILLER], "mana": 2, "library": library },
        "p2": { "hand": [FILLER], "library": library },
    }))
    .state()
    .clone()
}

/// The cast of the Competition `state`'s AI holds.
fn cast(state: &GameState) -> ActionBody {
    let id = state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == COMPETITION)
        .map(|card| card.id.clone())
        .unwrap_or_else(|| panic!("the Competition in hand"));
    legal_actions(state, AI)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == id))
        .unwrap_or_else(|| panic!("its cast is legal"))
}

/// The right answers the search expects of the face's sequence (#2929).
fn expected_streak(radiant: bool) -> i32 {
    if radiant {
        AI_KY_COMPETITION_RADIANT_EXPECTED_STREAK
    } else {
        AI_KY_COMPETITION_EXPECTED_STREAK
    }
}

mod r1184002_the_ai_works_a_question_out_from_its_statement {
    use super::*;

    #[test]
    fn r1184002_r1563005_each_answer_is_given_at_once_with_no_search_a_letter_never_stop_until_a_miss_or_the_cap()
     {
        for radiant in [false, true] {
            let mut state = asked(&format!("r1184002-run-{radiant}"), radiant);
            let mut ai = AiOptions::with_budget(create_rng("r1184002-run", 0), AI_BUDGET);
            let mut right = 0;
            while state.pending.is_some() {
                let decision = decide(&state, AI, &mut ai).unwrap_or_else(|| panic!("a decision"));
                assert_eq!(decision.reason, DecisionReason::Prompt);
                assert_eq!(decision.stats.nodes, 0, "no search");
                let ActionBody::Answer { selection, .. } = &decision.action else {
                    panic!("an answer, not {:?}", decision.action);
                };
                assert_ne!(
                    selection,
                    &vec![Selection::Mode {
                        option: ANSWER_STOP_KEY.to_string()
                    }]
                );
                assert!(legal_actions(&state, AI).contains(&decision.action));
                let was_right = selection == &vec![right_selection(&state)];
                state = act(&state, AI, &decision.action);
                if !was_right {
                    break;
                }
                right += 1;
                assert!(right <= KY_COMPETITION_MAX);
            }
            assert!(state.pending.is_none(), "a miss or the cap ends the sequence");
            let summoned = kenny_x(&state);
            if right == 0 {
                assert!(summoned.is_empty());
            } else {
                assert_eq!(summoned, vec![right]);
            }
        }
    }

    #[test]
    fn r1563005_it_answers_right_85_percent_of_the_time_and_93_on_the_radiant_face() {
        for (radiant, accuracy) in [
            (false, AI_KY_COMPETITION_ACCURACY),
            (true, AI_KY_COMPETITION_RADIANT_ACCURACY),
        ] {
            let mut asked_count = 0;
            let mut right = 0;
            for n in 0..400 {
                let state = asked(&format!("r1563005-{radiant}-{n}"), radiant);
                let mut ai = AiOptions::with_budget(create_rng(&format!("r1563005-ai-{n}"), 0), AI_BUDGET);
                let decision = decide(&state, AI, &mut ai).unwrap_or_else(|| panic!("a decision"));
                let ActionBody::Answer { selection, .. } = &decision.action else {
                    panic!("an answer");
                };
                asked_count += 1;
                if selection == &vec![right_selection(&state)] {
                    right += 1;
                }
            }
            let rate = f64::from(right) / f64::from(asked_count);
            assert!(
                (rate - accuracy).abs() < 0.06,
                "radiant {radiant}: {rate} against {accuracy}"
            );
        }
    }

    #[test]
    fn r1184002_r465_it_reads_the_statement_never_the_key_two_states_that_differ_in_the_key_alone_answer_alike()
     {
        let state = asked("r1184002-key", false);
        let mut twin = state.clone();
        let key = answer_key_of(
            &state
                .pending
                .as_ref()
                .map(|p| p.resume.data.clone())
                .unwrap_or_default(),
        );
        if let Some(pending) = twin.pending.as_mut() {
            pending.resume.data.insert(
                ANSWER_KEY.to_string(),
                json!(if key.as_deref() == Some("A") { "B" } else { "A" }),
            );
        }
        let answer = |state: &GameState| {
            decide(
                state,
                AI,
                &mut AiOptions::with_budget(create_rng("r1184002-key", 0), AI_BUDGET),
            )
            .map(|decision| decision.action)
        };
        assert_eq!(answer(&state), answer(&twin));
        // What it reads is the redacted prompt: its statement and options, with no key in it.
        let seen = redact(&state, AI);
        let pending = seen.pending.unwrap_or_else(|| panic!("the AI sees its question"));
        assert!(!pending.resume.data.contains_key(ANSWER_KEY));
        let worked = worked_answer(&pending.prompt).map(|value| value.to_string());
        let ActionBody::Answer { selection, .. } = answer(&state).unwrap_or_else(|| panic!("an answer"))
        else {
            panic!("an answer");
        };
        let picked = pending
            .options
            .iter()
            .find(|option| selection.contains(&option.selection));
        assert_eq!(picked.map(|option| option.label.clone()), worked);
        assert_ne!(picked.map(|option| option.key.as_str()), Some(ANSWER_STOP_KEY));
    }

    #[test]
    fn r1184002_a_question_it_cannot_work_out_goes_to_the_search_which_answers_with_a_legal_action() {
        let mut state = asked("r1184002-garbled", false);
        if let Some(pending) = state.pending.as_mut() {
            pending.prompt = "∫₀¹ 2x dx = ?".to_string();
        }
        let decision = decide(
            &state,
            AI,
            &mut AiOptions::with_budget(create_rng("r1184002-garbled", 0), AI_BUDGET),
        )
        .unwrap_or_else(|| panic!("a decision"));
        assert_eq!(decision.reason, DecisionReason::Prompt);
        assert!(legal_actions(&state, AI).contains(&decision.action));
    }

    #[test]
    fn r1184002_r1563005_its_turn_answers_until_a_miss_or_the_cap_and_plays_on_inside_play_ai_turn_s_ceiling()
    {
        let state = asked("r1184002-turn", false);
        let mut ai = AiOptions::with_budget(create_rng("r1184002-turn", 0), AI_BUDGET);
        let turn = play_ai_turn(&state, AI, &mut ai);
        let answers = turn
            .decisions
            .iter()
            .take_while(|decision| matches!(decision.action, ActionBody::Answer { .. }))
            .count();
        assert!((1..=KY_COMPETITION_MAX as usize).contains(&answers));
        assert!(
            turn.decisions
                .iter()
                .take(answers)
                .all(|decision| decision.stats.nodes == 0)
        );
        // Every answer but a last miss was right: the Kenny's X is the streak.
        let kenny = kenny_x(&turn.state);
        assert!(kenny.is_empty() || kenny == vec![answers as i32] || kenny == vec![answers as i32 - 1]);
        assert!(
            turn.state
                .pending
                .as_ref()
                .is_none_or(|pending| pending.kind != PromptKind::Answer)
        );
        assert_ne!(turn.state.active, AI);
    }
}

// #2929: the search expects the AI's own sequence to be answered as `decide` answers it, right until the
// streak its accuracy earns on average and then Stop, so a cast is worth that Kenny and not the 1/1 of
// the best immediate answer, which is to stop at once.
mod r1563005_the_search_expects_the_streak_the_ai_will_answer_to {
    use super::*;

    #[test]
    fn r1563005_a_simulated_cast_summons_the_kenny_of_the_expected_streak_on_either_face() {
        for radiant in [false, true] {
            let public = redact(&holding(&format!("r2929-sim-{radiant}"), radiant), AI);
            let counter = create_node_counter(AI_BUDGET.nodes, None);
            let next = simulate(&public, AI, &cast(&public), &counter, Prices::NONE)
                .and_then(Result::ok)
                .unwrap_or_else(|| panic!("radiant {radiant}: the cast simulates"));
            assert!(
                next.pending.is_none(),
                "radiant {radiant}: the sequence ends inside the cast's step"
            );
            let expected = expected_streak(radiant);
            assert_eq!(kenny_x(&next), vec![expected], "radiant {radiant}");
            // One node for the cast, one for each right answer and one for the Stop.
            assert_eq!(counter.used(), expected as usize + 2, "radiant {radiant}");
        }
    }

    #[test]
    fn r1563005_r465_ahead_its_own_question_is_worked_out_from_the_statement_never_the_key() {
        for radiant in [false, true] {
            let state = asked(&format!("r2929-ahead-{radiant}"), radiant);
            let mut twin = state.clone();
            let key = state
                .pending
                .as_ref()
                .and_then(|pending| answer_key_of(&pending.resume.data));
            if let Some(pending) = twin.pending.as_mut() {
                pending.resume.data.insert(
                    ANSWER_KEY.to_string(),
                    json!(if key.as_deref() == Some("A") { "B" } else { "A" }),
                );
            }
            let ahead = |state: &GameState| {
                let counter = create_node_counter(AI_BUDGET.nodes, None);
                let score = own_prompt_score(state, AI, &counter, Prices::NONE);
                (score, counter.used())
            };
            let (score, used) = ahead(&state);
            assert!(score.is_some(), "radiant {radiant}: the sequence is scored");
            // One node for each right answer and one for the Stop.
            assert_eq!(used, expected_streak(radiant) as usize + 1, "radiant {radiant}");
            assert_eq!(
                ahead(&twin),
                ahead(&state),
                "radiant {radiant}: the key changes nothing"
            );
            assert_eq!(ahead(&redact(&state, AI)), ahead(&state), "radiant {radiant}");
        }
    }

    #[test]
    fn r1563005_the_greedy_baselines_simulation_leaves_the_question_open() {
        let public = redact(&holding("r2929-greedy", false), AI);
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        let next = simulate_first_answers(&public, AI, &cast(&public), &counter)
            .and_then(Result::ok)
            .unwrap_or_else(|| panic!("the cast simulates"));
        assert_eq!(
            next.pending
                .as_ref()
                .map(|pending| (pending.player_id, pending.kind)),
            Some((AI, PromptKind::Answer))
        );
        assert_eq!(counter.used(), 1);
    }

    #[test]
    fn r1563005_with_the_radiant_competition_in_hand_and_the_mana_to_cast_it_the_ai_casts_it() {
        let state = holding("r2929-cast", true);
        let decision = decide(&state, AI, &mut AiOptions::new(create_rng("r2929-cast", 0)))
            .unwrap_or_else(|| panic!("a decision"));
        assert_eq!(decision.action, cast(&state));
    }
}
