//! SPEC §8.13 IA #143 Zero-Click ((2) Spell, Legendary), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Transform two enemy Units into Corrupted Processes."
//!   Radiant: "Transform two enemy Units into Corrupted Processes. Lock their zones."
//!
//! Two declared enemy Units (R81, R279: the named plural), each replaced by IA #143.1 Corrupted
//! Process where it stands (S C+ T-AI-8's shape, §6.3 Transform); on the Radiant face their zones
//! lock first, so the Processes arrive in locked zones (M #98 Showdown's shape, R15).

use jackioh_engine::effects::{LockArgs, ZoneSpec, lock, transform};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-143";

/// IA #143.1 Corrupted Process: the token both faces write.
const TOKEN: &str = "infiniteassault-143-1";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Harm),
        ..TargetDecl::target(2, 2, json!({ "side": "enemy", "of": ["unit"] }))
    }]
}

fn zero_click(radiant: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let mut effects = Vec::new();
            if radiant {
                // Each chosen Unit's zone, named by the live pick: the Radiant face locks both
                // before it transforms them.
                for pick in ctx.targets.clone() {
                    let instance = match &pick {
                        Selection::Instance { instance_id } => find_instance(&*ctx.state, instance_id),
                        _ => None,
                    };
                    let Some(unit) = instance else {
                        continue;
                    };
                    let Some(slot) = slot_of(&*ctx.state, unit) else {
                        continue;
                    };
                    effects.push(lock(LockArgs {
                        zone: ZoneSpec::Lane {
                            row: slot.row,
                            lane: slot.lane,
                            player: Some(if slot.player == ctx.controller {
                                PlayerSpec::SelfSide
                            } else {
                                PlayerSpec::Enemy
                            }),
                        },
                    }));
                }
            }
            for index in 0..2 {
                effects.push(transform(json_as(json!({
                    "target": { "of": "chosen", "index": index },
                    "defId": TOKEN,
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: zero_click(false),
        radiant: zero_click(true),
    }
}

// IA #143 Zero-Click — SPEC §8.13 row 143 (R279).
#[cfg(test)]
mod tests {
    use super::{ID, TOKEN};
    use jackioh_engine::testkit::PlayerId::P2;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    fn board(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 },
            "p2": { "hand": ["core-005"], "field": [VANILLA, VANILLA] },
        }))
    }

    fn both(s: &Scenario) -> (CardInstance, CardInstance) {
        (s.unit_at(P2, 1), s.unit_at(P2, 2))
    }

    fn cast(s: &mut Scenario) {
        let (first, second) = both(s);
        s.play(
            ID,
            json!({ "targets": [
                { "pick": "instance", "instanceId": first.id },
                { "pick": "instance", "instanceId": second.id },
            ] }),
        );
    }

    fn locked(s: &Scenario, lane: i32) -> bool {
        jackioh_engine::zones::is_locked(s.state(), ZoneSlot { player: P2, row: Row::Units, lane })
    }

    mod base {
        use super::*;

        #[test]
        fn transforms_two_enemy_units_into_corrupted_processes() {
            let mut s = board(false, "zero-click");
            cast(&mut s);
            let (first, second) = both(&s);
            assert_eq!(first.def_id, TOKEN);
            assert_eq!(second.def_id, TOKEN);
            assert!(!locked(&s, 1), "the base face locks nothing");
            assert!(!locked(&s, 2));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn locks_both_zones_and_transforms_both_units() {
            let mut s = board(true, "zero-click-radiant");
            cast(&mut s);
            let (first, second) = both(&s);
            assert_eq!(first.def_id, TOKEN);
            assert_eq!(second.def_id, TOKEN);
            assert!(locked(&s, 1), "their zones lock first");
            assert!(locked(&s, 2));
        }
    }
}
