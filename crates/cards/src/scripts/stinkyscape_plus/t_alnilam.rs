//! SPEC §8.12 S+ #T-alnilam Alnilam (4) Unit, Terminus, Mythic, a Terminus token of the held
//! Stinkyscape+ set (R1420, R3535000; R3569003).
//!   Base:    10/10, Indestructible, Untributable, Immune to Spells, Brittle 3; "Death: Lose the game."
//!   Radiant: 20/20, Indestructible, Untributable, Immune to Spells, Brittle 5; "Death: Lose the game."
//!
//! The stats and keywords are catalog data; the two faces share one script. Its Death hook makes its
//! controller lose the game (R850, the Last Word S #4.4 way: `winGame` for the other seat, which a hero
//! at 0 still loses to first). Indestructible turns every destroy aside (R46) and Untributable stops a
//! Sacrifice (R1220), so only a Death that is no destroy (a health taken away, R69) reaches it; an
//! exile or a Transform takes it out of the game with no Death and loses nothing. A Brittle count that
//! runs out crumbles it, a destroy Indestructible ignores, the count staying at 0 (B3.3 rule 3), so the
//! spent count is itself what ends the game: at the controller's next Start of turn, the spent count
//! loses it (R3569003).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-alnilam";

/// "Lose the game": the other seat wins at the state check closing the list (R850).
fn lose_the_game() -> Vec<Effect> {
    vec![win_game(json_as(json!({ "player": "enemy" })))]
}

fn alnilam() -> Script {
    Script {
        death: Some(hook(|_ctx| lose_the_game())),
        start_of_turn: Some(hook(|ctx| {
            if ctx.self_.as_ref().is_some_and(brittle_spent) {
                lose_the_game()
            } else {
                vec![]
            }
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: alnilam(),
        radiant: alnilam(),
    }
}

// S+ #T-alnilam — SPEC §8.12 (R3569003; R46, R69, R850, R1220).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::resolve::{HookOptions, make_context};
    use jackioh_engine::testkit::*;

    use crate::{filler, next_turn_of, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    fn game(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": "alnilam",
            "p1": {
                "field": [{ "def": ID, "radiant": radiant }],
                "library": filler(12),
                "health": 100,
            },
            "p2": { "library": filler(12), "health": 100 },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    /// Runs `effects` as p1's and lets the state check settle what they did.
    fn settled(s: &mut Scenario, effects: Vec<Effect>) {
        let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            {
                let mut ctx = make_context(
                    &mut sink,
                    None,
                    HookOptions {
                        controller: Some(P1),
                        ..HookOptions::default()
                    },
                );
                apply_effects(&effects, &mut ctx);
            }
            settle(&mut sink, SettleOptions::default());
        }
        s.state_mut().rng_cursor = rng.cursor();
    }

    mod base {
        use super::*;

        #[test]
        fn r3569003_it_is_10_10_indestructible_untributable_immune_and_brittle_3() {
            crate::register_all();
            let def = def_of(None, ID);
            assert_eq!((def.base.attack, def.base.health), (Some(10), Some(10)));
            for keyword in [
                KeywordKind::Indestructible,
                KeywordKind::Untributable,
                KeywordKind::ImmuneToSpells,
            ] {
                assert!(
                    def.base.keywords.iter().any(|held| held.kind() == keyword),
                    "{keyword:?}"
                );
            }
            assert!(
                def.base
                    .keywords
                    .iter()
                    .any(|held| matches!(held, Keyword::Brittle { n: 3 }))
            );
        }

        #[test]
        fn r3569003_its_death_loses_its_controller_the_game() {
            let mut s = game(false);
            let id = s.unit_at(P1, 1).id;
            assert!(s.state().result.is_none());
            // A destroy is turned aside by Indestructible (R46)...
            settled(
                &mut s,
                vec![destroy(json_as(
                    json!({ "target": { "of": "instance", "instanceId": id } }),
                ))],
            );
            assert!(s.state().result.is_none(), "it stands");
            // ...but a Unit with no health left dies past it (R69), and its Death loses the game.
            settled(
                &mut s,
                vec![buff(json_as(json!({
                    "target": { "of": "instance", "instanceId": id },
                    "health": -10,
                })))],
            );
            let result = s.state().result.expect("the game is over");
            assert_eq!(result.winner, Winner::P2);
            assert_eq!(result.reason, GameOverReason::WonByEffect);
        }

        #[test]
        fn r3569003_a_spent_brittle_count_loses_at_its_controllers_turn_start() {
            let mut s = game(false);
            // Brittle 3 ticks once per own turn after the turn it was made in: spent on the third.
            for _ in 0..2 {
                next_turn_of(&mut s, P1);
                assert!(
                    s.state().result.is_none(),
                    "turn {}: the count is not yet spent",
                    s.state().turn
                );
            }
            next_turn_of(&mut s, P1);
            let result = s.state().result.expect("the spent count lost the game");
            assert_eq!(result.winner, Winner::P2);
            assert_eq!(result.reason, GameOverReason::WonByEffect);
        }

        #[test]
        fn r3569003_an_exile_loses_nothing() {
            let mut s = game(false);
            settled(&mut s, vec![exile_board(json_as(json!({})))]);
            assert!(s.state().result.is_none());
            assert!(s.unit(P1, 1).is_none());
            next_turn_of(&mut s, P1);
            assert!(s.state().result.is_none());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3569003_radiant_alnilam_is_20_20_brittle_5() {
            crate::register_all();
            let def = def_of(None, ID);
            assert_eq!((def.radiant.attack, def.radiant.health), (Some(20), Some(20)));
            assert!(
                def.radiant
                    .keywords
                    .iter()
                    .any(|held| matches!(held, Keyword::Brittle { n: 5 }))
            );
            let mut s = game(true);
            let id = s.unit_at(P1, 1).id;
            let stats = s.stats(id.as_str());
            assert_eq!((stats.attack, stats.health), (20, 20));
            // The count is spent on the fifth of its controller's turns, not the third.
            for _ in 0..4 {
                next_turn_of(&mut s, P1);
                assert!(s.state().result.is_none(), "turn {}", s.state().turn);
            }
            next_turn_of(&mut s, P1);
            assert_eq!(s.state().result.map(|result| result.winner), Some(Winner::P2));
        }

        #[test]
        fn r3569003_radiant_alnilam_death_also_loses() {
            let mut s = game(true);
            let id = s.unit_at(P1, 1).id;
            settled(
                &mut s,
                vec![buff(json_as(json!({
                    "target": { "of": "instance", "instanceId": id },
                    "health": -20,
                })))],
            );
            assert_eq!(s.state().result.map(|result| result.winner), Some(Winner::P2));
        }
    }
}
