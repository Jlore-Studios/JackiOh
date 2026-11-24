//! IA #46 Mirage Sovereign (SPEC §8.13 row 46), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Cloaked. Whenever a Figment of yours reveals, summon a copy of the attacking Unit
//!            for yourself."
//!   Radiant: "Cloaked. Whenever a Figment of yours reveals, summon a copy of the attacking Unit
//!            for yourself, then destroy the attacking Unit."
//!
//! Cloaked is catalog data. A Figment's own firing is an ordinary trigger that runs after the
//! combat (§10.3): this answers its `trapFired` — one of mine, the Figment's, no refire
//! (`again`, R2657001) — and the firing names the attack it answered (`attackerId`, R4586007).
//! The copy is made only of an attacker still on the field (R57); the Radiant face then destroys
//! it. A reveal with no attacker to name summons nothing.
//!
//! Engine: `summon_copy`, `destroy` (R4586007). Built by C2 (#4586).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-046";

/// The Figment whose reveal this answers.
const FIGMENT: &str = "infiniteassault-025-1";

/// A reveal of one of mine Figments that names its attacker.
fn figment_reveal(ctx: &mut EffectContext<'_>, event: &GameEvent) -> Option<String> {
    let GameEvent::TrapFired {
        controller,
        def_id,
        again,
        attacker_id,
        ..
    } = event
    else {
        return None;
    };
    if *controller != ctx.controller || *def_id != FIGMENT || *again == Some(true) {
        return None;
    }
    attacker_id.clone()
}

fn sovereign(destroy_attacker: bool) -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "mirage-sovereign",
            &[GameEventType::TrapFired],
            move |ctx, event| {
                let Some(attacker) = figment_reveal(ctx, event) else {
                    return vec![];
                };
                // R57: only an attacker still on the field is copied.
                let standing = find_instance(&*ctx.state, &attacker)
                    .is_some_and(|card| card.zone.z() == ZoneName::Field);
                if !standing {
                    return vec![];
                }
                let mut effects = vec![summon_copy(json_as(json!({
                    "of": { "of": "instance", "instanceId": attacker },
                    "player": "self",
                })))];
                if destroy_attacker {
                    effects.push(destroy(json_as(json!({
                        "target": { "of": "instance", "instanceId": attacker },
                    }))));
                }
                effects
            },
        )
        .with_when(|ctx, event| figment_reveal(ctx, event).is_some())],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: sovereign(false),
        radiant: sovereign(true),
    }
}

// IA #46 Mirage Sovereign — SPEC §8.13 row 46 (R4586007).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::{FIGMENT, ID};
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const ATTACKER: &str = "core-008"; // Mr. Vanilla: a 4/4.

    fn standing(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "active": "p2",
            "p1": {
                "field": [{ "def": ID, "radiant": radiant }],
                "backrow": [FIGMENT],
                "hand": [STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": { "field": [ATTACKER], "hand": [STOCKPILE], "library": crate::filler(4) },
        }))
    }

    fn copies(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .filter(|card| card.def_id == ATTACKER)
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4586007_a_figments_reveal_summons_a_copy_of_the_attacker() {
            let mut s = standing(false, "sovereign-base");
            s.attack(ATTACKER, "hero");
            let landed = copies(&s);
            assert_eq!(landed.len(), 1, "one copy of the attacker");
            s.expect_in_zone(FIGMENT, "graveyard");
            assert_eq!(s.state().players[P2].units.iter().flatten().flatten().count(), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586007_the_copy_lands_and_the_attacker_is_destroyed() {
            let mut s = standing(true, "sovereign-radiant");
            let victim = s.unit_at(P2, 1).id.clone();
            s.attack(ATTACKER, "hero");
            assert_eq!(copies(&s).len(), 1);
            s.expect_in_zone(&victim, "graveyard");
        }
    }
}
