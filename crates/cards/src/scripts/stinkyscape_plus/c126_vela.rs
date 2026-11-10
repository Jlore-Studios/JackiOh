//! SPEC §8.12 S+ #126 Vela, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S13 (#3560).
//! 0/10  
//! **Can’t attack, Can’t defend**  
//! **Tribute 2**  
//! **Activate 1: Choose** an enemy tile *{backrow lane 2, unit row lane 4, etc}* deal 8 **Pierce** deal damage to all units between that row and this *{draw a line KY’s papaya style}*  
//! \~\~\~  
//! 0/20  
//! **Can’t attack, Can’t defend**  
//! **Tribute 2**  
//! **Activate 1: Choose** an enemy tile *{backrow lane 2, unit row lane 4, etc}* deal 8 **Cleave Pierce** deal damage to all units between that row and this *{draw a line KY’s papaya style}*
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-126";

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
