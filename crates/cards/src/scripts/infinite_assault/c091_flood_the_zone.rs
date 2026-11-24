//! SPEC §8.13 IA #91 Flood the Zone ((3) Spell, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Fill your board with Drones. They have Rush."
//!   Radiant: "Fill your board with Radiant Drones. They have Rush."
//!
//! One summon of IA #4.1 into every open unit zone (`fill_board_zones`), Radiant on the Radiant face,
//! each given Rush as it lands (S T-AI-8 Drone Uplink's `keywords`, as IA #7's are).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-091";

/// IA #4.1 Drone.
const DRONE: &str = "infiniteassault-004-1";

fn flood_the_zone(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            fill_board_zones(ctx.state, ctx.controller)
                .iter()
                .map(|zone| {
                    let mut args = json!({ "defId": DRONE, "player": "self", "lane": zone.lane });
                    if radiant {
                        args["radiant"] = json!(true);
                    }
                    args["keywords"] = json!([{ "kind": "Rush" }]);
                    summon(json_as(args))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: flood_the_zone(false),
        radiant: flood_the_zone(true),
    }
}

// IA #91 Flood the Zone — SPEC §8.13 row 91.
#[cfg(test)]
mod tests {
    use super::{DRONE, ID};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";

    fn after_play(radiant: bool) -> (Scenario, Vec<CardInstance>) {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "field": [UNIT], "mana": 10 },
            "p2": { "library": crate::filler(2) },
        }));
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
        fn the_board_fills_with_rush_drones() {
            let (s, drones) = after_play(false);
            assert_eq!(active_units_of(s.state(), P1).len(), UNIT_ZONES as usize);
            assert_eq!(drones.len(), (UNIT_ZONES - 1) as usize);
            for drone in nonempty(drones, "the Drones") {
                assert!(!drone.radiant);
                assert!(has_keyword(&s.stats(&drone).keywords, KeywordKind::Rush));
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_board_fills_with_radiant_rush_drones() {
            let (s, drones) = after_play(true);
            assert_eq!(active_units_of(s.state(), P1).len(), UNIT_ZONES as usize);
            assert_eq!(drones.len(), (UNIT_ZONES - 1) as usize);
            for drone in nonempty(drones, "the Drones") {
                assert!(drone.radiant);
                assert!(has_keyword(&s.stats(&drone).keywords, KeywordKind::Rush));
            }
        }
    }
}
