//! SPEC §8.13 IA #94 Batch Monarch ((3) Unit, Legendary, 4/6 → 8/12), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Start of turn: Summon a copy of your highest-Attack Unit. Give it Brittle 1."
//!   Radiant: "Start of turn: Summon a copy of your highest-Attack Unit."
//!
//! IA #86 Replica Hatchery at the start of its controller's turn (R68) over the highest-Attack Unit
//! (R4588001: the highest shown Attack, one match-rng pick of a tie): itself among the candidates,
//! the base face's copy with Brittle 1. Its intro (row 10 of `docs/ADDING_CARDS.md`) is the release's.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-094";

/// "Give it Brittle 1".
const BRITTLE: i32 = 1;

fn batch_monarch(gives_brittle: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| {
            let mut effects = vec![for_extreme_unit(ForExtremeUnitArgs {
                extreme: UnitExtreme::HighestAttack,
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
        base: batch_monarch(true),
        radiant: batch_monarch(false),
    }
}

// IA #94 Batch Monarch — SPEC §8.13 row 94 (R4588001).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SMALL: &str = "stand-in (1) 2/2";
    const BIG: &str = "stand-in (3) 9/9";

    fn after_start_of_turn(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "field": [{ "def": ID, "radiant": radiant }, SMALL, BIG],
                "library": crate::filler(2),
            },
            "p2": { "library": crate::filler(2) },
        }));
        s.start_turn();
        s
    }

    fn copies_of(s: &Scenario) -> Vec<CardInstance> {
        active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == BIG)
            .cloned()
            .collect()
    }

    fn brittle(s: &Scenario, unit: &CardInstance) -> bool {
        has_keyword(&s.stats(unit).keywords, KeywordKind::Brittle)
    }

    mod base {
        use super::*;

        #[test]
        fn r4588001_copies_the_highest_attack_with_brittle() {
            let s = after_start_of_turn(false);
            let copies = copies_of(&s);
            assert_eq!(copies.len(), 2, "the highest-Attack Unit is copied");
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
        fn r4588001_copies_the_highest_attack_with_no_brittle() {
            let s = after_start_of_turn(true);
            let copies = copies_of(&s);
            assert_eq!(copies.len(), 2, "the highest-Attack Unit is copied");
            for copy in nonempty(copies, "the copies") {
                assert!(!brittle(&s, &copy), "the Radiant face's copy has none");
            }
        }
    }
}
