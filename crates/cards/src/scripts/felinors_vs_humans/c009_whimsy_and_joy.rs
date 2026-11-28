//! FvH #9 Whimsy and Joy! (SPEC §8.14 row 9): (1) Spell, Common, of the Felinors vs Humans set.
//!   Base:    "Add a Whimsy! and a Joy! to your hand."
//!   Radiant: "Add a Radiant Whimsy! and a Radiant Joy! to your hand."
//!
//! Two `add_to_hand`s, FvH #7 Whimsy! and then FvH #8 Joy!, each on the running face: a full hand burns
//! what does not fit (R317). Declares no numbers.

use jackioh_engine::prelude::*;

use crate::scripts::felinors_vs_humans::c007_whimsy::ID as WHIMSY;
use crate::scripts::felinors_vs_humans::c008_joy::ID as JOY;

pub const ID: &str = "felinorsvshumans-009";

fn whimsy_and_joy(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            [WHIMSY, JOY]
                .into_iter()
                .map(|def_id| add_to_hand(json_as(json!({ "defId": def_id, "radiant": radiant }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: whimsy_and_joy(false),
        radiant: whimsy_and_joy(true),
    }
}

// FvH #9 Whimsy and Joy! — SPEC §8.14 row 9.
#[cfg(test)]
mod tests {
    use super::{ID, JOY, WHIMSY};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    /// Stockpile: a Spell the hand keeps, so the turn does not end by itself.
    const FILLER: &str = "core-005";

    /// p1 plays Whimsy and Joy! on `radiant`'s face: the hand's cards after, by def id and face.
    fn added(radiant: bool) -> Vec<(String, bool)> {
        let mut s = scenario(json!({
            "seed": "whimsy-and-joy",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(ID, json!({}));
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id != FILLER)
            .map(|card| (card.def_id, card.radiant))
            .collect()
    }

    #[test]
    fn base_adds_a_whimsy_and_a_joy() {
        assert_eq!(
            added(false),
            vec![(WHIMSY.to_string(), false), (JOY.to_string(), false)]
        );
    }

    #[test]
    fn radiant_adds_a_radiant_whimsy_and_a_radiant_joy() {
        assert_eq!(
            added(true),
            vec![(WHIMSY.to_string(), true), (JOY.to_string(), true)]
        );
    }
}
