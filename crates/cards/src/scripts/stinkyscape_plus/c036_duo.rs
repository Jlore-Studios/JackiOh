//! SPEC §8.12 S+ #36 Duo (Unit 2/1 → 4/2, Mindustrial, Common, cost 1), a card of the Stinkyscape+ set (R1420).
//!   Base:    "Flexible, Can't Defend, Ranged, Windfury"
//!   Radiant: "Flexible, Can't Defend, Ranged, Windfury, Pierce"
//!
//! Every word is a catalog keyword (§6.1), so there is no script to run. Flexible lets it stand in the
//! backrow and attack from there as a Unit (R3543000); Ranged takes no damage back (R783000); Windfury
//! gives two attacks a turn (R636); the Radiant face's Pierce ignores Armor (R346).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-036";

pub fn script() -> CardScripts {
    // Keywords only: catalog data on both faces.
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #36 Duo — SPEC §8.12 row 36 (R3543000; R346, R636, R783000).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// An enemy that would strike back hard, if it could.
    const BRUISER: &str = "stand-in (1) 5/40";
    const ARMORED: &str = "stand-in (1) 5/40 Armor 3";

    /// What each face hits for.
    const ATTACK: i32 = 2;
    const RADIANT_ATTACK: i32 = 4;
    const RADIANT_HEALTH: i32 = 2;
    /// Windfury's attacks a turn.
    const ATTACKS: i32 = 2;

    /// Duo standing in p1's backrow, `p2` for the enemy field.
    fn standing(radiant: bool, seed: &str, p2: Value) -> Scenario {
        weapon_board(
            seed,
            json!({ "field": [{ "def": ID, "radiant": radiant, "row": "backrow", "lane": 2 }] }),
            json!({ "field": p2 }),
        )
    }

    mod base {
        use super::*;

        #[test]
        fn r3543000_it_attacks_twice_from_the_backrow_and_takes_nothing_back() {
            let mut s = standing(false, "s036-base", json!([BRUISER]));
            let enemy = s.unit_at(P2, 1).id;
            assert!(s.backrow(P1, 2).is_some_and(|card| card.def_id == ID), "it stands in the backrow");
            for done in 1..=ATTACKS {
                s.attack(ID, enemy.as_str());
                assert_eq!(s.card(enemy.as_str()).damage, ATTACK * done);
                assert_eq!(s.card(ID).damage, 0, "Ranged: no strike back");
            }
            s.expect_refused(|s| s.attack(ID, enemy.as_str()));
            assert_eq!(s.card(enemy.as_str()).damage, ATTACK * ATTACKS, "no third attack landed");
        }

        #[test]
        fn r346_its_attack_does_not_pierce_armor() {
            let mut s = standing(false, "s036-armor", json!([ARMORED]));
            let enemy = s.unit_at(P2, 1).id;
            s.attack(ID, enemy.as_str());
            assert_eq!(s.card(enemy.as_str()).damage, 0, "Armor 3 takes all of 2");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r346_it_pierces_armor_and_still_attacks_twice() {
            let mut s = standing(true, "s036-radiant", json!([ARMORED]));
            let enemy = s.unit_at(P2, 1).id;
            for done in 1..=ATTACKS {
                s.attack(ID, enemy.as_str());
                assert_eq!(s.card(enemy.as_str()).damage, RADIANT_ATTACK * done, "Pierce: Armor 3 takes nothing");
                assert_eq!(s.card(ID).damage, 0, "Ranged: no strike back");
            }
            s.expect_refused(|s| s.attack(ID, enemy.as_str()));
        }

        #[test]
        fn its_health_is_doubled() {
            let s = standing(true, "s036-health", json!([]));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (RADIANT_ATTACK, RADIANT_HEALTH));
        }
    }
}
