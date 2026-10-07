// M1's replay gate, over games this suite generates from its own script-less fixture catalog.
//
// BUILD M5-T3's replay — the log a BROWSER recorded, folded with the real catalog and the 110 card
// scripts — cannot live here: it needs `@jackioh/cards`, and the dependency runs the other way
// (cards -> engine). It is `packages/cards/test/hotseat-replay.test.ts`.
//
// Port of `packages/engine/test/replay.test.ts`.

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{play_random_game, setup_catalog};

fn seeds() -> Vec<String> {
    (0..100).map(|i| format!("smoke-{}", i + 1)).collect()
}

/// M1 gate: folding a recorded log reaches exactly the same state.
mod replay_m1_gate {
    use super::*;

    // R389: the 60-turn cap doubles the length of a game that reaches it, which most random ones do.
    #[test]
    fn folds_100_recorded_games_to_the_same_state_hash() {
        for seed in seeds() {
            let live = play_random_game(&seed, None);
            setup_catalog();
            let replayed = fold(&FoldArgs {
                seed: seed.clone(),
                decks: live.decks.clone(),
                log: live.log.clone(),
                ..Default::default()
            });

            assert_eq!(replayed.errors.len(), 0, "{seed}");
            assert_eq!(hash_state(&replayed.state), hash_state(&live.state), "{seed}");
            assert_eq!(replayed.state.result, live.state.result, "{seed}");
        }
    }

    #[test]
    fn a_different_seed_gives_a_different_hash() {
        let a = play_random_game("hash-a", None);
        let b = play_random_game("hash-b", None);
        assert_ne!(hash_state(&a.state), hash_state(&b.state));
    }
}
