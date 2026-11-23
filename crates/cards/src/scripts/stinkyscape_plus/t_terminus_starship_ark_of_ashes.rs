//! S+ Ark of Ashes, a Terminus Starship (SPEC §8.12; §6.1 Dormant, Starship; R3539001): 0/0
//! Unit, Starship Token, Flying, Ranged. "Dormant until it launches. It comes holding Patcher
//! Nanobots, Defensive Crystal and Hull Booster, and can't be launched by hand." The Last Voyage
//! and The Far Vagabond summon it holding its loadout; its script is the shared Terminus hull
//! (`c080_the_last_voyage::terminus_hull`).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-terminus-starship-ark-of-ashes";

const PATCHER: &str = "stinkyscape-093"; // Patcher Nanobots (2)
const CRYSTAL: &str = "stinkyscape-132"; // Defensive Crystal (2)
const BOOSTER: &str = "stinkyscape-098"; // Hull Booster (1)

/// R3539001: the fixed loadout it comes holding: 5 mana of Starship Pieces.
pub const PIECES: &[&str] = &[PATCHER, CRYSTAL, BOOSTER];

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
