//! SPEC §8.13 IA #22 Assault Mainframe ((4) Unit, Epic, 6/10 → 12/20), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Taunt\nAura: Your Created Units have +{buff}/+{buff}." (buff 2)
//!   Radiant: "Taunt\nAura: Your Created Units have +{buff}/+{buff}." (buff 4)
//!
//! C+ #46 Felinor Flagbearer's aura over its controller's Units on the field, narrowed to the Created
//! ones (R943), this one among them when it is Created. Taunt is the catalog's.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-022";

/// "Your Created Units have +N/+N".
fn created_units_aura() -> AuraHook {
    aura_hook(move |ctx| {
        let amount = param(&ctx, "buff");
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == ctx.self_.controller && in_units_row_or_standing(ctx.state, unit) && is_created(unit)
            }),
            mod_: StatMod {
                attack: Some(amount),
                max_health: Some(amount),
                ..StatMod::default()
            },
        }]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        aura: Some(created_units_aura()),
        ..Script::default()
    };
    // The same script: the Radiant face's +4/+4 is its declared `buff`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #22 Assault Mainframe — SPEC §8.13 row 22.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";
    const HIT_JOB: &str = "core-016"; // Hit Job: the kill

    fn board(radiant: bool, created: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [HIT_JOB],
                "field": [
                    { "def": ID, "radiant": radiant, "created": created },
                    { "def": UNIT, "created": true },
                    UNIT,
                ],
                "mana": 10,
            },
            "p2": { "field": [{ "def": UNIT, "created": true }] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn your_created_units_have_2_2_and_no_other_unit() {
            let mut s = board(false, false);
            let (made, dealt, enemy) = (s.unit_at(P1, 2), s.unit_at(P1, 3), s.unit_at(P2, 1));
            s.expect_stats(&made, json!({ "attack": 4, "health": 4 }));
            s.expect_stats(&dealt, json!({ "attack": 2, "health": 2 }));
            s.expect_stats(&enemy, json!({ "attack": 2, "health": 2 }));
            let mainframe = s.unit_at(P1, 1);
            s.expect_stats(&mainframe, json!({ "attack": 6, "health": 10 }));
            assert!(has_keyword(&s.stats(&mainframe).keywords, KeywordKind::Taunt));
        }

        #[test]
        fn a_created_mainframe_has_it_too_and_it_ends_when_the_mainframe_leaves() {
            let mut s = board(false, true);
            let mainframe = s.unit_at(P1, 1);
            s.expect_stats(&mainframe, json!({ "attack": 8, "health": 12 }));
            let made = s.unit_at(P1, 2);
            s.play(HIT_JOB, pick::targets(pick::instance(&mainframe)));
            s.expect_stats(&made, json!({ "attack": 2, "health": 2 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn your_created_units_have_4_4() {
            let mut s = board(true, false);
            let (made, dealt) = (s.unit_at(P1, 2), s.unit_at(P1, 3));
            s.expect_stats(&made, json!({ "attack": 6, "health": 6 }));
            s.expect_stats(&dealt, json!({ "attack": 2, "health": 2 }));
            let mainframe = s.unit_at(P1, 1);
            s.expect_stats(&mainframe, json!({ "attack": 12, "health": 20 }));
        }
    }
}
