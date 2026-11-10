//! SPEC §8.12 S+ #21 Jlockeed Shredder-12, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S16 (#3553).
//! 12/16  
//! **Tribute 1**  
//! **Pierce**  
//! **End of turn:** Deal 2 damage to all enemies, twice  
//! \~\~\~  
//! 24/32  
//! **Tribute 1**  
//! **Pierce**  
//! **End of turn:** Deal 5 damage to all enemies, twice
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-021";

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
