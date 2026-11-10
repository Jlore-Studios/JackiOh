//! SPEC §8.12 S+ #24 Forced Propagation, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S18 (#3557).
//! Choose a non-**fused** card in your hand or field, shuffle 10 embiggen 40 copies of a card into your deck *{does no preserve cost changes or buffs on them}*  
//! \~\~\~  
//! Choose a non-**fused** card in your hand or field, shuffle 10 embiggen 40 copies of a card into your deck, then **buff** them *{does no preserve cost changes or buffs on them}*
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-024";

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
