//! SPEC §8.12 S+ #35 Scatter, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S07 (#3559).
//! **Flexible**  
//! **Can’t attack**  
//! **Can’t defend**  
//! 0/6  
//! **Activate 1:** Deal 0\~2 damage to all enemies, if the unit is flying deal 8 damage instead  
//! \~\~\~  
//! **Flexible**  
//! **Can’t attack**  
//! **Can’t defend**  
//! 0/12  
//! **Activate 2:** Deal 1\~3 damage to all enemies, if the unit is flying deal 10 damage instead
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-035";

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
