//! SPEC §8.1 #5 Stockpile — Spell, cost 1.
//! Base: "Draw 2; heal your hero 2". Radiant: "Draw 5; heal 5" — the cell changes only the numbers
//! of the base clause (§8 Conventions), and "heal 5" is still the hero.
//!
//! Engine cell: the hand cap applies. That is the draw pipeline's job (§2.4, R4: hand size 10, a
//! card drawn into a full hand is burned to the graveyard), so this file just asks for the draws.
//! A hero has no maximum health (§3), so the heal may take it above 30; `heal` with an `amount` on
//! a hero adds health with no cap (effects/heal.rs).
//!
//! A Spell's on-resolve script hangs off `cry` (§10.5 step 5).

use jackioh_engine::effects::{draw, heal};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-005";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| {
            vec![
                draw(json_as(json!({ "count": 2 }))),
                heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": 2 }))),
            ]
        })),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![
                draw(json_as(json!({ "count": 5 }))),
                heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": 5 }))),
            ]
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// SPEC §8.1 #5 Stockpile. BUILD M4-T4 row 5: "Draw 2, heal 2 (hero may exceed 30); hand cap burns;
// radiant 5/5".
//
// §3 gives a hero no maximum health, so the heal runs past 30; R4 caps a hand at 10 and burns what
// a draw cannot fit to the graveyard. `library[0]` is the top of the library (the next card drawn).
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
                "p1": { "hand": ["core-005"], "health": 20, "library": ["core-025", "core-007", "core-012"] }
            }));
            s.play("core-005", json!({}));

            s.expect_health(PlayerId::P1, 22);
            s.expect_in_zone("core-025", "hand");
            s.expect_in_zone("core-007", "hand");
            s.expect_in_zone("core-012", "library");
            s.expect_in_zone("core-005", "graveyard");
            assert_eq!(s.hand(PlayerId::P1).len(), 2);
        }

        #[test]
        fn the_hero_may_go_above_30_it_has_no_maximum_health_s3() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-005"], "health": 30, "library": ["core-025", "core-007"] }
            }));
            s.play("core-005", json!({}));
            s.expect_health(PlayerId::P1, 32);
        }

        #[test]
        fn r4_the_hand_cap_burns_the_extra_draw_to_the_graveyard() {
            crate::register_all();
            let mut hand = vec![json!("core-005")];
            hand.extend(NINE_FILLERS.iter().map(|id| json!(id)));
            let mut s = scenario(json!({
                // The Stockpile plus nine fillers is a hand of 10; playing it leaves 9, so the first
                // draw fills the hand back to the cap and the second is burned.
                "p1": { "hand": hand, "health": 20, "library": ["core-025", "core-007"] }
            }));
            s.play("core-005", json!({}));

            assert_eq!(s.hand(PlayerId::P1).len(), 10);
            s.expect_in_zone("core-025", "hand");
            s.expect_in_zone("core-007", "graveyard");
            s.expect_health(PlayerId::P1, 22);
            s.expect_events(json!(["cardPlayed", "burned"]));
        }

        #[test]
        fn radiant_draws_5_and_heals_5() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": "core-005", "radiant": true }],
                    "health": 20,
                    "library": ["core-025", "core-007", "core-012", "core-011", "core-020", "core-008"]
                }
            }));
            s.play("core-005", json!({}));

            s.expect_health(PlayerId::P1, 25);
            assert_eq!(s.hand(PlayerId::P1).len(), 5);
            s.expect_in_zone("core-020", "hand");
            s.expect_in_zone("core-008", "library");
        }
    }
}
