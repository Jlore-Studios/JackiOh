//! SPEC §8.13 IA #79 Thread Pool ((3) Unit, Common, 4/4 → 8/8), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Summon a Drone for each other Unit you control."
//!   Radiant: "Cry: Summon a Radiant Drone for each other Unit you control."
//!
//! One summon of IA #4.1 for every other Unit its controller controls as the Cry resolves, into
//! R64's leftmost free zones; alone it summons none, and a full board takes fewer (§3.2).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-079";

/// IA #4.1 Drone.
const DRONE: &str = "infiniteassault-004-1";

fn thread_pool(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // Itself excluded when it already stands among them: only the other Units count.
            let me = ctx.self_.as_ref().map(|me| me.id.clone());
            let others = active_units_of(ctx.state, ctx.controller)
                .into_iter()
                .filter(|unit| Some(&unit.id) != me.as_ref())
                .count();
            (0..others)
                .map(|_| {
                    let mut args = json!({ "defId": DRONE, "player": "self" });
                    if radiant {
                        args["radiant"] = json!(true);
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
        base: thread_pool(false),
        radiant: thread_pool(true),
    }
}

// IA #79 Thread Pool — SPEC §8.13 row 79.
#[cfg(test)]
mod tests {
    use super::{DRONE, ID};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";

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
        fn summons_a_drone_for_each_other_unit() {
            let (s, drones) = drones_after(false, json!([UNIT, UNIT]));
            assert_eq!(drones.len(), 2);
            for drone in nonempty(drones, "the Drones") {
                assert!(!drone.radiant);
                assert_eq!((s.stats(&drone).attack, s.stats(&drone).health), (1, 1));
            }
        }

        #[test]
        fn alone_it_summons_none() {
            let (_, drones) = drones_after(false, json!([]));
            assert!(drones.is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn summons_radiant_drones_for_each_other_unit() {
            let (s, drones) = drones_after(true, json!([UNIT]));
            assert_eq!(drones.len(), 1);
            for drone in nonempty(drones, "the Drones") {
                assert!(drone.radiant);
                assert_eq!((s.stats(&drone).attack, s.stats(&drone).health), (2, 2));
            }
        }
    }
}
