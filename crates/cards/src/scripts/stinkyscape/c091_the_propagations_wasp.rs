//! S #91 The Propagation's Wasp (SPEC §8.10 row 91): (1) Unit, Swarm, Common, 3/3 → 6/6, of the
//! Stinkyscape set.
//!   Base:    "After this attacks, summon a copy of this, then evolve. / Evolve: Gain Flying."
//!   Radiant: "After this attacks, summon a copy of this, then evolve. / Evolve: Gain Flying."
//! Engine: `after_attack` (R1849001): once the state check closing each of its combats has run, declared
//! or forced, `summon_copy` of it (R57, into R64's zone, no Cry, R1), made from it before it evolves,
//! then `evolve` on it, which runs its `Script.evolve` hook: Gain Flying (`grant_keyword`). The copy has
//! no Flying and the original has it; one that died attacking does nothing, neither copy nor evolve
//! (R2285001); a full unit row summons no copy. Radiant: the same
//! text, the copy of a Radiant Wasp a Radiant 6/6.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-091";

pub fn script() -> CardScripts {
    let base = Script {
        after_attack: Some(hook(|_ctx| {
            vec![
                summon_copy(json_as(json!({ "of": { "of": "self" } }))),
                evolve(json_as(json!({ "target": { "of": "self" } }))),
            ]
        })),
        evolve: Some(hook(|_ctx| {
            vec![grant_keyword(json_as(
                json!({ "target": { "of": "self" }, "keyword": { "kind": "Flying" } }),
            ))]
        })),
        ..Script::default()
    };
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// S #91 The Propagation's Wasp — SPEC §8.10 row 91, BUILD M12 row S 91.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    const FILLER: &str = "core-005"; // (1) Spell: a card in hand, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const MENACE: &str = "core-019"; // (3) 9/9 Taunt: kills a Wasp that attacks it.
    const MOTHS: &str = "core-009"; // (2) 1/14: at its controller's start of turn every enemy Unit attacks it.

    fn board(radiant: bool, p1_field: Value, p2_field: Value) -> Scenario {
        let mut field = vec![json!({ "def": ID, "radiant": radiant })];
        field.extend(p1_field.as_array().cloned().unwrap_or_default());
        crate::scenario(json!({
            "p1": { "hand": [FILLER], "field": field, "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "field": p2_field, "library": crate::filler(4) },
        }))
    }

    fn flies(s: &Scenario, card: &CardInstance) -> bool {
        has_keyword(&s.stats(card).keywords, KeywordKind::Flying)
    }

    fn copies(s: &Scenario) -> Vec<CardInstance> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { instance_id, def_id, .. } if def_id == ID => Some(s.card(instance_id).clone()),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn after_attacking_a_copy_without_flying_then_it_flies() {
            let mut s = board(false, json!([]), json!([]));
            let wasp = s.unit_at(P1, 1);
            assert!(!flies(&s, &wasp));
            s.attack(&wasp, "hero");
            s.expect_health(P2, HERO_HEALTH - 3);
            let copies = copies(&s);
            assert_eq!(copies.len(), 1);
            assert_eq!(s.unit_at(P1, 2).id, copies[0].id, "R64: the leftmost open zone");
            assert!(!flies(&s, &copies[0]), "made before it evolved");
            assert!(flies(&s, &wasp));
            s.expect_stats(&copies[0], json!({ "attack": 3, "health": 3 }));
        }

        #[test]
        fn a_forced_attack_copies_too() {
            let mut s = board(false, json!([]), json!([MOTHS]));
            let wasp = s.unit_at(P1, 1);
            // p2's start of turn: the Moths makes the Wasp attack it (R1849001: forced attacks count).
            s.end_turn();
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::AttackDeclared { attacker_id, forced: true, .. } if *attacker_id == wasp.id
            )));
            assert_eq!(copies(&s).len(), 1);
            assert!(flies(&s, &wasp));
        }

        #[test]
        fn r2285001_one_that_died_attacking_leaves_no_copy() {
            let mut s = board(false, json!([]), json!([MENACE]));
            let wasp = s.unit_at(P1, 1);
            s.attack(&wasp, s.unit_at(P2, 1));
            s.expect_in_zone(&wasp, "graveyard");
            assert!(copies(&s).is_empty());
        }

        #[test]
        fn a_full_board_summons_none() {
            let mut s = board(false, json!([VANILLA, VANILLA, VANILLA, VANILLA]), json!([]));
            let wasp = s.unit_at(P1, 1);
            s.attack(&wasp, "hero");
            assert!(copies(&s).is_empty());
            assert!(flies(&s, &wasp), "it evolves all the same");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn copies_at_double_size() {
            let mut s = board(true, json!([]), json!([]));
            let wasp = s.unit_at(P1, 1);
            s.expect_stats(&wasp, json!({ "attack": 6, "health": 6 }));
            s.attack(&wasp, "hero");
            s.expect_health(P2, HERO_HEALTH - 6);
            let copies = copies(&s);
            assert_eq!(copies.len(), 1);
            assert!(copies[0].radiant);
            s.expect_stats(&copies[0], json!({ "attack": 6, "health": 6 }));
            assert!(!flies(&s, &copies[0]));
            assert!(flies(&s, &wasp));
        }
    }
}
