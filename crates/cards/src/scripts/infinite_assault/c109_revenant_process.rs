//! SPEC §8.13 IA #109 Revenant Process ((2) Unit, Rare, 4/2 → 8/4), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reborn\nWhen this returns by Reborn, it gains +{buff}/+{buff}." (buff 3)
//!   Radiant: "Reborn\nWhen this returns by Reborn, it gains +{buff}/+{buff}." (buff 6)
//!
//! S T-Drone Corvette's `arrived` shape on `Summoned`: the trigger fires only when the event names
//! its own id and the live card came back by Reborn (`returned_by_reborn`, R4589004 — `Summoned.via`
//! never reaches a trigger, R2783001). The return sets the body to 1 Health first, so a 4/2 comes
//! back with 7 Attack and 4 of 5 Health: the buff lands after the return.

use jackioh_engine::effects::buff;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-109";

fn process() -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "revenant-return",
            &[GameEventType::Summoned],
            move |ctx, event| {
                let view: &EffectContext = ctx;
                let GameEvent::Summoned { instance_id, .. } = event else {
                    return vec![];
                };
                let Some(me) = view.live_self() else {
                    return vec![];
                };
                if *instance_id != me.id || !returned_by_reborn(me) {
                    return vec![];
                }
                let gain = param(view, "buff");
                vec![buff(json_as(json!({
                    "target": { "of": "self" },
                    "attack": gain,
                    "health": gain,
                })))]
            },
        )
        .with_when(|ctx, event| {
            let GameEvent::Summoned { instance_id, .. } = event else {
                return false;
            };
            ctx.live_self().is_some_and(|me| {
                *instance_id == me.id && returned_by_reborn(me)
            })
        })],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: process(),
        radiant: process(),
    }
}

// IA #109 Revenant Process — SPEC §8.13 row 109 (R4589004).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill

    /// p1 plays the Revenant, then kills it.
    fn returned(radiant: bool) -> (Scenario, CardInstance) {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, HIT_JOB],
                "library": crate::filler(3),
                "mana": 10,
            },
            "p2": { "library": crate::filler(3) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        let revenant = s.unit_at(P1, 1);
        s.play(HIT_JOB, pick::targets(pick::instance(&revenant)));
        let back = s.unit_at(P1, 1);
        (s, back)
    }

    mod base {
        use super::*;

        #[test]
        fn r4589004_gains_on_its_reborn_return() {
            let (mut s, back) = returned(false);
            // A 4/2 back at 1 Health, then +3/+3: 7 Attack and 4 of 5 Health.
            s.expect_stats(&back, json!({ "attack": 7, "health": 4 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4589004_radiant_gains_on_its_reborn_return() {
            let (mut s, back) = returned(true);
            // An 8/4 back at 1 Health, then +6/+6: 14 Attack and 7 of 10 Health.
            s.expect_stats(&back, json!({ "attack": 14, "health": 7 }));
        }
    }
}
