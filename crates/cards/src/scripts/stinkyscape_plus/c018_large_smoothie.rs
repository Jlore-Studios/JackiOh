//! SPEC §8.12 S+ #18 Large Smoothie (4) Spell, Fruit, Rare, a card of the Stinkyscape+ set (R1420;
//! R3558003, R3566001).
//!   Base:    "There is X, Y and Z inside the smoothie. When added to your hand, select three random
//!            fruits for it."
//!   Radiant: "There is Radiant X, Radiant Y and Radiant Z inside the smoothie." (SP5: the list's
//!            "Radiant X, Radiant X, & Radiant X" means X, Y and Z.)
//!
//! Engine: #17 Smoothie's script with three Fruits (`c017_smoothie::smoothie`): as it is added to a hand,
//! three random Fruit Spells of the shipped sets, the Smoothies left out (R3566001), are fused into it at
//! once (E8's `fuseRandomInto` `count`, R3558003). It keeps its instance and price (R470); played, it
//! casts all three, each asking its own targets (R77); the Radiant face's go in Radiant (R469).

use jackioh_engine::prelude::*;

use super::c017_smoothie::smoothie;

pub const ID: &str = "stinkyscapeplus-018";

/// "select three random fruits".
const FRUITS: i32 = 3;

pub fn script() -> CardScripts {
    CardScripts {
        base: smoothie(FRUITS, false),
        radiant: smoothie(FRUITS, true),
    }
}

// S+ #18 Large Smoothie — SPEC §8.12 row 18 (R3558003, R3566001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scripts::stinkyscape_plus::c017_smoothie::tests::{assert_fruits, fruits_in, handed};

    #[test]
    fn r3566001_added_to_a_hand_it_holds_three_fruits_and_keeps_its_price() {
        for n in 0..8 {
            let (s, card) = handed(&format!("r3566001-large-{n}"), ID, false);
            let fruits = fruits_in(&s, &card, ID);
            assert_fruits(&s, &fruits, 3);
            assert_eq!(effective_cost(s.state(), &card, Default::default()), 4);
        }
    }

    #[test]
    fn r3566001_radiant_its_three_fruits_go_in_radiant() {
        let (s, card) = handed("r3566001-large-radiant", ID, true);
        assert!(card.radiant);
        let fruits = fruits_in(&s, &card, ID);
        assert_fruits(&s, &fruits, 3);
        let fused = def_of(Some(s.state()), &card.def_id).clone();
        for fruit in &fruits {
            assert!(fused.base.text.contains(&crate::card_def(fruit).radiant.text), "{fruit}");
        }
    }
}
