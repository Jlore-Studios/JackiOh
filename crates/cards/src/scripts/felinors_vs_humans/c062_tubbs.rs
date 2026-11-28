//! SPEC §8.14 FvH #62 Tubbs ((2) Unit, Felinor, Common, 12/12 → 24/24), a card of the held Felinors vs
//! Humans set (R5235000).
//!   Base:    "Dormant for {dormant|turn|turns}." (dormant 3)
//!   Radiant: "Dormant for {dormant|turn|turns}." (dormant 2)
//!
//! Engine: Dormant (§6.1, R1839000), `StaticFlags.slumber`, S+ #63 Overdrive Dome's shape: it fills its
//! zone and nothing reaches it until it awakens at the start of its controller's 3rd turn from now
//! (Radiant 2nd, R1839001), summoning sick that turn (R2259000). The printed countdown is what its declared
//! `dormant` tunes (R386). The stats are catalog data. Built by S01 (#5251).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-062";

/// "Dormant for 3 turns", the printed countdown its declared `dormant` tunes (R386).
const DORMANT_TURNS: i32 = 3;
/// "Dormant for 2 turns" on the Radiant face.
const RADIANT_DORMANT_TURNS: i32 = 2;

fn tubbs(turns: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            slumber: Some(SlumberText {
                turns: Some(turns),
                wakes_by: None,
            }),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: tubbs(DORMANT_TURNS),
        radiant: tubbs(RADIANT_DORMANT_TURNS),
    }
}

// FvH #62 Tubbs — SPEC §8.14 row 62 (R1839000, R1839001, R2259000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, next_turn_of, scenario, upgrade_number};

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    /// p1 holds Tubbs (`radiant` its face) to play this turn.
    fn holding(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(8), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(8) },
        }))
    }

    /// R1839001: played, it lies Dormant through `sleeps` of its controller's next turn starts and is
    /// awake at the one after, summoning sick that turn (R2259000), free to attack the turn after.
    fn wakes_after(radiant: bool, sleeps: i32) {
        let mut s = holding(radiant);
        s.play(ID, json!({ "zone": 1 }));
        let tubbs = s.unit_at(P1, 1);
        assert_eq!(slumber_turns_of(s.card(&tubbs)), Some(sleeps + 1));
        for start in 1..=sleeps {
            next_turn_of(&mut s, P1);
            assert!(is_slumbering(s.card(&tubbs)), "still Dormant at own start {start}");
            assert!(s.attacks_of(&tubbs).is_empty());
        }
        next_turn_of(&mut s, P1);
        assert!(!is_slumbering(s.card(&tubbs)), "awake");
        assert!(s.attacks_of(&tubbs).is_empty(), "summoning sick the turn it awakens");
        next_turn_of(&mut s, P1);
        assert!(!s.attacks_of(&tubbs).is_empty(), "it may attack the turn after");
        let attack = s.stats(&tubbs).attack;
        s.attack(&tubbs, "hero");
        s.expect_health(P2, HERO_HEALTH - attack);
    }

    mod base {
        use super::*;

        #[test]
        fn r1839001_dormant_for_3_turns_it_awakens_at_its_controllers_third_turn_start() {
            wakes_after(false, 2);
        }

        #[test]
        fn r386_an_upgrade_of_its_dormant_number_shortens_the_countdown() {
            let mut s = holding(false);
            // An Upgrade moves the declared dormant down (R386): 3 becomes 2.
            upgrade_number(&mut s, ID, "dormant");
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(slumber_turns_of(s.card(ID)), Some(2));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1839001_dormant_for_2_turns_it_awakens_at_its_controllers_second_turn_start() {
            wakes_after(true, 1);
        }
    }
}
