//! SPEC §8.13 IA #135 Mana Leech ((2) Unit, Rare, 2/4 → 4/8), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Steal one of your opponent's mana crystals."
//!   Radiant: "Cry: Steal two of your opponent's mana crystals."
//!
//! `stealSomeManaCrystals` (R4590002): N crystals counted from the capped maximum — the victim's
//! own first, then crystals they had stolen — held past the cap by the thief.

use jackioh_engine::effects::steal_some_mana_crystals;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-135";

/// "Steal one" (base) or "two" (Radiant) of the opponent's mana crystals.
const CRYSTALS: i32 = 1;
const RADIANT_CRYSTALS: i32 = 2;

fn leech(crystals: i32) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![steal_some_mana_crystals(StealSomeManaCrystalsArgs { count: crystals })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: leech(CRYSTALS),
        radiant: leech(RADIANT_CRYSTALS),
    }
}

// IA #135 Mana Leech — SPEC §8.13 row 135 (R4590002).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn leeched(radiant: bool, turns: i32) -> Scenario {
        let mut s = scenario(json!({
            "seed": "mana-leech",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 },
            "p2": { "hand": ["core-005"], "mana": 10 },
        }));
        s.state_mut().players.p2.turns_started = turns;
        s
    }

    fn crystals(s: &Scenario, player: PlayerId) -> i32 {
        jackioh_engine::mana::mana_crystal_count(&s.state().players[player])
    }

    mod base {
        use super::*;

        #[test]
        fn r4590002_cry_steals_one_mana_crystal() {
            let mut s = leeched(false, 4);
            assert_eq!(crystals(&s, P2), 4);
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(crystals(&s, P2), 3, "the victim drops to three inside the cap");
            assert_eq!(
                jackioh_engine::mana::stolen_mana_crystals_of(&s.state().players[P1]),
                1,
                "the thief holds it past the cap"
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4590002_cry_steals_two_mana_crystals() {
            let mut s = leeched(true, 4);
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(crystals(&s, P2), 2, "the victim drops to two inside the cap");
            assert_eq!(
                jackioh_engine::mana::stolen_mana_crystals_of(&s.state().players[P1]),
                2,
                "the thief holds them past the cap"
            );
        }
    }
}
