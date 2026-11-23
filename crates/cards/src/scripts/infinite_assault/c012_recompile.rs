//! SPEC §8.13 IA #12 Recompile ((1) Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Transform a friendly Unit into a random Infinite Assault Unit that costs ({step}) more." (step 1)
//!   Radiant: "Transform a friendly Unit into a random Infinite Assault Unit that costs ({step}) more." (step 2)
//!
//! C+ #73.1 Classic Golem's `transformRandom` over the set-named pool of Units (R1846001, R4577000)
//! at the chosen Unit's printed cost plus `step` (R65), keeping its face (R4577006). With no such
//! Unit the pool is empty and nothing changes (R129); Immutable refuses it (R23).

use jackioh_engine::effects::transform_random;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-012";

/// The set every "Infinite Assault card" names (R4577000).
const SET: &str = "Infinite Assault";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Help),
        ..TargetDecl::target(1, 1, json!({ "side": "ally", "of": ["unit"] }))
    }]
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            let Some(unit) = instance_of(ctx, &TargetSpec::Chosen { index: None }) else {
                return vec![];
            };
            let cost = query_cost(def_of(Some(&*ctx.state), &unit.def_id)) + param(&*ctx, "step");
            vec![transform_random(json_as(json!({
                "target": { "of": "chosen" },
                "query": { "set": SET, "type": "Unit", "cost": cost },
                "radiant": "keep",
            })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's (2) is its declared `step`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #12 Recompile — SPEC §8.13 row 12 (R4577006).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::catalog::def_of;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SEED_DRONE: &str = "infiniteassault-001"; // Seed Drone
    const BIG: &str = "stand-in (4) 4/4";

    fn recompiled(radiant: bool, unit: Value, seed: &str) -> (Scenario, CardInstance) {
        let mut s = scenario(json!({ "seed": seed, "p1": { "hand": [{ "def": ID, "radiant": radiant }], "field": [unit] } }));
        let before = s.unit_at(P1, 1);
        s.play(ID, pick::targets(pick::instance(&before)));
        let after = s.unit_at(P1, 1);
        (s, after)
    }

    fn expect_an_infinite_assault_unit_of(radiant: bool, cost: i32) {
        for seed in 0..8 {
            let (s, after) = recompiled(radiant, json!(SEED_DRONE), &format!("recompile-{seed}"));
            let def = def_of(Some(s.state()), &after.def_id);
            assert_eq!(def.set, SetName::InfiniteAssault, "{}", def.id);
            assert_eq!(def.type_, CardType::Unit, "{}", def.id);
            assert_eq!(query_cost(def), cost, "{}", def.id);
            assert!(!def.is_token(), "{}", def.id);
            assert!(!after.radiant);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn a_1_cost_unit_becomes_a_random_2_cost_infinite_assault_unit() {
            expect_an_infinite_assault_unit_of(false, 2);
        }

        #[test]
        fn r4577006_with_no_unit_at_that_cost_nothing_changes() {
            let (_, after) = recompiled(false, json!(BIG), "recompile");
            assert_eq!(after.def_id, BIG);
        }

        #[test]
        fn r4577006_the_new_unit_keeps_the_radiant_face() {
            let (_, after) = recompiled(false, json!({ "def": SEED_DRONE, "radiant": true }), "recompile");
            assert_ne!(after.def_id, SEED_DRONE);
            assert!(after.radiant);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_1_cost_unit_becomes_a_random_3_cost_infinite_assault_unit() {
            expect_an_infinite_assault_unit_of(true, 3);
        }
    }
}
