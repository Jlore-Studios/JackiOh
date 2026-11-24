//! SPEC §8.13 IA #70 Redline Overlord ((3) Unit, Legendary, 6/6 → 12/12), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Aura: While your hero has {health} or less Health, your Units have Charge and +{attack} Attack." (health 15, attack 2)
//!   Radiant: "Aura: While your hero has {health} or less Health, your Units have Charge and +{attack} Attack." (health 15, attack 4)
//!
//! IA #58 Overclocked Core's controller's-Units aura with a threshold: while its controller's hero
//! sits at the declared `health` or less, read as the aura reads, its controller's Units on the
//! field — itself included — have Charge and the declared `attack` (layer 5, §10.4). Above that
//! Health the aura contributes nothing.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-070";

pub fn script() -> CardScripts {
    let base = Script {
        aura: Some(aura_hook(|ctx| {
            if hero_of(ctx.state, ctx.self_.controller).health > param(&ctx, "health") {
                return vec![];
            }
            let amount = param(&ctx, "attack");
            vec![AuraEntry {
                applies: Box::new(move |unit: &CardInstance| {
                    unit.controller == ctx.self_.controller
                        && in_units_row_or_standing(ctx.state, unit)
                }),
                mod_: StatMod {
                    attack: Some(amount),
                    keywords: Some(vec![Keyword::Charge]),
                    ..StatMod::default()
                },
            }]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's +4 is its declared `attack`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #70 Redline Overlord — SPEC §8.13 row 70.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";

    fn board(radiant: bool, health: i32) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [UNIT],
                "field": [{ "def": ID, "radiant": radiant }],
                "health": health,
            },
            "p2": { "field": [UNIT] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn above_15_health_nothing() {
            let mut s = board(false, 16);
            s.play(UNIT, json!({ "zone": 2 }));
            let (overlord, unit, enemy) = (s.unit_at(P1, 1), s.unit_at(P1, 2), s.unit_at(P2, 1));
            s.expect_stats(&overlord, json!({ "attack": 6, "health": 6 }));
            s.expect_stats(&unit, json!({ "attack": 2, "health": 2 }));
            assert!(!has_keyword(&s.stats(&unit).keywords, KeywordKind::Charge));
            s.expect_stats(&enemy, json!({ "attack": 2, "health": 2 }));
        }

        #[test]
        fn at_15_health_your_units_have_charge_and_2_attack() {
            let mut s = board(false, 15);
            // The sick Unit played this turn may attack the hero at once.
            s.play(UNIT, json!({ "zone": 2 }));
            let (overlord, unit) = (s.unit_at(P1, 1), s.unit_at(P1, 2));
            s.expect_stats(&overlord, json!({ "attack": 8, "health": 6 }));
            s.expect_stats(&unit, json!({ "attack": 4, "health": 2 }));
            assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::Charge));
            s.attack(&unit, "hero");
            s.expect_health(P2, HERO_HEALTH - 4);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn at_15_health_your_units_have_charge_and_4_attack() {
            let mut s = board(true, 15);
            s.play(UNIT, json!({ "zone": 2 }));
            let (overlord, unit) = (s.unit_at(P1, 1), s.unit_at(P1, 2));
            s.expect_stats(&overlord, json!({ "attack": 16, "health": 12 }));
            s.expect_stats(&unit, json!({ "attack": 6, "health": 2 }));
            s.attack(&unit, "hero");
            s.expect_health(P2, HERO_HEALTH - 6);
        }
    }
}
