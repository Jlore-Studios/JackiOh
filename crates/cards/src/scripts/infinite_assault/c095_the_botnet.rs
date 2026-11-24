//! SPEC §8.13 IA #95 The Botnet ((4) Field Spell, Legendary), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Your Units have +{buff}/+{buff} for each other friendly Unit." (buff 1)
//!   Radiant: "Your Units have +{buff}/+{buff} for each other friendly Unit and Rush." (buff 1)
//!
//! An aura over every Unit its controller controls (R4588004): the declared `buff` for every other
//! friendly Unit, read live (§10.4 layer 5), so Health falls as Units leave; the Radiant face lends
//! Rush beside it. With nothing to count the base face projects no entry. Its intro (row 10 of
//! `docs/ADDING_CARDS.md`) is the release's.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-095";

/// "Your Units have +N/+N for each other friendly Unit".
fn the_botnet(gives_rush: bool) -> AuraHook {
    aura_hook(move |ctx| {
        let per = param(&ctx, "buff");
        let controller = ctx.self_.controller;
        let state = ctx.state;
        let others = active_units_of(state, controller).len().saturating_sub(1);
        let amount = per.saturating_mul(i32::try_from(others).unwrap_or(0));
        if amount == 0 && !gives_rush {
            return vec![];
        }
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == controller && in_units_row_or_standing(state, unit)
            }),
            mod_: StatMod {
                attack: Some(amount),
                max_health: Some(amount),
                keywords: gives_rush.then(|| vec![Keyword::Rush]),
                ..StatMod::default()
            },
        }]
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            aura: Some(the_botnet(false)),
            ..Script::default()
        },
        radiant: Script {
            aura: Some(the_botnet(true)),
            ..Script::default()
        },
    }
}

// IA #95 The Botnet — SPEC §8.13 row 95 (R4588004).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";
    const HIT_JOB: &str = "core-016"; // Hit Job: the kill

    fn board(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [HIT_JOB],
                "field": field,
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": true, "lane": 1 }],
                "mana": 10,
                "library": crate::filler(2),
            },
            "p2": { "library": crate::filler(2) },
        }))
    }

    fn rushed(s: &Scenario, unit: &CardInstance) -> bool {
        has_keyword(&s.stats(unit).keywords, KeywordKind::Rush)
    }

    mod base {
        use super::*;

        #[test]
        fn r4588004_each_unit_has_one_per_other() {
            let mut s = board(false, json!([UNIT, UNIT]));
            for lane in [1, 2] {
                s.expect_stats(s.unit_at(P1, lane), json!({ "attack": 3, "health": 3 }));
            }
            let other = s.unit_at(P1, 2);
            s.play(HIT_JOB, pick::targets(pick::instance(&other)));
            let survivor = active_units_of(s.state(), P1)
                .into_iter()
                .find(|unit| unit.def_id == UNIT)
                .cloned()
                .expect("the surviving Unit");
            s.expect_stats(&survivor, json!({ "attack": 2, "health": 2 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4588004_each_unit_has_one_per_other_and_rush() {
            let mut s = board(true, json!([UNIT, UNIT]));
            for lane in [1, 2] {
                let unit = s.unit_at(P1, lane);
                s.expect_stats(&unit, json!({ "attack": 3, "health": 3 }));
                assert!(rushed(&s, &unit));
            }
        }
    }
}
