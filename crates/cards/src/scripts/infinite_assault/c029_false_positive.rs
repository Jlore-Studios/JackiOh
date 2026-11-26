//! IA #29 False Positive (SPEC §8.13 row 29), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Reveals when an enemy Unit attacks: Cancel the attack and Freeze the attacker."
//!   Radiant: "Reveals when an enemy Unit attacks: Cancel the attack and Freeze the attacker.
//!            Then set a Figment."
//!
//! Any declared enemy attack springs it — on a Unit or the hero — but a forced one declares
//! nothing (R53), and its own controller's attacks never do. The cancel lands first, then the
//! Freeze, then the Radiant face's Figment (`set_figments`). Every condition is in `when`, so any
//! other attack leaves it set (R61).
//!
//! Engine: §4.2 step 4's trap window on `attackDeclared`, `cancel_attack` (R44), `freeze`, and a
//! Figment `summon` (R4586000). Built by C2 (#4586).

use jackioh_engine::prelude::*;

use crate::scripts::infinite_assault::c025_1_figment::set_figments;

pub const ID: &str = "infiniteassault-029";

/// A declared enemy attack's attacker, if this trap answers it.
fn answered_attacker(ctx: &mut EffectContext<'_>, event: &GameEvent) -> Option<String> {
    let GameEvent::AttackDeclared {
        attacker_id,
        forced,
        ..
    } = event
    else {
        return None;
    };
    if *forced {
        return None;
    };
    let attacker = find_instance(&*ctx.state, attacker_id)?;
    (attacker.controller != ctx.controller).then(|| attacker_id.clone())
}

fn positive(figment: bool) -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "false-positive",
            &[GameEventType::AttackDeclared],
            move |ctx, event| {
                let Some(attacker) = answered_attacker(ctx, event) else {
                    return vec![];
                };
                let mut effects = vec![
                    cancel_attack(CancelAttackArgs {
                        destroy_attacker: None,
                    }),
                    freeze(json_as(json!({ "target": { "of": "instance", "instanceId": attacker } }))),
                ];
                if figment {
                    effects.extend(set_figments(1, false));
                }
                effects
            },
        )
        .with_when(|ctx, event| answered_attacker(ctx, event).is_some())],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: positive(false),
        radiant: positive(true),
    }
}

// IA #29 False Positive — SPEC §8.13 row 29 (R4586000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FIGMENT: &str = "infiniteassault-025-1";
    const ATTACKER: &str = "core-008";
    const VANILLA: &str = "core-001";
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn armed(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "active": "p2",
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant }],
                "field": [VANILLA],
                "hand": [STOCKPILE],
                "library": crate::filler(6),
            },
            "p2": { "field": [ATTACKER], "hand": [STOCKPILE], "library": crate::filler(6) },
        }))
    }

    fn figments(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1]
            .backrow
            .iter()
            .filter_map(|slot| slot.clone())
            .filter(|card| card.def_id == FIGMENT)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn an_attack_on_a_unit_is_canceled_and_the_attacker_frozen() {
            let mut s = armed(false, "positive-base");
            s.attack(ATTACKER, VANILLA);
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::AttackCancelled { .. }
            )));
            s.expect_stats(VANILLA, json!({ "health": 7 }));
            let attacker = s.card(ATTACKER).id.clone();
            assert!(s.card(&attacker).frozen.is_some(), "the attacker is Frozen");
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn an_attack_on_the_hero_springs_it_too() {
            let mut s = armed(false, "positive-hero");
            let before = s.state().players[P1].hero.health;
            s.attack(ATTACKER, "hero");
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::AttackCancelled { .. }
            )));
            assert_eq!(s.state().players[P1].hero.health, before);
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn your_own_attack_leaves_it_set() {
            let mut s = scenario(json!({
                "seed": "positive-own",
                "p1": {
                    "backrow": [ID],
                    "field": [ATTACKER],
                    "hand": [STOCKPILE],
                    "library": crate::filler(6),
                },
                "p2": { "field": [VANILLA], "hand": [STOCKPILE], "library": crate::filler(6) },
            }));
            s.attack(ATTACKER, VANILLA);
            s.expect_in_zone(ID, "field");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_radiant_face_sets_a_figment_beside_the_cancel() {
            let mut s = armed(true, "positive-radiant");
            s.attack(ATTACKER, VANILLA);
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::AttackCancelled { .. }
            )));
            let attacker = s.card(ATTACKER).id.clone();
            assert!(s.card(&attacker).frozen.is_some());
            assert_eq!(figments(&s).len(), 1, "then a Figment");
        }
    }
}
