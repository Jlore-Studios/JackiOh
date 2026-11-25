//! SPEC §8.13 IA #89 Queen of the Cluster ((4) Unit, Epic, 6/8 → 12/16), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Aura: Your other Units have +{buff}/+{buff}.\nEnd of turn: Summon a Drone." (buff 2)
//!   Radiant: "Aura: Your other Units have +{buff}/+{buff}.\nEnd of turn: Summon a Radiant Drone." (buff 4)
//!
//! IA #75 Load Balancer's aura over its controller's other Units, both stats at once, with IA #4 Wave
//! Spawner's end-of-turn summon of IA #4.1 beside it.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-089";

/// IA #4.1 Drone.
const DRONE: &str = "infiniteassault-004-1";

/// "Your other Units have +N/+N".
fn other_units_aura() -> AuraHook {
    aura_hook(move |ctx| {
        let amount = param(&ctx, "buff");
        let controller = ctx.self_.controller;
        let state = ctx.state;
        let me = ctx.self_.id.clone();
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == controller && in_units_row_or_standing(state, unit) && unit.id != me
            }),
            mod_: StatMod {
                attack: Some(amount),
                max_health: Some(amount),
                ..StatMod::default()
            },
        }]
    })
}

fn queen_of_the_cluster(radiant: bool) -> Script {
    Script {
        aura: Some(other_units_aura()),
        end_of_turn: Some(hook(move |_ctx| {
            let mut args = json!({ "defId": DRONE, "player": "self" });
            if radiant {
                args["radiant"] = json!(true);
            }
            vec![summon(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: queen_of_the_cluster(false),
        radiant: queen_of_the_cluster(true),
    }
}

// IA #89 Queen of the Cluster — SPEC §8.13 row 89.
#[cfg(test)]
mod tests {
    use super::{DRONE, ID};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";
    const HIT_JOB: &str = "core-016"; // Hit Job: the kill

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [HIT_JOB],
                "field": [{ "def": ID, "radiant": radiant }, UNIT],
                "mana": 10,
                "library": crate::filler(2),
            },
            "p2": { "field": [UNIT], "library": crate::filler(2) },
        }))
    }

    fn drones(s: &Scenario) -> Vec<CardInstance> {
        active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == DRONE)
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn your_other_units_have_2_2_and_no_other_unit() {
            let mut s = board(false);
            let (other, enemy) = (s.unit_at(P1, 2), s.unit_at(P2, 1));
            s.expect_stats(&other, json!({ "attack": 4, "health": 4 }));
            s.expect_stats(&enemy, json!({ "attack": 2, "health": 2 }));
            let queen = s.unit_at(P1, 1);
            s.expect_stats(&queen, json!({ "attack": 6, "health": 8 }));
        }

        #[test]
        fn the_aura_ends_when_the_queen_leaves() {
            let mut s = board(false);
            let queen = s.unit_at(P1, 1);
            let other = s.unit_at(P1, 2);
            s.play(HIT_JOB, pick::targets(pick::instance(&queen)));
            s.expect_stats(&other, json!({ "attack": 2, "health": 2 }));
        }

        #[test]
        fn a_drone_lands_at_its_end() {
            let mut s = board(false);
            assert!(drones(&s).is_empty(), "not before the end of turn");
            s.end_turn();
            let landed = drones(&s);
            assert_eq!(landed.len(), 1);
            assert!(!landed[0].radiant);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn your_other_units_have_4_4_and_a_radiant_drone_lands() {
            let mut s = board(true);
            let other = s.unit_at(P1, 2);
            s.expect_stats(&other, json!({ "attack": 6, "health": 6 }));
            s.end_turn();
            assert_eq!(drones(&s).len(), 1);
            assert!(drones(&s)[0].radiant);
        }
    }
}
