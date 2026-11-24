//! SPEC §8.13 IA #87 Swarm Shield ((1) Trap, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reveals when an enemy Unit attacks: Summon three Workers. Give them Taunt."
//!   Radiant: "Reveals when an enemy Unit attacks: Summon three Radiant Workers. Give them Taunt."
//!
//! S #22 Gatinsir's answer to an attack an enemy Unit declares (R1875002; never a forced one, R121),
//! with no chance to miss: three summons of IA #78.1 into R64's leftmost free zones, each given Taunt
//! as it lands (S T-AI-8 Drone Uplink's `keywords`, as IA #7's Rush is). The attack then goes on.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-087";

/// IA #78.1 Worker.
const WORKER: &str = "infiniteassault-078-1";

/// "three Workers".
const WORKERS: usize = 3;

/// R1875002: "when an enemy Unit attacks": an attack declared by a Unit its controller's opponent
/// controls; a forced attack is declared by no one (R121).
fn enemy_attacks(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    match event {
        GameEvent::AttackDeclared {
            attacker_id,
            forced: false,
            ..
        } => find_instance(&*ctx.state, attacker_id)
            .is_some_and(|attacker| attacker.controller != ctx.controller),
        _ => false,
    }
}

fn swarm_shield(radiant: bool) -> Script {
    let ambush = TriggerDef::new(
        "swarm-shield",
        &[GameEventType::AttackDeclared],
        move |_ctx, _event| {
            (0..WORKERS)
                .map(|_| {
                    let mut args = json!({ "defId": WORKER, "player": "self" });
                    if radiant {
                        args["radiant"] = json!(true);
                    }
                    args["keywords"] = json!([{ "kind": "Taunt" }]);
                    summon(json_as(args))
                })
                .collect()
        },
    )
    .with_when(|ctx, event| enemy_attacks(ctx, event));
    Script {
        triggers: vec![ambush],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: swarm_shield(false),
        radiant: swarm_shield(true),
    }
}

// IA #87 Swarm Shield — SPEC §8.13 row 87.
#[cfg(test)]
mod tests {
    use super::{ID, WORKER};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const ATTACKER: &str = "stand-in (2) 3/3";

    fn ambushed(radiant: bool) -> (Scenario, Vec<CardInstance>) {
        let mut s = scenario(json!({
            "active": "p2",
            "p1": {
                "field": ["stand-in (1) 2/2"],
                "backrow": [{ "def": ID, "radiant": radiant, "lane": 1 }],
                "library": crate::filler(2),
            },
            "p2": { "field": [{ "def": ATTACKER, "lane": 1 }], "library": crate::filler(2) },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        let attacker = s.unit_at(P2, 1);
        s.attack(&attacker, "hero");
        let workers = active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == WORKER)
            .cloned()
            .collect();
        (s, workers)
    }

    mod base {
        use super::*;

        #[test]
        fn an_enemy_attack_summons_three_workers_with_taunt() {
            let (s, workers) = ambushed(false);
            assert_eq!(workers.len(), 3);
            for worker in nonempty(workers, "the Workers") {
                assert!(!worker.radiant);
                assert!(has_keyword(&s.stats(&worker).keywords, KeywordKind::Taunt));
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn an_enemy_attack_summons_three_radiant_workers_with_taunt() {
            let (s, workers) = ambushed(true);
            assert_eq!(workers.len(), 3);
            for worker in nonempty(workers, "the Workers") {
                assert!(worker.radiant);
                assert!(has_keyword(&s.stats(&worker).keywords, KeywordKind::Taunt));
            }
        }
    }
}
