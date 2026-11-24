//! IA #217 Echo-1, First Hallucination (SPEC §8.13 row 217), a card of the held Infinite Assault
//! set (R4577000).
//!   Base:    "Cloaked. Start of turn: Set a Figment. Whenever a Figment of yours reveals, deal
//!            {damage} damage to the attacker." (3)
//!   Radiant: "Cloaked. Start of turn: Set two Figments. Whenever a Figment of yours reveals, deal
//!            {damage} damage to the attacker." (6)
//!
//! Cloaked is catalog data. The Start of turn sets one Figment (two Radiant ones on the Radiant
//! face, `set_figments`, R4586000). A Figment of its own revealing is an ordinary trigger that runs
//! after the combat (§10.3): this answers its `trapFired` — one of mine, the Figment's, no refire
//! (`again`, R2657001) — and the firing names the attack it answered (`attackerId`, R4586007). The
//! hit lands only on an attacker still on the field (R57); a firing with no attacker to hit, or an
//! enemy Figment's, answers nothing.
//!
//! Engine: a Figment `summon` on `startOfTurn`, `damage` on `trapFired` (R4594000). Built by C10
//! (#4594).

use jackioh_engine::prelude::*;

use crate::scripts::infinite_assault::c025_1_figment::set_figments;

pub const ID: &str = "infiniteassault-217";

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

fn echo(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| set_figments(if radiant { 2 } else { 1 }, radiant))),
        triggers: vec![TriggerDef::new(
            "echo-1",
            &[GameEventType::TrapFired],
            move |ctx, event| {
                let Some(attacker) = figment_reveal(ctx, event) else {
                    return vec![];
                };
                // R57: only an attacker still on the field is hit.
                let standing = find_instance(&*ctx.state, &attacker)
                    .is_some_and(|card| card.zone.z() == ZoneName::Field);
                if !standing {
                    return vec![];
                }
                vec![damage(json_as(json!({
                    "to": { "of": "instance", "instanceId": attacker },
                    "amount": param(&*ctx, "damage"),
                })))]
            },
        )
        .with_when(|ctx, event| figment_reveal(ctx, event).is_some())],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: echo(false), radiant: echo(true) }
}

// IA #217 Echo-1, First Hallucination — SPEC §8.13 row 217 (R4594000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::{FIGMENT, ID};
    use crate::scenario;

    const ATTACKER: &str = "core-008"; // Mr. Vanilla: a 4/4.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const BLOCKER: &str = "stand-in (3) 5/5"; // Kills the attacker in combat.

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

    fn attacker_health(s: &Scenario, player: PlayerId) -> i32 {
        unit_view(s.state(), &s.unit_at(player, 1)).health
    }

    fn figments(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.state().players[player]
            .backrow
            .iter()
            .filter_map(|slot| slot.clone())
            .filter(|card| card.def_id == FIGMENT)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4594000_a_figments_reveal_hits_the_attacker_for_3() {
            let mut s = standing(false, "echo-hit");
            assert_eq!(attacker_health(&s, P2), 4);
            s.attack(ATTACKER, "hero");
            assert_eq!(attacker_health(&s, P2), 1, "3 damage on the 4-health attacker");
            s.expect_in_zone(FIGMENT, "graveyard");
        }

        #[test]
        fn r4594000_a_dead_attacker_takes_nothing() {
            let mut s = scenario(json!({
                "seed": "echo-dead",
                "active": "p2",
                "p1": {
                    "field": [{ "def": ID }, BLOCKER],
                    "backrow": [FIGMENT],
                    "hand": [STOCKPILE],
                    "library": crate::filler(4),
                },
                "p2": { "field": [ATTACKER], "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            // The attacker dies in combat; the Figment still fires, and the answer fizzles.
            s.attack(ATTACKER, BLOCKER);
            s.expect_in_zone(ATTACKER, "graveyard");
            s.expect_in_zone(FIGMENT, "graveyard");
        }

        #[test]
        fn r4594000_an_enemy_figments_reveal_answers_nothing() {
            let mut s = scenario(json!({
                "seed": "echo-enemy",
                "active": "p1",
                "p1": {
                    "field": [{ "def": ID }, ATTACKER],
                    "hand": [STOCKPILE],
                    "library": crate::filler(4),
                },
                "p2": { "backrow": [FIGMENT], "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            let hero = s.state().players[P2].hero.health;
            s.attack(ATTACKER, "hero");
            assert_eq!(attacker_health(&s, P1), 4, "no answer on an enemy Figment's reveal");
            assert_eq!(s.state().players[P2].hero.health, hero - 4, "the attack still lands");
        }

        #[test]
        fn start_of_turn_sets_a_figment() {
            let mut s = scenario(json!({
                "seed": "echo-set",
                "active": "p1",
                "p1": { "field": [{ "def": ID }], "hand": [STOCKPILE], "library": crate::filler(4) },
                "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            s.start_turn();
            assert_eq!(figments(&s, P1).len(), 1, "one Figment");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4594000_a_figments_reveal_hits_the_attacker_for_6() {
            let mut s = standing(true, "echo-radiant-hit");
            s.attack(ATTACKER, "hero");
            s.expect_in_zone(ATTACKER, "graveyard");
        }

        #[test]
        fn start_of_turn_sets_two_figments() {
            let mut s = scenario(json!({
                "seed": "echo-radiant-set",
                "active": "p1",
                "p1": { "field": [{ "def": ID, "radiant": true }], "hand": [STOCKPILE], "library": crate::filler(4) },
                "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            s.start_turn();
            let set = figments(&s, P1);
            assert_eq!(set.len(), 2, "two Figments");
            assert!(set.iter().all(|card| card.radiant), "Radiant ones");
        }
    }
}
