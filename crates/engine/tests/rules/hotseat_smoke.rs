//! Port of `packages/engine/test/hotseat.smoke.test.ts`.

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::play_random_game;

/// M1 gate: 100 random-policy games between script-less decks, start to finish.
mod hotseat_smoke_m1_gate {
    use super::*;

    // R389: the 60-turn cap doubles the length of a game that reaches it, which most random ones do.
    #[test]
    fn finishes_100_seeded_games_by_hero_death_or_the_turn_cap_without_throwing() {
        let seeds: Vec<String> = (1..=100).map(|i| format!("smoke-{i}")).collect();
        let mut reasons: IndexMap<String, usize> = IndexMap::new();

        for seed in &seeds {
            let game = play_random_game(seed, None);
            let state = &game.state;
            let result = state.result.as_ref();
            assert!(result.is_some(), "{seed}");
            let reason = result.map(|r| r.reason.as_str().to_string()).unwrap_or_default();
            assert!(
                ["hero-death", "both-heroes-dead", "turn-cap"].contains(&reason.as_str()),
                "{seed}: {reason}"
            );
            *reasons.entry(reason).or_insert(0) += 1;

            assert!(state.turn <= TURN_CAP_PLAYER_TURNS);
            assert_eq!(state.phase, Phase::Over);
            for player in PLAYER_IDS {
                let side = &state.players[player];
                assert!(side.hand.len() <= 10);
                // Nothing dead is left standing, and no pile is a stray empty array.
                for pile in &side.units {
                    assert!(pile.as_ref().is_none_or(|cards| !cards.is_empty()));
                }
            }
        }

        assert_eq!(reasons.values().sum::<usize>(), seeds.len());
    }
}
