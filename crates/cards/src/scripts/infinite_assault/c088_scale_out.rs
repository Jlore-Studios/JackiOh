//! SPEC §8.13 IA #88 Scale Out ((2) Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Summon an Instance for each enemy Unit."
//!   Radiant: "Summon a Radiant Instance for each enemy Unit."
//!
//! One summon of IA #74.1 for every enemy Unit as the cry resolves, into R64's leftmost free zones;
//! with no enemy Unit nothing is summoned, and a full board takes fewer (§3.2).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-088";

/// IA #74.1 Instance.
const INSTANCE: &str = "infiniteassault-074-1";

fn scale_out(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let enemies = active_units_of(ctx.state, opponent_of(ctx.controller)).len();
            (0..enemies)
                .map(|_| {
                    let mut args = json!({ "defId": INSTANCE, "player": "self" });
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
        base: scale_out(false),
        radiant: scale_out(true),
    }
}

// IA #88 Scale Out — SPEC §8.13 row 88.
#[cfg(test)]
mod tests {
    use super::{ID, INSTANCE};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";

    fn instances_after(radiant: bool, p2_field: Value) -> (Scenario, Vec<CardInstance>) {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }] },
            "p2": { "field": p2_field },
        }));
        s.play(ID, json!({}));
        let instances = active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == INSTANCE)
            .cloned()
            .collect();
        (s, instances)
    }

    mod base {
        use super::*;

        #[test]
        fn summons_an_instance_for_each_enemy_unit() {
            let (s, instances) = instances_after(false, json!([UNIT, UNIT, UNIT]));
            assert_eq!(instances.len(), 3);
            for instance in nonempty(instances, "the Instances") {
                assert!(!instance.radiant);
                assert_eq!((s.stats(&instance).attack, s.stats(&instance).health), (2, 2));
            }
        }

        #[test]
        fn with_no_enemy_unit_nothing_is_summoned() {
            let (s, instances) = instances_after(false, json!([]));
            assert!(instances.is_empty());
            assert!(active_units_of(s.state(), P1).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn summons_a_radiant_instance_for_each_enemy_unit() {
            let (s, instances) = instances_after(true, json!([UNIT, UNIT]));
            assert_eq!(instances.len(), 2);
            for instance in nonempty(instances, "the Instances") {
                assert!(instance.radiant);
                assert_eq!((s.stats(&instance).attack, s.stats(&instance).health), (4, 4));
            }
        }
    }
}
