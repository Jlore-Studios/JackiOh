//! SPEC §8.12 S+ #17 Smoothie (3) Spell, Fruit, Rare, a card of the Stinkyscape+ set (R1420; R3558003,
//! R3566001).
//!   Base:    "There is X and Y inside the smoothie. When added to your hand, select two random fruits
//!            for it."
//!   Radiant: "There is Radiant X and Radiant Y inside the smoothie."
//!
//! Engine: E8's `fuseRandomInto` with `count` and `unfusedOnly` (R3558003). As it is added to a hand
//! (`entersHand`), two random Fruit Spells of the shipped sets are fused into it at once, as one fusion:
//! it keeps its instance and its price (R470), and played it casts both Fruits, each asking its own
//! targets (R77). The pool is every Fruit Spell but the two Smoothies (R3566001), never this card
//! (R387), and a card already a fusion is never filled again, so a Smoothie that comes back to a hand
//! keeps what it holds. The Radiant face's Fruits go in on their Radiant faces (R469).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-017";

/// "select two random fruits".
const FRUITS: i32 = 2;

/// R3566001: the Fruit Spells a Smoothie is filled from, the Smoothies left out.
pub(crate) fn fruit_pool() -> Value {
    json!({
        "tags": ["Fruit"],
        "type": "Spell",
        "excludeDefId": [ID, super::c018_large_smoothie::ID],
    })
}

/// R3558003: `count` random Fruits fused into the arriving card, its face's (`radiant`).
pub(crate) fn smoothie(count: i32, radiant: bool) -> Script {
    Script {
        enters_hand: Some(hook(move |_ctx| {
            vec![fuse_random_into(json_as(json!({
                "into": { "target": { "of": "self" } },
                "query": fruit_pool(),
                "count": count,
                "unfusedOnly": true,
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: smoothie(FRUITS, false),
        radiant: smoothie(FRUITS, true),
    }
}

// S+ #17 Smoothie — SPEC §8.12 row 17 (R3558003, R3566001).
#[cfg(test)]
pub(crate) mod tests {
    use jackioh_engine::subsystems::fuse::fused_ingredients;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    /// p1 draws `id` (`radiant` its face) at the start of their turn: what it holds once it lands.
    pub(crate) fn handed(seed: &str, id: &str, radiant: bool) -> (Scenario, CardInstance) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [FILLER], "library": [{ "def": id, "radiant": radiant }, FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.start_turn();
        let card = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id != FILLER)
            .expect("it was drawn");
        (s, card)
    }

    /// The Fruits fused into `card`, in order, without the card itself.
    pub(crate) fn fruits_in(s: &Scenario, card: &CardInstance, own: &str) -> Vec<String> {
        let ingredients = fused_ingredients(s.state(), &card.def_id).expect("a filled Smoothie");
        assert!(ingredients.contains(&own.to_string()), "{ingredients:?}");
        ingredients.into_iter().filter(|def| def != own).collect()
    }

    /// Every fused Fruit is a Fruit Spell of a shipped set, never a Smoothie.
    pub(crate) fn assert_fruits(s: &Scenario, fruits: &[String], count: usize) {
        assert_eq!(fruits.len(), count, "{fruits:?}");
        for fruit in fruits {
            let def = def_of(Some(s.state()), fruit);
            assert!(def.tags.contains(&Tag::Fruit), "{fruit}");
            assert_eq!(def.type_, CardType::Spell, "{fruit}");
            assert!(set_ships(def.set), "{fruit}");
            assert!(
                ![super::ID, crate::scripts::stinkyscape_plus::c018_large_smoothie::ID].contains(&fruit.as_str()),
                "{fruit}: never a Smoothie"
            );
        }
    }

    #[test]
    fn r3566001_added_to_a_hand_it_holds_two_fruits_and_keeps_its_price() {
        for n in 0..8 {
            let (s, card) = handed(&format!("r3566001-smoothie-{n}"), ID, false);
            let fruits = fruits_in(&s, &card, ID);
            assert_fruits(&s, &fruits, 2);
            let fused = def_of(Some(s.state()), &card.def_id).clone();
            for fruit in &fruits {
                assert!(fused.base.text.contains(&crate::card_def(fruit).base.text), "{n}: {fruit}");
            }
            assert_eq!(effective_cost(s.state(), &card, Default::default()), 3);
        }
    }

    #[test]
    fn r3566001_radiant_its_fruits_go_in_on_their_radiant_faces() {
        for n in 0..4 {
            let (s, card) = handed(&format!("r3566001-smoothie-radiant-{n}"), ID, true);
            assert!(card.radiant);
            let fruits = fruits_in(&s, &card, ID);
            assert_fruits(&s, &fruits, 2);
            let fused = def_of(Some(s.state()), &card.def_id).clone();
            for fruit in &fruits {
                assert!(fused.base.text.contains(&crate::card_def(fruit).radiant.text), "{fruit}");
            }
        }
    }
}
