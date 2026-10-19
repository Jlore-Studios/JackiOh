//! R1184002: the AI's answer to Meditative+ #20 KY's Competition's question sequence (R1184000). The
//! AI works each question out from the statement its redacted prompt shows, at once and with no
//! search, and never stops: thirty questions cost it what one redaction does, inside its decision
//! budget, and it never reads the key (R465).

use jackioh_ai::{AI_BUDGET, AiOptions, DecisionReason, decide, play_ai_turn, redact};
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
    fn r1184002_each_answer_is_the_right_one_given_at_once_with_no_search_until_the_cap_ends_it() {
        for radiant in [false, true] {
            let mut state = asked(&format!("r1184002-run-{radiant}"), radiant);
            let mut ai = AiOptions::with_budget(create_rng("r1184002-run", 0), AI_BUDGET);
            let mut answered = 0;
            while state.pending.is_some() {
                let decision = decide(&state, AI, &mut ai).unwrap_or_else(|| panic!("a decision"));
                assert_eq!(decision.reason, DecisionReason::Prompt);
                assert_eq!(decision.stats.nodes, 0, "no search");
                let ActionBody::Answer { selection, .. } = &decision.action else {
                    panic!("an answer, not {:?}", decision.action);
                };
                assert_eq!(selection, &vec![right_selection(&state)]);
                assert!(legal_actions(&state, AI).contains(&decision.action));
                state = act(&state, AI, &decision.action);
                answered += 1;
                assert!(answered <= KY_COMPETITION_MAX);
            }
            assert_eq!(answered, KY_COMPETITION_MAX);
            assert_eq!(kenny_x(&state), vec![KY_COMPETITION_MAX]);
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
    fn r1184002_its_turn_answers_every_question_and_plays_on_inside_play_ai_turn_s_ceiling() {
        let state = asked("r1184002-turn", false);
        let mut ai = AiOptions::with_budget(create_rng("r1184002-turn", 0), AI_BUDGET);
        let turn = play_ai_turn(&state, AI, &mut ai);
        let answers = turn
            .decisions
            .iter()
            .take_while(|decision| matches!(decision.action, ActionBody::Answer { .. }))
            .count();
        assert_eq!(answers, KY_COMPETITION_MAX as usize);
        assert!(
            turn.decisions
                .iter()
                .take(answers)
                .all(|decision| decision.stats.nodes == 0)
        );
        assert_eq!(kenny_x(&turn.state), vec![KY_COMPETITION_MAX]);
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
