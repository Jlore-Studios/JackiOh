//! M #48 Tranquility (SPEC §8.8 row 48): (2) Spell, Epic.
//!
//! Base:    "End your turn. Your hero is immune to damage until your next turn begins."
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
                vec![
                    end_turn(json_as(json!({}))),
                    make_hero_immune(json_as(json!({}))),
                ]
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
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TRANQUILITY: &str = "meditative-048";
    const FILLER: &str = "core-005";

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
