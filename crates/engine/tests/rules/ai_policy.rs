//! The random legal-action policy of SPEC §10.7 (BUILD M3-T7, R44): determinism from the seed, a
//! choice that is always one `legalActions` offered, the end-of-turn rule, uniform prompt answers,
//! and a playout that always terminates. The endless fixture card lives here rather than in a
//! shared fixture, as statecheck.test.ts does (CLAUDE.md, BUILD §0).
//!
//! Port of `packages/engine/test/aiPolicy.test.ts`.

use jackioh_engine::effects::choose_mode;
use jackioh_engine::subsystems::ai_policy::{
    AI_SKIPPED_ACTIONS, PlayoutResult, PlayoutStop, PolicyOptions, choose_action, play_out_turn, policy_actions,
};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::combat::{big_body, plain};
use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, slot};

/// A fixture with no Core counterpart: answering its prompt re-opens the same prompt, so the policy
/// is offered an answer and never an `endTurn`. It exists to prove the playout guard, since nothing
/// else in the engine can loop forever.
fn endless_question() -> CardDef {
    json_as(json!({
        "id": "ai-endless-question",
        "index": "902",
        "name": "Endless Question (fixture)",
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": "answering it asks again" },
        "radiant": { "keywords": [], "text": "answering it asks again" },
    }))
}

fn ask() -> Effect {
    choose_mode(json_as(json!({ "options": ["again", "and again"], "step": "ask" })))
}

fn ask_again() -> Script {
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert("ask", hook(|_ctx| vec![ask()]));
    Script {
        cry: Some(hook(|_ctx| vec![ask()])),
        resume,
        ..Script::default()
    }
}

fn endless_scripts() -> CardScripts {
    CardScripts {
        base: ask_again(),
        radiant: ask_again(),
    }
}

/// p1's main phase on turn 4, so nothing placed with `put` is summoning sick (§4.1).
fn board(seed: &str) -> GameState {
    let mut state = new_game(seed);
    let mut catalog = registered_catalog().clone();
    catalog.insert(endless_question().id, endless_question());
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    scripts.insert(endless_question().id, endless_scripts());
    register_scripts(scripts);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state.players.p1.mana.current = 4;
    state.players.p1.mana.max = 4;
    state
}

/// A board with something to do: cards to play, a unit to attack with and a unit to attack.
fn busy_board(seed: &str) -> GameState {
    let mut state = board(seed);
    in_hand(&mut state, "fx-1", P1, 2);
    in_hand(&mut state, "fx-2", P1, 2);
    put(&mut state, &big_body().id, slot(P1, Row::Units, 1));
    put(&mut state, &plain().id, slot(P2, Row::Units, 1));
    state
}

/// A resume nothing can service: answering the prompt then just clears it (§10.6).
fn inert_resume() -> Resume {
    Resume {
        def_id: "ai-no-script".into(),
        hook: "resume".into(),
        step: "none".into(),
        radiant: false,
        instance_id: None,
        data: IndexMap::new(),
    }
}

fn mode_options(options: &[&str]) -> Vec<PromptOption> {
    options
        .iter()
        .map(|option| PromptOption {
            key: format!("mode:{option}"),
            label: option.to_string(),
            selection: Selection::Mode {
                option: option.to_string(),
            },
            cost: None,
            radiant: None,
        })
        .collect()
}

fn target_prompt(player: PlayerId, prompt: &str, options: &[&str]) -> OpenPromptArgs {
    OpenPromptArgs {
        player,
        kind: PromptKind::Target,
        aim: None,
        prompt: prompt.into(),
        options: mode_options(options),
        min: None,
        max: None,
        budget: None,
        owner: None,
        resume: inert_resume(),
    }
}

fn sequence(state: &GameState, seed: &str, steps: usize) -> Vec<Option<ActionBody>> {
    let mut rng = Rng::new(seed, 0);
    (0..steps)
        .map(|_| choose_action(state, P1, &mut rng, PolicyOptions::default()))
        .collect()
}

/// TS `playout(seed)`: the board the sink drove (the sink's own state, moved in place), the events
/// it collected, and the playout's result.
fn playout(seed: &str) -> (GameState, Vec<GameEvent>, PlayoutResult) {
    let mut original = busy_board(seed);
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&original.seed, original.rng_cursor);
    let result = {
        let mut sink = EngineSink::new(&mut original, &mut events, &mut rng);
        play_out_turn(&mut sink, P1, PolicyOptions::default())
    };
    (original, events, result)
}

