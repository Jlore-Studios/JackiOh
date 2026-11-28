//! SPEC §8.14 FvH #89 GIGA Felinor ((4) Unit, Felinor, Common, 16/16 → 48/48), a card of the held Felinors
//! vs Humans set (R5235000).
//!   Base:    "GIGA"
//!   Radiant: "GIGA"
//!
//! Engine: GIGA is catalog data (§6.1, R3546000): four zones, two lanes wide in both rows, taken as it enters
//! the field (R3546002), in both its lanes for every lane rule (R3546001), moving only whole (R3546003). So
//! there is no script to run. The Radiant face's 48/48, triple the base, is the designer's data (FV2).
//! Declares no numbers. Built by S01 (#5251).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-089";

pub fn script() -> CardScripts {
    let base = Script::default();
    // GIGA and both lines are catalog data on either face.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// FvH #89 GIGA Felinor — SPEC §8.14 row 89 (R3546000, R3546001, R3546002).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    /// R3546000, R3546001: played at lane 1, it fills lanes 1 and 2 of both rows.
    fn fills_two_lanes(radiant: bool, attack: i32) {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        let felinor = s.unit_at(P1, 1);
        assert!(is_giga(&felinor), "it entered GIGA");
        assert_eq!(lanes_of(&felinor), vec![1, 2]);
        for row in [Row::Units, Row::Backrow] {
            for lane in [1, 2] {
                let slot = ZoneSlot { player: P1, row, lane };
                assert!(!is_empty(s.state(), slot), "{row:?} {lane} is filled by it");
            }
            let slot = ZoneSlot { player: P1, row, lane: 3 };
            assert!(is_empty(s.state(), slot), "{row:?} 3 is left open");
        }
        s.expect_stats(&felinor, json!({ "attack": attack, "health": attack }));
    }

    #[test]
    fn declares_no_tuned_numbers() {
        crate::register_all();
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r3546000_played_it_fills_four_zones_two_lanes_wide() {
            fills_two_lanes(false, 16);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3546000_played_radiant_it_fills_four_zones_two_lanes_wide() {
            fills_two_lanes(true, 48);
        }
    }
}
