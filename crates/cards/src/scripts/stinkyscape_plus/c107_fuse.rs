//! SPEC §8.12 S+ #107 Fuse, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S07 (#3559).
//! 2/5  
//! **Armor 5**  
//! **Can’t Defend**  
//! **Ranged**  
//! Deal 5x damage to units in this lane, deals 3x damage to units in adjacent lanes  
//! **Pierce**  
//! \~\~\~  
//! 4/10  
//! **Armor 10**  
//! **Can’t Defend**  
//! **Ranged**  
//! Deal 7x damage to units in this lane, deals 4x damage to units in adjacent lanes  
//! **Pierce**
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-107";

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