fn types_of(actions: &[ActionBody]) -> Vec<ActionType> {
    actions.iter().map(|action| action.action_type()).collect()
}

mod r44_the_ai_policy_m3_t7_s10_7 {
    use super::*;

    #[test]
    fn r44_returns_the_same_action_sequence_for_the_same_state_and_seed() {
        let state = busy_board("determinism");

        assert_eq!(
            choose_action(&state, P1, &mut Rng::new("policy", 0), PolicyOptions::default()),
            choose_action(&state, P1, &mut Rng::new("policy", 0), PolicyOptions::default())
        );
        assert_eq!(sequence(&state, "policy", 12), sequence(&state, "policy", 12));
        // A different seed walks a different sequence, so the seed is really what decides.
        assert_ne!(sequence(&state, "other-policy", 12), sequence(&state, "policy", 12));
    }

    #[test]
    fn r84_only_ever_returns_an_action_legal_actions_offered_and_never_a_concede_or_a_draw_offer() {
        let state = busy_board("offered");
        let offered = legal_actions(&state, P1);

        for i in 0..60 {
            let chosen = choose_action(&state, P1, &mut Rng::new(&format!("seed-{i}"), 0), PolicyOptions::default());
            assert!(chosen.is_some());
            let chosen = chosen.unwrap();
            assert!(offered.contains(&chosen));
            assert!(!AI_SKIPPED_ACTIONS.contains(&chosen.action_type()));
        }

        // The filter is doing work: `legalActions` does offer those, and `skip: []` is the literal set.
        assert!(types_of(&offered).contains(&ActionType::Concede));
        assert!(types_of(&offered).contains(&ActionType::OfferDraw));
        assert!(!types_of(&policy_actions(&state, P1, PolicyOptions::default())).contains(&ActionType::Concede));
        assert_eq!(policy_actions(&state, P1, PolicyOptions { skip: Some(vec![]) }), offered);
    }

    #[test]
    fn r44_ends_the_turn_when_nothing_else_is_on_offer_and_takes_no_rng_draw_to_decide_it() {
        let state = board("only-end-turn"); // empty hand, empty board
        let mut rng = Rng::new("unused", 0);

        assert_eq!(policy_actions(&state, P1, PolicyOptions::default()), vec![ActionBody::EndTurn]);
        assert_eq!(
            choose_action(&state, P1, &mut rng, PolicyOptions::default()),
            Some(ActionBody::EndTurn)
        );
        assert_eq!(rng.cursor(), 0);

        // With other actions available it ends the turn only on the AI_END_TURN_PROBABILITY roll.
        let busy = busy_board("sometimes-end-turn");
        let ends = (0..200)
            .map(|i| choose_action(&busy, P1, &mut Rng::new(&format!("roll-{i}"), 0), PolicyOptions::default()))
            .filter(|action| matches!(action, Some(ActionBody::EndTurn)))
            .count();
        assert!(ends > 0);
        assert!((ends as f64) < 200.0 * AI_END_TURN_PROBABILITY * 3.0);
    }

    #[test]
    fn r44_answers_the_open_prompt_of_its_own_player_uniformly_and_nothing_for_the_other_player() {
        let mut state = board("prompt");
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        open_prompt(&mut sink, target_prompt(P1, "pick one", &["a", "b", "c"]));

        let mut picked: IndexSet<String> = IndexSet::new();
        for i in 0..40 {
            let chosen = choose_action(sink.state, P1, &mut Rng::new(&format!("answer-{i}"), 0), PolicyOptions::default());
            assert_eq!(chosen.as_ref().map(|action| action.action_type()), Some(ActionType::Answer));
            let Some(ActionBody::Answer { choice_id, selection }) = chosen else {
                continue;
            };
            assert_eq!(Some(choice_id), sink.state.pending.as_ref().map(|pending| pending.id.clone()));
            if let Some(Selection::Mode { option }) = selection.first() {
                picked.insert(option.clone());
            }
        }
        // Uniform over the options, so all three come up across 40 seeds.
        let mut sorted: Vec<String> = picked.into_iter().collect();
        sorted.sort();
        assert_eq!(sorted, vec!["a", "b", "c"]);

        // The prompt is not p2's, so p2 has nothing to do (§9.3).
        assert_eq!(
            choose_action(sink.state, P2, &mut Rng::new("p2", 0), PolicyOptions::default()),
            None
        );

        // A playout answers it, and the answer the reducer gets is a legal one.
        let result = play_out_turn(&mut sink, P1, PolicyOptions::default());
        assert_eq!(result.error, None);
        assert_eq!(result.actions.first().map(|action| action.action_type()), Some(ActionType::Answer));
        assert!(sink.state.pending.is_none());
    }

