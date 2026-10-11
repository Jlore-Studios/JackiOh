//! M #30.1 Love Bomb (SPEC §8.8 row 30.1, §7; R40, R58, R70, R413, R635, R748).
//!
//! Base:    "Cast on draw: Heal your hero {heal}."
//! Radiant: "Cast on draw: Heal your hero {heal}."
//! Engine: Cast on draw, as M #28.1. It runs `heal` of `heal` on its caster's hero (§6.3 Heal), with
//! no cap. C+ #22 Blood Moon's would-be-healed replacement applies when that card's controller's
//! enemy casts it (R413). M #30 and Meditative #67 make this card.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-030-1";

fn bomb() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| {
            vec![heal(json_as(json!({
                "target": { "of": "selfHero" },
                "amount": param(&*ctx, "heal"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // Both faces share the one script: only the declared `heal` differs (7, radiant 14).
    let base = bomb();
    let radiant = bomb();
    CardScripts { base, radiant }
}

// M #30.1 Love Bomb — SPEC §8.8 row 30.1, BUILD M10 row M 30.1: "Cast on draw: heals its caster's
// hero 7 with no cap, then the draw repeats (§2.4); an enemy C+ 22 Blood Moon turns it into 7 Pierce
// damage; uncast in a hand it may be played for (1); heal reads through `param()`; radiant 14".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOMB: &str = "meditative-030-1";
    const MOON: &str = "classicplus-022"; // Trap: a heal on its controller's enemy becomes Pierce damage.
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// The `healed` events on `target` ("hero-p1"), as amounts.
    fn healed(s: &Scenario, target: &str) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Healed { target_id, amount } if target_id == target => Some(*amount),
                _ => None,
            })
            .collect()
    }

    /// The bomb on top of its owner's library, so the next draw casts it.
    fn bombed_top(radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": if radiant_face { "bomb-radiant" } else { "bomb-drawn" },
            "p1": {
                "hand": [FILLER],
                "library": [{ "def": BOMB, "radiant": radiant_face }, FILLER, FILLER, FILLER],
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    mod m30_1_love_bomb {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn drawn_it_heals_its_caster_7_past_30_then_the_draw_repeats() {
                let mut s = bombed_top(false);

                s.start_turn();

                // No cap: 30 becomes 37.
                assert_eq!(healed(&s, "hero-p1"), vec![7]);
                assert_eq!(s.state().players[P1].hero.health, 37);
                s.expect_in_zone(BOMB, "graveyard");
                assert!(s.hand(P1).iter().any(|card| card.def_id == FILLER));
            }

            #[test]
            fn an_enemy_blood_moon_turns_it_into_7_pierce_damage() {
                crate::register_all();
                // p1 holds Blood Moon face-down; p2 draws the bomb on p2's turn, so the cast is
                // p2's and the Moon's replacement (its controller's enemy healing) applies.
                let mut s = scenario(json!({
                    "seed": "bomb-moon",
                    "p1": {
                        "hand": [FILLER],
                        "backrow": [{ "def": MOON, "lane": 1, "faceUp": false }],
                        "library": filler(3),
                    },
                    "p2": {
                        "hand": [FILLER],
                        "library": [BOMB, FILLER, FILLER, FILLER],
                    },
                }));

                s.end_turn();

                assert!(
                    s.events()
                        .iter()
                        .any(|event| matches!(event, GameEvent::TrapFired { def_id, .. } if def_id == MOON))
                );
                assert!(healed(&s, "hero-p2").is_empty());
                assert_eq!(s.state().players[P2].hero.health, 23);
                s.expect_in_zone(BOMB, "graveyard");
            }

            #[test]
            fn played_from_hand_for_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "bomb-played",
                    "p1": { "hand": [BOMB, FILLER], "library": filler(3) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));

                s.play(BOMB, json!({}));

                assert_eq!(healed(&s, "hero-p1"), vec![7]);
                assert_eq!(s.state().players[P1].hero.health, 37);
                s.expect_in_zone(BOMB, "graveyard");
                s.expect_mana(P1, 3);
            }

            #[test]
            fn heal_reads_through_param() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "bomb-param",
                    "p1": { "hand": [FILLER], "library": [BOMB, FILLER, FILLER, FILLER] },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                set_param(s.card_mut(BOMB), "heal", 3);

                s.start_turn();

                assert_eq!(healed(&s, "hero-p1"), vec![3]);
                assert_eq!(s.state().players[P1].hero.health, 33);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn heals_14() {
                let mut s = bombed_top(true);

                s.start_turn();

                assert_eq!(healed(&s, "hero-p1"), vec![14]);
                assert_eq!(s.state().players[P1].hero.health, 44);
                s.expect_in_zone(BOMB, "graveyard");
            }
        }
    }
}
