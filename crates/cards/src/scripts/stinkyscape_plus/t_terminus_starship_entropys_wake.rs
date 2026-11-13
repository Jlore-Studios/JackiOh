//! S+ Entropy's Wake, a Terminus Starship (SPEC §8.12; §6.1 Dormant, Starship; R3539001): 0/0
//! Unit, Starship Token, Flying, Ranged. "Dormant until it launches. It comes holding Disintegrex,
//! Flak v4, Shield Amplifier and Hull Booster, and can't be launched by hand." The Last Voyage and
//! The Far Vagabond summon it holding its loadout; its script is the shared Terminus hull
//! (`c080_the_last_voyage::terminus_hull`).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-terminus-starship-entropys-wake";

const DISINTEGREX: &str = "stinkyscape-105"; // Disintegrex (2)
const FLAK: &str = "stinkyscape-057"; // Flak v4 (3)
const AMPLIFIER: &str = "stinkyscape-038"; // Shield Amplifier (1)
const BOOSTER: &str = "stinkyscape-098"; // Hull Booster (1)

/// R3539001: the fixed loadout it comes holding: 7 mana of Starship Pieces.
pub const PIECES: &[&str] = &[DISINTEGREX, FLAK, AMPLIFIER, BOOSTER];

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
