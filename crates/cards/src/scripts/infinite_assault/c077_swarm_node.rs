//! SPEC §8.13 IA #77 Swarm Node ((1) Unit, Common, 2/3 → 4/6), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Death: Give a random friendly Unit +{buff}/+{buff}." (buff 2)
//!   Radiant: "Death: Give a random friendly Unit +{buff}/+{buff}." (buff 4)
//!
//! A `buffRandomUnit` over its controller's side from its Death: one Unit still standing gets the
//! declared buff as layer-4 stats (§10.4); with none standing nothing happens (R129).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-077";

pub fn script() -> CardScripts {
    let base = Script {
        death: Some(hook(|ctx| {
            let amount = param(&*ctx, "buff");
            vec![buff_random_unit(json_as(json!({ "attack": amount, "health": amount })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's +4/+4 is its declared `buff`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #77 Swarm Node — SPEC §8.13 row 77.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";
    const HIT_JOB: &str = "core-016"; // Hit Job: the kill

    fn after_its_death(field: Value) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [HIT_JOB], "field": field, "mana": 10 },
            "p2": { "library": crate::filler(2) },
        }));
        let node = s.unit_at(P1, 1);
        assert_eq!(node.def_id, ID);
        s.play(HIT_JOB, pick::targets(pick::instance(&node)));
        s
    }

    fn other_unit(s: &Scenario) -> CardInstance {
        active_units_of(s.state(), P1)
            .into_iter()
            .find(|unit| unit.def_id == UNIT)
            .cloned()
            .expect("the other Unit")
    }

    mod base {
        use super::*;

        #[test]
        fn the_other_unit_gets_2_2() {
            let mut s = after_its_death(json!([{ "def": ID }, UNIT]));
            let other = other_unit(&s);
            s.expect_stats(&other, json!({ "attack": 4, "health": 4 }));
        }

        #[test]
        fn with_nothing_standing_nothing_happens() {
            let s = after_its_death(json!([{ "def": ID }]));
            assert!(active_units_of(s.state(), P1).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_other_unit_gets_4_4() {
            let mut s = after_its_death(json!([{ "def": ID, "radiant": true }, UNIT]));
            let other = other_unit(&s);
            s.expect_stats(&other, json!({ "attack": 6, "health": 6 }));
        }
    }
}
