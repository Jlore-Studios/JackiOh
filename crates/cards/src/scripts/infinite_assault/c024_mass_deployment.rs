//! SPEC §8.13 IA #24 Mass Deployment ((4) Spell, Legendary), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Summon a random Infinite Assault Unit of each Cost from (1) to (3). They have Rush."
//!   Radiant: "Summon a random Radiant Infinite Assault Unit of each Cost from (1) to (3). They have Rush."
//!
//! S+ #142.1 Mindustrial Golem's `summonRandomWith`, one per printed cost (R65), lowest first, each
//! from the set-named pool of Units at that cost (R1846001, R4577000; this card and the tokens left out, R387, §5.1) and given
//! Rush as it lands; an empty pool or a full board summons nothing for its cost (R129, §3.2). The (1)
//! and the (3) are prices the pools' filters name (R1748003).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-024";

/// The set every "Infinite Assault card" names (R4577000).
const SET: &str = "Infinite Assault";

/// "of each Cost from (1) to (3)".
const LOWEST: i32 = 1;
const HIGHEST: i32 = 3;

fn mass_deployment(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            (LOWEST..=HIGHEST)
                .map(|cost| {
                    summon_random_with(json_as(json!({
                        "query": { "set": SET, "type": "Unit", "cost": cost },
                        "player": "self",
                        "radiant": radiant,
                        "keywords": [{ "kind": "Rush" }],
                    })))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: mass_deployment(false),
        radiant: mass_deployment(true),
    }
}

// IA #24 Mass Deployment — SPEC §8.13 row 24.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::catalog::def_of;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn expect_one_of_each_cost(radiant: bool) {
        for seed in 0..6 {
            let mut s = scenario(json!({
                "seed": format!("mass-deployment-{seed}"),
                "p1": { "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 },
            }));
            s.play(ID, json!({}));
            let units: Vec<CardInstance> = active_units_of(s.state(), P1).into_iter().cloned().collect();
            let costs: Vec<i32> = units.iter().map(|unit| query_cost(def_of(Some(s.state()), &unit.def_id))).collect();
            assert_eq!(costs, [1, 2, 3], "seed {seed}: lowest first");
            for unit in nonempty(units, "the summoned Units") {
                let def = def_of(Some(s.state()), &unit.def_id);
                assert_eq!(def.set, SetName::InfiniteAssault, "{}", def.id);
                assert!(!def.is_token(), "{}", def.id);
                assert_eq!(unit.radiant, radiant, "{}", def.id);
                assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::Rush), "{}", def.id);
            }
        }
    }

    mod base {
        use super::*;

        #[test]
        fn summons_a_rushing_infinite_assault_unit_of_each_cost_1_to_3() {
            expect_one_of_each_cost(false);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn summons_a_rushing_radiant_infinite_assault_unit_of_each_cost_1_to_3() {
            expect_one_of_each_cost(true);
        }
    }
}
