//! SPEC §8.14 FvH #28 Book of Design Principles. (1) Spell, Book, Epic. Built by S03 (#5253).
//!   Base:    "Add {cards|Goetzmann Game Design|Goetzmann Game Designs} to your hand." — 2
//!   Radiant: "Add {cards|Radiant Goetzmann Game Design|Radiant Goetzmann Game Designs} to your hand." — 2
//!
//! TG #40 Goetzmann Game Design, added the declared `cards` times (R386), Radiant on the Radiant face. Its
//! Quickdraw does nothing once it is in the hand mid-game, and the card it adds costs (1) less as ever
//! (R4154210); a full hand burns the rest (R317).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-028";

/// TG #40 Goetzmann Game Design.
const DESIGN: &str = "traininggrounds-040";

fn design_principles(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            (0..param(&*ctx, "cards").max(0))
                .map(|_| {
                    add_to_hand(AddToHandArgs {
                        def_id: Some(DESIGN.to_string()),
                        radiant: radiant.then_some(true),
                        ..AddToHandArgs::default()
                    })
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: design_principles(false),
        radiant: design_principles(true),
    }
}

// FvH #28 Book of Design Principles — SPEC §8.14 row 28 (R4154210, R317).
#[cfg(test)]
mod tests {
    use super::{DESIGN, ID};
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005"; // Stockpile

    fn holding(radiant: bool, seed: &str, fillers: usize) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend((0..fillers).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// p1's Goetzmann Game Designs in hand: whether each is Radiant.
    fn designs(s: &Scenario) -> Vec<bool> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id == DESIGN)
            .map(|card| card.radiant)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn adds_two_goetzmann_game_designs() {
            let mut s = holding(false, "design", 1);
            s.play(ID, json!({}));
            assert_eq!(designs(&s), vec![false, false]);
        }

        #[test]
        fn r317_a_full_hand_burns_the_rest() {
            let mut s = holding(false, "design-full", 9);
            s.play(ID, json!({}));
            assert_eq!(designs(&s).len(), 1);
            let burned = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Burned { def_id, .. } if def_id == DESIGN))
                .count();
            assert_eq!(burned, 1);
        }

        #[test]
        fn r386_cards_reads_through_param() {
            let mut s = holding(false, "design-param", 1);
            step_param(s.card_mut(ID), "cards", 1);
            s.play(ID, json!({}));
            assert_eq!(designs(&s).len(), 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn adds_two_radiant_goetzmann_game_designs() {
            let mut s = holding(true, "design-radiant", 1);
            s.play(ID, json!({}));
            assert_eq!(designs(&s), vec![true, true]);
        }
    }
}
