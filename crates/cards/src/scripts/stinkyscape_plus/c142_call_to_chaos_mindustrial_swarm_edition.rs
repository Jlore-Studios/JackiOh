//! SPEC §8.12 S+ #142 Call to Chaos (Mindustrial-Swarm Edition), a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S24 (#3570).
//! ???  
//! *(One of the following random effects)*
//!
//! - Add 3 random **Paperclip** cards to your hand, they cost (0), **Refresh 2 mana**   
//! - Add 3 random **Evolved Swarm** cards to your hand, they cost (0)   
//! - Summon two random **Radiant** units, make them **GIGA**  
//! - Cast 5 **Jlockheed** cards  
//! - Add 3 **Terminus** cards to your hand, **Refresh 4 mana**  
//! - Add the **Toxopid** to your hand, it costs (0)  
//! - **Toxic 3** and **Burn 3** all enemies   
//! - For the rest of the game, when you **Launch** a **starship** cast a random **Call to Chaos**, summon a random (2) cost **Weapon**  
//! - Summon a **Mindustrial Golem** and a **Swarm Golem**  
//! - Cast a random **Call to Chaos**
//!
//! \~\~\~  
//! \!\!\!  
//! *(three effects)*
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-142";

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
