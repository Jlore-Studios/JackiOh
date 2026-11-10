//! SPEC §8.12 S+ #66 Plastanium Wall, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S08 (#3555).
//! 0/4 embiggen 0/16  
//! **Can’t attack**  
//! **Can’t defend**  
//! **End of turn:** Gain **Divine Shield**  
//! **Embiggen: GIGA**  
//! \~\~\~  
//! 0/8 embiggen 0/32  
//! **Can’t attack**  
//! **Can’t defend**  
//! **End of turn:** Gain **Divine Shield**  
//! **Armor 2**  
//! **Embiggen: GIGA**
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-066";

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
