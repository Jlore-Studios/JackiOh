//! Port of `packages/engine/test/generation-replay.test.ts`.
//!
//! Whole games on decks of the generation fixtures (docs/classic-sets.md B5 E19, E23–E25), played by
//! the random policy of §10.7 and folded back from their logs (§9.2, §9.3, `replay.ts`): every
//! placement prompt, fusion, digest id, transform and recruit a game reaches replays to the same
//! state, and every state a prompt paused on survives a JSON round trip.

use jackioh_engine::testkit::*;

use crate::rules::fixtures::generation::{
    big_body, body, charger, crawler, deck_fusion, dusting, fuse_a, fuse_b, fuser, juhan, lab, mutate, outbreak,
    pile_on, plague_book, quiet_trap, register_generation, scatter, slime, slop, toxins,
};
use crate::rules::fixtures::harness::setup_catalog;

fn deck() -> Vec<String> {
    [
        plague_book.clone(),
        slime.clone(),
        crawler.clone(),
        toxins.clone(),
        outbreak.clone(),
        dusting.clone(),
        scatter.clone(),
        charger.clone(),
        quiet_trap.clone(),
        body.clone(),
        big_body.clone(),
        fuse_a.clone(),
        fuse_b.clone(),
        lab.clone(),
        slop.clone(),
        deck_fusion.clone(),
        mutate.clone(),
        fuser.clone(),
        juhan.clone(),
        pile_on.clone(),
    ]
    .into_iter()
    .map(|entry| entry.id)
    .collect()
}

/// What a game reached, so the run can assert it exercised the systems it claims to replay.
#[derive(Clone, Copy, Debug, Default)]
struct Reached {
    prompts: usize,
    placements: usize,
    fusions: usize,
    transforms: usize,
    summons: usize,
}

struct Played {
    state: GameState,
    log: Vec<Action>,
    reached: Reached,
}

/// `JSON.parse(JSON.stringify(state))`.
fn round_trip(state: &GameState) -> GameState {
    serde_json::from_value(serde_json::to_value(state).expect("a state serialises")).expect("a state parses")
}

fn play(seed: &str) -> Played {
    setup_catalog();
    register_generation();
    let deck = deck();
    let created = create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: (deck.clone(), deck),
        ..Default::default()
    });
    let mut state = begin_game(&created).state;
    let mut policy = Rng::new(&format!("policy-{seed}"), 0);
    let mut log: Vec<Action> = Vec::new();
    let mut reached = Reached::default();

    let mut step = 0;
    while state.result.is_none() {
        assert!(step <= 4000, "game {seed} did not finish");
        step += 1;
        if state.pending.is_some() {
            reached.prompts += 1;
            // §9.3: a paused state is plain data.
            assert_eq!(round_trip(&state), state);
        }
        let player = seat_to_act(&state).expect("a seat to act");
        let actions: Vec<ActionBody> = legal_actions(&state, player)
            .into_iter()
            .filter(|action| {
                !matches!(action, ActionBody::Concede | ActionBody::OfferDraw | ActionBody::AnswerDraw { .. })
            })
            .collect();
        assert!(!actions.is_empty(), "no legal action for {player} in game {seed}");
        let others: Vec<&ActionBody> = actions.iter().filter(|action| !matches!(action, ActionBody::EndTurn)).collect();
        let end_turn = actions.iter().find(|action| matches!(action, ActionBody::EndTurn));
        let chosen: ActionBody =
            if others.is_empty() || (end_turn.is_some() && policy.chance(AI_END_TURN_PROBABILITY)) {
                match end_turn {
                    Some(end) => end.clone(),
                    None => others[policy.int(others.len() as i32) as usize].clone(),
                }
            } else {
                others[policy.int(others.len() as i32) as usize].clone()
            };
        let action = Action::new(chosen, player, format!("g{}", log.len()));
        let result = reduce(&state, &action);
        if let Some(error) = &result.error {
            panic!("{} rejected in {seed}: {error}", action.action_type());
        }
        for event in &result.events {
            match event {
                GameEvent::CounterChanged { placed: Some(_), .. } => reached.placements += 1,
                GameEvent::Fused { .. } => reached.fusions += 1,
                GameEvent::Transformed { .. } => reached.transforms += 1,
                GameEvent::Summoned { .. } => reached.summons += 1,
                _ => {}
            }
        }
        log.push(action);
        state = result.state;
    }
    Played { state, log, reached }
}

mod e19_e23_e25_whole_games_replay_s9_3 {
    use super::*;

    #[test]
    fn r113_random_games_on_the_generation_decks_fold_back_to_the_same_state_through_every_pause() {
        let mut total = Reached::default();
        for seed in ["gen-a", "gen-b", "gen-c", "gen-d", "gen-e", "gen-f"] {
            let game = play(seed);
            let deck = deck();
            let folded = fold(&FoldArgs {
                seed: seed.to_string(),
                decks: (deck.clone(), deck),
                log: game.log.clone(),
                ..Default::default()
            });
            assert!(folded.errors.is_empty(), "{seed}");
            assert_eq!(hash_state(&folded.state), hash_state(&game.state), "{seed}");
            total.prompts += game.reached.prompts;
            total.placements += game.reached.placements;
            total.fusions += game.reached.fusions;
            total.transforms += game.reached.transforms;
            total.summons += game.reached.summons;
        }
        assert_eq!(deck().len(), DECK_SIZE as usize);
        // The run is seeded end to end, so what it reaches is fixed; it must reach every system.
        assert!(total.prompts > 0);
        assert!(total.placements > 0);
        assert!(total.fusions > 0);
        assert!(total.transforms + total.summons > 0);
    }
}
