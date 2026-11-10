//! SPEC §8.12 S+ #5 Dingleberry Droat, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S15 (#3566).
//! 2/1  
//! **Cry: Fuse** Fruits together until you make one that costs at least (5). Add it to your hand. It costs (2).  
//! \~\~\~  
//! 4/2  
//! **Cry: Fuse** Radiant Fruits together until you make one that costs at least (5). Add it to your hand. It costs (2).
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-005";

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
