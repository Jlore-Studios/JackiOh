//! SPEC §8.12 S+ #31 Soul Read, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S19 (#3567).
//! **Cry:** Look at your opponent's hand and choose a card secretly.  
//! Reveals if your opponent plays the chosen card before any of the other cards you saw. Take control of that card instead. Otherwise, Tribute this Trap.  
//! \~\~\~  
//! **Cry:** Look at your opponent's hand and choose a card secretly.  
//! Reveals if your opponent plays the chosen card. Take control of that card instead.
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-031";

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
