//! SPEC §8.13 IA #111 Redundant Copy ((1) Trap, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reveals when a friendly Unit dies: Summon a copy of it."
//!   Radiant: "Reveals when a friendly Unit dies: Summon a copy of it with +{buff}/+{buff}." (buff 2)
//!
//! C #14 Shadowstep's trigger shape: the trap answers its controller's Unit deaths (`is_unit_death`,
//! R4423000), summoning a fresh copy of the card that died on the face it died with (R4577002),
//! with no Cry (R1). The Radiant face then buffs every copy the firing summoned, S Z'Zoth's shape
//! (`summoned_so_far`, R4589001's one pick). A backrow Trap fires once; the Field Trap IA #117
//! fires on every death.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-111";

fn redundant_copy(face_radiant: bool) -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "redundant-copy",
            &[GameEventType::Destroyed],
            move |ctx, event| {
                let view: &EffectContext = ctx;
                let GameEvent::Destroyed {
                    def_id, radiant, ..
                } = event
                else {
                    return vec![];
                };
                let mut effects = vec![summon(json_as(json!({
                    "defId": def_id.clone(),
                    "radiant": *radiant == Some(true),
                })))];
                if face_radiant {
                    let gain = param(view, "buff");
                    effects.push(for_each_card(ForEachCardArgs {
                        cards: Arc::new(|ctx: &mut EffectContext<'_>| summoned_so_far(ctx)),
                        each: Arc::new(move |instance_id: &str| {
                            buff(json_as(json!({
                                "target": { "of": "instance", "instanceId": instance_id },
                                "attack": gain,
                                "health": gain,
                            })))
                        }),
                    }));
                }
                effects
            },
        )
        .with_when(|ctx, event| {
            matches!(event, GameEvent::Destroyed { controller, .. } if *controller == ctx.controller)
                && is_unit_death(ctx.state, event)
        })],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: redundant_copy(false),
        radiant: redundant_copy(true),
    }
}

// IA #111 Redundant Copy — SPEC §8.13 row 111.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const FODDER: &str = "stand-in (1) 2/2";
    const FILLER: &str = "stand-in (0) Spell";

    /// The Copy set face-down behind p1's Fodder; p1 kills the Fodder.
    fn copied(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [HIT_JOB],
                "field": [FODDER],
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": false, "lane": 1 }],
                "library": crate::filler(3),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(3) },
        }));
        let fodder = s.unit_at(P1, 1);
        s.play(HIT_JOB, pick::targets(pick::instance(&fodder)));
        s
    }

    fn copies(s: &Scenario) -> Vec<CardInstance> {
        active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == FODDER)
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn reveals_and_summons_a_copy_of_the_unit_that_died() {
            let mut s = copied(false);
            let copies = copies(&s);
            assert_eq!(copies.len(), 1);
            for copy in nonempty(copies, "the copy") {
                s.expect_stats(&copy, json!({ "attack": 2, "health": 2 }));
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn reveals_and_summons_a_copy_with_2_2() {
            let mut s = copied(true);
            let copies = copies(&s);
            assert_eq!(copies.len(), 1);
            for copy in nonempty(copies, "the copy") {
                s.expect_stats(&copy, json!({ "attack": 4, "health": 4 }));
            }
        }
    }
}
