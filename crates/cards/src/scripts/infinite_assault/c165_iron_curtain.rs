//! SPEC §8.13 IA #165 Iron Curtain ((3) Field Trap, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reveals when an enemy Unit attacks: Deal {damage} damage to it." (damage 2)
//!   Radiant: "Reveals when an enemy Unit attacks: Deal {damage} damage to it." (damage 4)
//!
//! Engine: §4.2 step 4's trap window, on `attackDeclared`: an enemy Unit's declared attack on any
//! target (R121: a forced attack declares nothing and springs nothing) takes the declared {damage}
//! as its attacker. A Field Trap is never consumed, so it fires on every declared attack; every
//! condition is in `when`, so any other attack leaves it set (R61).

use jackioh_engine::effects::damage;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-165";

/// The attacker of an enemy Unit's declared attack this trap answers.
fn curtain_attacker(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<String> {
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

fn curtain() -> Script {
    let reveal = TriggerDef::new("iron-curtain", &[GameEventType::AttackDeclared], move |ctx, event| {
        let Some(attacker) = curtain_attacker(ctx, event) else {
            return vec![];
        };
        vec![damage(json_as(json!({
            "to": { "of": "instance", "instanceId": attacker },
            "amount": param(&*ctx, "damage"),
        })))]
    })
    .with_when(move |ctx, event| curtain_attacker(ctx, event).is_some());
    Script {
        triggers: vec![reveal],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // One script: the Radiant face's 4 is its declared `damage`, read off the running face.
    let base = curtain();
    CardScripts {
        base: base.clone(),
        radiant: base,
    }
}

// IA #165 Iron Curtain — SPEC §8.13 row 165 (R61, R121).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const CURTAIN: &str = "infiniteassault-165";
    const ATTACKER: &str = "stand-in (2) 3/6";
    const VANILLA: &str = "stand-in (1) 2/5";

    fn set(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "iron-curtain",
            "active": "p2",
            "p1": {
                "backrow": [{ "def": CURTAIN, "radiant": radiant }],
                "field": [VANILLA],
                "hand": [filler(1)],
                "library": filler(6),
            },
            "p2": { "field": [ATTACKER, ATTACKER], "hand": [filler(1)], "library": filler(6) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn every_declared_attack_takes_2_and_the_curtain_stays() {
            let mut s = set(false);
            let first = s.unit_at(P1, 1);
            let attacker = s.unit_at(P2, 1);
            s.attack(&attacker, &first);
            // 2 from the curtain in the window, then 2 back from the 2/5: 6 - 4.
            s.expect_stats(&s.unit_at(P2, 1), json!({ "attack": 3, "health": 2 }));
            s.expect_stats(&s.unit_at(P1, 1), json!({ "attack": 2, "health": 2 }));
            s.expect_in_zone(CURTAIN, "field");
            let second = s.unit_at(P2, 2);
            s.attack(&second, "hero");
            s.expect_stats(&s.unit_at(P2, 2), json!({ "attack": 3, "health": 4 }));
            s.expect_health(P1, 27);
            assert_eq!(s.count("trapFired"), 2, "it fires on every declared attack");
            s.expect_in_zone(CURTAIN, "field");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn every_declared_attack_takes_4() {
            let mut s = set(true);
            let attacker = s.unit_at(P2, 1);
            s.attack(&attacker, "hero");
            // 4 from the curtain in the window; no strike back from a hero.
            s.expect_stats(&s.unit_at(P2, 1), json!({ "attack": 3, "health": 2 }));
            s.expect_health(P1, 27);
            s.expect_in_zone(CURTAIN, "field");
        }
    }
}
