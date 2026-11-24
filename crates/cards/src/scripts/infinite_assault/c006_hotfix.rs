//! SPEC §8.13 IA #6 Hotfix ((1) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Give a friendly Unit +{buff}/+{buff}. If it's Created, draw a card." (buff 2)
//!   Radiant: "Give a friendly Unit +{buff}/+{buff} and Divine Shield. If it's Created, draw a card." (buff 4)
//!
//! TG #2 Personal Trainer's friendly target and buff (aimed to help, R656); whether the Unit is
//! Created (R943) is read before anything resolves, so the draw follows the Unit chosen.

use jackioh_engine::effects::{buff, draw, grant_keyword};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-006";

/// "draw a card".
const DRAWS: i32 = 1;

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Help),
        ..TargetDecl::target(1, 1, json!({ "side": "ally", "of": ["unit"] }))
    }]
}

fn hotfix(radiant: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let Some(unit) = instance_of(ctx, &TargetSpec::Chosen { index: None }) else {
                return vec![];
            };
            let amount = param(&*ctx, "buff");
            let mut effects =
                vec![buff(json_as(json!({ "target": { "of": "chosen" }, "attack": amount, "health": amount })))];
            if radiant {
                effects.push(grant_keyword(json_as(
                    json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Divine Shield" } }),
                )));
            }
            if is_created(&unit) {
                effects.push(draw(json_as(json!({ "count": DRAWS }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: hotfix(false),
        radiant: hotfix(true),
    }
}

// IA #6 Hotfix — SPEC §8.13 row 6.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [{ "def": UNIT, "created": true }, UNIT],
                "library": crate::filler(3),
            },
            "p2": { "field": [UNIT] },
        }))
    }

    fn fix(s: &mut Scenario, lane: i32) -> CardInstance {
        let unit = s.unit_at(P1, lane);
        s.play(ID, pick::targets(pick::instance(&unit)));
        unit
    }

    fn shielded(s: &Scenario, unit: &CardInstance) -> bool {
        has_keyword(&s.stats(unit).keywords, KeywordKind::DivineShield)
    }

    mod base {
        use super::*;

        #[test]
        fn a_created_unit_gets_2_2_and_you_draw_a_card() {
            let mut s = board(false);
            let unit = fix(&mut s, 1);
            s.expect_stats(&unit, json!({ "attack": 4, "health": 4 }));
            assert_eq!(s.hand(P1).len(), 1, "the draw");
            assert!(!shielded(&s, &unit));
        }

        #[test]
        fn a_dealt_unit_gets_2_2_and_no_draw() {
            let mut s = board(false);
            let unit = fix(&mut s, 2);
            s.expect_stats(&unit, json!({ "attack": 4, "health": 4 }));
            assert!(s.hand(P1).is_empty());
        }

        #[test]
        fn r81_an_enemy_unit_is_no_legal_pick() {
            let s = board(false);
            let enemy = s.unit_at(P2, 1);
            assert!(!pick::instance_ids(&s.play_targets(ID)).contains(&enemy.id));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_created_unit_gets_4_4_and_divine_shield_and_you_draw() {
            let mut s = board(true);
            let unit = fix(&mut s, 1);
            s.expect_stats(&unit, json!({ "attack": 6, "health": 6 }));
            assert!(shielded(&s, &unit));
            assert_eq!(s.hand(P1).len(), 1);
        }

        #[test]
        fn a_dealt_unit_gets_4_4_and_divine_shield_and_no_draw() {
            let mut s = board(true);
            let unit = fix(&mut s, 2);
            s.expect_stats(&unit, json!({ "attack": 6, "health": 6 }));
            assert!(shielded(&s, &unit));
            assert!(s.hand(P1).is_empty());
        }
    }
}
