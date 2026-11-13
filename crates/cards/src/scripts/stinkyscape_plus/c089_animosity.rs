//! SPEC §8.12 S+ #89 Animosity (2) Spell, Terminus, Mythic, a card of the Stinkyscape+ set
//! (R1420, R3535000; R3569007).
//!   Base:    "Always starts on the bottom of your deck. If several cards do, their order is random.
//!            When your exile has {card} cards, cast this, wherever in your deck it is.
//!            Add a random Terminus card to your hand."
//!   Radiant: "… Add a random Radiant Terminus card to your hand."
//!
//! Its Cry: one card drawn from the Terminus pool, every Terminus card of the shipped sets, tokens
//! included (R3552001, R380), through the generator, so it is never this card (R387) and R673's Glitch
//! odds apply; the Radiant face adds it Radiant (R3569007).
//!
//! The rest is E8's (#3558): `StaticFlags.starts_at_bottom` sinks it below every other card of its
//! owner's deck after the opening shuffle, several such cards shuffled among themselves there
//! (R3558006, the owner's call of 2026-11-09); and `cast_from_deck_when` casts it from its owner's
//! deck, wherever it lies, at the first settled check where their exile holds at least `card` cards
//! (R3558007, R3569007), its Cry resolving as a cast's, and the card ending in the graveyard.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-089";

fn animosity(radiant: bool) -> Script {
    Script {
        static_flags: Some(json_as(json!({ "startsAtBottom": true }))),
        cast_from_deck_when: Some(read_hook(|args| {
            // R3569007: "When your exile has {card} cards": at least that many.
            zone_count(args.state, args.self_.owner, OffFieldZone::Exile) >= param(&args, "card")
        })),
        cry: Some(hook(move |_ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": crate::query::terminus_cards(),
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: animosity(false),
        radiant: animosity(true),
    }
}

// S+ #89 Animosity — SPEC §8.12 row 89 (R3569007; R3552001, R387, R673).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    fn game(seed: &str, radiant: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": filler(10), "mana": 4 },
            "p2": { "library": filler(10) },
        }))
    }

    /// The Terminus pool of the shipped sets, tokens included.
    fn pool() -> Vec<String> {
        crate::register_all();
        crate::query::query(&crate::query::terminus_cards())
            .into_iter()
            .map(|def| def.id.clone())
            .collect()
    }

    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id != FILLER)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3569007_animosity_adds_a_random_terminus_card() {
            let pool = pool();
            let mut seen: Vec<String> = Vec::new();
            for n in 0..12 {
                let mut s = game(&format!("animosity-base-{n}"), false);
                s.play(ID, json!({}));
                let made = added(&s);
                assert_eq!(made.len(), 1, "seed {n}: {made:?}");
                assert!(
                    pool.contains(&made[0].def_id),
                    "{} is no Terminus card",
                    made[0].def_id
                );
                assert_ne!(made[0].def_id, ID, "never itself (R387)");
                assert!(!made[0].radiant);
                if !seen.contains(&made[0].def_id) {
                    seen.push(made[0].def_id.clone());
                }
            }
            assert!(seen.len() > 1, "a random pick: only {seen:?}");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3569007_radiant_animosity_adds_a_radiant_terminus_card() {
            let pool = pool();
            for n in 0..6 {
                let mut s = game(&format!("animosity-radiant-{n}"), true);
                s.play(ID, json!({}));
                let made = added(&s);
                assert_eq!(made.len(), 1, "seed {n}: {made:?}");
                assert!(pool.contains(&made[0].def_id));
                assert_ne!(made[0].def_id, ID);
                assert!(made[0].radiant, "seed {n}");
            }
        }
    }

    /// p1's deck holds Animosity (`radiant` its face) beneath two fillers, with `exiled` cards in their
    /// exile; p1 ends the turn, whose settled check casts it from the deck once that is at least `card`.
    fn deck_board(seed: &str, radiant: bool, exiled: usize) -> Scenario {
        crate::register_all();
        let exile: Vec<&str> = std::iter::repeat_n(FILLER, exiled).collect();
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [FILLER],
                "library": [FILLER, FILLER, { "def": ID, "radiant": radiant }, FILLER],
                "exile": exile,
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.end_turn();
        s
    }

    fn in_library(s: &Scenario) -> bool {
        s.pile(P1, "library").iter().any(|card| card.def_id == ID)
    }

    mod from_the_deck {
        use super::*;

        #[test]
        fn r3569007_the_fifth_exiled_card_casts_it_from_deep_in_the_deck() {
            let pool = pool();
            let four = deck_board("animosity-deck-four", false, 4);
            assert!(in_library(&four), "four exiled cast nothing");
            let s = deck_board("animosity-deck", false, 5);
            assert!(!in_library(&s), "the fifth casts it");
            assert!(s.pile(P1, "graveyard").iter().any(|card| card.def_id == ID));
            let made = added(&s);
            assert_eq!(made.len(), 1, "{made:?}");
            assert!(pool.contains(&made[0].def_id));
            assert!(!made[0].radiant);
        }

        #[test]
        fn r3569007_more_than_five_exiled_cast_it_too() {
            let s = deck_board("animosity-deck-six", false, 6);
            assert!(!in_library(&s), "at least five");
            assert_eq!(added(&s).len(), 1);
        }

        #[test]
        fn r3569007_none_exiled_cast_nothing() {
            let s = deck_board("animosity-deck-none", false, 0);
            assert!(in_library(&s));
            assert!(added(&s).is_empty());
        }

        #[test]
        fn r3569007_radiant_cast_from_the_deck_adds_a_radiant_terminus_card() {
            let s = deck_board("animosity-deck-radiant", true, 5);
            assert!(!in_library(&s));
            let made = added(&s);
            assert_eq!(made.len(), 1, "{made:?}");
            assert!(made[0].radiant);
        }

        #[test]
        fn r3558006_it_starts_on_the_bottom_of_its_owners_deck() {
            for face in [&script().base, &script().radiant] {
                assert_eq!(face.flags().starts_at_bottom, Some(true));
            }
            crate::register_all();
            let others = [
                "core-002", "core-005", "core-006", "core-008", "core-011", "core-012", "core-013", "core-015",
                "core-016", "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043",
                "core-044", "core-053", "core-055", "core-001", "core-003", "core-004", "core-009", "core-010",
            ];
            for n in 0..6 {
                let mut deck: Vec<String> = others.iter().map(|id| id.to_string()).collect();
                deck.insert(n, ID.to_string());
                let created = create_game(&CreateGameOptions {
                    seed: format!("animosity-bottom-{n}"),
                    decks: (deck.clone(), deck),
                    ..CreateGameOptions::default()
                });
                let state = begin_game(&created).state;
                let last = state.players.p1.library.last().map(|card| card.def_id.as_str());
                assert_eq!(last, Some(ID), "seed {n}: it lies at the bottom");
            }
        }
    }
}
