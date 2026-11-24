//! SPEC §8.13 IA #74 Parallel Instance ((2) Unit, Common, 2/2 → 4/4), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Summon an Instance."
//!   Radiant: "Cry: Summon a Radiant Instance."
//!
//! One summon of IA #74.1 into R64's leftmost free zone; the Radiant face's is Radiant (a 4/4).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-074";

/// IA #74.1 Instance.
const INSTANCE: &str = "infiniteassault-074-1";

fn parallel_instance(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            let mut args = json!({ "defId": INSTANCE, "player": "self" });
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
        base: parallel_instance(false),
        radiant: parallel_instance(true),
    }
}

// IA #74 Parallel Instance — SPEC §8.13 row 74.
#[cfg(test)]
mod tests {
    use super::{ID, INSTANCE};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn instances_after(radiant: bool) -> (Scenario, Vec<CardInstance>) {
        let mut s = scenario(json!({ "p1": { "hand": [{ "def": ID, "radiant": radiant }] } }));
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
        fn summons_one_base_instance() {
            let (s, instances) = instances_after(false);
            assert_eq!(instances.len(), 1);
            for instance in nonempty(instances, "the Instance") {
                assert!(!instance.radiant);
                assert_eq!((s.stats(&instance).attack, s.stats(&instance).health), (2, 2));
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn summons_one_radiant_instance() {
            let (s, instances) = instances_after(true);
            assert_eq!(instances.len(), 1);
            for instance in nonempty(instances, "the Instance") {
                assert!(instance.radiant);
                assert_eq!((s.stats(&instance).attack, s.stats(&instance).health), (4, 4));
            }
        }
    }
}
