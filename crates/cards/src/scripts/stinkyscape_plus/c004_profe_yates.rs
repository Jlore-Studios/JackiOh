//! SPEC §8.12 S+ #4 Profe Yates, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S19 (#3567).
//! 7/7  
//! **Cry**: Exile every card with a M, O, U, S, E in its name *{cards in CN are immune}* then summon two random **traps** then choose one of the following **Activates:**
//!
//! - Activate 1: Overwhelm: Freeze all units in this lane & surrounding lanes  
//! - Activate 3: Wolf’s Maw: Pull a flying unit down *{it loses flying}*, and deal 4 damage to it  
//! - Activate 1: Iron Mandate: Fuse this into a unit, *{does not trigger this cry again}*
//!
//! *{Spawn voiceline: Every pack needs an alpha I don't see anyone else stepping up}*  
//! *{Attack voiceline: a mouse once bit my sister}*  
//! *{Death voiceline: I will not see the Northern Lights again}*  
//! \~\~\~  
//! 14/14  
//! **Cry**: Exile every card with a M, O, U, S, E in its name *{cards in CN are immune}* then summon three random **Radiant** **traps** hen choose one of the following **Activates:**
//!
//! - Activate 1: Overwhelm: Freeze all units in this lane & surrounding lanes  
//! - Activate 3: Wolf’s Maw: Pull a flying unit down *{it loses flying}*, and deal 4 damage to it  
//! - Activate 1: Iron Mandate: Fuse this into a unit, *{does not trigger this cry again}*
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-004";

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
