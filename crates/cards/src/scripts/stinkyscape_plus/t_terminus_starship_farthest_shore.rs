//! S+ Farthest Shore, a Terminus Starship (SPEC §8.12; §6.1 Dormant, Starship; R3539001): 0/0
//! Unit, Starship Token, Flying, Ranged. "Dormant until it launches. It comes holding Repeater v5,
//! After Burner, Patcher Nanobots, Ultra Capacitor and Healing Capsule, and can't be launched by
//! hand." The Last Voyage and The Far Vagabond summon it holding its loadout; its script is the
//! shared Terminus hull (`c080_the_last_voyage::terminus_hull`).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-terminus-starship-farthest-shore";

const REPEATER: &str = "stinkyscape-059"; // Repeater v5 (4)
const AFTER_BURNER: &str = "stinkyscape-090"; // After Burner (2)
const PATCHER: &str = "stinkyscape-093"; // Patcher Nanobots (2)
const CAPACITOR: &str = "stinkyscape-133"; // Ultra Capacitor (1)
const CAPSULE: &str = "stinkyscape-043"; // Healing Capsule (1)

/// R3539001: the fixed loadout it comes holding: 10 mana of Starship Pieces.
pub const PIECES: &[&str] = &[REPEATER, AFTER_BURNER, PATCHER, CAPACITOR, CAPSULE];

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
