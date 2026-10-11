//! M #48 Tranquility (SPEC §8.8 row 48): (2) Spell, Epic.
//!
//! Base:    "End the turn. Your hero is immune to damage until your next turn begins."
//! Radiant: "Your hero is immune to damage until your next turn begins."
//! Engine: the end of the turn is E10's `end_turn` (§6.3, R456) — the rest of the list resolves
//! first, then every end-of-turn step runs; on the opponent's turn a cast has no turn of theirs to
//! end. The immunity is ME-HERO-IMMUNE (MD-C21): a player modifier read as a cap of 0 at §4.4 step
//! 3, expiring at the cleanup that ends the opponent's next turn (R757's Armor Up expiry), so it is
//! gone as your next turn begins.

use jackioh_engine::effects::{end_turn, make_hero_immune};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-048";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| {
                vec![end_turn(json_as(json!({}))), make_hero_immune(json_as(json!({})))]
            })),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| vec![make_hero_immune(json_as(json!({})))])),
            ..Script::default()
        },
    }
}

// M #48 Tranquility — SPEC §8.8 row 48, BUILD M10 row M 48: "Ends your turn after the rest of its
// list, every end-of-turn step running (R456); until your next turn begins every damage instance to
// your hero is 0, combat, effects and fatigue alike (MD-C21) … the immunity is gone at your next
// start of turn; cast on the opponent's turn it ends nothing; radiant the immunity without ending
// the turn".
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::{cast_new, damage, draw, lose_health, set_health};
    use jackioh_engine::prelude::Effect;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TRANQUILITY: &str = "meditative-048";
    const FILLER: &str = "core-005";
    const MENACE: &str = "core-019"; // Midrange Menace 9/9: "End of turn: Heal this to full."
    const JILLIAX: &str = "core-056"; // 3/2 Rush, Taunt, Lifesteal, Divine Shield.
    const ECLIPSE: &str = "core-035"; // Lunar Eclipse: deal 3 damage.

    /// Runs `effects` as `player`'s, then settles: an effect from outside any card (the shape the
    /// engine's own `hero_immune.rs` uses), for a hit or a cast on a turn the scenario can't reach.
    fn apply_as(s: &mut Scenario, player: PlayerId, effects: &[Effect]) -> Vec<GameEvent> {
        let (seed, cursor) = (s.state().seed.clone(), s.state().rng_cursor);
        let mut rng = Rng::new(&seed, cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            {
                let mut ctx = make_context(
                    &mut sink,
                    None,
                    HookOptions {
                        controller: Some(player),
                        ..HookOptions::default()
                    },
                );
                apply_effects(effects, &mut ctx);
            }
            settle(&mut sink, SettleOptions::default());
        }
        s.state_mut().rng_cursor = rng.cursor();
        events
    }

    /// p1 casts the base face with `p2` as the opponent's setup; it is p2's turn afterwards.
    fn cast_against(seed: &str, p2: Value) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [TRANQUILITY, FILLER],
                "library": [FILLER, FILLER, FILLER, FILLER],
                "mana": 4,
            },
            "p2": p2,
        }));
        s.play(TRANQUILITY, json!({}));
        assert_eq!(s.state().active, P2);
        s
    }

    fn cast(seed: &str, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": TRANQUILITY, "radiant": radiant }, FILLER],
                "library": [FILLER, FILLER, FILLER, FILLER],
                // 4 mana: 2 left after the cast, so the turn never auto-ends (§2.5, R82) and only
                // the card's own `end_turn` can end it.
                "mana": 4,
            },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }));
        s.play(TRANQUILITY, json!({}));
        s
    }

    mod m48_tranquility {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r456_it_ends_your_turn_after_the_rest_of_its_list() {
                let s = cast("tranquility-base", false);
                // The turn ended as if its player had pressed End turn: the opponent is active.
                assert_eq!(s.state().active, P2);
            }

            #[test]
            fn r456_every_end_of_turn_step_runs_after_the_rest_of_its_list() {
                let mut s = scenario(json!({
                    "seed": "tranquility-eot",
                    "p1": {
                        "field": [{ "def": MENACE, "lane": 1, "damage": 4 }],
                        "hand": [TRANQUILITY, FILLER],
                        "library": [FILLER, FILLER, FILLER, FILLER],
                        "mana": 4,
                    },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                let menace = s.unit(P1, 1).expect("menace");

                s.play(TRANQUILITY, json!({}));

                // The immunity after `end_turn` in its list still resolved first, then the turn
                // ended with its end-of-turn steps: the Menace healed itself to full.
                assert_eq!(s.state().active, P2);
                assert_eq!(jackioh_engine::damage::hero_damage_cap(s.state(), P1), Some(0));
                s.expect_stats(&menace, json!({ "health": 9, "maxHealth": 9 }));
                assert!(s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::TurnEnded { player, .. } if *player == P1
                )));
            }

            #[test]
            fn r1021_a_combat_hit_on_your_hero_is_0() {
                let mut s = cast_against(
                    "tranquility-combat",
                    json!({
                        "field": [{ "def": "core-008", "lane": 1 }],
                        "hand": [FILLER],
                        "library": [FILLER, FILLER],
                    }),
                );
                let vanilla = s.unit(P2, 1).expect("vanilla");
                s.attack(&vanilla, "hero");
                s.expect_health(P1, 30);
            }

            #[test]
            fn r1021_an_effect_hit_on_your_hero_is_0() {
                let mut s = cast_against(
                    "tranquility-effect",
                    json!({ "hand": [ECLIPSE, FILLER], "library": [FILLER, FILLER] }),
                );
                s.play(
                    ECLIPSE,
                    json!({ "targets": [{ "pick": "hero", "player": "p1" }] }),
                );
                s.expect_health(P1, 30);
            }

            #[test]
            fn r1021_a_fatigue_hit_on_your_hero_is_0() {
                let mut s = cast_against(
                    "tranquility-fatigue",
                    json!({ "hand": [FILLER], "library": [FILLER, FILLER] }),
                );
                s.state_mut().players.p1.library.clear();

                // p1 draws from an empty library on p2's turn: the fatigue hit is 0.
                let events = apply_as(&mut s, P1, &[draw(json_as(json!({ "count": 2 })))]);
                assert!(
                    events
                        .iter()
                        .any(|event| matches!(event, GameEvent::Fatigue { .. }))
                );

                s.expect_health(P1, 30);
            }

            #[test]
            fn r1021_an_attackers_lifesteal_against_it_heals_nothing() {
                let mut s = cast_against(
                    "tranquility-lifesteal",
                    json!({
                        "field": [{ "def": JILLIAX, "lane": 1 }],
                        "hand": [FILLER],
                        "library": [FILLER, FILLER],
                        "health": 20,
                    }),
                );
                let jilliax = s.unit(P2, 1).expect("jilliax");
                s.attack(&jilliax, "hero");
                // The hit was 0, so its Lifesteal healed nothing.
                s.expect_health(P1, 30);
                s.expect_health(P2, 20);
            }

            #[test]
            fn r1021_lose_health_and_set_health_still_change_it() {
                let mut s = cast_against(
                    "tranquility-lose-set",
                    json!({ "hand": [FILLER], "library": [FILLER, FILLER] }),
                );
                apply_as(
                    &mut s,
                    P1,
                    &[lose_health(json_as(json!({ "player": "self", "amount": 5 })))],
                );
                s.expect_health(P1, 25);
                apply_as(
                    &mut s,
                    P1,
                    &[set_health(json_as(
                        json!({ "to": { "of": "selfHero" }, "value": 10 }),
                    ))],
                );
                s.expect_health(P1, 10);
            }

            #[test]
            fn r1021_the_immunity_is_gone_at_your_next_start_of_turn() {
                let mut s = cast_against(
                    "tranquility-gone",
                    json!({ "hand": [FILLER], "library": [FILLER, FILLER] }),
                );
                // It holds through the opponent's whole turn …
                apply_as(
                    &mut s,
                    P2,
                    &[damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": 5 }),
                    ))],
                );
                s.expect_health(P1, 30);
                // … and is gone as p1's next turn begins: a hit lands again.
                s.end_turn();
                assert_eq!(s.state().active, P1);
                apply_as(
                    &mut s,
                    P2,
                    &[damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": 5 }),
                    ))],
                );
                s.expect_health(P1, 25);
            }

            #[test]
            fn r456_cast_on_the_opponents_turn_it_ends_nothing() {
                let mut s = scenario(json!({
                    "seed": "tranquility-their-turn",
                    "active": "p2",
                    "p1": { "hand": [FILLER], "library": [FILLER, FILLER] },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));

                // p1 casts it on p2's turn: there is no turn of p1's to end.
                let events = apply_as(&mut s, P1, &[cast_new(json_as(json!({ "def": TRANQUILITY })))]);

                assert!(
                    events
                        .iter()
                        .any(|event| matches!(event, GameEvent::CardResolved { .. }))
                );
                assert_eq!(s.state().active, P2);
                assert!(
                    !events
                        .iter()
                        .any(|event| matches!(event, GameEvent::TurnEnded { .. }))
                );
                // The immunity still applies.
                assert_eq!(jackioh_engine::damage::hero_damage_cap(s.state(), P1), Some(0));
            }

            #[test]
            fn r1021_until_your_next_turn_every_hit_to_your_hero_is_0() {
                let s = cast("tranquility-immune", false);
                assert_eq!(
                    jackioh_engine::damage::hero_damage_cap(s.state(), P1),
                    Some(0),
                    "the immunity is a cap of 0 at §4.4 step 3"
                );
                // Gone as its player's next turn begins: opponent ends, then the cap is gone.
                let mut s = s;
                s.end_turn();
                assert_eq!(s.state().active, P1);
                assert_eq!(jackioh_engine::damage::hero_damage_cap(s.state(), P1), None);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r1021_radiant_the_immunity_without_ending_the_turn() {
                let s = cast("tranquility-radiant", true);
                // The turn goes on with the hero still immune.
                assert_eq!(s.state().active, P1);
                assert_eq!(jackioh_engine::damage::hero_damage_cap(s.state(), P1), Some(0));
            }
        }
    }
}
