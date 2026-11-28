//! SPEC §8.14 FvH #166 Plagiarized: Thought Accelerator, a frozen copy of IA #206 Thought Accelerator
//! (R2195003, FV4), a card of the held Felinors vs Humans set, built by #5235.
//!
//! The code is a copy of the IA card's as it stood when the set was founded, not a re-export, so a later IA
//! patch does not move it (R2195003). Its `ID` is its own, so every read of it names this card; it is named
//! `Plagiarized: Thought Accelerator`, tagged Plagiarized only, and a token it makes stays the IA token. Its
//! change from IA #206: it gains Tradable on both faces (a catalog keyword, R1838000). The IA file's notes,
//! as they stood:
//!
//! IA #206 Thought Accelerator (SPEC §8.13 row 206), a card of the held Infinite Assault
//! set (R4577000).
//!   Base:    "Cry: Your Dormant Units awaken."
//!   Radiant: "Cry: Your Dormant Units awaken. They gain +{buff}/+{buff}."
//!
//! Engine: a Cry (R1): each Dormant card its controller has on the field, in lane order,
//! `awaken`s — each "When this awakens" answering (R1839002) — and on the Radiant face each
//! takes a permanent `buff` of the declared `buff` after it wakes (R4593011). Built by C9
//! (#4593).

use jackioh_engine::effects::{awaken, buff};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-166";

fn accelerator(radiant_face: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let sleepers: Vec<String> = units_on_field_of(ctx.state, ctx.controller)
                .into_iter()
                .filter(|card| is_slumbering(card))
                .map(|card| card.id.clone())
                .collect();
            let gain = if radiant_face { param(&*ctx, "buff") } else { 0 };
            let mut effects = Vec::new();
            for id in sleepers {
                effects.push(awaken(json_as(json!({
                    "target": { "of": "instance", "instanceId": id },
                }))));
                if radiant_face {
                    effects.push(buff(json_as(json!({
                        "target": { "of": "instance", "instanceId": id },
                        "attack": gain,
                        "health": gain,
                    }))));
                }
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: accelerator(false),
        radiant: accelerator(true),
    }
}

// FvH #166 Plagiarized: Thought Accelerator — SPEC §8.14 row 166; IA #206's tests (SPEC §8.13 row 206,
// R4593011), on this card.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const SLEEPER: &str = "infiniteassault-199"; // Patient Machine: Dormant for 1 turn.
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn accelerated(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [SLEEPER, { "def": ID, "radiant": radiant }, FILLER],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        // Played from the hand, the sleeper enters Dormant.
        s.play(SLEEPER, json!({ "zone": 1 }));
        assert!(is_slumbering(&s.unit_at(P1, 1)), "Dormant as it lands");
        s.play(ID, json!({ "zone": 2 }));
        s
    }

    /// FV4: the card gains Tradable on both faces, a catalog keyword the engine plays (R1838000):
    /// traded from the hand at no mana, it goes into the deck and a card is drawn.
    fn expect_tradable(radiant: bool, seed: &str) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.state_mut().players[P1].auto_end_turn = Some(false);
        assert!(is_tradable(s.state(), s.card(ID)), "radiant {radiant}");
        let (hand, mana) = (s.hand(P1).len(), s.state().players[P1].mana.current);
        s.trade(ID);
        s.expect_events(json!(["traded", "shuffledIn", "drawn"]));
        assert_eq!(
            s.hand(P1).len(),
            hand,
            "one card went into the deck and one was drawn"
        );
        assert_eq!(s.state().players[P1].mana.current, mana, "at no mana");
    }

    mod base {
        use super::*;

        #[test]
        fn r4593011_a_dormant_unit_wakes_with_no_buff() {
            let s = accelerated(false, "accelerator-base");
            let unit = s.unit_at(P1, 1);
            assert!(!is_slumbering(&unit));
            let view = unit_view(s.state(), &unit);
            assert_eq!((view.attack, view.health), (5, 6));
        }

        #[test]
        fn r1838000_it_is_tradable() {
            expect_tradable(false, "accelerator-trade-base");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4593011_a_dormant_unit_wakes_with_2_2() {
            let s = accelerated(true, "accelerator-radiant");
            let unit = s.unit_at(P1, 1);
            assert!(!is_slumbering(&unit));
            let view = unit_view(s.state(), &unit);
            assert_eq!((view.attack, view.health), (7, 8));
        }

        #[test]
        fn r1838000_it_is_tradable() {
            expect_tradable(true, "accelerator-trade-radiant");
        }
    }
}
