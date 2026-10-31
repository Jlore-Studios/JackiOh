//! SPEC §8.11 Training Grounds #42 Stockpile, the snapshot of Core #5 Stockpile (`core-005`),
//! a card of the held Training Grounds set (R1420, R2194000), built by #2195.
//! Snapshot code not re-exported (R2195003); shares Core name (R2195001), deck holds one (R2195000).
//! Core #5 (SPEC §8, §8.1): Draw 2, heal 2; radiant Draw 4, heal 4 (§2.4, R4 full hand burn; §3, §10.5).

use jackioh_engine::effects::{draw, heal};
use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-042";

/// The draw and the heal are the declared numbers `draw` and `heal` (R386): 2 and 2, 4 and 4 on the
/// Radiant face, read off the face that is running.
pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![
                draw(json_as(json!({ "count": param(&*ctx, "draw") }))),
                heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": param(&*ctx, "heal") }))),
            ]
        })),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// BUILD M4-T4 row 5: Draw 2 heal 2, Radiant draw 4 heal 4 (§3, R4).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    /// Nine simple cards, enough to fill a hand to the cap next to the Stockpile being played.
    const NINE_FILLERS: [&str; 9] = [
        "core-001", "core-002", "core-003", "core-004", "core-008", "core-011", "core-012", "core-015", "core-020",
    ];

    mod n5_stockpile {
        use super::*;

        #[test]
        fn draws_2_and_heals_the_hero_2() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["traininggrounds-042"], "health": 20, "library": ["core-025", "core-007", "core-012"] }
            }));
            s.play("traininggrounds-042", json!({}));

            s.expect_health(PlayerId::P1, 22);
            s.expect_in_zone("core-025", "hand");
            s.expect_in_zone("core-007", "hand");
            s.expect_in_zone("core-012", "library");
            s.expect_in_zone("traininggrounds-042", "graveyard");
            assert_eq!(s.hand(PlayerId::P1).len(), 2);
        }

        #[test]
        fn the_hero_may_go_above_30_it_has_no_maximum_health_s3() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["traininggrounds-042"], "health": 30, "library": ["core-025", "core-007"] }
            }));
            s.play("traininggrounds-042", json!({}));
            s.expect_health(PlayerId::P1, 32);
        }

        #[test]
        fn r4_the_hand_cap_burns_the_extra_draw_to_the_graveyard() {
            crate::register_all();
            let mut hand = vec![json!("traininggrounds-042")];
            hand.extend(NINE_FILLERS.iter().map(|id| json!(id)));
            let mut s = scenario(json!({
                // The Stockpile plus nine fillers is a hand of 10; playing it leaves 9, so the first
                // draw fills the hand back to the cap and the second is burned.
                "p1": { "hand": hand, "health": 20, "library": ["core-025", "core-007"] }
            }));
            s.play("traininggrounds-042", json!({}));

            assert_eq!(s.hand(PlayerId::P1).len(), 10);
            s.expect_in_zone("core-025", "hand");
            s.expect_in_zone("core-007", "graveyard");
            s.expect_health(PlayerId::P1, 22);
            s.expect_events(json!(["cardPlayed", "burned"]));
        }

        #[test]
        fn radiant_draws_4_and_heals_4() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": "traininggrounds-042", "radiant": true }],
                    "health": 20,
                    "library": ["core-025", "core-007", "core-012", "core-011", "core-020", "core-008"]
                }
            }));
            s.play("traininggrounds-042", json!({}));

            s.expect_health(PlayerId::P1, 24);
            assert_eq!(s.hand(PlayerId::P1).len(), 4);
            s.expect_in_zone("core-011", "hand");
            s.expect_in_zone("core-020", "library");
        }

        #[test]
        fn r386_an_upgrade_draws_3_or_heals_3_and_a_degrade_draws_1_or_heals_1() {
            for (upgrade, key, cards, health) in
                [(true, "draw", 3, 22), (true, "heal", 2, 23), (false, "draw", 1, 22), (false, "heal", 2, 21)]
            {
                crate::register_all();
                let mut s = scenario(json!({
                    // #10 Rapid Replenish, a (0) Spell, keeps the turn from auto-ending (R82).
                    "p1": { "hand": ["traininggrounds-042", "core-010"], "health": 20, "library": ["core-025", "core-007", "core-012", "core-011"] }
                }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, "traininggrounds-042", key)
                } else {
                    crate::degrade_number(&mut s, "traininggrounds-042", key)
                };
                assert_eq!(moved, if upgrade { 3 } else { 1 });
                s.play("traininggrounds-042", json!({}));
                assert_eq!(s.hand(PlayerId::P1).len(), cards + 1);
                s.expect_health(PlayerId::P1, health);
            }
        }
    }

    /// R2195003: the snapshot plays exactly as core-005, on both faces: draws and heals.
    #[test]
    fn r2195003_plays_exactly_as_core_005() {
        for radiant in [false, true] {
            crate::expect_plays_as(
                "core-005",
                super::ID,
                |id| json!({ "p1": { "hand": [{ "def": id, "radiant": radiant }], "library": crate::filler(6), "health": 20 } }),
                |s, id| {
                    s.play(id, json!({}));
                },
            );
        }
    }
}
