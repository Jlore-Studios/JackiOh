//! SPEC §8.12 S+ #107 Fuse (Unit 2/5 → 4/10, Mindustrial, Epic, cost 2), a card of the Stinkyscape+ set (R1420).
//!   Base:    "Armor 5, Can't Defend, Ranged, Pierce
//!            Deal {sweepLane}x damage to Units in this lane, deals {sweepAdjacent}x damage to Units in
//!            adjacent lanes." — sweepLane 5, sweepAdjacent 3
//!   Radiant: "Armor 10, Can't Defend, Ranged, Pierce
//!            Deal {sweepLane}x damage to Units in this lane, deals {sweepAdjacent}x damage to Units in
//!            adjacent lanes." — sweepLane 7, sweepAdjacent 4
//!
//! Its script file and the card's script name say `fuse_unit`, to keep it apart from the Fuse verb (§6.3); its
//! catalog id is the foundation's, `stinkyscapeplus-107`.
//!
//! Engine: the keywords are catalog data; Ranged takes it no damage back (R783000) and Pierce ignores Armor
//! (R346). The weights are the static flags `sweepLane` and `sweepAdjacent` (R3554008): its attack hits every
//! enemy Unit in its lane for `sweepLane` times its attack and every enemy Unit in the adjacent lanes for
//! `sweepAdjacent` times, as one damage instance each; a declared target outside the sweep takes a normal
//! hit. The weights read the card's declared numbers first (R386), so an Upgrade of either lifts the sweep and
//! the projections agree with combat. Both faces share the script's shape; the flags carry each face's number.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-107";

/// "Deal 5x damage to Units in this lane, 3x to Units in adjacent lanes": the catalog's `sweepLane` and
/// `sweepAdjacent` (R386).
const SWEEP_LANE: i32 = 5;
const SWEEP_ADJACENT: i32 = 3;
/// The Radiant face: 7x and 4x.
const RADIANT_SWEEP_LANE: i32 = 7;
const RADIANT_SWEEP_ADJACENT: i32 = 4;

/// R3554008: a face's two weights.
fn sweep(lane: i32, adjacent: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            sweep_lane: Some(lane),
            sweep_adjacent: Some(adjacent),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: sweep(SWEEP_LANE, SWEEP_ADJACENT),
        radiant: sweep(RADIANT_SWEEP_LANE, RADIANT_SWEEP_ADJACENT),
    }
}

