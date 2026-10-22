//! R1184002: the AI's answer to Meditative+ #20 KY's Competition's question sequence (R1184000). The
//! AI works each question out from the statement its redacted prompt shows, at once and with no
//! search, and never stops: thirty questions cost it what one redaction does, inside its decision
//! budget, and it never reads the key (R465). R1563005: it answers right 85% of the time on the base
//! face and 93% on the Radiant one, a miss one of the wrong answers, drawn from its own stream.

use jackioh_ai::{
    AI_BUDGET, AI_KY_COMPETITION_ACCURACY, AI_KY_COMPETITION_RADIANT_ACCURACY, AiOptions, DecisionReason,
    decide, play_ai_turn, redact,
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
        // The turn went on after the sequence and ended: the AI's seat is no longer to act.
        assert_ne!(turn.state.active, AI);
    }
}
