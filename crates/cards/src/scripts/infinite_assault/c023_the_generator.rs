//! SPEC §8.13 IA #23 The Generator ((3) Unit, Legendary, 4/6 → 8/12), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Start of turn: Add a random Infinite Assault card to your hand. It costs ({setCost})." (setCost 0)
//!   Radiant: "Start of turn: Add two random Infinite Assault cards to your hand. They cost ({setCost})." (setCost 0)
//!
//! M #38 H1B Printer's start-of-turn add at a set price (R386), from the set-named pool (R1846001,
//! R4577000; this card and the tokens left out, R387, §5.1); a full hand burns what it cannot hold (§2.4).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-023";

/// The set every "Infinite Assault card" names (R4577000).
const SET: &str = "Infinite Assault";

/// "a random … card", Radiant "two random … cards".
const CARDS: i32 = 1;
const RADIANT_CARDS: i32 = 2;

fn generator(cards: i32) -> Script {
    Script {
        start_of_turn: Some(hook(move |ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "set": SET },
                "count": cards,
                "costOverride": param(&*ctx, "setCost"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: generator(CARDS),
        radiant: generator(RADIANT_CARDS),
    }
}

// IA #23 The Generator — SPEC §8.13 row 23.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::catalog::def_of;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const DEALT: &str = "core-010"; // Rapid Replenish: the library's cards

    fn added_at_its_start(radiant: bool, active: &str) -> (Scenario, Vec<CardInstance>) {
        let mut s = scenario(json!({
            "seed": "the-generator",
            "active": active,
            "p1": { "field": [{ "def": ID, "radiant": radiant }], "library": [DEALT, DEALT] },
            "p2": { "library": [DEALT, DEALT] },
        }));
        s.start_turn();
        let added = s.hand(P1).into_iter().filter(|card| card.def_id != DEALT).collect();
        (s, added)
    }

    fn expect_free_cards(radiant: bool, count: usize) {
        let (s, added) = added_at_its_start(radiant, "p1");
        assert_eq!(added.len(), count);
        for card in nonempty(added, "the added cards") {
            let def = def_of(Some(s.state()), &card.def_id);
            assert_eq!(def.set, SetName::InfiniteAssault, "{}", def.id);
            assert!(!def.is_token(), "{}", def.id);
            assert_eq!(cost_now(s.state(), &card), 0, "{}", def.id);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn start_of_turn_adds_a_random_infinite_assault_card_that_costs_0() {
            expect_free_cards(false, 1);
        }

        #[test]
        fn r68_none_at_the_opponents_start_of_turn() {
            let (_, added) = added_at_its_start(false, "p2");
            assert!(added.is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn start_of_turn_adds_two_that_cost_0() {
            expect_free_cards(true, 2);
        }
    }
}
