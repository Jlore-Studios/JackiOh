//! IA #209 Takeoff (SPEC §8.13 row 209), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Double a friendly Unit's Attack and Health."
//!   Radiant: "Double a friendly Unit's Attack and Health. It has Rush."
//!
//! Engine: one declared friendly Unit, aimed to help (R656); a Cry (R1): a permanent `buff` of
//! its Attack and Health now, read through the layers as the Cry resolves (§10.4), capped so no
//! multiplied stat passes `MULTIPLIED_STAT_CAP` (R4079000); the Radiant face then `grant_keyword`s
//! it Rush. Built by C9 (#4593).

use jackioh_engine::effects::{buff, grant_keyword};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-209";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Help),
        ..TargetDecl::target(1, 1, json!({ "side": "ally", "of": ["unit"] }))
    }]
}

fn takeoff(radiant_face: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let Some(unit) = instance_of(ctx, &json_as::<TargetSpec>(json!({ "of": "chosen" })))
            else {
                return vec![];
            };
            let view = unit_view(ctx.state, &unit);
            let attack = view.attack.max(0);
            let gain_attack = attack.min(MULTIPLIED_STAT_CAP - attack).max(0);
            let health = view.health.max(0);
            let gain_health = health.min(MULTIPLIED_STAT_CAP - health).max(0);
            let mut effects = vec![buff(json_as(json!({
                "target": { "of": "chosen" },
                "attack": gain_attack,
                "health": gain_health,
            })))];
            if radiant_face {
                effects.push(grant_keyword(json_as(json!({
                    "target": { "of": "chosen" },
                    "keyword": { "kind": "Rush" },
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: takeoff(false),
        radiant: takeoff(true),
    }
}

// IA #209 Takeoff — SPEC §8.13 row 209.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const UNIT: &str = "stand-in (1) 3/4";
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn cast(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "field": [UNIT],
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        let unit = s.unit_at(P1, 1);
        s.play(ID, pick::targets(pick::instance(&unit)));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn doubles_attack_and_health() {
            let s = cast(false, "takeoff-base");
            let unit = s.unit_at(P1, 1);
            let view = unit_view(s.state(), &unit);
            assert_eq!((view.attack, view.health), (6, 8));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn doubles_attack_and_health_and_gives_rush() {
            let s = cast(true, "takeoff-radiant");
            let unit = s.unit_at(P1, 1);
            let view = unit_view(s.state(), &unit);
            assert_eq!((view.attack, view.health), (6, 8));
            assert!(
                card_keywords(s.state(), &unit).iter().any(|keyword| *keyword == Keyword::Rush),
                "it has Rush"
            );
        }
    }
}
