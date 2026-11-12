//! SPEC §8.12 S+ #83 Offworld Replicant (Unit, Swarm 6/6 → 12/12, cost 2, Common), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Stack
//!            Evolves when this is Stacked on something.
//!            Evolve: Add a copy of this to your hand."
//!   Radiant: "Stack
//!            Evolves when this is Stacked on something.
//!            Evolve: Add a copy of this to your hand."
//!
//! Stack is the catalog's keyword (§3.2): played onto an occupied unit zone it tops the pile and the
//! card beneath it goes beneath a Stack (R13). "Stacked on something" is a standing condition, read at
//! the settled state check (`evolves_when`, R3536001): the zone it tops holds a card beneath it
//! (`zones::beneath_at`), so it evolves as it lands and not while it stands alone. The Evolve text runs
//! once (R3536000): `add_to_hand` of a fresh Offworld Replicant on the face this one wears, a Radiant
//! copy of a Radiant one (R3541001); it carries none of this card's buffs or damage, and a full hand adds
//! none. The copy in the hand evolves only when it is Stacked in its turn.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-083";

/// R3536001: the card tops a pile, with another card beneath it.
fn stacked_on_something() -> HasHook {
    read_hook(|a| slot_of(a.state, a.self_).is_some_and(|slot| !beneath_at(a.state, slot).is_empty()))
}

/// R3541001: "Add a copy of this to your hand", a fresh card of this one's face.
fn copy_to_hand() -> Hook {
    hook(|ctx| {
        let mut args = json!({ "defId": ID });
        if ctx.radiant {
            args["radiant"] = json!(true);
        }
        vec![add_to_hand(json_as(args))]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        evolves_when: Some(stacked_on_something()),
        evolve: Some(copy_to_hand()),
        ..Script::default()
    };
    // The Radiant face prints the same text; its copy is a Radiant one (`ctx.radiant`).
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #83 Offworld Replicant — SPEC §8.12 row 83.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    fn evolved_events(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Evolved)
            .count()
    }

    fn copies_in_hand(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id == ID).collect()
    }

    /// Plays the Replicant of `face` onto a Vanilla Unit holding zone 1.
    fn stack_onto_a_unit(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "mana": 10,
                "hand": [{ "def": ID, "radiant": radiant }, VANILLA],
                "field": [VANILLA],
                "library": crate::filler(4),
            },
            "p2": { "library": crate::filler(4) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r3541001_stacked_replicant_evolves_and_adds_its_faces_copy_to_hand() {
            let s = stack_onto_a_unit(false);
            let replicant = s.unit_at(P1, 1);
            assert_eq!(replicant.def_id, ID);
            assert_eq!(s.stats(&replicant).attack, 6);
            assert_eq!(s.card(&replicant).evolved, Some(true));
            assert_eq!(evolved_events(&s), 1);
            let copies = copies_in_hand(&s);
            assert_eq!(copies.len(), 1, "one copy joins the hand");
            assert!(!copies[0].radiant, "on the face it shows");
            assert!(
                s.card(&copies[0]).tuning.is_none(),
                "fresh: it carries nothing of this one"
            );
            assert!(s.state().pending.is_none(), "it opens no prompt");
        }

        #[test]
        fn setup_stacked_it_evolves_at_the_first_state_check() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [VANILLA],
                    "field": [VANILLA, { "def": ID, "stack": true }],
                    "library": crate::filler(4),
                },
                "p2": { "library": crate::filler(4) },
            }));
            let replicant = s.unit_at(P1, 1);
            assert_eq!(s.card(&replicant).evolved, Some(true));
            s.end_turn();
            assert_eq!(copies_in_hand(&s).len(), 1);
        }

        #[test]
        fn played_onto_an_empty_zone_it_evolves_nothing() {
            let mut s = scenario(json!({
                "p1": { "mana": 10, "hand": [ID, VANILLA], "library": crate::filler(4) },
                "p2": { "library": crate::filler(4) },
            }));
            s.play(ID, json!({}));
            let replicant = s.unit_at(P1, 1);
            assert_ne!(s.card(&replicant).evolved, Some(true));
            assert!(copies_in_hand(&s).is_empty());
            assert_eq!(evolved_events(&s), 0);
        }

        #[test]
        fn it_evolves_once_a_second_turn_adds_no_second_copy() {
            let mut s = stack_onto_a_unit(false);
            s.end_turn();
            s.end_turn();
            assert_eq!(copies_in_hand(&s).len(), 1);
            assert_eq!(evolved_events(&s), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3541001_stacked_radiant_replicant_adds_a_radiant_copy() {
            let mut s = stack_onto_a_unit(true);
            let replicant = s.unit_at(P1, 1);
            assert_eq!(s.card(&replicant).evolved, Some(true));
            s.expect_stats(&replicant, json!({ "attack": 12, "health": 12 }));
            let copies = copies_in_hand(&s);
            assert_eq!(copies.len(), 1);
            assert!(copies[0].radiant, "a Radiant Replicant adds a Radiant copy");
            assert_eq!(evolved_events(&s), 1);
        }
    }
}
