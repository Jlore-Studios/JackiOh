//! IA #222 Swarm Uplink (SPEC §8.13 row 222), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Summon three Drones. Give your Units +{buff}/+{buff}." (2)
//!   Radiant: "Summon three Radiant Drones. Give your Units +{buff}/+{buff}." (4)
//!
//! "Drone" is IA #4.1 (R4577004): three fresh summons, Radiant ones on the Radiant face, then the
//! buff over its controller's Units — the new Drones among them (R4594003). A crowded board
//! summons what fits. The Prime Generator (IA #235), Immune to Spells, is passed by the Spell's
//! buff (R4453000).
//!
//! Engine: `summon`, `buffAllUnits` (R4594003). Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-222";

/// "Drone": IA #4.1.
const DRONE: &str = "infiniteassault-004-1";

/// Three Drones.
const DRONES: i32 = 3;

fn uplink(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let buff = param(&*ctx, "buff");
            let mut effects: Vec<Effect> = (0..DRONES)
                .map(|_| {
                    summon(json_as(json!({ "defId": DRONE, "player": "self", "radiant": radiant })))
                })
                .collect();
            effects.push(buff_all_units(json_as(json!({
                "side": "self",
                "attack": buff,
                "health": buff,
            }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: uplink(false), radiant: uplink(true) }
}

// IA #222 Swarm Uplink — SPEC §8.13 row 222 (R4594003).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::{DRONE, ID};
    use crate::scenario;

    const PRIME: &str = "infiniteassault-235"; // The Prime Generator: a Commander, no Drone.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn summon_stats(s: &Scenario) -> Vec<(i32, i32, bool)> {
        s.state().players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .filter(|card| card.def_id == DRONE)
            .map(|card| {
                let view = unit_view(s.state(), card);
                (view.attack, view.health, card.radiant)
            })
            .collect()
    }

    fn played(radiant: bool, seed: &str, field: Value) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE], "field": field, "library": crate::filler(4) },
            "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
        }));
        s.play(ID, json!({}));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r4594003_three_drones_land_3_3() {
            let s = played(false, "uplink-base", json!([]));
            let drones = summon_stats(&s);
            assert_eq!(drones.len(), 3, "three Drones");
            assert!(drones.iter().all(|stats| *stats == (1 + 2, 1 + 2, false)));
        }

        #[test]
        fn r4594003_a_crowded_board_summons_what_fits() {
            let s = played(
                false,
                "uplink-crowded",
                json!([{ "def": PRIME }, { "def": PRIME }, { "def": PRIME }, { "def": PRIME }]),
            );
            // One zone open: one Drone, buffed.
            let drones = summon_stats(&s);
            assert_eq!(drones.len(), 1, "one Drone fits");
            assert_eq!(drones[0], (3, 3, false));
            // The Prime Generators, Immune to Spells, are passed by the Spell's buff.
            let primes: Vec<(i32, i32)> = s.state().players[P1]
                .units
                .iter()
                .flatten()
                .flatten()
                .filter(|card| card.def_id == PRIME)
                .map(|card| {
                    let view = unit_view(s.state(), card);
                    (view.attack, view.health)
                })
                .collect();
            assert_eq!(primes.len(), 4);
            assert!(primes.iter().all(|stats| *stats == (8, 12)));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4594003_three_radiant_drones_land_5_5() {
            let s = played(true, "uplink-radiant", json!([]));
            let drones = summon_stats(&s);
            assert_eq!(drones.len(), 3, "three Radiant Drones");
            assert!(drones.iter().all(|stats| *stats == (2 + 4, 2 + 4, true)));
        }
    }
}