// S+ #107 Fuse — SPEC §8.12 row 107 (R3554008; R346, R386, R783000).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{upgrade_number, weapon_board};
    use jackioh_engine::testkit::*;

    const P2: PlayerId = PlayerId::P2;

    /// A target no sweep brings down, and one whose Armor would stop a lot of it.
    const TOUGH: &str = "stand-in (1) 0/60";
    const ARMORED: &str = "stand-in (1) 0/60 Armor 4";

    /// What each face hits for before its weight, and its health and Armor.
    const ATTACK: i32 = 2;
    const RADIANT_ATTACK: i32 = 4;
    const HEALTH: i32 = 5;
    const RADIANT_HEALTH: i32 = 10;

    /// Fuse in p1's lane 3, p2's Units in lanes 1 to 5 as given.
    fn board(radiant: bool, seed: &str, p2: Value) -> Scenario {
        weapon_board(
            seed,
            json!({ "field": [{ "def": ID, "radiant": radiant, "lane": 3 }] }),
            json!({ "field": p2 }),
        )
    }

    fn five(def: &str) -> Value {
        json!([def, def, def, def, def])
    }

    /// p2's damage in lanes 1 to 5.
    fn lanes(s: &Scenario) -> Vec<i32> {
        (1..=5).map(|lane| s.unit_at(P2, lane).damage).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3554008_it_sweeps_its_lane_five_times_and_beside_three_times() {
            let mut s = board(false, "s107-base", five(TOUGH));
            let target = s.unit_at(P2, 3).id;
            s.attack(ID, target.as_str());
            assert_eq!(
                lanes(&s),
                vec![
                    0,
                    SWEEP_ADJACENT * ATTACK,
                    SWEEP_LANE * ATTACK,
                    SWEEP_ADJACENT * ATTACK,
                    0
                ],
                "10 in its lane, 6 beside it, none two lanes off"
            );
            assert_eq!(s.card(ID).damage, 0, "Ranged: no strike back");
        }

        #[test]
        fn r3554008_the_sweep_pierces_armor() {
            let mut s = board(false, "s107-armor", json!([TOUGH, TOUGH, TOUGH, ARMORED, TOUGH]));
            let target = s.unit_at(P2, 3).id;
            s.attack(ID, target.as_str());
            assert_eq!(s.unit_at(P2, 4).damage, SWEEP_ADJACENT * ATTACK, "Armor 4 took nothing of it");
        }

        #[test]
        fn r3554008_a_declared_target_outside_the_sweep_takes_a_normal_hit() {
            // Lane 1 is two lanes from Fuse's lane 3: it takes the plain attack, and the sweep still lands.
            let mut s = board(false, "s107-outside", five(TOUGH));
            let target = s.unit_at(P2, 1).id;
            s.attack(ID, target.as_str());
            assert_eq!(
                lanes(&s),
                vec![ATTACK, SWEEP_ADJACENT * ATTACK, SWEEP_LANE * ATTACK, SWEEP_ADJACENT * ATTACK, 0],
                "a normal hit on the target, the sweep on lanes 2 to 4"
            );
        }

        #[test]
        fn r3554008_an_attack_on_the_hero_still_sweeps_and_the_hero_takes_a_normal_hit() {
            let mut s = board(false, "s107-hero", five(TOUGH));
            s.attack(ID, "hero");
            s.expect_health(P2, HERO_HEALTH - ATTACK);
            assert_eq!(
                lanes(&s),
                vec![0, SWEEP_ADJACENT * ATTACK, SWEEP_LANE * ATTACK, SWEEP_ADJACENT * ATTACK, 0]
            );
        }

        #[test]
        fn r386_an_upgrade_of_its_lane_weight_lifts_the_sweep() {
            let mut s = board(false, "s107-upgrade", five(TOUGH));
            let target = s.unit_at(P2, 3).id;
            let weight = upgrade_number(&mut s, ID, "sweepLane");
            assert_eq!(weight, SWEEP_LANE + 1);
            s.attack(ID, target.as_str());
            assert_eq!(s.unit_at(P2, 3).damage, weight * ATTACK);
            assert_eq!(s.unit_at(P2, 2).damage, SWEEP_ADJACENT * ATTACK, "the adjacent weight is as it was");
        }

        #[test]
        fn its_armor_and_health_are_printed() {
            let s = board(false, "s107-stats", json!([]));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (ATTACK, HEALTH));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3554008_it_sweeps_its_lane_seven_times_and_beside_four_times() {
            let mut s = board(true, "s107-radiant", five(TOUGH));
            let target = s.unit_at(P2, 3).id;
            s.attack(ID, target.as_str());
            assert_eq!(
                lanes(&s),
                vec![
                    0,
                    RADIANT_SWEEP_ADJACENT * RADIANT_ATTACK,
                    RADIANT_SWEEP_LANE * RADIANT_ATTACK,
                    RADIANT_SWEEP_ADJACENT * RADIANT_ATTACK,
                    0
                ],
                "28 in its lane, 16 beside it"
            );
        }

        #[test]
        fn r3554008_the_radiant_sweep_pierces_armor_too() {
            let mut s = board(true, "s107-radiant-armor", json!([ARMORED, ARMORED, ARMORED, ARMORED, ARMORED]));
            let target = s.unit_at(P2, 3).id;
            s.attack(ID, target.as_str());
            assert_eq!(s.unit_at(P2, 3).damage, RADIANT_SWEEP_LANE * RADIANT_ATTACK);
            assert_eq!(s.unit_at(P2, 2).damage, RADIANT_SWEEP_ADJACENT * RADIANT_ATTACK);
        }

        #[test]
        fn its_stats_are_doubled() {
            let s = board(true, "s107-radiant-stats", json!([]));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (RADIANT_ATTACK, RADIANT_HEALTH));
        }
    }
}
