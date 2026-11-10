//! SPEC §8.12 S+ #142.1 Mindustrial Golem, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S24 (#3570).
//! 5/5  
//! **Rush**  
//! **Ranged**  
//! After this attacks summon a **Random Mindustrial card** that **can’t attack**  
//! \~\~\~   
//! 10/10  
//! **Rush**  
//! **Ranged**  
//! After this attacks summon a **Random Radiant Mindustrial card** that **can’t attack**
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-142-1";

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
