//! SPEC §8.12 S+ #18 Large Smoothie (4) Spell, Fruit, Rare, a card of the Stinkyscape+ set (R1420;
//! R3558003, R3566001, R4440006).
//!   Base:    "There is X, Y and Z inside the smoothie. When added to your hand, select three random
//!            fruits for it."
//!   Radiant: "There is Radiant X, Radiant Y and Radiant Z inside the smoothie." (SP5: the list's
//!            "Radiant X, Radiant X, & Radiant X" means X, Y and Z.)
//!
//! Engine: #17 Smoothie's script with three Fruits (`c017_smoothie::smoothie`): as it is added to a hand,
//! three random Fruit Spells of the shipped sets, the Smoothies left out (R3566001), are drawn and it gains
//! their text (R4440006): it stays a Large Smoothie, no fusion, at its own price; played, it does each
//! Fruit's effect in turn, each asking its own targets (R77), and plays no Fruit; the Radiant face's go in
//! Radiant (R469). Through v0.6.11d they were fused into it (`script_v0_6_11d`, R767000).

use jackioh_engine::prelude::*;

use super::c017_smoothie::{smoothie, smoothie_v0_6_11d};

pub const ID: &str = "stinkyscapeplus-018";

/// "select three random fruits".
const FRUITS: i32 = 3;

pub fn script() -> CardScripts {
    CardScripts {
        base: smoothie(FRUITS, false),
        radiant: smoothie(FRUITS, true),
    }
}

/// R767000: through v0.6.11d its Fruits were fused into it (issue #4440); `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_6_11d() -> CardScripts {
    CardScripts {
        base: smoothie_v0_6_11d(FRUITS, false),
        radiant: smoothie_v0_6_11d(FRUITS, true),
    }
}

// S+ #18 Large Smoothie — SPEC §8.12 row 18 (R3558003, R3566001, R4440006).
#[cfg(test)]
mod tests {
    use jackioh_engine::subsystems::fuse::fused_ingredients;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scripts::stinkyscape_plus::c017_smoothie::tests::{
        assert_fruits, fruits_in, gained_text, handed,
    };

    #[test]
    fn r3566001_r4440006_added_to_a_hand_it_stays_itself_gains_three_fruits_text_and_keeps_its_price() {
        for n in 0..8 {
            let (s, card) = handed(&format!("r3566001-large-{n}"), ID, false);
            let fruits = fruits_in(&s, &card, ID);
            assert_fruits(&s, &fruits, 3);
            assert_eq!(def_of(Some(s.state()), &card.def_id).name, "Large Smoothie");
            assert_eq!(effective_cost(s.state(), &card, Default::default()), 4);
        }
    }

    #[test]
    fn r3566001_r4440006_radiant_its_three_fruits_go_in_radiant() {
        let (s, card) = handed("r3566001-large-radiant", ID, true);
        assert!(card.radiant);
        let fruits = fruits_in(&s, &card, ID);
        assert_fruits(&s, &fruits, 3);
        let text = gained_text(&s, &card);
        for fruit in &fruits {
            assert!(text.contains(&crate::card_def(fruit).radiant.text), "{fruit}");
        }
    }

    /// R767000: the script kept for v0.6.11d fused the three Fruits into it (R3558003).
    #[test]
    fn r767000_the_script_kept_for_v0_6_11d_fuses_its_three_fruits_into_it() {
        crate::under_kept(ID, super::script_v0_6_11d(), || {
            let (s, card) = handed("r767000-large-kept", ID, false);
            assert_ne!(card.def_id, ID, "a fusion");
            let ingredients = fused_ingredients(s.state(), &card.def_id).expect("a filled Large Smoothie");
            assert_eq!(ingredients.len(), 4, "{ingredients:?}");
        });
    }
}
