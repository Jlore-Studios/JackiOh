//! SPEC §8.12 S+ #33 True Conservative, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S17 (#3538).
//! 3/3  
//! **Cry:** Transform ALL Units into a version from their Past.  
//! {For AI: look in the patch notes. If a card doesn’t have a past version, it’s untouched}  
//! \~\~\~  
//! 6/6  
//! **Cry:** Transform ALL Units into a version from their Past. Yours become their strongest version.
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-033";

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
