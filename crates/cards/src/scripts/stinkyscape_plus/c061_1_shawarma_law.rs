//! SPEC §8.12 S+ #61.1 Shawarma Law, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S15 (#3566).
//! **End of turn:** Every unit eats a delicious **Shawarma** causing one of the following effects:
//!
//! - First **Shawarma** eaten gain **\+3 health** *{Shawarma was very tasty}*  
//! - Second **Shawarma** eaten gain **Can’t attack** *{too busy eating Shawarma}*  
//! - Third **Shawarma** eaten gain **Can’t defend** and **Brittle 2** *{Ate too much Shawarma}*  
//! - Subsequent **Shawarma** eaten **Freeze** *{Shawarma overload}*
//!
//! \~\~\~  
//! **End of turn:** Every unit eats a delicious **Shawarma** causing one of the following effects:
//!
//! - First **Shawarma** eaten gain **\+3 health** *{Shawarma was very tasty}*  
//! - Second **Shawarma** eaten gain **Can’t attack** *{too busy eating Shawarma}*  
//! - Third **Shawarma** eaten gain **Can’t defend** and **Brittle 2** *{Ate too much Shawarma}*  
//! - Subsequent **Shawarma** eaten **Freeze** *{Shawarma overload}*
//!
//! Ally units gain discipline and stop at one **Shawarma**
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-061-1";

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
