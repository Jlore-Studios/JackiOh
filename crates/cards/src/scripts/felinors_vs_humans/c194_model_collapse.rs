//! SPEC §8.14 FvH #194 Plagiarized: Model Collapse, a frozen copy of IA #233 Model Collapse (R2195003, FV4),
//! a card of the held Felinors vs Humans set, built by #5235.
//!
//! The code is a copy of the IA card's as it stood when the set was founded, not a re-export, so a later IA
//! patch does not move it (R2195003). Its `ID` is its own, so every read of it names this card; it is named
//! `Plagiarized: Model Collapse`, tagged Plagiarized only, and a token it makes stays the IA token. Its
//! change from IA #233: none. The IA file's notes, as they stood:
//!
//! IA #233 Model Collapse (SPEC §8.13 row 233), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Start of turn: Nerf every enemy Unit and every card in your opponent's hand."
//!   Radiant: "Start of turn: Nerf every enemy Unit and every card in your opponent's hand twice."
//!
//! Two Nerfs, each once on the base face and twice on the Radiant face: one over the enemy Units
//! on the field, one over every card in the opponent's hand (the hand's changes silent to the
//! other seat, R440). Its controller's cards are untouched.
//!
//! Engine: `degrade`. Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-194";

/// How many times each Nerf lands.
const TIMES: i32 = 1;
const RADIANT_TIMES: i32 = 2;

fn collapse(times: i32) -> Script {
    Script {
        start_of_turn: Some(hook(move |_| {
            vec![
                degrade(json_as(json!({
                    "scope": { "side": "enemy", "zones": ["field"], "types": ["Unit"] },
                    "times": times,
                }))),
                degrade(json_as(json!({
                    "scope": { "side": "enemy", "zones": ["hand"] },
                    "times": times,
                }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: collapse(TIMES), radiant: collapse(RADIANT_TIMES) }
}

// FvH #194 Plagiarized: Model Collapse — SPEC §8.14 row 194; IA #233's tests (SPEC §8.13 row 233), on this
// card.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const VICTIM: &str = "core-008"; // Mr. Vanilla
    const FRIEND: &str = "core-008"; // Mr. Vanilla: untouched on its own side.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn degraded(s: &Scenario) -> usize {
        s.events().iter().filter(|event| event.event_type().as_str() == "degraded").count()
    }

    fn stats(s: &Scenario, player: PlayerId, lane: i32) -> (i32, i32) {
        let view = unit_view(s.state(), &s.unit_at(player, lane));
        (view.attack, view.health)
    }

    /// What the field menu can change on a Vanilla (B3.4): stats or a harmful keyword. The draw is
    /// random, so the test checks movement of either, never exact numbers.
    fn changed(s: &Scenario) -> ((i32, i32), Vec<String>) {
        let view = unit_view(s.state(), &s.unit_at(P2, 1));
        let mut keywords: Vec<String> = view.keywords.iter().map(|keyword| format!("{:?}", keyword.kind())).collect();
        keywords.sort();
        ((view.attack, view.health), keywords)
    }

    fn standing(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "active": "p1",
            "p1": {
                "field": [{ "def": ID, "radiant": radiant }, FRIEND],
                "hand": [STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": {
                "field": [VICTIM],
                "hand": [STOCKPILE, STOCKPILE],
                "library": crate::filler(4),
            },
        }));
        s.start_turn();
        s
    }

    mod base {
        use super::*;

        #[test]
        fn the_enemy_unit_and_hand_are_nerfed_friends_untouched() {
            let s = standing(false, "collapse-base");
            // One field Unit and two hand cards, once each.
            assert_eq!(degraded(&s), 1 + 2);
            assert_ne!(changed(&s), ((4, 4), vec![]), "the enemy Unit changed");
            assert_eq!(stats(&s, P1, 2), (4, 4));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_radiant_face_nerfs_twice() {
            let s = standing(true, "collapse-radiant");
            assert_eq!(degraded(&s), (1 + 2) * 2);
            assert_ne!(changed(&s), ((4, 4), vec![]), "the enemy Unit changed");
            assert_eq!(stats(&s, P1, 2), (4, 4));
        }
    }
}
