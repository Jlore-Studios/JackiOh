//! SPEC §8.12 S+ #88 Colony Watcher (Unit, Swarm 2/7 → 4/14, cost 2, Common), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Evolve: When this moves from Defense Position to attack position.
//!            Evolve: Recruit a Swarm card."
//!   Radiant: "Evolve: When this moves from Defense Position to attack position.
//!            Evolve: Recruit {card} Swarm cards." (card 4)
//!
//! "Evolve when this moves from Defense to Attack Position" is the card's own `positionSwitched` trigger
//! evolving itself (R3536001): the event names this card and the Attack Position, so a move into Defense,
//! or another Unit's move, evolves nothing. The Evolve text runs once (R3536000): `recruit` (§6.3), the
//! library's first Swarm card from the top down, summoned into the leftmost open zone (R64, no Cry, R1),
//! then the next for each more; one that finds no Swarm card or no zone recruits nothing. A "Swarm card"
//! is any permanent of the tag, so S+'s two Swarm Field Spells are reached as its Units are.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-088";

/// "Recruit a Swarm card": the base face recruits one (the Radiant face's count is its declared `card`).
const BASE_CARDS: i32 = 1;

/// "a Swarm card": any permanent in the library tagged Swarm.
fn swarm() -> Value {
    json!({ "tags": ["Swarm"] })
}

/// R3536001: an event condition is the card's own trigger evolving itself.
fn evolves_when_it_moves_to_attack_position() -> TriggerDef {
    TriggerDef::new(
        "evolveOnRising",
        &[GameEventType::PositionSwitched],
        |ctx, event| {
            let GameEvent::PositionSwitched {
                instance_id,
                position,
            } = event
            else {
                return vec![];
            };
            let mine = ctx.self_.as_ref().is_some_and(|me| me.id == *instance_id);
            if mine && *position == Position::Atk {
                vec![evolve(json_as(json!({ "target": { "of": "self" } })))]
            } else {
                vec![]
            }
        },
    )
}

/// "Evolve: Recruit [N] Swarm card[s]."
fn recruit_swarm() -> Hook {
    hook(|ctx| {
        let count = if ctx.radiant {
            param(ctx, "card")
        } else {
            BASE_CARDS
        };
        vec![recruit(json_as(json!({ "filter": swarm(), "count": count })))]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        evolve: Some(recruit_swarm()),
        triggers: vec![evolves_when_it_moves_to_attack_position()],
        ..Script::default()
    };
    // One script: the face's count is `BASE_CARDS` or its declared `card` (`ctx.radiant`).
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #88 Colony Watcher — SPEC §8.12 row 88.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SWARM: &str = "stinkyscapeplus-086"; // Astrophage: a Swarm Unit
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    /// A Colony Watcher in `position`, over a library of one Vanilla Unit on top and five Swarm cards.
    fn board(radiant: bool, position: &str) -> Scenario {
        let mut library = vec![json!(VANILLA)];
        library.extend((0..5).map(|_| json!(SWARM)));
        scenario(json!({
            "p1": {
                "hand": [VANILLA],
                "field": [{ "def": ID, "radiant": radiant, "position": position }],
                "library": library,
            },
            "p2": { "library": crate::filler(4) },
        }))
    }

    fn evolved_events(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Evolved)
            .count()
    }

    fn recruited(s: &Scenario) -> usize {
        (1..=5)
            .filter_map(|lane| s.unit(P1, lane))
            .filter(|card| card.def_id == SWARM)
            .count()
    }

    fn library_swarm(s: &Scenario) -> usize {
        s.pile(P1, "library")
            .iter()
            .filter(|card| card.def_id == SWARM)
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn moving_to_attack_position_recruits_one_swarm_card() {
            let mut s = board(false, "DEF");
            let watcher = s.unit_at(P1, 1);
            s.switch_position(&watcher);
            assert_eq!(s.card(&watcher).evolved, Some(true));
            assert_eq!(evolved_events(&s), 1);
            assert_eq!(recruited(&s), 1, "one Swarm card");
            assert_eq!(s.unit_at(P1, 2).def_id, SWARM, "R64: the leftmost open zone");
            assert_eq!(library_swarm(&s), 4);
            assert_eq!(
                s.pile(P1, "library")[0].def_id,
                VANILLA,
                "the Vanilla on top is passed over, not drawn"
            );
            assert!(s.state().pending.is_none(), "it opens no prompt");
        }

        #[test]
        fn moving_to_defense_position_evolves_nothing() {
            let mut s = board(false, "ATK");
            let watcher = s.unit_at(P1, 1);
            s.switch_position(&watcher);
            assert_ne!(s.card(&watcher).evolved, Some(true));
            assert_eq!(recruited(&s), 0);
            assert_eq!(evolved_events(&s), 0);
        }

        #[test]
        fn it_evolves_once_a_second_rise_recruits_nothing_more() {
            let mut s = board(false, "DEF");
            let watcher = s.unit_at(P1, 1);
            s.switch_position(&watcher);
            s.end_turn();
            s.end_turn();
            s.switch_position(&watcher);
            s.end_turn();
            s.end_turn();
            s.switch_position(&watcher);
            assert_eq!(recruited(&s), 1);
            assert_eq!(evolved_events(&s), 1);
        }

        #[test]
        fn a_library_with_no_swarm_card_recruits_nothing_and_it_has_evolved() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [VANILLA],
                    "field": [{ "def": ID, "position": "DEF" }],
                    "library": [VANILLA, VANILLA],
                },
                "p2": { "library": crate::filler(4) },
            }));
            let watcher = s.unit_at(P1, 1);
            s.switch_position(&watcher);
            assert_eq!(s.card(&watcher).evolved, Some(true));
            assert!(s.unit(P1, 2).is_none());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn moving_to_attack_position_recruits_four_swarm_cards() {
            let mut s = board(true, "DEF");
            let watcher = s.unit_at(P1, 1);
            s.expect_stats(&watcher, json!({ "attack": 4, "health": 14 }));
            s.switch_position(&watcher);
            assert_eq!(s.card(&watcher).evolved, Some(true));
            assert_eq!(recruited(&s), 4);
            assert_eq!(library_swarm(&s), 1);
            assert_eq!(evolved_events(&s), 1);
            assert!(s.state().pending.is_none());
        }

        #[test]
        fn moving_to_defense_position_evolves_nothing() {
            let mut s = board(true, "ATK");
            let watcher = s.unit_at(P1, 1);
            s.switch_position(&watcher);
            assert_ne!(s.card(&watcher).evolved, Some(true));
            assert_eq!(recruited(&s), 0);
        }
    }
}
