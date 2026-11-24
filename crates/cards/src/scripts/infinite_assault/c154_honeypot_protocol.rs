//! SPEC §8.13 IA #154 Honeypot Protocol ((2) Trap, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reveals when an enemy Unit attacks: Summon a Decoy Server. Redirect the attack to it."
//!   Radiant: "Reveals when an enemy Unit attacks: Summon a Radiant Decoy Server. Redirect the attack to it."
//!
//! Engine: §4.2 step 4's trap window, on `attackDeclared`: an enemy Unit's declared attack on any
//! target (R121: a forced attack declares nothing and springs nothing). The fresh Decoy Server
//! (Radiant on the Radiant face) takes the open attack (R4591005), so the combat resolves against
//! it (R1122). With no open zone nothing is summoned and the attack goes on. Every condition is in
//! `when`, so any other attack leaves it set (R61).

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-154";

/// Honeypot Protocol's Decoy Server.
const DECOY: &str = "infiniteassault-154-1";

/// The attacker of an enemy Unit's declared attack this trap answers.
fn honeypot_attacker(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<String> {
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

fn honeypot(radiant: bool) -> Script {
    let reveal = TriggerDef::new(
        "honeypot-protocol",
        &[GameEventType::AttackDeclared],
        move |ctx, event| {
            if honeypot_attacker(ctx, event).is_none() {
                return vec![];
            }
            vec![summon(json_as(json!({
                "defId": DECOY,
                "radiant": radiant,
                "takesAttack": true,
            })))]
        },
    )
    .with_when(move |ctx, event| honeypot_attacker(ctx, event).is_some());
    Script {
        triggers: vec![reveal],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: honeypot(false),
        radiant: honeypot(true),
    }
}

// IA #154 Honeypot Protocol — SPEC §8.13 row 154 (R61, R121, R1122, R4591005).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const HONEYPOT: &str = "infiniteassault-154";
    const DECOY: &str = "infiniteassault-154-1";
    const ATTACKER: &str = "stand-in (2) 3/3";
    const VANILLA: &str = "stand-in (1) 2/5";

    fn set(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "honeypot",
            "active": "p2",
            "p1": {
                "backrow": [{ "def": HONEYPOT, "radiant": radiant }],
                "field": [VANILLA],
                "hand": [filler(1)],
                "library": filler(6),
            },
            "p2": { "field": [ATTACKER], "hand": [filler(1)], "library": filler(6) },
        }))
    }

    fn decoy_of(s: &Scenario) -> CardInstance {
        (1..=5)
            .filter_map(|lane| s.unit(P1, lane))
            .find(|unit| unit.def_id == DECOY)
            .expect("a Decoy Server stands")
    }

    mod base {
        use super::*;

        #[test]
        fn r4591005_the_attack_hits_the_decoy() {
            let mut s = set(false);
            let attacker = s.unit_at(P2, 1);
            let target = s.unit_at(P1, 1);
            s.attack(&attacker, &target);
            let decoy = decoy_of(&s);
            s.expect_stats(&decoy, json!({ "attack": 0, "health": 5 }));
            assert!(!decoy.radiant);
            s.expect_stats(&s.unit_at(P1, 1), json!({ "health": 5 }), "the first unit is untouched");
            s.expect_stats(&s.unit_at(P2, 1), json!({ "health": 3 }), "the 0-attack decoy strikes nothing back");
            s.expect_in_zone(HONEYPOT, "graveyard");
        }

        #[test]
        fn r4591005_a_full_board_lets_the_attack_through() {
            let mut s = scenario(json!({
                "seed": "honeypot-full",
                "active": "p2",
                "p1": {
                    "backrow": [HONEYPOT],
                    "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA],
                    "hand": [filler(1)],
                    "library": filler(6),
                },
                "p2": { "field": [ATTACKER], "hand": [filler(1)], "library": filler(6) },
            }));
            let attacker = s.unit_at(P2, 1);
            let target = s.unit_at(P1, 1);
            s.attack(&attacker, &target);
            assert!(
                (1..=5).filter_map(|lane| s.unit(P1, lane)).all(|unit| unit.def_id != DECOY),
                "nothing summoned"
            );
            s.expect_stats(&s.unit_at(P1, 1), json!({ "health": 2 }), "the attack goes on");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_decoy_is_radiant() {
            let mut s = set(true);
            let attacker = s.unit_at(P2, 1);
            let target = s.unit_at(P1, 1);
            s.attack(&attacker, &target);
            let decoy = decoy_of(&s);
            assert!(decoy.radiant, "a Radiant Decoy Server");
            s.expect_stats(&decoy, json!({ "attack": 0, "health": 13 }));
        }
    }
}
