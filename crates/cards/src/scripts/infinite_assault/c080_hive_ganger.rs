//! SPEC §8.13 IA #80 Hive Ganger ((2) Unit, Common, 3/3 → 6/6), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Has +{attack} Attack for each other friendly Unit." (attack 1)
//!   Radiant: "Has +{attack} Attack for each other friendly Unit." (attack 2)
//!
//! An aura over itself alone, read live (§10.4 layer 5, R4588004): the declared `attack` for every
//! other Unit its controller controls, so its Attack falls as Units leave.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-080";

/// "Has +N Attack for each other friendly Unit".
fn hive_ganger() -> AuraHook {
    aura_hook(move |ctx| {
        let per = param(&ctx, "attack");
        let me = ctx.self_.id.clone();
        let others = active_units_of(ctx.state, ctx.self_.controller)
            .into_iter()
            .filter(|unit| unit.id != me)
            .count();
        let amount = per.saturating_mul(i32::try_from(others).unwrap_or(0));
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| unit.id == me),
            mod_: StatMod {
                attack: Some(amount),
                ..StatMod::default()
            },
        }]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        aura: Some(hive_ganger()),
        ..Script::default()
    };
    // The same script: the Radiant face's +2 is its declared `attack`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #80 Hive Ganger — SPEC §8.13 row 80.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";
    const HIT_JOB: &str = "core-016"; // Hit Job: the kill

    fn board(p1_field: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [HIT_JOB], "field": p1_field, "mana": 10 },
            "p2": { "field": [UNIT] },
        }))
    }

    fn ganger(s: &Scenario) -> CardInstance {
        active_units_of(s.state(), P1)
            .into_iter()
            .find(|unit| unit.def_id == ID)
            .cloned()
            .expect("the Hive Ganger")
    }

    mod base {
        use super::*;

        #[test]
        fn r4588004_attack_follows_the_other_units() {
            let mut s = board(json!([{ "def": ID }, UNIT, UNIT]));
            s.expect_stats(&ganger(&s), json!({ "attack": 3 + 2 }));
            // The enemy's Unit is no friendly Unit, and a death drops it.
            let other = s.unit_at(P1, 2);
            s.play(HIT_JOB, pick::targets(pick::instance(&other)));
            s.expect_stats(&ganger(&s), json!({ "attack": 3 + 1 }));
        }

        #[test]
        fn alone_it_is_a_3_3() {
            let mut s = board(json!([{ "def": ID }]));
            s.expect_stats(&ganger(&s), json!({ "attack": 3, "health": 3 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4588004_attack_follows_the_other_units() {
            let mut s = board(json!([{ "def": ID, "radiant": true }, UNIT, UNIT]));
            s.expect_stats(&ganger(&s), json!({ "attack": 6 + 4 }));
        }
    }
}
