//! SPEC §8.12 S+ #157 Phantom Shipyard, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S23 (#3539).
//! 0/0  
//! **Start of turn: Fuse** a **random Starship Piece** into this  
//! **Dormant until launched**  
//! **Activate 3: Exile the shipyard and launch as a starship**  
//! *{for ai it should be similar to the Starship card where it summons a starship and such idk work on edge cases}*  
//! \~\~\~  
//! 0/0  
//! **Start of turn: Fuse** a **random Radiant Starship Piece** into this  
//! **Dormant until launched**  
//! **Activate 3: Exile the shipyard and launch as a starship**
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-157";

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
