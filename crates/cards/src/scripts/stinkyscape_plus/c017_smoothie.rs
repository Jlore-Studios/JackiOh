//! SPEC §8.12 S+ #17 Smoothie, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S15 (#3566).
//! There is X & Y inside the smoothie *{when added to your hand there select two random fruits to be added to the smoothie, their targets are merged together but essentially works like a fused card}*  
//! \~\~\~  
//! There is **Radiant** X & **Radiant** Y inside the smoothie
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-017";

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
