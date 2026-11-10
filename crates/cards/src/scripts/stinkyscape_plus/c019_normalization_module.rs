//! SPEC §8.12 S+ #19 Normalization Module, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S04 (#3552).
//! 0/2  
//! **Starship Piece**  
//! **Equip**  
//! **Activate: Vanilla** a unit  
//! \~\~\~  
//! 0/4  
//! **Starship Piece**  
//! **Equip**  
//! **Activate 2: Vanilla** a unit
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-019";

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
