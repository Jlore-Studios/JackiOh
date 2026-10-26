//! Issue #1843's endings and the AI (R1843003, R1843004): the Radiant fleer's choice is a `mode`
//! prompt the AI answers like any other — `legalActions` lists both answers, the search scores each
//! one through `reduce` — and a finished game is scored as it ended for the AI's seat, so the
//! opponent's flight is a draw for it, not a loss.
//!
//! The fixture card is test-only: Stinkyscape #148.1 Edge of the Universe is built later, behind the
//! set's gate (R1420), and its own tests cover the same prompt again.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, clone, is_legal};

/// The fixture whose continuation the fleer's prompt resumes into: a token Spell no pool deals.
const ESCAPE: &str = "ai-test-escape";
/// The prompt's options and its step.
const DRAW: &str = "draw";
const LOSS: &str = "loss";
const FLEE_STEP: &str = "flee";

fn escape_def() -> CardDef {
    json_as(json!({
        "id": ESCAPE,
        "index": "ai-test-1843",
        "name": "Escape Fixture (AI test)",
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Token",
        "token": true,
        "cost": 0,
        "base": { "keywords": [], "text": "Escape the universe." },
        "radiant": { "keywords": [], "text": "Escape the universe; choose what it leaves your opponent." },
    }))
}

/// The answered step flees, leaving the opponent what was chosen.
fn continuations() -> Script {
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert(
        FLEE_STEP,
        hook(|ctx| {
            let loss = effects::chosen_options(ctx).first().map(String::as_str) == Some(LOSS);
            vec![effects::flee(effects::FleeArgs {
                player: None,
                opponent: Some(if loss {
                    FleeOpponent::Loss
                } else {
                    FleeOpponent::Draw
                }),
            })]
        }),
    );
    Script {
        resume,
        ..Script::default()
    }
}

/// The real catalog and scripts plus the fixture, for this thread (the testkit's override).
fn install() {
    jackioh_cards::register_all();
    let mut catalog = registered_catalog().clone();
    catalog.insert(ESCAPE.to_string(), escape_def());
    register_catalog_as(catalog, catalog_version());
    let mut scripts = registered_scripts().clone();
    let script = continuations();
    scripts.insert(
        ESCAPE.to_string(),
        CardScripts {
            base: script.clone(),
            radiant: script,
        },
    );
    register_scripts(scripts);
}

/// A board with libraries on both sides and the fleer's question open for `fleer`, as the card's
/// Radiant face asks it.
fn asked(seed: &str, fleer: PlayerId) -> GameState {
    install();
    let mut state = scenario(json!({
        "seed": seed,
        "p1": { "library": ["core-053", "core-030"] },
        "p2": { "library": ["core-053", "core-030"], "hand": ["core-005"] },
    }))
    .state()
    .clone();
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = create_rng(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(fleer),
                ..Default::default()
            },
        );
        ctx.def_id = Some(ESCAPE.to_string());
        let ask = effects::choose_mode(json_as(json!({
            "options": [DRAW, LOSS],
            "step": FLEE_STEP,
            "prompt": "Leave your opponent",
        })));
        (ask.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    state
}

mod r1843003_the_ai_answers_the_fleers_choice {
    use super::*;

    #[test]
    fn r1843003_the_ai_answers_the_radiant_fleers_prompt_and_wins() {
        let state = asked("r1843003-ai", AI);
        assert_eq!(
            state
                .pending
                .as_ref()
                .map(|pending| (pending.player_id, pending.kind)),
            Some((AI, PromptKind::Mode))
        );
        let decision = decide(
            &clone(&state),
            AI,
            &mut AiOptions {
                rng: create_rng("r1843003-ai", 0),
                budget: AI_BUDGET,
                should_stop: None,
            },
        )
        .expect("a decision");
        // Both answers end the game in the AI's favour, so its lethal finder may take the answer before
        // the prompt search does; either way it answers the prompt.
        assert!(
            matches!(decision.reason, DecisionReason::Prompt | DecisionReason::Lethal),
            "{:?}",
            decision.reason
        );
        assert_eq!(decision.stats.sim_errors, 0);
        assert!(matches!(decision.action, ActionBody::Answer { .. }));
        assert!(is_legal(&state, AI, &decision.action));
        let next = reduce(&state, &Action::new(decision.action, AI, "ai-answer".to_string()));
        assert!(next.error.is_none(), "{:?}", next.error);
        let result = next.state.result.expect("the answer ends the game");
        assert_eq!(result.winner, Winner::from(AI));
        assert!(matches!(
            result.reason,
            GameOverReason::Fled | GameOverReason::FledDecisive
        ));
    }
}

mod r1843004_a_flight_is_scored_as_it_ended_for_the_seat {
    use super::*;

    #[test]
    fn r1843004_the_opponents_flight_is_a_draw_for_the_ai_and_its_own_a_win() {
        let state = asked("r1843004-ai", HUMAN);
        // The human answers: a draw for the AI.
        let draw = reduce(
            &state,
            &Action::new(
                ActionBody::Answer {
                    choice_id: state
                        .pending
                        .as_ref()
                        .map(|pending| pending.id.clone())
                        .unwrap_or_default(),
                    selection: vec![Selection::Mode {
                        option: DRAW.to_string(),
                    }],
                },
                HUMAN,
                "human-answer".to_string(),
            ),
        )
        .state;
        assert_eq!(
            draw.result.map(|result| (result.winner, result.reason)),
            Some((Winner::from(HUMAN), GameOverReason::Fled))
        );
        let drawn = evaluate(&draw, AI, NextSwing::Enemy, &AI_EVAL);
        assert_eq!(drawn, AI_EVAL.drawn, "the opponent's flight is the AI's draw");
        let won = evaluate(&draw, HUMAN, NextSwing::Enemy, &AI_EVAL);
        assert!(won > drawn, "the fleer's own flight is a win for it");

        // A decisive flight is the AI's loss.
        let loss = reduce(
            &state,
            &Action::new(
                ActionBody::Answer {
                    choice_id: state
                        .pending
                        .as_ref()
                        .map(|pending| pending.id.clone())
                        .unwrap_or_default(),
                    selection: vec![Selection::Mode {
                        option: LOSS.to_string(),
                    }],
                },
                HUMAN,
                "human-answer".to_string(),
            ),
        )
        .state;
        assert_eq!(
            loss.result.map(|result| result.reason),
            Some(GameOverReason::FledDecisive)
        );
        assert!(evaluate(&loss, AI, NextSwing::Enemy, &AI_EVAL) < drawn);
    }
}
