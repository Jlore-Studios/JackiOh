//! S+ Nightfall Covenant, a Terminus Starship (SPEC §8.12; §6.1 Dormant, Starship; R3539001):
//! 0/0 Unit, Starship Token, Flying, Ranged. "Dormant until it launches. It comes holding Flak v5,
//! Cannon v4, Vamp Rounds, Shield Amplifier and Spacemine, and can't be launched by hand." The Last
//! Voyage and The Far Vagabond summon it holding its loadout; its script is the shared Terminus hull
//! (`c080_the_last_voyage::terminus_hull`).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-terminus-starship-nightfall-covenant";

const FLAK: &str = "stinkyscape-061"; // Flak v5 (4)
const CANNON: &str = "stinkyscape-056"; // Cannon v4 (3)
const VAMP: &str = "stinkyscape-122"; // Vamp Rounds (1)
const AMPLIFIER: &str = "stinkyscape-038"; // Shield Amplifier (1)
const SPACEMINE: &str = "stinkyscape-049"; // Spacemine (2)

/// R3539001: the fixed loadout it comes holding: 11 mana of Starship Pieces.
pub const PIECES: &[&str] = &[FLAK, CANNON, VAMP, AMPLIFIER, SPACEMINE];

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
