//! SPEC §8.13 IA #235 The Prime Generator ((4) Unit, Mythic, 8/12 → 16/24), a card of the held
//! Infinite Assault set (R4577000).
//!   Base:    "Immune to Spells. Start of turn: Add a random Radiant Infinite Assault card to your
//!            hand. It costs ({setCost}). Death: Summon The Prime Generator, Iteration Two." (0)
//!   Radiant: "Immune to Spells. Start of turn: Add two random Radiant Infinite Assault cards to
//!            your hand. They cost ({setCost}). Death: Summon The Prime Generator, Iteration Two."
//!            (0)
//!
//! Immune to Spells is catalog data. The Start of turn adds from the set-named pool (R1846001,
//! R4577000; this card and the tokens left out, R387, §5.1) — Radiant cards at (0), a full hand
//! burning what it cannot hold (§2.4). The Death summons Iteration Two, Radiant on the Radiant
//! face (§7).
//!
//! Engine: `addRandomFromCatalog` Radiant, `summon` on `death`. Built by C10 (#4594).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-235";

/// The set every "Infinite Assault card" names (R4577000).
const SET: &str = "Infinite Assault";

/// "a random Radiant … card", Radiant "two random Radiant … cards".
const CARDS: i32 = 1;
const RADIANT_CARDS: i32 = 2;

/// Iteration Two, summoned by the Death.
const ITERATION_TWO: &str = "infiniteassault-235-1";

fn generator(cards: i32, radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "set": SET },
                "count": cards,
                "radiant": true,
                "costOverride": param(&*ctx, "setCost"),
            })))]
        })),
        death: Some(hook(move |_| {
            vec![summon(json_as(json!({ "defId": ITERATION_TWO, "radiant": radiant })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: generator(CARDS, false), radiant: generator(RADIANT_CARDS, true) }
}

// IA #235 The Prime Generator — SPEC §8.13 row 235.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::{ID, ITERATION_TWO};
    use crate::scenario;

    const BIG: &str = "stand-in (4) 7/7"; // Two of them trade the 8/12 Generator down.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    /// The Immune Generator answers no Spell: 7-attack Units trade it down in combat, two for
    /// the 12-health base face, four for the 24-health Radiant one.
    fn combat_death(radiant_gen: bool) -> Scenario {
        let beatters = if radiant_gen { 4 } else { 2 };
        let mut field = vec![];
        field.extend((0..beatters).map(|_| json!(BIG)));
        let mut s = scenario(json!({
            "seed": "prime-death",
            "active": "p2",
            "p1": { "field": [{ "def": ID, "radiant": radiant_gen }], "hand": [STOCKPILE], "library": crate::filler(4) },
            "p2": { "field": json!(field), "hand": [STOCKPILE], "library": crate::filler(4) },
        }));
        let victim = s.unit_at(P1, 1).id.clone();
        let ids: Vec<String> = (1..=beatters).map(|lane| s.unit_at(P2, lane).id.clone()).collect();
        for id in ids {
            s.attack(id, &victim);
        }
        s.expect_in_zone(&victim, "graveyard");
        s
    }

    fn added_at_its_start(radiant: bool, active: &str) -> (Scenario, Vec<CardInstance>) {
        let mut s = scenario(json!({
            "seed": "the-prime-generator",
            "active": active,
            "p1": { "field": [{ "def": ID, "radiant": radiant }], "library": [STOCKPILE, STOCKPILE] },
            "p2": { "library": [STOCKPILE, STOCKPILE] },
        }));
        s.start_turn();
        let added = s.hand(P1);
        (s, added)
    }

    fn expect_free_radiant_ia(radiant: bool, count: usize) {
        let (s, hand) = added_at_its_start(radiant, "p1");
        // The turn's natural draw is no IA card, so only the pool's additions count.
        let added: Vec<CardInstance> = hand
            .into_iter()
            .filter(|card| def_of(Some(s.state()), &card.def_id).set == SetName::InfiniteAssault)
            .collect();
        assert_eq!(added.len(), count);
        for card in nonempty(added, "the added cards") {
            let def = def_of(Some(s.state()), &card.def_id);
            assert_eq!(def.set, SetName::InfiniteAssault, "{}", def.id);
            assert!(!def.is_token(), "no token: {}", def.id);
            assert_ne!(def.id, ID, "never itself");
            assert!(card.radiant, "a Radiant card");
            assert_eq!(cost_now(s.state(), &card), 0, "{}", def.id);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn start_of_turn_adds_a_free_radiant_infinite_assault_card() {
            expect_free_radiant_ia(false, 1);
        }

        #[test]
        fn r68_none_at_the_opponents_start_of_turn() {
            let (_, added) = added_at_its_start(false, "p2");
            assert!(added.is_empty());
        }

        #[test]
        fn a_combat_death_summons_iteration_two() {
            let s = combat_death(false);
            let token = s.unit_at(P1, 1);
            assert_eq!(token.def_id, ITERATION_TWO, "Iteration Two stands");
            assert!(!token.radiant, "a plain token");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn start_of_turn_adds_two() {
            expect_free_radiant_ia(true, 2);
        }

        #[test]
        fn the_death_summons_a_radiant_iteration_two() {
            let s = combat_death(true);
            let token = s.unit_at(P1, 1);
            assert_eq!(token.def_id, ITERATION_TWO, "Iteration Two stands");
            assert!(token.radiant, "a Radiant token");
        }
    }
}
