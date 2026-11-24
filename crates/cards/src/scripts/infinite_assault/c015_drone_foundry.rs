//! SPEC §8.13 IA #15 Drone Foundry ((2) Field Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Start of turn: Summon a Drone. Then, if you have {drones} or more Drones, give your Drones
//!             +{buff}/+{buff}." (drones 4, buff 1)
//!   Radiant: "Start of turn: Summon two Drones. Then, if you have {drones} or more Drones, give your Drones
//!             +{buff}/+{buff}." (drones 4, buff 1)
//!
//! The summons of IA #4.1, then C+ #20 Mushroom Power's `forEachCard`, whose list is read once they
//! have landed: the Drones its controller has on the field, IA #4.1 alone (R4577004), every one of
//! them when they number `drones` or more, none otherwise.

use jackioh_engine::effects::buff;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-015";

/// IA #4.1 Drone.
const DRONE: &str = "infiniteassault-004-1";

/// "a Drone", Radiant "two Drones".
const SUMMONS: usize = 1;
const RADIANT_SUMMONS: usize = 2;

/// R4577004: its controller's Drones on the field.
fn drones_of(ctx: &EffectContext<'_>) -> Vec<String> {
    active_units_of(ctx.state, ctx.controller)
        .into_iter()
        .filter(|unit| unit.def_id == DRONE)
        .map(|unit| unit.id.clone())
        .collect()
}

fn foundry(summons: usize) -> Script {
    Script {
        start_of_turn: Some(hook(move |ctx| {
            let (threshold, amount) = (param(&*ctx, "drones"), param(&*ctx, "buff"));
            let mut effects: Vec<Effect> = (0..summons)
                .map(|_| summon(json_as(json!({ "defId": DRONE, "player": "self" }))))
                .collect();
            effects.push(for_each_card(ForEachCardArgs {
                cards: Arc::new(move |each: &mut EffectContext<'_>| {
                    let drones = drones_of(each);
                    if i32::try_from(drones.len()).unwrap_or(i32::MAX) < threshold {
                        return vec![];
                    }
                    drones
                }),
                each: Arc::new(move |instance_id: &str| {
                    buff(json_as(json!({
                        "target": { "of": "instance", "instanceId": instance_id },
                        "attack": amount,
                        "health": amount,
                    })))
                }),
            }));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: foundry(SUMMONS),
        radiant: foundry(RADIANT_SUMMONS),
    }
}

// IA #15 Drone Foundry — SPEC §8.13 row 15 (R4577004).
#[cfg(test)]
mod tests {
    use super::{DRONE, ID};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const ESCORT: &str = "infiniteassault-010-1"; // Drone Escort

    /// The Foundry face-up in p1's backrow over `field`, and p1's turn started.
    fn turn_starts(radiant: bool, field: Value) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "field": field,
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": true, "lane": 1 }],
                "library": crate::filler(2),
            },
            "p2": { "library": crate::filler(2) },
        }));
        s.start_turn();
        s
    }

    fn drone_stats(s: &Scenario) -> Vec<(i32, i32)> {
        active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == DRONE)
            .map(|unit| (s.stats(unit).attack, s.stats(unit).health))
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn the_fourth_drone_lands_and_every_drone_gets_1_1() {
            let s = turn_starts(false, json!([DRONE, DRONE, DRONE]));
            assert_eq!(drone_stats(&s), [(2, 2); 4]);
        }

        #[test]
        fn three_drones_after_the_summon_get_nothing() {
            let s = turn_starts(false, json!([DRONE, DRONE]));
            assert_eq!(drone_stats(&s), [(1, 1); 3]);
        }

        #[test]
        fn r4577004_a_drone_escort_is_no_drone() {
            let s = turn_starts(false, json!([ESCORT, ESCORT, ESCORT]));
            assert_eq!(drone_stats(&s), [(1, 1)]);
            let escorts: Vec<(i32, i32)> = active_units_of(s.state(), P1)
                .into_iter()
                .filter(|unit| unit.def_id == ESCORT)
                .map(|unit| (s.stats(unit).attack, s.stats(unit).health))
                .collect();
            assert_eq!(escorts, [(3, 3); 3]);
        }

        #[test]
        fn r68_none_at_the_opponents_start_of_turn() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "field": [DRONE, DRONE, DRONE], "backrow": [{ "def": ID, "faceUp": true, "lane": 1 }] },
                "p2": { "library": crate::filler(2) },
            }));
            s.start_turn();
            assert_eq!(drone_stats(&s), [(1, 1); 3]);
            assert!(active_units_of(s.state(), P2).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn two_drones_land_and_the_fourth_sets_off_the_buff() {
            let s = turn_starts(true, json!([DRONE, DRONE]));
            assert_eq!(drone_stats(&s), [(2, 2); 4]);
        }

        #[test]
        fn one_drone_and_two_summoned_get_nothing() {
            let s = turn_starts(true, json!([DRONE]));
            assert_eq!(drone_stats(&s), [(1, 1); 3]);
        }
    }
}