    #[test]
    fn r44_play_out_turn_plays_the_turn_out_ends_it_and_repeats_exactly_from_the_seed() {
        let (first_state, first_events, first) = playout("playout");
        let (second_state, _second_events, second) = playout("playout");

        assert_eq!(first.error, None);
        assert_eq!(first.stopped, PlayoutStop::TurnEnded);
        assert!(!first.actions.is_empty());
        assert!(first.actions.iter().all(|action| action.player_id == P1));
        for action in &first.actions {
            assert!(!AI_SKIPPED_ACTIONS.contains(&action.action_type()));
        }

        // The turn really passed, and the sink's own state object is the one that moved (in Rust the
        // sink borrows the board it was built over, so the state handed back is that object).
        assert_eq!(first_state.active, P2);
        let ended: Vec<Value> = events_of_type(&first_events, GameEventType::TurnEnded)
            .iter()
            .map(|event| serde_json::to_value(event).unwrap()["player"].clone())
            .collect();
        assert_eq!(ended, vec![json!("p1")]);

        assert_eq!(second.actions, first.actions);
        assert_eq!(second.stopped, first.stopped);
        assert_eq!(second.error, first.error);
        assert_eq!(
            serde_json::to_string(&second_state).unwrap(),
            serde_json::to_string(&first_state).unwrap()
        );
    }

    #[test]
    fn r44_play_out_turn_stops_when_the_open_prompt_belongs_to_the_other_player() {
        let mut state = busy_board("elsewhere");
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        open_prompt(&mut sink, target_prompt(P2, "not yours", &["x", "y"]));

        let before = serde_json::to_string(&*sink.state).unwrap();
        let result = play_out_turn(&mut sink, P1, PolicyOptions::default());

        assert_eq!(result.stopped, PlayoutStop::PromptElsewhere);
        assert!(result.actions.is_empty());
        assert_eq!(serde_json::to_string(&*sink.state).unwrap(), before);
    }

    #[test]
    fn r44_play_out_turn_stops_when_the_game_is_over_or_the_turn_has_already_passed() {
        fn play_out(mut state: GameState) -> PlayoutResult {
            let mut events: Vec<GameEvent> = Vec::new();
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            play_out_turn(&mut sink, P1, PolicyOptions::default())
        }
        fn assert_stopped(result: PlayoutResult, stopped: PlayoutStop) {
            assert!(result.actions.is_empty());
            assert_eq!(result.stopped, stopped);
            assert_eq!(result.error, None);
        }

        let mut over = busy_board("game-over");
        over.result = Some(GameResult {
            winner: Winner::P2,
            reason: GameOverReason::Concede,
        });
        over.phase = Phase::Over;
        assert_stopped(play_out(over), PlayoutStop::GameOver);

        let mut their_turn = busy_board("their-turn");
        their_turn.active = P2;
        assert_stopped(play_out(their_turn), PlayoutStop::TurnEnded);

        // Outside the main phase `legalActions` offers only a concede, which the policy never takes.
        let mut starting = busy_board("starting");
        starting.phase = Phase::Start;
        assert_stopped(play_out(starting), PlayoutStop::TurnEnded);
    }

    #[test]
    fn r44_play_out_turn_stops_at_the_step_cap_instead_of_looping_forever() {
        let mut state = board("endless");
        in_hand(&mut state, &endless_question().id, P1, 1);
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);

        let result = play_out_turn(&mut sink, P1, PolicyOptions::default());

        // Every step after the first is an answer to a prompt the last answer re-opened, so the policy
        // is never offered an `endTurn`: only the guard stops it.
        assert_eq!(result.actions.first().map(|action| action.action_type()), Some(ActionType::Play));
        assert_eq!(result.stopped, PlayoutStop::StepCap);
        assert_eq!(result.actions.len(), AI_PLAYOUT_STEP_CAP);
        assert_eq!(result.error, None);
        assert_eq!(sink.state.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Mode));
        assert!(sink.state.result.is_none());

        // Nonces are unique, so no action was silently deduped into an earlier one's events.
        let nonces: IndexSet<&str> = result.actions.iter().map(|action| action.nonce.as_str()).collect();
        assert_eq!(nonces.len(), result.actions.len());
    }
}
