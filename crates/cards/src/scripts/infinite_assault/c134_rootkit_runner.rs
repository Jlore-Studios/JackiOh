//! SPEC §8.13 IA #134 Rootkit Runner ((2) Unit, Rare, 3/3 → 6/6), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cloaked\nWhen this is revealed, steal a random card from your opponent's hand."
//!   Radiant: "Cloaked\nWhen this is revealed, steal two random cards from your opponent's hand."
//!
//! Cloaked is catalog data; the second line is the engine's `revealed` hook (R4590000), run once the
//! Unit has come out and its Cry — it has none — has run, as its controller. It never runs on a
//! face-up play. The steal is #133 Backdoor's random hand steal (§6.3 Steal, R60).

use jackioh_engine::effects::give_from_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-134";

/// "Steal a random card" (base) or "two random cards" (Radiant) from the opponent's hand.
const STEALS: i32 = 1;
const RADIANT_STEALS: i32 = 2;

fn runner(steals: i32) -> Script {
    Script {
        revealed: Some(hook(move |_ctx| {
            vec![give_from_hand(json_as(json!({
                "from": "enemy",
                "cards": "random",
                "count": steals,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: runner(STEALS),
        radiant: runner(RADIANT_STEALS),
    }
}

// IA #134 Rootkit Runner — SPEC §8.13 row 134 (R4590000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::alt_play::CLOAKED_REVEAL;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": crate::filler(3),
                "mana": 10,
            },
            "p2": { "hand": [VANILLA, VANILLA, FILLER], "library": crate::filler(3) },
        }))
    }

    /// Set the Runner in hand face-down by its Cloaked; its fresh id.
    fn set_runner(s: &mut Scenario) -> String {
        let id = s
            .hand(P1)
            .iter()
            .find(|card| card.def_id == ID)
            .map(|card| card.id.clone())
            .expect("a Rootkit Runner in hand");
        s.play(id.as_str(), json!({ "zone": 1, "row": "backrow", "faceDown": "cloaked" }));
        s.backrow(P1, 1).expect("the set Runner").id
    }

    fn reveal(s: &mut Scenario, id: &str) {
        s.activate(id, json!({ "ability": CLOAKED_REVEAL }));
    }

    mod base {
        use super::*;

        #[test]
        fn r4590000_the_reveal_steals_one_random_card() {
            let mut s = board(false);
            let set = set_runner(&mut s);
            reveal(&mut s, &set);
            let unit = s.unit(P1, 1).expect("revealed");
            assert_eq!(unit.def_id, ID);
            assert_eq!(s.hand(P1).len(), 2, "the Filler and the one stolen card");
            assert_eq!(s.hand(P2).len(), 2, "one card taken");
        }

        #[test]
        fn r4590000_a_face_up_play_steals_nothing() {
            let mut s = board(false);
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(s.hand(P1).len(), 1, "the Filler alone");
            assert_eq!(s.hand(P2).len(), 3, "nothing taken");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4590000_the_reveal_steals_two_random_cards() {
            let mut s = board(true);
            let set = set_runner(&mut s);
            reveal(&mut s, &set);
            let unit = s.unit(P1, 1).expect("revealed");
            assert_eq!(unit.def_id, ID);
            assert_eq!(s.hand(P1).len(), 3, "the Filler and the two stolen cards");
            assert_eq!(s.hand(P2).len(), 1, "two cards taken");
        }
    }
}
