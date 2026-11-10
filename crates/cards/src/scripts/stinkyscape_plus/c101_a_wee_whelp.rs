//! SPEC §8.12 S+ #101 A Wee Whelp (Unit, Swarm 1/1 → 2/2, cost 1, Common), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Evolves after this destroys a Unit.
//!            Evolve: Set stats to 10/10."
//!   Radiant: "Evolves after this destroys a Unit.
//!            Evolve: Set stats to 20/20."
//!
//! "Evolves after this destroys a Unit" is the card's own `destroyed` trigger, the one whose killer is
//! this card (R42), evolving itself (R3536001): an attack's kill or a defending kill, Cleave and Poisonous
//! kills included. "After" is R2396000's timing: it answers only for a card still on the field, so a Whelp
//! that trades itself away in the same combat evolves nothing (a "Whenever" would, `even_if_gone`). The
//! Evolve text runs once (R3536000): `set_stats` on this card, layer 1, so its buffs and the stat half of
//! its tuning go and its damage is healed (R2198000, §10.4); the 10/10 and 20/20 are no params, each face's
//! own (R1748003). R78's reset takes the stats off as it leaves the field.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-101";

/// "Set stats to 10/10": the base face's Attack and Health.
const BASE_SET: i32 = 10;

/// "Set stats to 20/20": the Radiant face's.
const RADIANT_SET: i32 = 20;

/// R3536001, R2396000: the trigger of a kill whose killer is this card, for a card still on the field.
fn evolves_after_it_destroys_a_unit() -> TriggerDef {
    TriggerDef::new("evolveOnKill", &[GameEventType::Destroyed], |ctx, event| {
        let GameEvent::Destroyed { killer_id, .. } = event else {
            return vec![];
        };
        let mine = ctx
            .self_
            .as_ref()
            .is_some_and(|me| killer_id.as_deref() == Some(me.id.as_str()));
        if mine {
            vec![evolve(json_as(json!({ "target": { "of": "self" } })))]
        } else {
            vec![]
        }
    })
}

/// "Evolve: Set stats to N/N."
fn set_to(stats: i32) -> Hook {
    hook(move |_ctx| {
        vec![set_stats(json_as(json!({
            "target": { "of": "self" },
            "attack": stats,
            "health": stats,
        })))]
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            evolve: Some(set_to(BASE_SET)),
            triggers: vec![evolves_after_it_destroys_a_unit()],
            ..Script::default()
        },
        radiant: Script {
            evolve: Some(set_to(RADIANT_SET)),
            triggers: vec![evolves_after_it_destroys_a_unit()],
            ..Script::default()
        },
    }
}

// S+ #101 A Wee Whelp — SPEC §8.12 row 101.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FRAGILE: &str = "stand-in (1) 0/1";
    const EQUAL: &str = "stand-in (1) 1/1";
    const WALL: &str = "stand-in (1) 0/3";

    fn board(radiant: bool, foe: &str) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [VANILLA],
                "field": [{ "def": ID, "radiant": radiant }],
                "library": crate::filler(4),
            },
            "p2": { "hand": [VANILLA], "field": [foe], "library": crate::filler(4) },
        }))
    }

    fn evolved_events(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Evolved)
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn destroying_a_unit_evolves_it_to_10_10() {
            let mut s = board(false, FRAGILE);
            let whelp = s.unit_at(P1, 1);
            let foe = s.unit_at(P2, 1);
            s.attack(&whelp, &foe);
            s.expect_in_zone(&foe, "graveyard");
            assert_eq!(s.card(&whelp).evolved, Some(true));
            s.expect_stats(&whelp, json!({ "attack": 10, "health": 10 }));
            assert_eq!(evolved_events(&s), 1);
            assert!(s.state().pending.is_none(), "it opens no prompt");
        }

        #[test]
        fn trading_itself_away_in_the_same_combat_evolves_nothing() {
            let mut s = board(false, EQUAL);
            let whelp = s.unit_at(P1, 1);
            let foe = s.unit_at(P2, 1);
            s.attack(&whelp, &foe);
            s.expect_in_zone(&foe, "graveyard");
            s.expect_in_zone(&whelp, "graveyard");
            assert_ne!(
                s.card(&whelp).evolved,
                Some(true),
                "After: only for a card still on the field"
            );
            assert_eq!(evolved_events(&s), 0);
        }

        #[test]
        fn failing_to_kill_evolves_nothing() {
            let mut s = board(false, WALL);
            let whelp = s.unit_at(P1, 1);
            s.attack(&whelp, s.unit_at(P2, 1));
            assert_ne!(s.card(&whelp).evolved, Some(true));
            s.expect_stats(&whelp, json!({ "attack": 1, "health": 1 }));
            assert_eq!(evolved_events(&s), 0);
        }

        #[test]
        fn it_evolves_once_a_second_kill_sets_nothing_again() {
            let mut s = scenario(json!({
                "p1": { "hand": [VANILLA], "field": [ID], "library": crate::filler(4) },
                "p2": { "hand": [VANILLA], "field": [FRAGILE, FRAGILE], "library": crate::filler(4) },
            }));
            let whelp = s.unit_at(P1, 1);
            s.attack(&whelp, s.unit_at(P2, 1));
            s.end_turn();
            s.end_turn();
            // A buff after it evolved stays: the Evolve text does not run a second time.
            s.card_mut(&whelp).buffs.attack += 5;
            s.attack(&whelp, s.unit_at(P2, 2));
            assert_eq!(evolved_events(&s), 1);
            s.expect_stats(&whelp, json!({ "attack": 15, "health": 10 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn destroying_a_unit_evolves_it_to_20_20() {
            let mut s = board(true, FRAGILE);
            let whelp = s.unit_at(P1, 1);
            s.expect_stats(&whelp, json!({ "attack": 2, "health": 2 }));
            s.attack(&whelp, s.unit_at(P2, 1));
            assert_eq!(s.card(&whelp).evolved, Some(true));
            s.expect_stats(&whelp, json!({ "attack": 20, "health": 20 }));
            assert_eq!(evolved_events(&s), 1);
        }

        #[test]
        fn a_kill_made_defending_evolves_it_too() {
            let mut s = board(true, EQUAL);
            let whelp = s.unit_at(P1, 1);
            let foe = s.unit_at(P2, 1);
            s.end_turn();
            // The attacker's 1 damage leaves a 2/2 Whelp standing; its strike back kills the 1/1.
            s.attack(&foe, &whelp);
            s.expect_in_zone(&foe, "graveyard");
            assert_eq!(s.card(&whelp).evolved, Some(true));
            s.expect_stats(&whelp, json!({ "attack": 20, "health": 20 }));
            assert_eq!(s.card(&whelp).damage, 0, "set stats heals it");
        }

        #[test]
        fn failing_to_kill_evolves_nothing() {
            let mut s = board(true, "stand-in (1) 0/5");
            let whelp = s.unit_at(P1, 1);
            s.attack(&whelp, s.unit_at(P2, 1));
            assert_ne!(s.card(&whelp).evolved, Some(true));
            s.expect_stats(&whelp, json!({ "attack": 2, "health": 2 }));
        }
    }
}
