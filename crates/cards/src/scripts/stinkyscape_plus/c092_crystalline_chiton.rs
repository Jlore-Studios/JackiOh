//! SPEC §8.12 S+ #92 Crystalline Chiton (Unit, Swarm 2/4 → 4/8, cost 1, Common), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Evolves when this survives damage.
//!            Evolve: Gain {jade} Jade." (jade 2)
//!   Radiant: "Evolves when this survives damage.
//!            Evolve: Gain {jade} Jade." (jade 4)
//!
//! "Survives damage" is the card's own `damage` trigger on itself evolving itself (R3536001): the hit
//! landed on this Unit and left it standing, so a lethal or Poisonous hit, which sends it to the
//! graveyard, evolves nothing. The Evolve text runs once (R3536000): `add_jade` of the face's declared
//! `jade` onto its controller's Jade Counter (ME-JADE, MD-C2), whose crossings of 5 and 10 run inside the
//! add, as every Jade does (MD-C3). The Radiant face is the same text with the Radiant number.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-092";

/// R3536001: an event condition is the card's own trigger evolving itself.
fn evolves_when_it_survives_damage() -> TriggerDef {
    TriggerDef::new("evolveOnSurviving", &[GameEventType::Damage], |ctx, event| {
        let GameEvent::Damage { target_id, .. } = event else {
            return vec![];
        };
        if !ctx.self_.as_ref().is_some_and(|me| me.id == *target_id) {
            return vec![];
        }
        // "Survives": a hit that leaves no health or marks it destroyed sends it to the graveyard.
        let standing = ctx.live_self().is_some_and(|card| {
            acts_on_field(ctx.state, card)
                && card.marked_destroyed != Some(true)
                && unit_view(ctx.state, card).health > 0
        });
        if standing {
            vec![evolve(json_as(json!({ "target": { "of": "self" } })))]
        } else {
            vec![]
        }
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        evolve: Some(hook(|ctx| {
            vec![add_jade(json_as(json!({ "amount": param(ctx, "jade") })))]
        })),
        triggers: vec![evolves_when_it_survives_damage()],
        ..Script::default()
    };
    // One script: the face's Jade is its declared `jade` (2, Radiant 4).
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #92 Crystalline Chiton — SPEC §8.12 row 92.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

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

    fn jade(s: &Scenario) -> Option<i32> {
        s.state().players.p1.jade
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
        fn surviving_damage_evolves_it_and_gains_2_jade() {
            let mut s = board(false, "stand-in (4) 3/3");
            let chiton = s.unit_at(P1, 1);
            s.attack(&chiton, s.unit_at(P2, 1));
            assert_eq!(s.card(&chiton).damage, 3);
            s.expect_in_zone(&chiton, "field");
            assert_eq!(s.card(&chiton).evolved, Some(true));
            assert_eq!(evolved_events(&s), 1);
            assert_eq!(jade(&s), Some(2));
            assert!(s.state().pending.is_none(), "it opens no prompt");
        }

        #[test]
        fn lethal_damage_sends_it_to_the_graveyard_with_no_evolution_and_no_jade() {
            let mut s = board(false, "stand-in (4) 4/4");
            let chiton = s.unit_at(P1, 1);
            s.attack(&chiton, s.unit_at(P2, 1));
            s.expect_in_zone(&chiton, "graveyard");
            assert_ne!(s.card(&chiton).evolved, Some(true));
            assert_eq!(evolved_events(&s), 0);
            assert_eq!(jade(&s), None);
        }

        #[test]
        fn a_poisonous_hit_it_would_survive_on_health_still_evolves_nothing() {
            let mut s = board(false, "stand-in (4) 1/1 Poisonous");
            let chiton = s.unit_at(P1, 1);
            s.attack(&chiton, s.unit_at(P2, 1));
            s.expect_in_zone(&chiton, "graveyard");
            assert_eq!(evolved_events(&s), 0);
            assert_eq!(jade(&s), None);
        }

        #[test]
        fn it_evolves_once_and_gains_jade_once() {
            let mut s = board(false, "stand-in (4) 1/9");
            let chiton = s.unit_at(P1, 1);
            s.attack(&chiton, s.unit_at(P2, 1));
            assert_eq!(jade(&s), Some(2));
            s.end_turn();
            s.end_turn();
            s.attack(&chiton, s.unit_at(P2, 1));
            assert_eq!(jade(&s), Some(2));
            assert_eq!(evolved_events(&s), 1);
        }

        #[test]
        fn damage_to_another_unit_evolves_nothing() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [VANILLA],
                    "field": [ID, "stand-in (1) 4/4"],
                    "library": crate::filler(4),
                },
                "p2": { "hand": [VANILLA], "field": ["stand-in (1) 1/9"], "library": crate::filler(4) },
            }));
            let chiton = s.unit_at(P1, 1);
            let other = s.unit_at(P1, 2);
            s.attack(&other, s.unit_at(P2, 1));
            assert_ne!(s.card(&chiton).evolved, Some(true));
            assert_eq!(jade(&s), None);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn surviving_damage_evolves_it_and_gains_4_jade() {
            let mut s = board(true, "stand-in (4) 7/7");
            let chiton = s.unit_at(P1, 1);
            s.expect_stats(&chiton, json!({ "attack": 4, "health": 8 }));
            s.attack(&chiton, s.unit_at(P2, 1));
            assert_eq!(s.card(&chiton).damage, 7);
            assert_eq!(s.card(&chiton).evolved, Some(true));
            assert_eq!(jade(&s), Some(4));
            assert_eq!(evolved_events(&s), 1);
        }

        #[test]
        fn lethal_damage_evolves_nothing() {
            let mut s = board(true, "stand-in (4) 8/8");
            let chiton = s.unit_at(P1, 1);
            s.attack(&chiton, s.unit_at(P2, 1));
            s.expect_in_zone(&chiton, "graveyard");
            assert_eq!(jade(&s), None);
        }
    }
}
