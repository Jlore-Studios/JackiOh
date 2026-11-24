//! SPEC §8.13 IA #148 Quarantine ((1) Trap, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reveals when an enemy Unit attacks: Cancel the attack. The attacker has {attack} Attack this turn." (attack 1)
//!   Radiant: "Reveals when an enemy Unit attacks: Cancel the attack and Freeze the attacker. The attacker has
//!            {attack} Attack this turn." (attack 1)
//!
//! Engine: §4.2 step 4's trap window, on `attackDeclared`: an enemy Unit's declared attack on any
//! target (R121: a forced attack declares nothing and springs nothing). Then `cancel_attack` (R44),
//! the Radiant face's Freeze of the attacker (which thaws at the cleanup of its controller's next
//! turn, R1174000), and a set Attack this turn to the declared {attack} (R4591000). Every condition
//! is in `when`, so any other attack leaves it set (R61).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-148";

/// The attacker of an enemy Unit's declared attack this trap answers.
fn quarantined_attacker(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<String> {
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
    Some(attacker_id.clone())
}

fn quarantine(radiant: bool) -> Script {
    let reveal = TriggerDef::new("quarantine", &[GameEventType::AttackDeclared], move |ctx, event| {
        let Some(attacker) = quarantined_attacker(ctx, event) else {
            return vec![];
        };
        let mut effects = vec![cancel_attack(CancelAttackArgs {
            destroy_attacker: None,
        })];
        if radiant {
            effects.push(freeze(json_as(json!({
                "target": { "of": "instance", "instanceId": attacker }
            }))));
        }
        effects.push(set_attack_this_turn(json_as(json!({
            "target": { "of": "instance", "instanceId": attacker },
            "attack": param(&*ctx, "attack"),
        }))));
        effects
    })
    .with_when(move |ctx, event| quarantined_attacker(ctx, event).is_some());
    Script {
        triggers: vec![reveal],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: quarantine(false),
        radiant: quarantine(true),
    }
}

// IA #148 Quarantine — SPEC §8.13 row 148 (R44, R61, R121, R1174000, R4591000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const QUARANTINE: &str = "infiniteassault-148";
    const ATTACKER: &str = "stand-in (2) 3/3";
    const VANILLA: &str = "stand-in (1) 2/5";
    const MOTHS: &str = "core-009";

    fn set(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "quarantine",
            "active": "p2",
            "p1": {
                "backrow": [{ "def": QUARANTINE, "radiant": radiant }],
                "field": [VANILLA],
                "hand": [filler(1)],
                "library": filler(6),
            },
            "p2": { "field": [ATTACKER], "hand": [filler(1)], "library": filler(6) },
        }))
    }

    fn fired(s: &Scenario) -> bool {
        s.events().iter().any(|event| matches!(event, GameEvent::TrapFired { .. }))
    }

    fn canceled(s: &Scenario) -> bool {
        s.events().iter().any(|event| matches!(event, GameEvent::AttackCancelled { .. }))
    }

    mod base {
        use super::*;

        #[test]
        fn r4591000_quarantine_cancels_and_leaves_the_attacker_1_attack() {
            let mut s = set(false);
            let attacker = s.unit_at(P2, 1);
            let target = s.unit_at(P1, 1);
            s.attack(&attacker, &target);
            assert!(fired(&s));
            assert!(canceled(&s));
            s.expect_stats(&s.unit_at(P2, 1), json!({ "attack": 1, "health": 3 }));
            s.expect_stats(&s.unit_at(P1, 1), json!({ "health": 5 }));
            assert!(s.card(attacker.id.as_str()).frozen.is_none());
            s.expect_in_zone(QUARANTINE, "graveyard");
        }

        #[test]
        fn an_attack_on_the_hero_springs_it_too() {
            let mut s = set(false);
            let attacker = s.unit_at(P2, 1);
            s.attack(&attacker, "hero");
            assert!(fired(&s));
            assert!(canceled(&s));
            s.expect_health(P1, 30);
        }

        #[test]
        fn the_set_attack_ends_with_the_turn() {
            let mut s = set(false);
            let attacker = s.unit_at(P2, 1);
            let target = s.unit_at(P1, 1);
            s.attack(&attacker, &target);
            s.expect_stats(&s.unit_at(P2, 1), json!({ "attack": 1 }));
            s.end_turn();
            s.end_turn();
            s.expect_stats(&s.unit_at(P2, 1), json!({ "attack": 3 }));
        }

        #[test]
        fn r121_a_forced_attack_springs_nothing() {
            let mut s = scenario(json!({
                "seed": "quarantine-forced",
                "active": "p2",
                "p1": {
                    "backrow": [QUARANTINE],
                    "field": [MOTHS],
                    "hand": [filler(1)],
                    "library": filler(6),
                },
                "p2": { "field": [ATTACKER], "hand": [filler(1)], "library": filler(6) },
            }));
            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert!(
                s.last_events().iter().any(
                    |event| matches!(event, GameEvent::Damage { target_id, .. } if *target_id == s.card(MOTHS).id)
                ),
                "the forced attack happened"
            );
            assert!(!fired(&s));
            assert!(!canceled(&s));
            s.expect_in_zone(QUARANTINE, "field");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1174000_it_freezes_the_attacker_too() {
            let mut s = set(true);
            let attacker = s.unit_at(P2, 1);
            let target = s.unit_at(P1, 1);
            s.attack(&attacker, &target);
            assert!(fired(&s));
            assert!(canceled(&s));
            s.expect_stats(&s.unit_at(P2, 1), json!({ "attack": 1, "health": 3 }));
            assert!(s.card(attacker.id.as_str()).frozen.is_some(), "the attacker is Frozen");
        }
    }
}
