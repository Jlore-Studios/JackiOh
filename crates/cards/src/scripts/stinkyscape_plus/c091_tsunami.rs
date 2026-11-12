//! SPEC §8.12 S+ #91 Tsunami (Unit 3/6 → 6/12, Mindustrial, Rare, cost 2), a card of the Stinkyscape+ set (R1420).
//!   Base:    "Flexible, Ranged, Can't Defend
//!            Freeze any Unit damaged by this."
//!   Radiant: "Flexible, Ranged, Can't Defend, Cleave
//!            Freeze any Unit damaged by this."
//!
//! Engine: the keywords are catalog data (Flexible, R3543000: it attacks from the backrow as a Unit). "Freeze
//! any Unit damaged by this" is the static flag `freezesDamaged` (R3550002): a Unit dealt 1 or more by one of
//! this card's hits is Frozen at §4.4 step 7, never a hero; a hit Armor or a Divine Shield took whole freezes
//! nothing. The Radiant face's Cleave hits the Units beside the target, and each of them is Frozen as it takes
//! its damage. Both faces share the script.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-091";

/// R3550002: the one flag this card's text is.
fn tsunami() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            freezes_damaged: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: tsunami(),
        radiant: tsunami(),
    }
}

// S+ #91 Tsunami — SPEC §8.12 row 91 (R3550002; R3543000, R783000).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TOUGH: &str = "stand-in (1) 0/40";
    const ARMORED: &str = "stand-in (1) 0/40 Armor 9";

    /// What each face hits for, and its Radiant health.
    const ATTACK: i32 = 3;
    const RADIANT_ATTACK: i32 = 6;
    const RADIANT_HEALTH: i32 = 12;

    /// Tsunami in p1's unit row, `p2` for the enemy field.
    fn board(radiant: bool, seed: &str, p2: Value) -> Scenario {
        weapon_board(
            seed,
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            json!({ "field": p2 }),
        )
    }

    fn frozen(s: &Scenario, id: &str) -> bool {
        s.card(id).frozen.is_some()
    }

    mod base {
        use super::*;

        #[test]
        fn r3550002_the_unit_it_damages_is_frozen() {
            let mut s = board(false, "s091-base", json!([TOUGH, TOUGH]));
            let (target, other) = (s.unit_at(P2, 1).id, s.unit_at(P2, 2).id);
            assert!(!frozen(&s, &target));
            s.attack(ID, target.as_str());
            assert_eq!(s.card(target.as_str()).damage, ATTACK);
            assert!(frozen(&s, &target), "the Unit it damaged is Frozen");
            assert!(!frozen(&s, &other), "a Unit it did not damage is not");
        }

        #[test]
        fn r3550002_attacking_the_hero_freezes_nothing() {
            let mut s = board(false, "s091-hero", json!([TOUGH]));
            let unit = s.unit_at(P2, 1).id;
            s.attack(ID, "hero");
            s.expect_health(P2, HERO_HEALTH - ATTACK);
            assert!(!frozen(&s, &unit));
            assert!(
                !s.last_events().iter().any(|event| matches!(event, GameEvent::Marked { .. })),
                "no Frozen mark was placed anywhere"
            );
        }

        #[test]
        fn r3550002_a_hit_the_armor_took_whole_freezes_nothing() {
            let mut s = board(false, "s091-armor", json!([ARMORED]));
            let target = s.unit_at(P2, 1).id;
            s.attack(ID, target.as_str());
            assert_eq!(s.card(target.as_str()).damage, 0, "Armor 9 takes all of 3");
            assert!(!frozen(&s, &target), "a Unit dealt nothing is not Frozen");
        }

        #[test]
        fn r3543000_it_attacks_from_the_backrow() {
            let mut s = weapon_board(
                "s091-backrow",
                json!({ "field": [{ "def": ID, "row": "backrow", "lane": 2 }] }),
                json!({ "field": [TOUGH] }),
            );
            let target = s.unit_at(P2, 1).id;
            assert!(s.backrow(P1, 2).is_some_and(|card| card.def_id == ID), "it stands in the backrow");
            s.attack(ID, target.as_str());
            assert_eq!(s.card(target.as_str()).damage, ATTACK);
            assert!(frozen(&s, &target));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3550002_its_cleave_freezes_each_unit_beside_the_target() {
            let mut s = board(true, "s091-radiant", json!([TOUGH, TOUGH, TOUGH, TOUGH]));
            let lanes: Vec<String> = (1..=4).map(|lane| s.unit_at(P2, lane).id).collect();
            s.attack(ID, lanes[1].as_str());
            for (at, id) in lanes.iter().enumerate() {
                let hit = at <= 2;
                assert_eq!(s.card(id.as_str()).damage, if hit { RADIANT_ATTACK } else { 0 }, "lane {}", at + 1);
                assert_eq!(frozen(&s, id), hit, "lane {}", at + 1);
            }
        }

        #[test]
        fn its_health_is_doubled() {
            let s = board(true, "s091-health", json!([]));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (RADIANT_ATTACK, RADIANT_HEALTH));
        }
    }
}
