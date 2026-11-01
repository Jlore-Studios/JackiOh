//! S #92 The Propagation's Hornet (SPEC §8.10 row 92): (2) Unit, Swarm, Rare, 3/4 → 6/8 (4/4 → 8/8 until #2657), of the
//! Stinkyscape set.
//!   Base:    "Rush / After this attacks, summon a copy of this, then evolve. / Evolve: Gain Flying."
//!   Radiant: "Rush / After this attacks, summon a copy of this, then evolve. / Evolve: Gain Flying."
//! Engine: S #91 The Propagation's Wasp's script; Rush is a catalog keyword. `after_attack`
//! (R1849001): once the state check closing each of its combats has run, declared or forced,
//! `summon_copy` of it (R57, into R64's zone, no Cry, R1), made from it before it evolves, then `evolve`
//! on it, which runs its `Script.evolve` hook: Gain Flying (`grant_keyword`). The copy has no Flying and
//! the original has it; one that died attacking does nothing, neither copy nor evolve (R2285001); a
//! full unit row summons no copy. The copy has Rush too, a copy of
//! the card, so it may attack a Unit that turn and copy again. Radiant: the same text, the copy of a
//! Radiant Hornet a Radiant 6/8.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-092";

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

// S #92 The Propagation's Hornet — SPEC §8.10 row 92, BUILD M12 row S 92.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    const FILLER: &str = "core-005"; // (1) Spell: a card in hand, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const MENACE: &str = "core-019"; // (3) 9/9 Taunt: kills a Hornet that attacks it.
    const MOTHS: &str = "core-009"; // (2) 1/14: at its controller's start of turn every enemy Unit attacks it.

    /// p1 has the Hornet (`radiant` its face) in unit zone 1 and `p1_field` after it; p2 has `p2_field`.
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

    /// Every copy of this card summoned so far.
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
            let hornet = s.unit_at(P1, 1);
            assert!(!flies(&s, &hornet));
            s.attack(&hornet, "hero");
            s.expect_health(P2, HERO_HEALTH - 3);
            let copies = copies(&s);
            assert_eq!(copies.len(), 1);
            assert_eq!(s.unit_at(P1, 2).id, copies[0].id, "R64: the leftmost open zone");
            assert!(!flies(&s, &copies[0]), "made before it evolved");
            assert!(flies(&s, &hornet));
            s.expect_stats(&copies[0], json!({ "attack": 3, "health": 4 }));
        }

        #[test]
        fn has_rush() {
            let mut s = crate::scenario(json!({
                "p1": { "hand": [ID, FILLER], "library": crate::filler(4) },
                "p2": { "hand": [FILLER], "field": [MOTHS], "library": crate::filler(4) },
            }));
            s.play(ID, json!({ "zone": 1 }));
            let hornet = s.unit_at(P1, 1);
            assert!(has_keyword(&s.stats(&hornet).keywords, KeywordKind::Rush));
            // Rush: a Unit, not the hero, the turn it arrives (§6.1).
            s.expect_refused(|s| s.attack(&hornet, "hero"));
            s.attack(&hornet, s.unit_at(P2, 1));
            let copies: Vec<CardInstance> = copies(&s).into_iter().filter(|copy| copy.id != hornet.id).collect();
            assert_eq!(copies.len(), 1);
            // The copy is a Hornet: Rush too.
            assert!(has_keyword(&s.stats(&copies[0]).keywords, KeywordKind::Rush));
        }

        #[test]
        fn a_forced_attack_copies_too() {
            let mut s = board(false, json!([]), json!([MOTHS]));
            let hornet = s.unit_at(P1, 1);
            // p2's start of turn: the Moths makes the Hornet attack it (R1849001: forced attacks count).
            s.end_turn();
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::AttackDeclared { attacker_id, forced: true, .. } if *attacker_id == hornet.id
            )));
            assert_eq!(copies(&s).len(), 1);
            assert!(flies(&s, &hornet));
        }

        #[test]
        fn r2285001_one_that_died_attacking_leaves_no_copy() {
            let mut s = board(false, json!([]), json!([MENACE]));
            let hornet = s.unit_at(P1, 1);
            s.attack(&hornet, s.unit_at(P2, 1));
            s.expect_in_zone(&hornet, "graveyard");
            assert!(copies(&s).is_empty());
        }

        #[test]
        fn a_full_board_summons_none() {
            let mut s = board(false, json!([VANILLA, VANILLA, VANILLA, VANILLA]), json!([]));
            let hornet = s.unit_at(P1, 1);
            s.attack(&hornet, "hero");
            assert!(copies(&s).is_empty());
            assert!(flies(&s, &hornet), "it evolves all the same");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn copies_at_double_size() {
            let mut s = board(true, json!([]), json!([]));
            let hornet = s.unit_at(P1, 1);
            s.expect_stats(&hornet, json!({ "attack": 6, "health": 8 }));
            s.attack(&hornet, "hero");
            s.expect_health(P2, HERO_HEALTH - 6);
            let copies = copies(&s);
            assert_eq!(copies.len(), 1);
            assert!(copies[0].radiant);
            s.expect_stats(&copies[0], json!({ "attack": 6, "health": 8 }));
            assert!(!flies(&s, &copies[0]));
            assert!(flies(&s, &hornet));
        }
    }
}
