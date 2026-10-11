//! M #37 CN in a bottle (SPEC §8.8 row 37): (X) Spell, CN, Epic.
//!
//! Base:    "When this enters your hand: Replace it with a random Radiant card."
//! Radiant: "When this enters your hand: Replace it with a random Radiant card. It costs (0)."
//! Engine:
//! - **The hook:** the NEW Script hook `enters_hand`, run by `draw::run_arrival_hooks` on a hand
//!   arrival, beside R151's start-of-game clause (MD-B18, R925).
//! - **The replacement:** `transform_random` of the card itself in the hand (§6.3 Replace, R31;
//!   R671 keeps its place). The pool is the non-token cards of every open set except this one
//!   (R380, R387). The new card is Radiant and carries NEW riders: `chinese` (ME-CN) and, on the
//!   Radiant face, `costOverride: 0`.
//! - **What follows:** the replacement is a card generated into a hand, so R673's Glitch roll
//!   applies. It runs its own arrival hooks (R151, the `arrives` rider), so a Heroic Power rolls its
//!   power. Only the owner sees any of it (R97).
//! - **The (X)** is printed only (MD-B19, R926): the card is never played from a hand, it costs 0
//!   wherever a rule reads it out of play (R65), and a cast of it resolves with no effect. A full
//!   hand burns the card before it enters (§2.4, R4), so it reaches the graveyard unreplaced.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-037";

fn bottle(radiant: bool) -> Script {
    Script {
        enters_hand: Some(hook(move |_ctx| {
            let mut args = json!({
                "target": { "of": "self" },
                "radiant": true,
                "chinese": true,
                "arrives": true,
            });
            if radiant {
                args["costOverride"] = json!(0);
            }
            vec![transform_random(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // No cry on either face: a cast resolves with no effect (MD-B19, R926). The Radiant face's new
    // card costs (0).
    let base = bottle(false);
    let radiant = bottle(true);
    CardScripts { base, radiant }
}

// M #37 CN in a bottle — SPEC §8.8 row 37, BUILD M10 row M 37: "On every arrival in a hand (a draw,
// an add, a Discover, a bounce, a steal, the opening deal, a mulligan replacement; MD-B18) it is
// replaced in place by a random non-token Radiant card of any set but this one, carrying `chinese`,
// hidden from the opponent (R97); the new card is generated for R673 and runs its own arrival
// hooks (R151); a full hand burns it unreplaced; a cast or a random play of it does nothing, and
// out of play it costs 0 (MD-B19); radiant the new card costs (0)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOTTLE: &str = "meditative-037";
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2.
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    mod m37_cn_in_a_bottle {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r925_a_draw_replaces_it_in_place_radiant_chinese_hidden_from_the_opponent() {
                // Stockpile draws the bottle on top of the library, then a filler: the bottle is
                // replaced where it arrived.
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "bottle-draw",
                    "p1": {
                        "hand": [{ "def": STOCKPILE }, FILLER],
                        "library": [{ "def": BOTTLE }, FILLER, FILLER, FILLER],
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(STOCKPILE, json!({}));
                let hand = s.hand(P1);
                assert_eq!(hand.len(), 3);
                assert!(hand.iter().all(|card| card.def_id != BOTTLE));
                let new_card = hand[1].clone();
                assert_ne!(new_card.def_id, BOTTLE);
                assert!(new_card.radiant);
                assert_eq!(new_card.chinese, Some(true));
                assert_eq!(new_card.cost_override, None);
                // The filler drawn after it lands behind it: the replacement kept its place.
                assert_eq!(hand[2].def_id, FILLER);
                // The opponent sees a count, never the card (R97).
                let view: Value =
                    serde_json::to_value(view_for(s.state(), P2)).expect("the view serialises");
                assert_eq!(view["opponent"]["hand"], json!({ "count": 3 }));
            }

            #[test]
            fn r925_a_full_hand_burns_it_unreplaced() {
                // Ten in hand and the bottle alone in the library: the turn draw burns it before it
                // enters (§2.4, R4), so it reaches the graveyard as itself.
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "bottle-burn",
                    "p1": {
                        "hand": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                        "library": [{ "def": BOTTLE }],
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.end_turn();
                s.end_turn();
                assert_eq!(s.hand(P1).len(), 10);
                assert!(s.hand(P1).iter().all(|card| card.def_id != BOTTLE));
                let burned: Vec<&CardInstance> = s
                    .state()
                    .players
                    .p1
                    .graveyard
                    .iter()
                    .filter(|card| card.def_id == BOTTLE)
                    .collect();
                assert_eq!(burned.len(), 1);
            }

            #[test]
            fn r926_out_of_play_it_costs_0_and_declares_no_cry() {
                // The (X) is printed only: out of play the card costs 0 (R65).
                crate::register_all();
                let s = scenario(json!({
                    "seed": "bottle-cost",
                    "p1": {
                        "hand": [{ "def": BOTTLE }, FILLER],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                let held = s.hand(P1).into_iter().find(|card| card.def_id == BOTTLE).expect("the bottle");
                assert_eq!(effective_cost(s.state(), &held, Default::default()), 0);
                // And neither face runs a cry: a cast resolves with no effect (MD-B19).
                assert!(script().base.cry.is_none());
                assert!(script().radiant.cry.is_none());
                assert!(script().base.enters_hand.is_some());
                assert!(script().radiant.enters_hand.is_some());
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_new_card_costs_0() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "bottle-radiant",
                    "p1": {
                        "hand": [{ "def": STOCKPILE }, FILLER],
                        "library": [{ "def": BOTTLE, "radiant": true }, FILLER, FILLER, FILLER],
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(STOCKPILE, json!({}));
                let hand = s.hand(P1);
                assert_eq!(hand.len(), 3);
                assert!(hand.iter().all(|card| card.def_id != BOTTLE));
                let new_card = hand[1].clone();
                assert!(new_card.radiant);
                assert_eq!(new_card.chinese, Some(true));
                assert_eq!(new_card.cost_override, Some(0));
            }
        }
    }
}
