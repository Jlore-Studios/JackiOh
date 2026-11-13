//! SPEC §8.12 S+ #108 Swarmer (Unit 2/4 → 4/8, Mindustrial, Rare, cost 2), a card of the Stinkyscape+ set (R1420).
//!   Base:    "Armor 1, Can't Defend, Ranged, Cleave, Flexible"
//!   Radiant: "Armor 1, Can't Defend, Ranged, Cleave, Flexible, Pierce"
//!
//! Every word is a catalog keyword (§6.1), so there is no script to run. Flexible lets it stand in the
//! backrow and attack from there as a Unit (R3543000); Cleave hits the Units beside its target for its
//! attack, and Cleave's splash never strikes back (R783000); the Radiant face's Pierce ignores Armor on
//! every one of those hits (R346).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-108";

pub fn script() -> CardScripts {
    // Keywords only: catalog data on both faces.
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #108 Swarmer — SPEC §8.12 row 108 (R3543000; R346, R783000).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TOUGH: &str = "stand-in (1) 0/40";
    const ARMORED: &str = "stand-in (1) 0/40 Armor 3";

    /// What each face hits for.
    const ATTACK: i32 = 2;
    const RADIANT_ATTACK: i32 = 4;
    const RADIANT_HEALTH: i32 = 8;

    /// Swarmer standing in p1's backrow, p2's five Units across the lanes.
    fn standing(radiant: bool, seed: &str, p2: Value) -> Scenario {
        weapon_board(
            seed,
            json!({ "field": [{ "def": ID, "radiant": radiant, "row": "backrow", "lane": 3 }] }),
            json!({ "field": p2 }),
        )
    }

    mod base {
        use super::*;

        #[test]
        fn r3543000_it_cleaves_from_the_backrow_and_takes_nothing_back() {
            let mut s = standing(false, "s108-base", json!([TOUGH, TOUGH, TOUGH, TOUGH, TOUGH]));
            let lanes: Vec<String> = (1..=5).map(|lane| s.unit_at(P2, lane).id).collect();
            assert!(s.backrow(P1, 3).is_some_and(|card| card.def_id == ID), "it stands in the backrow");
            s.attack(ID, lanes[2].as_str());
            for (at, id) in lanes.iter().enumerate() {
                let beside = (1..=3).contains(&at);
                assert_eq!(s.card(id.as_str()).damage, if beside { ATTACK } else { 0 }, "lane {}", at + 1);
            }
            assert_eq!(s.card(ID).damage, 0);
        }

        #[test]
        fn r346_its_cleave_does_not_pierce_armor() {
            let mut s = standing(false, "s108-armor", json!([TOUGH, ARMORED, TOUGH]));
            let (left, target, right) = (s.unit_at(P2, 1).id, s.unit_at(P2, 2).id, s.unit_at(P2, 3).id);
            s.attack(ID, target.as_str());
            assert_eq!(s.card(target.as_str()).damage, 0, "Armor 3 takes all of 2");
            assert_eq!(s.card(left.as_str()).damage, ATTACK);
            assert_eq!(s.card(right.as_str()).damage, ATTACK);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r346_the_cleave_pierces_armor_on_each_unit_it_hits() {
            let mut s = standing(true, "s108-radiant", json!([ARMORED, ARMORED, ARMORED]));
            let (left, target, right) = (s.unit_at(P2, 1).id, s.unit_at(P2, 2).id, s.unit_at(P2, 3).id);
            s.attack(ID, target.as_str());
            for id in [left, target, right] {
                assert_eq!(s.card(id.as_str()).damage, RADIANT_ATTACK, "Armor 3 takes nothing");
            }
            assert_eq!(s.card(ID).damage, 0);
        }

        #[test]
        fn its_health_is_doubled() {
            let s = standing(true, "s108-health", json!([]));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (RADIANT_ATTACK, RADIANT_HEALTH));
        }
    }
}
