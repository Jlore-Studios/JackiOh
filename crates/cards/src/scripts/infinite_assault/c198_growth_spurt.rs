//! IA #198 Growth Spurt (SPEC §8.13 row 198), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Give a friendly Unit +{buff}/+{buff}. It gains +{gain}/+{gain} at the end of each of
//!            your turns."
//!   Radiant: "Give a friendly Unit +{buff}/+{buff}. It gains +{gain}/+{gain} at the end of each of
//!            your turns."
//!
//! Engine: one declared friendly Unit, aimed to help (R656); a Cry (R1): a permanent `buff` of the
//! declared `buff`, plus the caster's watched `delay` at the end of each of their turns (the cast
//! turn included, re-armed each time it fires), growing that Unit by the declared `gain`
//! (R4593003). It ends when the Unit leaves the field (R174) and outlasts a control change.
//! Built by C9 (#4593).

use jackioh_engine::effects::{buff, delay};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-198";

/// The delayed step, which grows the Unit and re-arms itself.
const STEP: &str = "grow";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Help),
        ..TargetDecl::target(1, 1, json!({ "side": "ally", "of": ["unit"] }))
    }]
}

fn grow_delay(instance_id: &str, gain: i32) -> Effect {
    delay(json_as(json!({
        "at": { "phase": "end", "player": "self" },
        "step": STEP,
        "data": { "instanceId": instance_id, "gain": gain },
        "watch": instance_id,
    })))
}

fn growth() -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            let Some(unit) = instance_of(ctx, &TargetSpec::Chosen { index: None }) else {
                return vec![];
            };
            let now = param(&*ctx, "buff");
            let gain = param(&*ctx, "gain");
            vec![
                buff(json_as(json!({
                    "target": { "of": "chosen" },
                    "attack": now,
                    "health": now,
                }))),
                grow_delay(&unit.id, gain),
            ]
        })),
        delayed: Some(hook(|ctx| {
            let Some(id) = ctx.data.get("instanceId").and_then(Value::as_str) else {
                return vec![];
            };
            let gain = ctx.data.get("gain").and_then(Value::as_i64).unwrap_or(0) as i32;
            vec![
                buff(json_as(json!({
                    "target": { "of": "instance", "instanceId": id },
                    "attack": gain,
                    "health": gain,
                }))),
                grow_delay(id, gain),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's numbers are its declared `buff` and `gain`.
    let radiant = growth();
    CardScripts {
        base: growth(),
        radiant,
    }
}

// IA #198 Growth Spurt — SPEC §8.13 row 198 (R4593003).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const HIT_JOB: &str = "core-016"; // Hit Job: Destroy a Unit.
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn cast(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "field": [VANILLA],
                "hand": [{ "def": ID, "radiant": radiant }, HIT_JOB, FILLER],
                "library": crate::filler(6),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        let unit = s.unit_at(P1, 1);
        s.play(ID, pick::targets(pick::instance(&unit)));
        s
    }

    fn stats(s: &Scenario) -> (i32, i32) {
        let unit = s.unit_at(P1, 1);
        let view = unit_view(s.state(), &unit);
        (view.attack, view.health)
    }

    mod base {
        use super::*;

        #[test]
        fn r4593003_grows_at_each_own_end_and_stops_when_the_unit_leaves() {
            let mut s = cast(false, "growth-base");
            assert_eq!(stats(&s), (6, 6));
            s.end_turn();
            assert_eq!(stats(&s), (7, 7), "the cast turn's end grows it");
            s.end_turn();
            assert_eq!(stats(&s), (7, 7), "the opponent's end grows nothing");
            s.end_turn();
            assert_eq!(stats(&s), (8, 8));
            s.end_turn();
            let unit = s.unit_at(P1, 1);
            s.play(HIT_JOB, pick::targets(pick::instance(&unit)));
            assert!(s.unit(P1, 1).is_none());
            s.end_turn();
            s.end_turn();
            assert!(s.unit(P1, 1).is_none(), "no regrowth after it leaves");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4593003_grows_twice_as_fast() {
            let mut s = cast(true, "growth-radiant");
            assert_eq!(stats(&s), (8, 8));
            s.end_turn();
            assert_eq!(stats(&s), (10, 10));
            s.end_turn();
            s.end_turn();
            assert_eq!(stats(&s), (12, 12));
        }
    }
}
