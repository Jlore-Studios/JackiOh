//! R465: a multiple-choice problem's key never leaves the engine (Classic+ #42 KY's Test, the `answer`
//! prompt kind, docs/classic-sets.md B5 E18). `view_for` never sends it; this file proves the AI's half:
//! `redact` (R185) strips it from what the AI reads — its own prompt's resume data included — exactly
//! as a human never sees it, so two states that differ only in which option is right redact alike and
//! the AI answers them alike, from what the prompt shows.
//!
//! Port of `packages/ai/test/answer-key.test.ts`.

use indexmap::{IndexMap, IndexSet};
use jackioh_ai::{AiOptions, decide, redact};
use jackioh_engine::testkit::{
    ANSWER_KEY, ActionBody, CardScripts, EngineSink, GameEvent, GameState, HookOptions, PlayerId, PromptKind,
    Script, Selection, answer_key_of, create_rng, effects, hash_state, hook, json, json_as, legal_actions,
    make_context, register_scripts,
};

use super::support::{act, clone, dealt_game, register_cards, scenario};

/// A test-only continuation: the right answer deals 10 to the enemy hero, a wrong one nothing.
const QUIZ_DEF: &str = "ai-test-quiz";

fn quiz() -> Script {
    let mut resume = IndexMap::new();
    resume.insert(
        "answered",
        hook(|ctx| {
            if effects::answered_correctly(ctx) {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": 10 }),
                ))]
            } else {
                vec![]
            }
        }),
    );
    Script {
        resume,
        ..Script::default()
    }
}

/// The key of the prompt open in `state`, as `answerKeyOf(state.pending?.resume.data ?? {})` reads it.
fn key_of(state: &GameState) -> Option<String> {
    let data = state
        .pending
        .as_ref()
        .map(|pending| pending.resume.data.clone())
        .unwrap_or_default();
    answer_key_of(&data)
}

/// p1's main phase with an `answer` prompt open for p1, its key the option at `right`.
fn asked(seed: &str, right: usize) -> GameState {
    register_cards();
    let mut scripts = jackioh_cards::scripts_of();
    scripts.insert(
        QUIZ_DEF.to_string(),
        CardScripts {
            base: quiz(),
            radiant: quiz(),
        },
    );
    register_scripts(scripts);
    let mut state = dealt_game(seed);
    let keep: Vec<String> = state.players.p1.hand.iter().map(|card| card.id.clone()).collect();
    state = act(&state, PlayerId::P1, ActionBody::Mulligan { keep });
    let keep: Vec<String> = state.players.p2.hand.iter().map(|card| card.id.clone()).collect();
    state = act(&state, PlayerId::P2, ActionBody::Mulligan { keep });
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = create_rng(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(PlayerId::P1),
                ..HookOptions::default()
            },
        );
        ctx.def_id = Some(QUIZ_DEF.to_string());
        let effect = effects::choose_answer(json_as(json!({
            "step": "answered",
            "statement": "7 + 5 = ?",
            "options": ["12", "11", "13", "75"],
            "correct": right,
            "shuffle": false,
        })));
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    state
}

mod r465_the_ai_never_reads_a_problems_key {
    use super::*;

    #[test]
    fn r465_redaction_strips_the_key_from_the_ais_own_prompt_and_states_that_differ_only_in_it_redact_alike()
    {
        let first = asked("r465-a", 0);
        let second = asked("r465-a", 3);
        assert_eq!(key_of(&first), Some("A".to_string()));
        assert_eq!(key_of(&second), Some("D".to_string()));
        for seat in [PlayerId::P1, PlayerId::P2] {
            let seen = redact(&first, seat);
            assert!(!serde_json::to_string(&seen).unwrap().contains(ANSWER_KEY));
            assert_eq!(hash_state(&seen), hash_state(&redact(&second, seat)));
        }
        // The chooser still sees the problem: its statement and its four options.
        let labels: Vec<String> = redact(&first, PlayerId::P1)
            .pending
            .map(|pending| pending.options.into_iter().map(|option| option.label).collect())
            .unwrap_or_default();
        assert_eq!(labels, vec!["12", "11", "13", "75"]);
        // The true state is untouched.
        assert_eq!(key_of(&first), Some("A".to_string()));
    }

    #[test]
    fn r465_the_ai_answers_from_what_the_prompt_shows_the_same_answer_whichever_option_is_right() {
        let decisions: Vec<String> = [0, 1, 2, 3]
            .into_iter()
            .map(|right| {
                let state = asked("r465-b", right);
                let decision = decide(
                    &clone(&state),
                    PlayerId::P1,
                    &mut AiOptions::new(create_rng("r465-decide", 0)),
                );
                assert!(decision.is_some());
                let legal: Vec<String> = legal_actions(&state, PlayerId::P1)
                    .iter()
                    .map(|action| serde_json::to_string(action).unwrap())
                    .collect();
                let chosen = serde_json::to_string(&decision.map(|d| d.action)).unwrap();
                assert!(legal.contains(&chosen));
                chosen
            })
            .collect();
        assert_eq!(decisions.into_iter().collect::<IndexSet<_>>().len(), 1);
    }

    #[test]
    fn r465_the_real_kys_test_c_plus_42_at_either_prompt_neither_seats_redaction_holds_the_key() {
        register_cards();
        let mut s = scenario(json!({
            "seed": "r465-kys-test",
            "p1": { "hand": ["classicplus-042", "core-005"] },
            "p2": { "hand": ["core-005"] },
        }));
        s.play("classicplus-042", json!({}));
        for difficulty in ["Easy", "Medium", "Hard"] {
            let offer = clone(s.state());
            let choice_id = offer
                .pending
                .as_ref()
                .map(|pending| pending.id.clone())
                .unwrap_or_default();
            let answered = act(
                &offer,
                PlayerId::P1,
                ActionBody::Answer {
                    choice_id,
                    selection: vec![Selection::Mode {
                        option: difficulty.to_string(),
                    }],
                },
            );
            assert_eq!(
                answered.pending.as_ref().map(|pending| pending.kind),
                Some(PromptKind::Answer)
            );
            assert!(key_of(&answered).is_some());
            for state in [&offer, &answered] {
                for seat in [PlayerId::P1, PlayerId::P2] {
                    assert!(
                        !serde_json::to_string(&redact(state, seat))
                            .unwrap()
                            .contains(ANSWER_KEY)
                    );
                }
            }
        }
    }
}
