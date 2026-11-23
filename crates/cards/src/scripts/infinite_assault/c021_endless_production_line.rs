//! SPEC §8.13 IA #21 Endless Production Line ((4) Field Spell, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "End of turn: Summon a random Infinite Assault Unit that costs (2) or less."
//!   Radiant: "End of turn: Summon two random Infinite Assault Units that cost (2) or less."
//!
//! M #16 Trenful Trickster's end-of-turn `summonRandom` over the set-named pool of Units printed at
//! (2) or less (R65, R1846001, R4577000; this card and the tokens left out, R387, §5.1), one draw per summon; a full board
//! takes fewer (§3.2). The (2) is a price the pool's filter names (R1748003).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-021";

/// The set every "Infinite Assault card" names (R4577000).
const SET: &str = "Infinite Assault";

/// "costs (2) or less".
const MAX_COST: i32 = 2;

/// "a random … Unit", Radiant "two random … Units".
const SUMMONS: usize = 1;
const RADIANT_SUMMONS: usize = 2;

fn production_line(summons: usize) -> Script {
    Script {
        end_of_turn: Some(hook(move |_ctx| {
            (0..summons)
                .map(|_| {
                    summon_random(json_as(json!({
                        "query": { "set": SET, "type": "Unit", "costRange": { "max": MAX_COST } },
                        "player": "self",
                    })))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: production_line(SUMMONS),
        radiant: production_line(RADIANT_SUMMONS),
    }
}

// IA #21 Endless Production Line — SPEC §8.13 row 21.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::catalog::def_of;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn summoned_at_its_end(radiant: bool, seed: &str) -> (Scenario, Vec<CardInstance>) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": true, "lane": 1 }],
                "library": crate::filler(2),
            },
            "p2": { "library": crate::filler(2) },
        }));
        s.end_turn();
        let units = active_units_of(s.state(), P1).into_iter().cloned().collect();
        (s, units)
    }

    fn expect_cheap_infinite_assault_units(radiant: bool, count: usize) {
        for seed in 0..8 {
            let (s, units) = summoned_at_its_end(radiant, &format!("production-line-{seed}"));
            assert_eq!(units.len(), count, "seed {seed}");
            for unit in nonempty(units, "the summoned Units") {
                let def = def_of(Some(s.state()), &unit.def_id);
                assert_eq!(def.set, SetName::InfiniteAssault, "{}", def.id);
                assert_eq!(def.type_, CardType::Unit, "{}", def.id);
                assert!(query_cost(def) <= 2, "{}", def.id);
                assert!(!def.is_token(), "{}", def.id);
            }
        }
    }

    mod base {
        use super::*;

        #[test]
        fn end_of_turn_summons_a_random_infinite_assault_unit_of_2_or_less() {
            expect_cheap_infinite_assault_units(false, 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn end_of_turn_summons_two() {
            expect_cheap_infinite_assault_units(true, 2);
        }
    }
}
