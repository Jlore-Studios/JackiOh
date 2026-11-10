//! SPEC §8.12 S+ #89 Animosity, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S22 (#3569).
//! **Always starts on the bottom of your deck** *{if multiple cards have this order is random}*  
//! When your exile has 5 cards cast this *{wherever in your deck this is}*  
//! Add a random **Terminus** card to your hand.  
//! \~\~\~  
//! **Always starts on the bottom of your deck** *{if multiple cards have this order is random}*  
//! When your exile has 5 cards cast this *{wherever in your deck this is}*  
//! Add a random **Radiant Terminus** card to your hand.
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-089";

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
