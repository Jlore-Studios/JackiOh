//! SPEC §8.12 S+ #6 Wines McCheese, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S15 (#3566).
//! 4/5  
//! **Start of Turn:** Add a Fine Fermented Grape, Stinky Cheese, or The Coin to your hand at random.  
//! \~\~\~  
//! 8/10  
//! **Start of Turn:** Add a Radiant Fine Fermented Grapes, Stinky Cheese, or The Coin to your hand at random.
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-006";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// STUB (#3535): the card's part must-pass tests replace this one with its script.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    #[test]
    fn the_stub_is_a_card_of_the_held_stinkyscape_plus_set() {
        crate::register_all();
        let def = catalog::def_of(None, super::ID);
        assert_eq!(def.set, SetName::StinkyscapePlus);
        assert!(!set_ships(def.set));
    }
}
