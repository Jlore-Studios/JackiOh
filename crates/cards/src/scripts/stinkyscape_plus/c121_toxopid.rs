//! SPEC §8.12 S+ #121 Toxopid, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S13 (#3560).
//! The first this game that this enters your hand, add a **Crawler** to your hand.  
//! **Float** reduces mana. Friendly **Crawlers Dying** reduces mana cost by (1)  
//! **Cry:** Summon 3 **Toxopid Legs** ANYWHERE on the field *{exiling the card below it, should trigger leg’s crater}*  
//! **Crater**  
//! **Immutable**  
//! **Indestructible** while **Toxopid Legs** are on the field  
//! 10/10  
//! **Rush**  
//! **Ignores Taunt**  
//! **End of turn: Toxic 2** all enemies  
//! \~\~\~  
//! The first this game that this enters your hand, add a **Crawler** to your hand.  
//! **Float** reduces mana. Friendly **Crawlers Dying** reduces mana cost by (1)  
//! **Cry:** Summon 4 **Radiant Toxopid Legs** on your opponents field *{exiling the card below it, should trigger leg’s crater}*  
//! **Crater**  
//! **Immutable**  
//! **Indestructible** while **Toxopid Legs** are on the field  
//! 20/20  
//! **Rush**  
//! **Ignores Taunt**  
//! **End of turn: Toxic 4** all enemies
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-121";

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
