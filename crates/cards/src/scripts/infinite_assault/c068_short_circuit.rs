//! SPEC §8.13 IA #68 Short Circuit ((2) Trap, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reveals when an enemy Unit attacks: Destroy it. Deal damage equal to its Attack to your hero."
//!   Radiant: "Reveals when an enemy Unit attacks: Destroy it. Deal damage equal to half its Attack, rounded down, to your hero."
//!
//! S T-AI-4 Predictive Shield's window (§4.2 step 4) on `attackDeclared`: any declared enemy attack
//! answers it, whatever it targets, forced attacks excepted (R1881002), and its controller's own
//! attacks leave it set. The Attack is read as it reveals, before the destroy (R4587003), then the
//! attacker is destroyed and its controller's hero takes that Attack — half of it, rounded down,
//! on the Radiant face. The hero is hit even if the destroy fails (Indestructible), and the
//! destroy stops the fight (R220).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-068";

/// The base face hits back the whole Attack.
const BASE_DIVISOR: i32 = 1;

/// The Radiant face hits back half the Attack, rounded down.
const RADIANT_DIVISOR: i32 = 2;

/// The attacker and its Attack of a declared enemy attack this trap answers: S T-AI-4's guarded
/// attacker with no guarded target — any target counts — and the Attack read up front (R4587003).
fn short_circuit_attacker(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<(String, i32)> {
    let GameEvent::AttackDeclared {
        attacker_id, forced, ..
    } = event
    else {
        return None;
    };
    if *forced {
        return None;
    }
    let attacker = find_instance(&*ctx.state, attacker_id)?;
    if attacker.controller == ctx.controller {
        return None;
    }
    let attack = unit_view(ctx.state, &attacker).attack;
    Some((attacker_id.clone(), attack))
}

fn short_circuit(divisor: i32) -> Script {
    Script {
        triggers: vec![
            TriggerDef::new(
                "short-circuit",
                &[GameEventType::AttackDeclared],
                move |ctx, event| {
                    let Some((attacker_id, attack)) = short_circuit_attacker(ctx, event) else {
                        return vec![];
                    };
                    let mut effects = vec![destroy(json_as(json!({
                        "target": { "of": "instance", "instanceId": attacker_id },
                    })))];
                    // R4587003: the hero is hit even if the destroy fails — the hit is its own
                    // effect, after the destroy; R220 stops the fight either way.
                    let back = attack / divisor;
                    if back > 0 {
                        effects.push(damage(json_as(json!({
                            "to": { "of": "selfHero" },
                            "amount": back,
                        }))));
                    }
                    effects
                },
            )
            .with_when(|ctx, event| short_circuit_attacker(ctx, event).is_some()),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: short_circuit(BASE_DIVISOR),
        radiant: short_circuit(RADIANT_DIVISOR),
    }
}

// IA #68 Short Circuit — SPEC §8.13 row 68 (R4587003).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: a card in hand, so no turn ends by itself
    const ATTACKER: &str = "stand-in (1) 5/5";

    fn set(radiant: bool) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant }],
                "hand": [FILLER],
                "library": crate::filler(4),
            },
            "p2": { "field": [ATTACKER], "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r4587003_the_attacker_is_destroyed_and_its_attack_hits_your_hero() {
            let mut s = set(false);
            let attacker = s.unit_at(P2, 1);
            s.attack(&attacker, "hero");
            // The attack never fought: the hero takes the 5 back, not the attack.
            s.expect_in_zone(&attacker, "graveyard");
            s.expect_health(P1, HERO_HEALTH - 5);
            s.expect_health(P2, HERO_HEALTH);
        }

        #[test]
        fn an_attack_on_your_unit_reveals_it_too() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": {
                    "backrow": [ID],
                    "field": ["stand-in (1) 1/1"],
                    "hand": [FILLER],
                    "library": crate::filler(4),
                },
                "p2": { "field": [ATTACKER], "hand": [FILLER], "library": crate::filler(4) },
            }));
            let attacker = s.unit_at(P2, 1);
            let victim = s.unit_at(P1, 1);
            s.attack(&attacker, victim);
            s.expect_in_zone(&attacker, "graveyard");
            s.expect_health(P1, HERO_HEALTH - 5);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4587003_the_attacker_is_destroyed_and_half_its_attack_hits_your_hero() {
            let mut s = set(true);
            let attacker = s.unit_at(P2, 1);
            s.attack(&attacker, "hero");
            s.expect_in_zone(&attacker, "graveyard");
            // Half of 5, rounded down.
            s.expect_health(P1, HERO_HEALTH - 2);
        }
    }
}
