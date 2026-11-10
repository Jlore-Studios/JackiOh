//! SPEC §8.12 S+ #11 Energy Booster, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S04 (#3552).
//! 1/1  
//! **Starship Piece**  
//! **Equip**  
//! **Activate 1: Refresh** *{e.g., if its activate 4, you may use this and then you’d get another 4 activates}*  
//! \~\~\~  
//! 2/2  
//! **Starship Piece**  
//! **Equip**  
//! **Activate 2: Refresh** *{e.g., if its activate 4, you may use this and then you’d get another 4 activates}*
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-011";

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
