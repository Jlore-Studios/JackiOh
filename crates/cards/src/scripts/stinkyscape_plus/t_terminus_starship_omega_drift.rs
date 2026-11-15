//! S+ Omega Drift, a Terminus Starship (SPEC §8.12; §6.1 Dormant, Starship; R3539001): 0/0 Unit, Starship Token,
//! Flying, Ranged. "Dormant until it launches. It comes holding Beam v5 and Warpfield Amplifier, and can't be
//! launched by hand." The Last Voyage and The Far Vagabond summon it holding its loadout; its script is the shared
//! Terminus hull (`c080_the_last_voyage::terminus_hull`).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-terminus-starship-omega-drift";

const BEAM: &str = "stinkyscape-058"; // Beam v5
const WARPFIELD: &str = "stinkyscape-106"; // Warpfield Amplifier

/// R3539001: the fixed loadout it comes holding: 5 mana of Starship Pieces (issue #3951; its loadout
/// through v0.6.4b is `c080_the_last_voyage::TERMINUS_STARSHIPS_V0_6_4B`'s).
pub const PIECES: &[&str] = &[BEAM, WARPFIELD];

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
