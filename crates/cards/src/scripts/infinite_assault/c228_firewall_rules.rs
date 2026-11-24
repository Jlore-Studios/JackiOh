//! IA #228 Firewall Rules (SPEC §8.13 row 228), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Reveals when an enemy Unit with {threshold} or more Attack attacks: Cancel the
//!            attack."
//!   Radiant: "Reveals when an enemy Unit with {threshold} or more Attack attacks: Cancel the
//!            attack and Freeze the attacker."
//!
//! Any declared enemy attack springs it, but the cancel only lands when the attacker's current
//! Attack — read through the layers at declaration — meets the declared `threshold` (R4594008):
//! a buffed 4-Attack attacker counts as much as a printed 5. A forced attack declares nothing and
//! never fires (R53); its own controller's attacks never do. It fires every time (R2657001).
//!
//! Engine: `cancel_attack` on `attackDeclared`, `freeze` (R4594008). Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-228";

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

fn rules(radiant: bool) -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "firewall-rules",
            &[GameEventType::AttackDeclared],
            move |ctx, event| {
                let Some(attacker) = answered_attacker(ctx, event) else {
                    return vec![];
                };
                // R4594008: the attacker's Attack through the layers at declaration.
                let strong = find_instance(&*ctx.state, &attacker).is_some_and(|card| {
                    unit_view(&*ctx.state, card).attack >= param(&*ctx, "threshold")
                });
                if !strong {
                    return vec![];
                }
                let mut effects = vec![cancel_attack(CancelAttackArgs {
                    destroy_attacker: None,
                })];
                if radiant {
                    effects.push(freeze(json_as(json!({
                        "target": { "of": "instance", "instanceId": attacker },
                    }))));
                }
                effects
            },
        )
        .with_when(|ctx, event| answered_attacker(ctx, event).is_some())],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: rules(false), radiant: rules(true) }
}

// IA #228 Firewall Rules — SPEC §8.13 row 228 (R4594008).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const SMALL: &str = "core-008"; // Mr. Vanilla: a 4/4, under the threshold.
    const BIG: &str = "stand-in (5) 5/5"; // At the threshold.
    const HOTFIX: &str = "infiniteassault-006"; // Hotfix: give a friendly Unit +2/+2.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn armed(radiant: bool, seed: &str, p2: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "active": "p2",
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant }],
                "hand": [STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": p2,
        }))
    }

    fn canceled(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::AttackCancelled { .. }))
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn r4594008_a_5_attack_attack_is_cancelled_a_4_attack_lands() {
            let mut s = armed(
                false,
                "firewall-threshold",
                json!({ "field": [BIG, SMALL], "hand": [STOCKPILE], "library": crate::filler(4) }),
            );
            let hero = s.state().players[P1].hero.health;
            s.attack(BIG, "hero");
            assert_eq!(canceled(&s), 1, "the 5-Attack attack is cancelled");
            assert_eq!(s.state().players[P1].hero.health, hero, "nothing lands");
            s.attack(SMALL, "hero");
            assert_eq!(s.state().players[P1].hero.health, hero - 4, "the 4-Attack attack lands");
        }

        #[test]
        fn r4594008_a_buff_lifting_to_5_counts() {
            let mut s = armed(
                false,
                "firewall-buffed",
                json!({ "field": [SMALL], "hand": [HOTFIX, STOCKPILE], "library": crate::filler(4) }),
            );
            let vanilla = s.unit_at(P2, 1).id.clone();
            s.play(HOTFIX, json!({ "targets": pick::id(&vanilla) }));
            let hero = s.state().players[P1].hero.health;
            s.attack(SMALL, "hero");
            assert_eq!(canceled(&s), 1, "6 Attack through the layers counts");
            assert_eq!(s.state().players[P1].hero.health, hero, "nothing lands");
        }

        #[test]
        fn r4594008_it_fires_every_time() {
            let mut s = armed(
                false,
                "firewall-again",
                json!({ "field": [BIG, BIG], "hand": [STOCKPILE], "library": crate::filler(4) }),
            );
            // One Unit cannot act twice a turn: two attackers, two answers.
            s.attack(BIG, "hero");
            let second = s.unit_at(P2, 2);
            s.attack(second, "hero");
            assert_eq!(canceled(&s), 2, "both attacks cancelled");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4594008_the_attack_is_cancelled_and_the_attacker_frozen() {
            let mut s = armed(
                true,
                "firewall-radiant",
                json!({ "field": [BIG], "hand": [STOCKPILE], "library": crate::filler(4) }),
            );
            s.attack(BIG, "hero");
            assert_eq!(canceled(&s), 1, "the attack is cancelled");
            let attacker = s.unit_at(P2, 1);
            assert!(s.card(&attacker.id).frozen.is_some(), "the attacker is frozen");
        }
    }
}
