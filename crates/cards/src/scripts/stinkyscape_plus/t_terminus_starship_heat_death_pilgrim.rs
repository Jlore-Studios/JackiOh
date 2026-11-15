//! S+ Heat Death Pilgrim, a Terminus Starship (SPEC §8.12; §6.1 Dormant, Starship; R3539001): 0/0 Unit, Starship
//! Token, Flying, Ranged. "Dormant until it launches. It comes holding Trila 3, RR-5000 and Vamp Rounds, and can't
//! be launched by hand." The Last Voyage and The Far Vagabond summon it holding its loadout; its script is the
//! shared Terminus hull (`c080_the_last_voyage::terminus_hull`).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-terminus-starship-heat-death-pilgrim";

const TRILA: &str = "stinkyscape-108"; // Trila 3
const RR_5000: &str = "stinkyscape-107"; // RR-5000
const VAMP: &str = "stinkyscape-122"; // Vamp Rounds

/// R3539001: the fixed loadout it comes holding: 6 mana of Starship Pieces (issue #3951; its loadout
/// through v0.6.2b is `c080_the_last_voyage::TERMINUS_STARSHIPS_V0_6_2B`'s).
pub const PIECES: &[&str] = &[TRILA, RR_5000, VAMP];

pub fn script() -> CardScripts {
    crate::scripts::stinkyscape_plus::c080_the_last_voyage::terminus_hull()
}

#[cfg(test)]
mod tests {
    use super::{ID, PIECES};
    use crate::scripts::stinkyscape_plus::c080_the_last_voyage::assert_terminus;

    #[test]
    fn r3539001_comes_holding_its_pieces_dormant_with_no_launch_by_hand() {
        assert_terminus(ID, PIECES, false);
    }

    #[test]
    fn radiant_comes_holding_them_radiant() {
        assert_terminus(ID, PIECES, true);
    }
}
