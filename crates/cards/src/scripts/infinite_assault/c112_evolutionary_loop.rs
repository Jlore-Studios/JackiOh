//! SPEC §8.13 IA #112 Evolutionary Loop ((2) Unit, Rare, 2/3 → 4/6), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Evolves when a friendly Unit dies.\nEvolve: Gain +{buff}/+{buff} and Reborn." (buff 2)
//!   Radiant: "Evolves when a friendly Unit dies.\nEvolve: Gain +{buff}/+{buff} and Reborn." (buff 4)
//!
//! S+ #93 Recessive Replicators' trigger without its `remember`: on the death of a friendly Unit
//! that is not this card (`is_unit_death`, R4423000), while this card is not yet evolved, evolve it.
//! The Evolve hook buffs it with the declared `buff` and grants it Reborn (`grant_keyword`).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-112";

fn evolutionary() -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "evolutionary-mourn",
            &[GameEventType::Destroyed],
            move |ctx, event| {
                let view: &EffectContext = ctx;
                let GameEvent::Destroyed {
                    instance_id,
                    controller,
                    ..
                } = event
                else {
                    return vec![];
                };
                let Some(this) = view.live_self() else {
                    return vec![];
                };
                if *controller != view.controller
                    || *instance_id == this.id
                    || this.evolved == Some(true)
                    || !is_unit_death(&*view.state, event)
                {
                    return vec![];
                }
                vec![evolve(json_as(json!({ "target": { "of": "self" }})))]
            },
        )
        .with_when(|ctx, event| {
            let GameEvent::Destroyed {
                instance_id,
                controller,
                ..
            } = event
            else {
                return false;
            };
            ctx.live_self().is_some_and(|me| {
                *controller == ctx.controller
                    && *instance_id != me.id
                    && me.evolved != Some(true)
                    && is_unit_death(ctx.state, event)
            })
        })],
        evolve: Some(hook(|ctx| {
            let view: &EffectContext = ctx;
            let gain = param(view, "buff");
            vec![
                buff(json_as(json!({
                    "target": { "of": "self" },
                    "attack": gain,
                    "health": gain,
                }))),
                grant_keyword(json_as(json!({
                    "target": { "of": "self" },
                    "keyword": { "kind": "Reborn" },
                }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: evolutionary(),
        radiant: evolutionary(),
    }
}

// IA #112 Evolutionary Loop — SPEC §8.13 row 112.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const FODDER: &str = "stand-in (1) 2/2";
    const FILLER: &str = "stand-in (0) Spell";

    /// p1's Loop and Fodder stand; p1 kills the Fodder.
    fn evolved(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [HIT_JOB],
                "field": [{ "def": ID, "radiant": radiant }, FODDER],
                "library": crate::filler(3),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(3) },
        }));
        let fodder = s.unit_at(P1, 2);
        s.play(HIT_JOB, pick::targets(pick::instance(&fodder)));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn a_friendly_death_evolves_it_2_2_with_reborn() {
            let mut s = evolved(false);
            let evolved = s.unit_at(P1, 1);
            assert_eq!(evolved.evolved, Some(true));
            s.expect_stats(&evolved, json!({ "attack": 4, "health": 5 }));
            assert!(has_keyword(&s.stats(&evolved).keywords, KeywordKind::Reborn));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_friendly_death_evolves_it_4_4_with_reborn() {
            let mut s = evolved(true);
            let evolved = s.unit_at(P1, 1);
            assert_eq!(evolved.evolved, Some(true));
            s.expect_stats(&evolved, json!({ "attack": 8, "health": 10 }));
            assert!(has_keyword(&s.stats(&evolved).keywords, KeywordKind::Reborn));
        }
    }
}
