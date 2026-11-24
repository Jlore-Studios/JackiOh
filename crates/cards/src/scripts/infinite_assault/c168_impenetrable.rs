//! SPEC §8.13 IA #168 Impenetrable ((4) Spell, Mythic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Your hero and Units are immune to damage until your next turn. Draw {draw|card|cards}." (draw 2)
//!   Radiant: "Your hero and Units are immune to damage until your next turn. Draw {draw|card|cards} and your hero
//!            gains {armor} Armor." (draw 3, armor 10)
//!
//! Engine: the controller's hero is made immune (MD-C21) and their Units are made immune (R4591004)
//! — every Unit they control while it holds, summoned later ones included — both with R757's Armor
//! Up expiry; then the declared {draw} cards are drawn, and on the Radiant face the hero gains the
//! declared {armor} Armor.

use jackioh_engine::effects::{draw, gain_hero_armor, make_hero_immune, make_units_immune};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-168";

fn impenetrable(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects = vec![
                make_hero_immune(json_as(json!({}))),
                make_units_immune(json_as(json!({}))),
                draw(json_as(json!({ "count": param(&*ctx, "draw") }))),
            ];
            if radiant {
                effects.push(gain_hero_armor(json_as(json!({
                    "amount": param(&*ctx, "armor"),
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: impenetrable(false),
        radiant: impenetrable(true),
    }
}

// IA #168 Impenetrable — SPEC §8.13 row 168 (R4591004).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 3/3";
    const HITTER: &str = "stand-in (2) 4/4";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "impenetrable",
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [UNIT],
                "library": crate::filler(6),
                "mana": 8,
            },
            "p2": { "field": [HITTER, HITTER], "library": crate::filler(6) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r4591004_impenetrable_hero_and_units_take_no_damage_and_you_draw_2() {
            let mut s = board(false);
            let hand = s.hand(P1).len();
            s.play(ID, json!({}));
            assert_eq!(s.hand(P1).len(), hand + 1, "drew 2 with the cast one spent");
            s.end_turn();
            let hitter = s.unit_at(P2, 1);
            let unit = s.unit_at(P1, 1);
            s.attack(&hitter, &unit);
            s.expect_stats(&s.unit_at(P1, 1), json!({ "health": 3 }), "the unit takes nothing");
            let second = s.unit_at(P2, 2);
            s.attack(&second, "hero");
            s.expect_health(P1, 30);
        }

        #[test]
        fn the_window_ends_as_your_next_turn_begins() {
            let mut s = board(false);
            s.play(ID, json!({}));
            s.end_turn();
            s.end_turn();
            let hitter = s.unit_at(P2, 1);
            let unit = s.unit_at(P1, 1);
            s.attack(&hitter, &unit);
            // The 4 lands on the 3/3 now that the window is gone.
            s.expect_in_zone(&unit, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn you_draw_3_and_gain_10_armor_behind_the_window() {
            let mut s = board(true);
            let hand = s.hand(P1).len();
            s.play(ID, json!({}));
            assert_eq!(s.hand(P1).len(), hand + 2, "drew 3 with the cast one spent");
            assert_eq!(s.state().players[P1].hero.armor, 10);
            s.end_turn();
            let hitter = s.unit_at(P2, 1);
            s.attack(&hitter, "hero");
            s.expect_health(P1, 30);
            assert_eq!(s.state().players[P1].hero.armor, 10);
        }
    }
}
