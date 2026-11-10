//! SPEC §8.12 S+ #78 Slashed Dream Cries in Red, a card of the held Stinkyscape+ set (R1420, R3535000). STUB (#3535): built in S21 (#3561).
//! Slash the board into two *{exiling the middle lane and deleting all permanents inside, for AI replace it with a deep red mark the reveals the endless sea, units and field spells only effect their side of the board, spells have to be played on either of the two boards, only thing that is defacto the same is the hand and health}*.   
//! \~\~\~  
//! Slash the board into two, you have control over the slash *{can pick lane 2, 3, or 4}*
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-078";

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
