//! SPEC §8.13 IA #90 Mass Production ((4) Spell, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Summon a copy of each friendly Unit."
//!   Radiant: "Summon a copy of each friendly Unit. They have Rush."
//!
//! One `summonCopy` of every friendly Unit there as it resolves, in order (R4588005): a full board
//! takes fewer (§3.2), and the copies are not copied themselves. The Radiant face then grants what it
//! summoned Rush (R136), read off `summoned_so_far` as IA #15 reads its Drones.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-090";

fn mass_production(rush_copies: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let ids: Vec<String> = active_units_of(ctx.state, ctx.controller)
                .iter()
                .map(|unit| unit.id.clone())
                .collect();
            let mut effects: Vec<Effect> = ids
                .iter()
                .map(|id| {
                    summon_copy(json_as(json!({
                        "of": { "of": "instance", "instanceId": id },
                        "player": "self",
                    })))
                })
                .collect();
            if rush_copies {
                effects.push(for_each_card(ForEachCardArgs {
                    cards: Arc::new(|ctx| summoned_so_far(ctx)),
                    each: Arc::new(|instance_id: &str| {
                        grant_keyword(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                            "keyword": { "kind": "Rush" },
                        })))
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
        base: mass_production(false),
        radiant: mass_production(true),
    }
}

// IA #90 Mass Production — SPEC §8.13 row 90 (R4588005).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SMALL: &str = "stand-in (1) 2/2";
    const BIG: &str = "stand-in (2) 3/4";

    fn after_play(radiant: bool, field: Value) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "field": field, "mana": 10 },
            "p2": { "library": crate::filler(2) },
        }));
        s.play(ID, json!({}));
        s
    }

    fn rushed(s: &Scenario, unit: &CardInstance) -> bool {
        has_keyword(&s.stats(unit).keywords, KeywordKind::Rush)
    }

    mod base {
        use super::*;

        #[test]
        fn r4588005_copies_each_unit_there_as_it_resolves() {
            let s = after_play(false, json!([SMALL, BIG]));
            let units = active_units_of(s.state(), P1);
            assert_eq!(units.len(), 4, "each friendly Unit is copied once");
            assert_eq!(
                units.iter().filter(|unit| unit.def_id == SMALL).count(),
                2,
                "the copies are not copied themselves"
            );
            for unit in units {
                assert!(!rushed(&s, unit), "no Rush on the base face");
            }
        }

        #[test]
        fn s3_2_a_full_board_takes_fewer() {
            let s = after_play(false, json!([SMALL, SMALL, SMALL, SMALL]));
            assert_eq!(active_units_of(s.state(), P1).len(), 5);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4588005_copies_each_unit_there_as_it_resolves() {
            let s = after_play(true, json!([SMALL, BIG]));
            let units = active_units_of(s.state(), P1);
            assert_eq!(units.len(), 4);
            let (copies, originals): (Vec<_>, Vec<_>) = units
                .into_iter()
                .partition(|unit| rushed(&s, unit));
            assert_eq!(copies.len(), 2, "the copies have Rush");
            assert_eq!(originals.len(), 2, "the originals do not");
        }
    }
}
