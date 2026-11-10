//! SPEC §8.12 S+ #86 Astrophage, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S02 (#3541).
//! 0/1  
//! **Evolve** when your opponent plays a Unit.  
//! **Evolve**: Place a Plague Counter on and steal a stat point from each Unit.  
//! \~\~\~  
//! 0/2  
//! **Evolve** when your opponent plays a Unit.  
//! **Evolve**: Place a Plague Counter on and steal 3 stat points from each Unit.
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-086";

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
