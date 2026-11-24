//! IA #196 Emergent Behavior (SPEC §8.13 row 196), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Give a friendly Unit +{buff}/+{buff} for each turn you've started this game."
//!   Radiant: "Give a friendly Unit +{buff}/+{buff} for each turn you've started this game."
//!
//! Engine: one declared friendly Unit, aimed to help (R656); a Cry (R1): a permanent `buff` of the
//! declared `buff` times the caster's `turns_started_of`, the current turn included (R4593001).
//! Built by C9 (#4593).

use jackioh_engine::effects::buff;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-196";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Help),
        ..TargetDecl::target(1, 1, json!({ "side": "ally", "of": ["unit"] }))
    }]
}

fn emergent() -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            let turns = turns_started_of(ctx.state, ctx.controller);
            let gain = param(&*ctx, "buff") * turns;
            vec![buff(json_as(json!({
                "target": { "of": "chosen" },
                "attack": gain,
                "health": gain,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's +2/+2 is its declared `buff`.
    let radiant = emergent();
    CardScripts {
        base: emergent(),
        radiant,
    }
}

// IA #196 Emergent Behavior — SPEC §8.13 row 196 (R4593001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    /// On p1's third started turn (turn 5), casting at the 4/4: the current turn counts.
    fn cast(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "turn": 5,
            "active": "p1",
            "p1": {
                "field": [VANILLA],
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        assert_eq!(turns_started_of(s.state(), P1), 3);
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
        fn r4593001_gives_1_1_for_each_turn_started() {
            let s = cast(false, "emergent-base");
            assert_eq!(stats(&s), (7, 7));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4593001_gives_2_2_for_each_turn_started() {
            let s = cast(true, "emergent-radiant");
            assert_eq!(stats(&s), (10, 10));
        }
    }
}
