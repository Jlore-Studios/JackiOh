//! SPEC §8.12 S+ #81 Doomsayer (2) Unit 0/7 → 0/14, Mythic, a card of the Stinkyscape+ set
//! (R1420, R3535000; R3569007).
//!   Base:    "Start of turn: Exile your entire board and add a random Terminus card to your hand."
//!   Radiant: "Start of turn: Exile your entire board and add a random Radiant Terminus card to your hand."
//!
//! Engine: at the start of its controller's turn the board is exiled whole, Dormant cards included and
//! this Unit with them (`exileBoard`, R3539002: no Death, no graveyard), and then one card is drawn
//! from the Terminus pool, every Terminus card of the shipped sets, tokens included (R3552001, R380,
//! R382), through the generator, so it is never this card (R387) and R673's Glitch odds apply. The
//! Radiant face adds it Radiant. The card is added whether or not the exile left it anything to
//! exile (R3569007).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-081";

fn doomsayer(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| {
            vec![
                exile_board(json_as(json!({}))),
                add_random_from_catalog(json_as(json!({
                    "query": crate::query::terminus_cards(),
                    "radiant": radiant,
                }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: doomsayer(false),
        radiant: doomsayer(true),
    }
}

// S+ #81 Doomsayer — SPEC §8.12 row 81 (R3569007; R3539002, R3552001, R387, R673).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, next_turn_of, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const UNIT: &str = "stand-in (1) 2/3";

    fn game(seed: &str, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "field": [{ "def": ID, "radiant": radiant }, UNIT, UNIT],
                "library": filler(10),
            },
            "p2": { "field": [UNIT], "library": filler(10) },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    /// The Terminus pool of the shipped sets, tokens included.
    fn pool() -> Vec<String> {
        crate::register_all();
        crate::query::query(&crate::query::terminus_cards())
            .into_iter()
            .map(|def| def.id.clone())
            .collect()
    }

    /// What p1's hand holds that is no filler: the card the start of turn added.
    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id != "core-005")
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3569007_doomsayer_exiles_its_side_and_adds_a_terminus_card() {
            let pool = pool();
            for n in 0..6 {
                let mut s = game(&format!("doomsayer-base-{n}"), false);
                let theirs = s.unit_at(P2, 1).id;
                next_turn_of(&mut s, P1);
                // The whole of p1's board went to exile, Doomsayer with it, and the enemy's stayed.
                assert!((1..=5).all(|lane| s.unit(P1, lane).is_none()), "seed {n}");
                assert_eq!(s.pile(P1, "exile").len(), 3, "seed {n}");
                assert!(s.pile(P1, "graveyard").is_empty(), "exiled, not destroyed");
                assert_eq!(s.unit_at(P2, 1).id, theirs);
                let made = added(&s);
                assert_eq!(made.len(), 1, "seed {n}: {made:?}");
                assert!(
                    pool.contains(&made[0].def_id),
                    "{} is no Terminus card",
                    made[0].def_id
                );
                assert!(!made[0].radiant);
            }
        }

        #[test]
        fn r3569007_it_acts_at_its_controllers_turn_start_only() {
            let mut s = game("doomsayer-turns", false);
            s.end_turn();
            assert!(
                s.unit(P1, 1).is_some(),
                "the opponent's turn starting leaves it be"
            );
            assert!(added(&s).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3569007_radiant_doomsayer_adds_a_radiant_one() {
            let pool = pool();
            for n in 0..6 {
                let mut s = game(&format!("doomsayer-radiant-{n}"), true);
                let stats = s.stats(s.unit_at(P1, 1).id.as_str());
                assert_eq!((stats.attack, stats.health), (0, 14));
                next_turn_of(&mut s, P1);
                assert!((1..=5).all(|lane| s.unit(P1, lane).is_none()), "seed {n}");
                let made = added(&s);
                assert_eq!(made.len(), 1, "seed {n}: {made:?}");
                assert!(pool.contains(&made[0].def_id));
                assert!(made[0].radiant, "seed {n}: {} is Radiant", made[0].def_id);
            }
        }
    }
}
