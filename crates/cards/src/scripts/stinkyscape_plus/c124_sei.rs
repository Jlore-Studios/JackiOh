//! SPEC §8.12 S+ #124 Sei, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S13 (#3560).
//! 5/12  
//! **Tribute 2**  
//! **Ranged**  
//! **Taunt**  
//! **Armor 5**  
//! **Activate 1:** Deal 2 **Pierce Cleave** damage  
//! \~\~\~  
//! 10/24  
//! **Tribute 2**  
//! **Ranged**  
//! **Taunt**  
//! **Armor 10**  
//! **Activate 2:** Deal 2 **Pierce Cleave** damage
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-124";

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
