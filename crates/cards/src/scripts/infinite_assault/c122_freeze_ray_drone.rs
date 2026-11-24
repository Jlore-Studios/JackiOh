//! SPEC §8.13 IA #122 Freeze Ray Drone ((1) Unit, Common, 1/3 → 2/6), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Freeze an enemy Unit."
//!   Radiant: "Cry: Freeze two enemy Units."
//!
//! A declared enemy Unit, Frozen as it is picked (S #1 Freeze Ray's shape, §6.3 Freeze, R1174000):
//! the Radiant face declares and Freezes two. With one Unit standing, it Freezes that one.

use jackioh_engine::effects::freeze;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-122";

/// "Freeze an enemy Unit" (base) or "Freeze two enemy Units" (Radiant).
fn targets(count: i32) -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Harm),
        ..TargetDecl::target(count, count, json!({ "side": "enemy", "of": ["unit"] }))
    }]
}

fn drone(count: i32) -> Script {
    Script {
        targets: targets(count),
        cry: Some(hook(move |_ctx| {
            (0..count)
                .map(|index| {
                    freeze(json_as(json!({
                        "target": { "of": "chosen", "index": index },
                    })))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: drone(1),
        radiant: drone(2),
    }
}

// IA #122 Freeze Ray Drone — SPEC §8.13 row 122.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P2;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "freeze-ray-drone",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }] },
            "p2": { "field": [VANILLA, VANILLA] },
        }))
    }

    fn frozen(s: &Scenario, card: &CardInstance) -> bool {
        s.card(card).frozen.is_some()
    }

    mod base {
        use super::*;

        #[test]
        fn cry_freezes_the_chosen_enemy_unit() {
            let mut s = board(false);
            let first = s.unit_at(P2, 1);
            let second = s.unit_at(P2, 2);
            s.play(ID, pick::targets(pick::instance(&second)));
            assert!(!frozen(&s, &first));
            assert!(frozen(&s, &second));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn cry_freezes_two_enemy_units() {
            let mut s = board(true);
            let first = s.unit_at(P2, 1);
            let second = s.unit_at(P2, 2);
            s.play(
                ID,
                json!({ "targets": [
                    { "pick": "instance", "instanceId": first.id },
                    { "pick": "instance", "instanceId": second.id },
                ] }),
            );
            assert!(frozen(&s, &first));
            assert!(frozen(&s, &second));
        }

        #[test]
        fn with_one_legal_target_it_freezes_that_one() {
            let mut s = scenario(json!({
                "seed": "freeze-ray-drone-one",
                "p1": { "hand": [{ "def": ID, "radiant": true }] },
                "p2": { "field": [VANILLA] },
            }));
            let only = s.unit_at(P2, 1);
            s.play(ID, pick::targets(pick::instance(&only)));
            assert!(frozen(&s, &only));
        }
    }
}
