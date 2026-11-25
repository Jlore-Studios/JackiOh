//! SPEC §8.13 IA #7 Bulk Order ((2) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Summon three Drones."
//!   Radiant: "Summon three Radiant Drones with Rush."
//!
//! Three summons of IA #4.1, each into R64's leftmost free zone; a full board takes fewer (§3.2). The
//! Radiant face's Drones are Radiant and given Rush as they land (S T-AI-8 Drone Uplink's `keywords`).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-007";

/// IA #4.1 Drone.
const DRONE: &str = "infiniteassault-004-1";

/// "three Drones".
const DRONES: usize = 3;

fn bulk_order(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            (0..DRONES)
                .map(|_| {
                    let mut args = json!({ "defId": DRONE, "player": "self" });
                    if radiant {
                        args["radiant"] = json!(true);
                        args["keywords"] = json!([{ "kind": "Rush" }]);
                    }
                    summon(json_as(args))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: bulk_order(false),
        radiant: bulk_order(true),
    }
}

// IA #7 Bulk Order — SPEC §8.13 row 7.
#[cfg(test)]
mod tests {
    use super::{DRONE, ID};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn drones_after(radiant: bool, field: Value) -> (Scenario, Vec<CardInstance>) {
        let mut s = scenario(json!({ "p1": { "hand": [{ "def": ID, "radiant": radiant }], "field": field } }));
        s.play(ID, json!({}));
        let drones = active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == DRONE)
            .cloned()
            .collect();
        (s, drones)
    }

    mod base {
        use super::*;

        #[test]
        fn summons_three_base_drones_without_rush() {
            let (s, drones) = drones_after(false, json!([]));
            assert_eq!(drones.len(), 3);
            for drone in nonempty(drones, "the Drones") {
                assert!(!drone.radiant);
                assert!(!has_keyword(&s.stats(&drone).keywords, KeywordKind::Rush));
                assert_eq!((s.stats(&drone).attack, s.stats(&drone).health), (1, 1));
            }
        }

        #[test]
        fn s3_2_a_fuller_board_takes_fewer() {
            let unit = "stand-in (1) 1/1";
            let (_, drones) = drones_after(false, json!([unit, unit, unit]));
            assert_eq!(drones.len(), (UNIT_ZONES - 3) as usize);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn summons_three_radiant_drones_with_rush() {
            let (s, drones) = drones_after(true, json!([]));
            assert_eq!(drones.len(), 3);
            for drone in nonempty(drones, "the Drones") {
                assert!(drone.radiant);
                assert!(has_keyword(&s.stats(&drone).keywords, KeywordKind::Rush));
                assert_eq!((s.stats(&drone).attack, s.stats(&drone).health), (2, 2));
            }
        }
    }
}
