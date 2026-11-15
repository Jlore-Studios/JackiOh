//! S+ Vacuum Psalm, a Terminus Starship (SPEC §8.12; §6.1 Dormant, Starship; R3539001): 0/0 Unit, Starship Token,
//! Flying, Ranged. "Dormant until it launches. It comes holding Cannon v3, Beam v3 and Flak v3, and can't be
//! launched by hand." The Last Voyage and The Far Vagabond summon it holding its loadout; its script is the shared
//! Terminus hull (`c080_the_last_voyage::terminus_hull`).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-terminus-starship-vacuum-psalm";

const CANNON: &str = "stinkyscape-052"; // Cannon v3
const BEAM: &str = "stinkyscape-050"; // Beam v3
const FLAK: &str = "stinkyscape-053"; // Flak v3

/// R3539001: the fixed loadout it comes holding: 6 mana of Starship Pieces (issue #3951; its loadout
/// through v0.6.4b is `c080_the_last_voyage::TERMINUS_STARSHIPS_V0_6_4B`'s).
pub const PIECES: &[&str] = &[CANNON, BEAM, FLAK];

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
