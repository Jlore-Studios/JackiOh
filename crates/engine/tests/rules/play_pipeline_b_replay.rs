//! Port of `packages/engine/test/play-pipeline-b-replay.test.ts`.
//!
//! Replay with play pipeline B's cards (SPEC §9.2, §9.3; docs/classic-sets.md B5 E11, E12, E15, B4.5):
//! whole games on decks of this workstream's fixtures — graveyard permissions, random casts, casts from
//! the graveyard, price rules, Forever&'s return, a Tribute card — played by §10.7's random policy,
//! then folded from `(seed, decks, log)` by `replay.fold` and compared state for state. The run
//! asserts that it reached what it exists to replay: plays from the graveyard, random casts, prompts
//! answered mid-cast, and a state at rest that never carries a cast's mode.

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::setup_catalog;
use crate::rules::fixtures::play_pipeline_b::{
    RANDOM_POOL, cast_trap, discover_spell, forever, grave_spell, grave_unit, jogg_box, mode_spell, monkey,
    named_caster, plantation, register_pipeline_b, second_wind, solarius, target_spell, tax, their_choice,
    titan, toe_cracker, trickster, tyrant, x_target,
};

fn deck() -> Vec<String> {
    [
        second_wind(),
        plantation(),
        jogg_box(),
        solarius(),
        forever(),
        trickster(),
        toe_cracker(),
        monkey(),
        tax(),
        grave_spell(),
        x_target(),
        target_spell(),
        mode_spell(),
        discover_spell(),
        their_choice(),
        titan(),
        named_caster(),
        tyrant(),
        cast_trap(),
        grave_unit(),
    ]
    .into_iter()
    .map(|card| card.id)
    .collect()
}

fn seeds() -> Vec<String> {
    (0..8).map(|at| format!("pb-replay-{}", at + 1)).collect()
}

// TS: "Each seed is played once and folded twice; long random-policy games on these decks take
// seconds", hence its TEST_TIMEOUT_MS of 120 000. Rust tests take no timeout.
const STEP_CAP: usize = 4000;

struct Played {
    state: GameState,
    log: Vec<Action>,
    events: Vec<GameEvent>,
    hashes: Vec<String>,
}

fn create(seed: &str) -> GameState {
    create_game(&CreateGameArgs {
        seed: seed.to_string(),
        decks: (deck(), deck()),
        ..Default::default()
    })
}

/// §10.7's random policy over `legalActions`, never conceding or offering a draw.
fn play(seed: &str) -> Played {
    setup_catalog();
    register_pipeline_b();
    let mut state = begin_game(&create(seed)).state;
    let mut policy = Rng::new(&format!("policy-{seed}"), 0);
    let mut log: Vec<Action> = Vec::new();
    let mut events: Vec<GameEvent> = Vec::new();
    let mut hashes: Vec<String> = Vec::new();
    let mut step = 0;
    while state.result.is_none() {
        if step > STEP_CAP {
            panic!("game {seed} did not finish");
        }
        let player = seat_to_act(&state).expect("a seat to act");
        let actions: Vec<ActionBody> = legal_actions(&state, player)
            .into_iter()
            .filter(|action| {
                !matches!(
                    action.action_type(),
                    ActionType::Concede | ActionType::OfferDraw | ActionType::AnswerDraw
                )
            })
            .collect();
        let others: Vec<ActionBody> = actions
            .iter()
            .filter(|action| action.action_type() != ActionType::EndTurn)
            .cloned()
            .collect();
        let end_turn = actions
            .iter()
            .find(|action| action.action_type() == ActionType::EndTurn)
            .cloned();
        // TS's short-circuit order: `chance` draws only when there are others and an end turn.
        let chosen = if others.is_empty() || (end_turn.is_some() && policy.chance(AI_END_TURN_PROBABILITY)) {
            match end_turn {
                Some(end) => end,
                None => others[policy.int(others.len() as i32) as usize].clone(),
            }
        } else {
            others[policy.int(others.len() as i32) as usize].clone()
        };
        let action = Action::new(chosen, player, format!("r{}", log.len()));
        let result = reduce(&state, &action);
        if let Some(error) = &result.error {
            panic!("{} rejected in {seed}: {error}", action.action_type());
        }
        log.push(action);
        events.extend(result.events);
        state = result.state;
        hashes.push(hash_state(&state));
        // R452: a cast's mode is in force only while its steps run; a state at rest never carries it.
        if state.casts_resolving.is_some() {
            panic!("a cast mode leaked into a state at rest in {seed}");
        }
        step += 1;
    }
    Played {
        state,
        log,
        events,
        hashes,
    }
}

fn of_type(events: &[GameEvent], kind: &str) -> Vec<Value> {
    events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .filter(|event| event["type"] == kind)
        .collect()
}

mod play_pipeline_b_folds_exactly_s9_2_s9_3 {
    use super::*;

    /// "R452 R453 R454 R455 games on graveyard plays, random casts and price rules replay to the same
    /// state, step by step"
    #[test]
    fn r452_r453_r454_r455_games_on_graveyard_plays_random_casts_and_price_rules_replay_to_the_same_state_step_by_step()
     {
        let mut graveyard_plays = 0;
        let mut random_casts = 0;
        let mut prompts = 0;
        let mut returns = 0;
        let mut x_asked = 0;
        for seed in seeds() {
            let live = play(&seed);
            let folded = fold(&FoldArgs {
                seed: seed.clone(),
                decks: (deck(), deck()),
                log: live.log.clone(),
                ..Default::default()
            });
            assert_eq!(folded.errors.len(), 0, "{seed}: the fold refused actions");
            assert_eq!(hash_state(&folded.state), hash_state(&live.state));

            // Step by step, through JSON at every step: each action from the round-tripped state lands on
            // the state the live game had.
            let mut state = begin_game(&create(&seed)).state;
            for (at, action) in live.log.iter().enumerate() {
                let round: GameState =
                    serde_json::from_value(serde_json::to_value(&state).expect("the state serialises"))
                        .expect("the state parses back");
                state = reduce(&round, action).state;
                assert_eq!(hash_state(&state), live.hashes[at], "{seed} step {at}");
            }

            graveyard_plays += of_type(&live.events, "cardPlayed")
                .iter()
                .filter(|event| event["from"] == "graveyard")
                .count();
            random_casts += of_type(&live.events, "cardPlayed")
                .iter()
                .filter(|event| {
                    event["costPaid"] == 0
                        && event["defId"]
                            .as_str()
                            .is_some_and(|def_id| RANDOM_POOL.contains(&def_id))
                })
                .count();
            prompts += of_type(&live.events, "promptOpened").len();
            x_asked += of_type(&live.events, "promptOpened")
                .iter()
                .filter(|event| event["kind"] == "number")
                .count();
            returns += of_type(&live.events, "addedToHand")
                .iter()
                .filter(|event| event["defId"] != jogg_box().id.as_str())
                .count();
        }
        // The run reached what it exists to replay.
        assert!(graveyard_plays > 0);
        assert!(random_casts > 0);
        assert!(prompts > 0);
        assert!(returns > 0);
        assert!(x_asked > 0);
    }
}
