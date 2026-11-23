//! SPEC §8.12 S+ #43 Segment (R3550005; R121, R1420):
//! (2) Field Trap, Mindustrial, Rare, a card of the Stinkyscape+ set.
//!   Base:    "The first enemy Ranged attack or Activate per turn is countered."
//!   Radiant: "All enemy Ranged attack or Activate are countered."
//!
//! The E6 counter (R3550005): a declared Ranged attack is cancelled before §4.2 step 4's trap
//! window and an `activate` action does nothing, the Activate's use and costs staying spent; one
//! budget a turn shared by both, and a forced attack is never countered (R121). The base face
//! counters the first one a turn, the Radiant face every one. A Field Trap set face-down, no script
//! beyond its flags.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-043";

/// R3550005: how many a turn the base face counters.
const COUNTERED_PER_TURN: i32 = 1;

fn segment(radiant: bool) -> Script {
    Script {
        static_flags: Some(if radiant {
            StaticFlags {
                counters_ranged_all: Some(true),
                ..StaticFlags::default()
            }
        } else {
            StaticFlags {
                counters_ranged: Some(COUNTERED_PER_TURN),
                ..StaticFlags::default()
            }
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: segment(false),
        radiant: segment(true),
    }
}

// S+ #43 Segment — SPEC §8.12 row S+ 43 (R3550005; R121).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A 3-attack Ranged Unit (Stinkyscape #68 Stratos).
    const STRATOS: &str = "stinkyscape-068";
    /// S+ #143 Fire Beam v3: a Unit with an Activate.
    const ZAPPER: &str = "stinkyscapeplus-143";

    /// p1 holds the Segment; `"mana": 10` sets it from hand.
    fn segmented(radiant: bool, seed: &str, p1_field: Value, p2_field: Value) -> Scenario {
        let mut s = weapon_board(
            seed,
            json!({
                "mana": 10,
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": p1_field,
                "library": filler(6),
            }),
            json!({
                "field": p2_field,
                "library": filler(6),
            }),
        );
        // A Trap played from hand is set face-down.
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn trap_fired_then_cancelled(s: &Scenario) -> bool {
        let mut kinds = s.last_events().iter().filter_map(|event| match event {
            GameEvent::TrapFired { .. } => Some(0),
            GameEvent::AttackCancelled { .. } => Some(1),
            _ => None,
        });
        matches!((kinds.next(), kinds.next()), (Some(0), Some(1)))
    }

    /// p2's zapper burns p1's body — countered, so the body never Burns.
    fn activate_zapper(s: &mut Scenario, zapper: &str, body: &str) {
        s.activate(zapper, json!({ "ability": "burn", "targets": pick::id(body) }));
    }

    mod base {
        use super::*;

        #[test]
        fn r3550005_base_counters_the_first_enemy_ranged_attack_a_turn_only() {
            let mut s = segmented(false, "s43-ranged", json!([STRATOS]), json!([STRATOS, STRATOS]));
            // p2's Ranged attacks at p1's hero are the enemy's to the Segment.
            s.end_turn();
            let first = s.unit(P2, 1).expect("a stratos there").id;
            let second = s.unit(P2, 2).expect("a stratos there").id;
            s.attack(first.as_str(), "hero");
            assert!(trap_fired_then_cancelled(&s), "trapFired, then attackCancelled");
            s.expect_health(P1, HERO_HEALTH);
            // The budget spent, the second attack lands.
            s.attack(second.as_str(), "hero");
            s.expect_health(P1, HERO_HEALTH - 3);
        }

        #[test]
        fn r3550005_it_counters_an_enemy_activate() {
            const BODY: &str = "stand-in (1) 4/4 R12/12";
            let mut s = segmented(false, "s43-activate", json!([BODY]), json!([ZAPPER]));
            s.end_turn();
            let zapper = s.unit(P2, 1).expect("a zapper there").id;
            let body = s.unit(P1, 1).expect("a body there").id;
            activate_zapper(&mut s, zapper.as_str(), body.as_str());
            // The Activate did nothing: the body never Burns.
            assert_eq!(jackioh_engine::query::burn_on(s.card(body.as_str())), 0);
            assert!(
                s.last_events().iter().any(|event| matches!(event, GameEvent::TrapFired { .. })),
                "the counter announces itself"
            );
        }

        #[test]
        fn r3550005_never_its_controllers_own() {
            let mut s = segmented(false, "s43-own", json!([STRATOS, STRATOS]), json!([]));
            let first = s.unit(P1, 1).expect("a stratos there").id;
            s.attack(first.as_str(), "hero");
            assert!(
                !s.last_events().iter().any(|event| matches!(event, GameEvent::TrapFired { .. })),
                "its controller's own Ranged attack is never countered"
            );
            s.expect_health(P2, HERO_HEALTH - 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3550005_radiant_counters_every_one() {
            let mut s = segmented(true, "s43-all", json!([]), json!([STRATOS, STRATOS]));
            s.end_turn();
            for lane in 1..=2 {
                let stratos = s.unit(P2, lane).expect("a stratos there").id;
                s.attack(stratos.as_str(), "hero");
                assert!(trap_fired_then_cancelled(&s), "attack {lane} countered");
            }
            s.expect_health(P1, HERO_HEALTH);
        }
    }
}
