//! SPEC §8.12 S+ #119 Eclipse, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S12 (#3545).
//! 7/15  
//! **Tribute 3**  
//! **Flying**  
//! **Ranged**  
//! **Pierce**  
//! Dealing damage **Burns 5**  
//! **Can’t Defend**  
//! **Crater**  
//! **Armor 3**  
//! \~\~\~  
//! 14/30  
//! **Tribute 3**  
//! **Flying**  
//! **Ranged**  
//! **Pierce**  
//! **Rush**  
//! Dealing damage **Burns 10**  
//! **Can’t Defend**  
//! **Crater**  
//! **Armor 3**
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-119";

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
