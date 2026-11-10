//! SPEC §8.12 S+ #38 Arc, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S07 (#3559).
//! **Can’t defend**  
//! **Activate 2:** Deal 2 damage to a unit, this randomly chains to an adjacent unit *{e.g., can change to a unit if in an adjacent lane or in the backrow, repeats until no valid targets, cannot chain to the same unit twice in one activate}*  
//! 0/5  
//! \~\~\~  
//! **Can’t defend**  
//! **Activate 5:** Deal 2 damage to a unit, this randomly chains to an adjacent unit *{e.g., can change to a unit if in an adjacent lane or in the backrow, repeats until no valid targets, cannot chain to the same unit twice in one activate}*  
//! 0/10
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-038";

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
