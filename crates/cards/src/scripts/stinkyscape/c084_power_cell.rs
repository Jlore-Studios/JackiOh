//! SPEC §8.10 Stinkyscape 84 Power Cell, a card of the Stinkyscape set.
//!   Base:    "Shuffle {scraps} Metal Scraps into your deck." (4)
//!   Radiant: the same (8).
//!
//! `shuffle_into` of fresh Metal Scraps (S #84.1) into its caster's deck, each at a random position,
//! openly, so its owner's list shows them (R311); a full deck refuses the rest (R80). The Scraps are
//! base Scraps on either face: the Radiant face prints only the larger number. Each one drawn later is
//! cast on draw and gains a Resource (S #84.1). The number is the declared `scraps` (R386).

use jackioh_engine::effects::shuffle_into;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-084";

/// S #84.1, the token it shuffles in.
const SCRAPS: &str = "stinkyscape-084-1";

fn face() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![shuffle_into(json_as(
                json!({ "defId": SCRAPS, "count": param(&*ctx, "scraps") }),
            ))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(),
        radiant: face(),
    }
}

// S 84 Power Cell — SPEC §8.10 row 84, BUILD M12 row S 84.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::{ID, SCRAPS};
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    fn cell(radiant: bool, library: usize) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(library) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn scraps_in_library(s: &Scenario) -> usize {
        s.pile(P1, "library")
            .iter()
            .filter(|card| card.def_id == SCRAPS)
            .count()
    }

    #[test]
    fn shuffles_4_scraps() {
        let mut s = cell(false, 10);
        s.play(ID, json!({}));
        assert_eq!(scraps_in_library(&s), 4);
        assert_eq!(s.pile(P1, "library").len(), 14);
        assert!(
            s.pile(P1, "library")
                .iter()
                .all(|card| !card.radiant || card.def_id != SCRAPS)
        );
    }

    #[test]
    fn r80_a_full_deck_takes_none() {
        let mut s = cell(false, LIBRARY_CAP as usize);
        s.play(ID, json!({}));
        assert_eq!(scraps_in_library(&s), 0);
        assert_eq!(s.pile(P1, "library").len(), LIBRARY_CAP as usize);
    }

    #[test]
    fn a_drawn_scrap_is_cast_and_gains() {
        let mut s = cell(false, 0);
        s.play(ID, json!({}));
        assert_eq!(scraps_in_library(&s), 4, "an empty library takes all four");
        s.start_turn();
        assert_eq!(
            scraps_in_library(&s),
            0,
            "each draw casts a Scrap and draws again (R58)"
        );
        assert_eq!(crate::resources_in_hand(&s, P1).len(), 4, "each gained a Resource");
    }

    #[test]
    fn scraps_reads_through_param() {
        let mut s = cell(false, 10);
        set_param(s.card_mut(ID), "scraps", 2);
        s.play(ID, json!({}));
        assert_eq!(scraps_in_library(&s), 2);
    }

    #[test]
    fn radiant_shuffles_8() {
        let mut s = cell(true, 10);
        s.play(ID, json!({}));
        assert_eq!(scraps_in_library(&s), 8);
    }
}
