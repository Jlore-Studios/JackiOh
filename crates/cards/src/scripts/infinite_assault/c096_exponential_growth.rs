//! SPEC §8.13 IA #96 Exponential Growth ((2) Spell, Mythic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Summon a copy of each friendly Unit. Then double the Attack and Health of your Units."
//!   Radiant: "Summon a copy of each friendly Unit. The copies have Rush. Then double the Attack and Health of your Units."
//!
//! IA #90 Mass Production's copies of every friendly Unit there as it resolves (R4588005), the
//! Radiant face's with Rush, then a `doubleStats` over its controller's Units read once the copies
//! have landed — copies among them — each capped (R4079000). Its intro (row 10 of
//! `docs/ADDING_CARDS.md`) is the release's.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-096";

fn exponential_growth(rush_copies: bool) -> Script {
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
            effects.push(for_each_card(ForEachCardArgs {
                cards: Arc::new(|ctx| {
                    active_units_of(ctx.state, ctx.controller)
                        .iter()
                        .map(|unit| unit.id.clone())
                        .collect()
                }),
                each: Arc::new(|instance_id: &str| {
                    double_stats(json_as(json!({
                        "target": { "of": "instance", "instanceId": instance_id },
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
        base: exponential_growth(false),
        radiant: exponential_growth(true),
    }
}

// IA #96 Exponential Growth — SPEC §8.13 row 96 (R4588005).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SMALL: &str = "stand-in (1) 2/2";
    const BIG: &str = "stand-in (2) 3/4";

    fn after_play(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "field": [SMALL, BIG], "mana": 10 },
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
            let s = after_play(false);
            let units = active_units_of(s.state(), P1);
            assert_eq!(units.len(), 4, "each friendly Unit is copied once");
            for unit in units {
                assert!(!rushed(&s, unit), "no Rush on the base face");
            }
        }

        #[test]
        fn the_units_are_doubled_after_the_copies_land() {
            let s = after_play(false);
            let mut stats: Vec<(i32, i32)> = active_units_of(s.state(), P1)
                .into_iter()
                .map(|unit| (s.stats(unit).attack, s.stats(unit).health))
                .collect();
            stats.sort_unstable();
            assert_eq!(stats, [(4, 4), (4, 4), (6, 8), (6, 8)]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4588005_copies_each_unit_there_as_it_resolves() {
            let s = after_play(true);
            let units = active_units_of(s.state(), P1);
            assert_eq!(units.len(), 4);
            let (copies, originals): (Vec<_>, Vec<_>) = units.into_iter().partition(|unit| rushed(&s, unit));
            assert_eq!(copies.len(), 2, "the copies have Rush");
            assert_eq!(originals.len(), 2, "the originals do not");
            let mut stats: Vec<(i32, i32)> = active_units_of(s.state(), P1)
                .into_iter()
                .map(|unit| (s.stats(unit).attack, s.stats(unit).health))
                .collect();
            stats.sort_unstable();
            assert_eq!(stats, [(4, 4), (4, 4), (6, 8), (6, 8)]);
        }
    }
}
