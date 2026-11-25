//! SPEC §8.13 IA #81 Overprovision ((2) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Summon two Instances. If you control {units} or more Units, draw a card." (units 4)
//!   Radiant: "Summon two Radiant Instances. If you control {units} or more Units, draw two cards." (units 4)
//!
//! Two summons of IA #74.1, then IA #15 Drone Foundry's `forEachCard`, whose list is read once they
//! have landed: the draw when its controller controls the declared `units` Units or more, counting
//! the Instances just summoned, none otherwise.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-081";

/// IA #74.1 Instance.
const INSTANCE: &str = "infiniteassault-074-1";

/// "two Instances".
const INSTANCES: usize = 2;

/// "draw a card", Radiant "draw two cards".
const DRAWS: i32 = 1;
const RADIANT_DRAWS: i32 = 2;

/// The draw the `forEachCard` carries: never read, only counted.
const DRAW_MARK: &str = "draw";

fn overprovision(radiant: bool, draws: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let threshold = param(&*ctx, "units");
            let mut effects: Vec<Effect> = (0..INSTANCES)
                .map(|_| {
                    let mut args = json!({ "defId": INSTANCE, "player": "self" });
                    if radiant {
                        args["radiant"] = json!(true);
                    }
                    summon(json_as(args))
                })
                .collect();
            effects.push(for_each_card(ForEachCardArgs {
                cards: Arc::new(move |each: &mut EffectContext<'_>| {
                    if i32::try_from(active_units_of(each.state, each.controller).len()).unwrap_or(i32::MAX)
                        >= threshold
                    {
                        vec![DRAW_MARK.to_string()]
                    } else {
                        vec![]
                    }
                }),
                each: Arc::new(move |_instance_id: &str| draw(json_as(json!({ "count": draws })))),
            }));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: overprovision(false, DRAWS),
        radiant: overprovision(true, RADIANT_DRAWS),
    }
}

// IA #81 Overprovision — SPEC §8.13 row 81.
#[cfg(test)]
mod tests {
    use super::{ID, INSTANCE};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";

    fn after_play(radiant: bool, field: Value) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "field": field, "library": crate::filler(4) },
            "p2": { "library": crate::filler(2) },
        }));
        s.play(ID, json!({}));
        s
    }

    fn instances(s: &Scenario) -> Vec<CardInstance> {
        active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == INSTANCE)
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn two_instances_and_a_draw_at_four_units() {
            let s = after_play(false, json!([UNIT, UNIT]));
            assert_eq!(instances(&s).len(), 2);
            for instance in nonempty(instances(&s), "the Instances") {
                assert!(!instance.radiant);
            }
            assert_eq!(s.hand(P1).len(), 1, "two Units and two Instances draw");
        }

        #[test]
        fn three_units_draw_nothing() {
            let s = after_play(false, json!([UNIT]));
            assert_eq!(instances(&s).len(), 2);
            assert!(s.hand(P1).is_empty(), "one Unit and two Instances draw nothing");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn two_radiant_instances_and_two_draws_at_four_units() {
            let s = after_play(true, json!([UNIT, UNIT]));
            assert_eq!(instances(&s).len(), 2);
            for instance in nonempty(instances(&s), "the Instances") {
                assert!(instance.radiant);
            }
            assert_eq!(s.hand(P1).len(), 2);
        }
    }
}
