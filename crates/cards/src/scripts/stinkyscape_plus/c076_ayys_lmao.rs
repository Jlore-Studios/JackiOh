//! SPEC §8.12 S+ #76 Ayys LMAO, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S18 (#3557).
//! Summon a mix of 3 **Cloaked** **Ayy** tokens or **Cloaked** **Small Asteroids**. {For AI: The Ayys spawn in the backrow if possible.}  
//! \~\~\~  
//! Summon a mix of 3 Radiant **Cloaked Ayy** tokens or **Cloaked** **Small Asteroids**.
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-076";

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
