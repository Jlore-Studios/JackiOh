//! IA #25.1 Figment (SPEC §8.13 row 25.1, §7), a token of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Set at a random cost from ({low}) to ({high}). Reveals when an enemy Unit attacks:
//!            Nothing happens." (1 to 4)
//!   Radiant: the same, from ({low}) to ({high}). (2 to 4)
//!
//! Setting one is a fresh summon, face-down, into the leftmost open backrow zone; with none open,
//! nothing happens (§3.2). Its `start_of_game` rolls `costOverride` uniformly from `low` to `high`
//! as it arrives (`set_cost_override_random`). It reveals on a declared enemy attack — a forced
//! attack declares nothing (R53) — does nothing (R61), then ceases to exist (R11). A Figment is
//! never offered as anything but its zone (R177); its cost shows anyway (R351).
//!
//! Engine: `summon` of a Trap (face-down), `setCostOverrideRandom` (R4586000), one `trapFired`
//! answered by nothing. Built by C2 (#4586).

use jackioh_engine::prelude::*;
use jackioh_engine::preview::is_face_down;

pub const ID: &str = "infiniteassault-025-1";

/// Set `count` Figments face-down, Radiant ones when `radiant`: the shared Cry, Death and
/// end-of-turn half of the Hallucination cards. Figments are Radiant only where §7 says so: the
/// Radiant faces of #25, #38 and #42.
pub fn set_figments(count: i32, radiant: bool) -> Vec<Effect> {
    (0..count.max(0))
        .map(|_| summon(json_as(json!({ "defId": ID, "radiant": radiant }))))
        .collect()
}

/// A declared enemy attack: the attack names its attacker, is not forced, and its controller is
/// the other seat.
fn enemy_attack(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::AttackDeclared {
        attacker_id,
        forced,
        ..
    } = event
    else {
        return false;
    };
    if *forced {
        return false;
    }
    find_instance(&*ctx.state, attacker_id).is_some_and(|attacker| attacker.controller != ctx.controller)
}

fn figment() -> Script {
    Script {
        start_of_game: Some(hook(|ctx| {
            let Some(me) = ctx.live_self() else {
                return vec![];
            };
            if me.zone.z() != ZoneName::Field || !is_face_down(&*ctx.state, me) {
                return vec![];
            }
            vec![set_cost_override_random(json_as(json!({
                "low": param(&*ctx, "low"),
                "high": param(&*ctx, "high"),
            })))]
        })),
        // It reveals on a declared enemy attack and does nothing (R61): an empty answer still
        // consumes the trap, so every condition lives in `when`.
        triggers: vec![TriggerDef::new("figment", &[GameEventType::AttackDeclared], |_, _| vec![])
            .with_when(enemy_attack)],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: figment(),
        // The same script: the Radiant face's (2) is its declared `low`.
        radiant: figment(),
    }
}

// IA #25.1 Figment — SPEC §8.13 row 25.1, §7 (R4586000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const WEAVER: &str = "infiniteassault-025"; // Mirage Weaver: Cry sets Figments.
    const MOTHS: &str = "core-009"; // Moths to the Flame: Start of turn forces every enemy Unit to attack it.
    const ATTACKER: &str = "core-008";
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn figments(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1]
            .backrow
            .iter()
            .filter_map(|slot| slot.clone())
            .filter(|card| card.def_id == ID)
            .collect()
    }

    fn set_weaver(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": WEAVER, "radiant": radiant }, STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": { "field": [ATTACKER], "hand": [STOCKPILE], "library": crate::filler(4) },
        }));
        s.play(WEAVER, json!({ "zone": 1 }));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r4586000_the_cry_sets_a_face_down_figment_costing_1_to_4() {
            let s = set_weaver(false, "figment-base");
            let set = figments(&s);
            assert_eq!(set.len(), 1, "one Figment");
            assert!(is_face_down(s.state(), &set[0]));
            for seed in 0..8 {
                let s = set_weaver(false, &format!("figment-cost-{seed}"));
                let cost = figments(&s)[0].cost_override.expect("a rolled cost");
                assert!((1..=4).contains(&cost), "rolled {cost}");
            }
        }

        #[test]
        fn r4586000_its_cost_reaches_1_and_4() {
            for (want, prefix) in [(1, "figment-one"), (4, "figment-four")] {
                let seed = seed_where(prefix, |probe| {
                    let s = set_weaver(false, probe);
                    figments(&s).first().and_then(|card| card.cost_override) == Some(want)
                });
                let s = set_weaver(false, &seed);
                assert_eq!(
                    figments(&s).first().and_then(|card| card.cost_override),
                    Some(want)
                );
            }
        }

        #[test]
        fn r4586000_a_full_backrow_fizzles_the_set() {
            let mut s = scenario(json!({
                "seed": "figment-full",
                "p1": {
                    "hand": [WEAVER, STOCKPILE],
                    "backrow": [STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE],
                    "library": crate::filler(4),
                },
                "p2": { "field": [ATTACKER], "hand": [STOCKPILE], "library": filler(4) },
            }));
            s.play(WEAVER, json!({ "zone": 1 }));
            assert!(figments(&s).is_empty(), "nowhere to set it");
        }

        #[test]
        fn r4586000_a_forced_attack_leaves_it_set() {
            let mut s = scenario(json!({
                "seed": "figment-forced",
                "active": "p2",
                "p1": {
                    "backrow": [ID],
                    "field": [MOTHS],
                    "hand": [STOCKPILE],
                    "library": crate::filler(4),
                },
                "p2": { "field": [ATTACKER], "hand": [STOCKPILE], "library": filler(4) },
            }));
            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(figments(&s).len(), 1, "the forced attack springs nothing");
        }

        #[test]
        fn r4586000_a_declared_enemy_attack_reveals_it_to_nothing() {
            let mut s = scenario(json!({
                "seed": "figment-reveal",
                "active": "p2",
                "p1": { "backrow": [ID], "hand": [STOCKPILE], "library": filler(4) },
                "p2": { "field": [ATTACKER], "hand": [STOCKPILE], "library": filler(4) },
            }));
            let before = s.state().players[P1].hero.health;
            let attacker = s.unit_at(PlayerId::P2, 1).id.clone();
            s.attack(&attacker, "hero");
            assert!(
                s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::TrapFired { def_id, .. } if *def_id == ID
                )),
                "it fired"
            );
            s.expect_in_zone(ID, "graveyard");
            assert_eq!(s.state().players[P1].hero.health, before - 4, "the attack still lands");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586000_the_radiant_cry_sets_figments_costing_2_to_4() {
            let s = set_weaver(true, "figment-radiant");
            let set = figments(&s);
            assert_eq!(set.len(), 2, "two Figments");
            assert!(set.iter().all(|card| card.radiant), "Radiant ones");
            for seed in 0..8 {
                let s = set_weaver(true, &format!("figment-rcost-{seed}"));
                for card in figments(&s) {
                    let cost = card.cost_override.expect("a rolled cost");
                    assert!((2..=4).contains(&cost), "rolled {cost}");
                }
            }
        }
    }

}
