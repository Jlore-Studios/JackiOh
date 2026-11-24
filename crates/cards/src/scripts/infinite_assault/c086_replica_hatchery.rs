//! SPEC §8.13 IA #86 Replica Hatchery ((2) Field Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "End of turn: Summon a copy of your cheapest Unit. Give it Brittle 1."
//!   Radiant: "End of turn: Summon a copy of your cheapest Unit."
//!
//! `forExtremeUnit` over the cheapest Unit (R4588001: the lowest `cost_now`, one match-rng pick of a
//! tie, nothing with no Unit), copied as R57 copies (the face that was cheapest); the base face then
//! gives what it summoned Brittle 1, read off `summoned_so_far` as IA #15 reads its Drones.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-086";

/// "Give it Brittle 1".
const BRITTLE: i32 = 1;

fn replica_hatchery(gives_brittle: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |_ctx| {
            let mut effects = vec![for_extreme_unit(ForExtremeUnitArgs {
                extreme: UnitExtreme::Cheapest,
                player: None,
                each: Arc::new(|instance_id: &str| {
                    summon_copy(json_as(json!({
                        "of": { "of": "instance", "instanceId": instance_id },
                        "player": "self",
                    })))
                }),
            })];
            if gives_brittle {
                effects.push(for_each_card(ForEachCardArgs {
                    cards: Arc::new(|ctx| summoned_so_far(ctx)),
                    each: Arc::new(|instance_id: &str| {
                        give_brittle(json_as(json!({ "instanceId": instance_id, "n": BRITTLE })))
                    }),
                }));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: replica_hatchery(true),
        radiant: replica_hatchery(false),
    }
}

// IA #86 Replica Hatchery — SPEC §8.13 row 86 (R4588001).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const CHEAP: &str = "stand-in (1) 2/2";
    const BIG: &str = "stand-in (3) 5/5";

    fn after_end_of_turn(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "field": [CHEAP, BIG],
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": true, "lane": 1 }],
                "library": crate::filler(2),
            },
            "p2": { "library": crate::filler(2) },
        }));
        s.end_turn();
        s
    }

    fn copies_of(s: &Scenario) -> Vec<CardInstance> {
        active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == CHEAP)
            .cloned()
            .collect()
    }

    fn brittle(s: &Scenario, unit: &CardInstance) -> bool {
        has_keyword(&s.stats(unit).keywords, KeywordKind::Brittle)
    }

    mod base {
        use super::*;

        #[test]
        fn r4588001_copies_the_cheapest_with_brittle() {
            let s = after_end_of_turn(false);
            let copies = copies_of(&s);
            assert_eq!(copies.len(), 2, "the cheapest Unit is copied");
            assert_eq!(
                copies.iter().filter(|copy| brittle(&s, copy)).count(),
                1,
                "only the copy has Brittle, never the original"
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4588001_copies_the_cheapest_with_no_brittle() {
            let s = after_end_of_turn(true);
            let copies = copies_of(&s);
            assert_eq!(copies.len(), 2, "the cheapest Unit is copied");
            for copy in nonempty(copies, "the copies") {
                assert!(!brittle(&s, &copy), "the Radiant face's copy has none");
            }
        }
    }
}
