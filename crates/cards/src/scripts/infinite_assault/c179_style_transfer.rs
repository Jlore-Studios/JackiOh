//! SPEC §8.13 IA #179 Style Transfer ((2) Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Transform a friendly Unit into a copy of an enemy Unit."
//!   Radiant: "Transform a friendly Unit into a Radiant copy of an enemy Unit."
//!
//! Built by C8 (#4592). R4592000, R4592007.

use jackioh_engine::effects::{TransformIntoCopyArgs, transform_into_copy};
use jackioh_engine::prelude::*;
use jackioh_engine::state::find_instance;

pub const ID: &str = "infiniteassault-179";

fn targets() -> Vec<TargetDecl> {
    vec![
        TargetDecl {
            aim: Some(TargetAim::Help),
            ..TargetDecl::target(1, 1, json!({ "side": "ally", "of": ["unit"] }))
        },
        TargetDecl {
            aim: Some(TargetAim::Harm),
            ..TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))
        },
    ]
}

fn style_transfer(radiant: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let mut ally = None;
            let mut enemy = None;
            for t in &ctx.targets {
                if let Selection::Instance { instance_id } = t {
                    if let Some(card) = find_instance(ctx.state, instance_id) {
                        if card.controller == ctx.controller {
                            ally = Some(instance_id.clone());
                        } else {
                            enemy = Some(instance_id.clone());
                        }
                    }
                }
            }
            let (Some(ally_id), Some(enemy_id)) = (ally, enemy) else {
                return vec![];
            };
            vec![transform_into_copy(TransformIntoCopyArgs {
                target: TargetSpec::Instance {
                    instance_id: ally_id,
                },
                of: TargetSpec::Instance {
                    instance_id: enemy_id,
                },
                radiant: if radiant { Some(true) } else { None },
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: style_transfer(false),
        radiant: style_transfer(true),
    }
}

// IA #179 Style Transfer — SPEC §8.13 row 179.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SMALL_UNIT: &str = "core-005"; // 1/1
    const BIG_UNIT: &str = "core-019"; // Midrange Menace 9/9 Taunt

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "field": [{ "def": SMALL_UNIT, "lane": 1 }], "mana": 10 },
            "p2": { "field": [{ "def": BIG_UNIT, "lane": 1 }] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn transforms_friendly_unit_into_copy_of_enemy_unit() {
            let mut s = board(false);
            let ally = s.unit_at(P1, 1).id.clone();
            let enemy = s.unit_at(PlayerId::P2, 1).id.clone();
            s.play(
                ID,
                json!({ "targets": [
                    { "pick": "instance", "instanceId": ally },
                    { "pick": "instance", "instanceId": enemy }
                ] }),
            );
            let transformed = s.unit_at(P1, 1);
            assert_eq!(transformed.def_id, BIG_UNIT);
            assert!(!transformed.radiant);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn transforms_friendly_unit_into_radiant_copy() {
            let mut s = board(true);
            let ally = s.unit_at(P1, 1).id.clone();
            let enemy = s.unit_at(PlayerId::P2, 1).id.clone();
            s.play(
                ID,
                json!({ "targets": [
                    { "pick": "instance", "instanceId": ally },
                    { "pick": "instance", "instanceId": enemy }
                ] }),
            );
            let transformed = s.unit_at(P1, 1);
            assert_eq!(transformed.def_id, BIG_UNIT);
            assert!(transformed.radiant);
        }
    }
}
