//! SPEC §8.13 IA #75 Load Balancer ((2) Unit, Common, 2/4 → 4/8), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Aura: Your other Units have +{health} Health." (health 1)
//!   Radiant: "Aura: Your other Units have +{health} Health." (health 2)
//!
//! IA #22 Assault Mainframe's aura, narrowed to its controller's other Units on the field: itself and
//! the enemy's Units take nothing.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-075";

/// "Your other Units have +N Health".
fn other_units_aura() -> AuraHook {
    aura_hook(move |ctx| {
        let amount = param(&ctx, "health");
        let controller = ctx.self_.controller;
        let state = ctx.state;
        let me = ctx.self_.id.clone();
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == controller && in_units_row_or_standing(state, unit) && unit.id != me
            }),
            mod_: StatMod {
                max_health: Some(amount),
                ..StatMod::default()
            },
        }]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        aura: Some(other_units_aura()),
        ..Script::default()
    };
    // The same script: the Radiant face's +2 is its declared `health`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #75 Load Balancer — SPEC §8.13 row 75.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";
    const HIT_JOB: &str = "core-016"; // Hit Job: the kill

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [HIT_JOB],
                "field": [{ "def": ID, "radiant": radiant }, UNIT],
                "mana": 10,
            },
            "p2": { "field": [UNIT] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn your_other_units_have_1_health_and_nothing_else() {
            let mut s = board(false);
            let (other, enemy) = (s.unit_at(P1, 2), s.unit_at(P2, 1));
            s.expect_stats(&other, json!({ "attack": 2, "health": 3 }));
            s.expect_stats(&enemy, json!({ "attack": 2, "health": 2 }));
            let balancer = s.unit_at(P1, 1);
            s.expect_stats(&balancer, json!({ "attack": 2, "health": 4 }));
        }

        #[test]
        fn the_health_ends_when_the_balancer_leaves() {
            let mut s = board(false);
            let balancer = s.unit_at(P1, 1);
            let other = s.unit_at(P1, 2);
            s.play(HIT_JOB, pick::targets(pick::instance(&balancer)));
            s.expect_stats(&other, json!({ "attack": 2, "health": 2 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn your_other_units_have_2_health() {
            let mut s = board(true);
            let other = s.unit_at(P1, 2);
            s.expect_stats(&other, json!({ "attack": 2, "health": 4 }));
            let balancer = s.unit_at(P1, 1);
            s.expect_stats(&balancer, json!({ "attack": 4, "health": 8 }));
        }
    }
}
