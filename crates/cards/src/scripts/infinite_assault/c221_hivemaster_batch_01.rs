//! IA #221 Hivemaster Batch-01 (SPEC §8.13 row 221), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Aura: Your Drones have +{buff}/+{buff}. Start of turn: Fill your board with Drones."
//!            (2)
//!   Radiant: "Aura: Your Drones have +{buff}/+{buff}. Start of turn: Fill your board with Radiant
//!            Drones." (4)
//!
//! "Drone" is IA #4.1 (R4577004): the aura reads its controller's Units of that definition in the
//! unit row or standing in the backrow (R1081000), on either face. The Start of turn fills the
//! board with Drones, Radiant ones on the Radiant face (R64).
//!
//! Engine: an aura over a definition, `fillBoard` (R4594003). Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-221";

/// "Drone": IA #4.1.
const DRONE: &str = "infiniteassault-004-1";

/// "Your Drones have +N/+N": its controller's Drones on the field.
fn drones_aura() -> AuraHook {
    aura_hook(move |ctx| {
        let amount = param(&ctx, "buff");
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == ctx.self_.controller
                    && in_units_row_or_standing(ctx.state, unit)
                    && unit.def_id == DRONE
            }),
            mod_: StatMod {
                attack: Some(amount),
                max_health: Some(amount),
                ..StatMod::default()
            },
        }]
    })
}

fn hivemaster(radiant: bool) -> Script {
    Script {
        aura: Some(drones_aura()),
        start_of_turn: Some(hook(move |_| {
            vec![fill_board(json_as(json!({ "defId": DRONE, "radiant": radiant })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: hivemaster(false), radiant: hivemaster(true) }
}

// IA #221 Hivemaster Batch-01 — SPEC §8.13 row 221 (R4594003).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::{DRONE, ID};
    use crate::scenario;

    const ESCORT: &str = "infiniteassault-010-1"; // Drone Escort: a 3/3, no Drone.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn stats(s: &Scenario, player: PlayerId, lane: i32) -> (i32, i32) {
        let view = unit_view(s.state(), &s.unit_at(player, lane));
        (view.attack, view.health)
    }

    fn drones(s: &Scenario, player: PlayerId) -> usize {
        s.state().players[player]
            .units
            .iter()
            .flatten()
            .flatten()
            .filter(|card| card.def_id == DRONE)
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn r4594003_drones_are_3_3() {
            let s = scenario(json!({
                "seed": "hive-base",
                "p1": { "field": [{ "def": ID }, { "def": DRONE }, ESCORT], "hand": [STOCKPILE], "library": crate::filler(4) },
                "p2": { "field": [DRONE], "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            // Drone 1/1 with +2/+2.
            assert_eq!(stats(&s, P1, 2), (3, 3));
            // A Drone Escort is no Drone.
            assert_eq!(stats(&s, P1, 3), (3, 3));
            // The enemy's Drone is not yours.
            assert_eq!(stats(&s, P2, 1), (1, 1));
        }

        #[test]
        fn start_of_turn_fills_the_board() {
            let mut s = scenario(json!({
                "seed": "hive-fill",
                "active": "p1",
                "p1": { "field": [{ "def": ID }], "hand": [STOCKPILE], "library": crate::filler(4) },
                "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            s.start_turn();
            assert_eq!(drones(&s, P1), 4, "a full board less the Hivemaster");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4594003_radiant_drones_are_6_6() {
            let s = scenario(json!({
                "seed": "hive-radiant",
                "p1": { "field": [{ "def": ID, "radiant": true }, { "def": DRONE }], "hand": [STOCKPILE], "library": crate::filler(4) },
                "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            assert_eq!(stats(&s, P1, 2), (1 + 4, 1 + 4));
            let drone = s.unit_at(P1, 2);
            assert!(!drone.radiant, "the standing Drone is no Radiant one");
        }

        #[test]
        fn start_of_turn_fills_radiant_drones() {
            let mut s = scenario(json!({
                "seed": "hive-radiant-fill",
                "active": "p1",
                "p1": { "field": [{ "def": ID, "radiant": true }], "hand": [STOCKPILE], "library": crate::filler(4) },
                "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            s.start_turn();
            let set: Vec<CardInstance> = s.state().players[P1]
                .units
                .iter()
                .flatten()
                .flatten()
                .filter(|card| card.def_id == DRONE)
                .cloned()
                .collect();
            assert_eq!(set.len(), 4);
            assert!(set.iter().all(|card| card.radiant), "Radiant Drones");
        }
    }
}
