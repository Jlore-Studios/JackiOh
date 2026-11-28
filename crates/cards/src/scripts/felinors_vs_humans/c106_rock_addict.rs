//! SPEC §8.14 FvH #106 Rock Addict ((1) Unit, Human, Common, 2/3 → 4/6), a card of the Felinors vs Humans
//! set (R5235000).
//!   Base:    "Cry: Add a Dud to your hand."
//!   Radiant: "Cry: Add a Dud and an Auspicious Rock to your hand."
//!
//! Cry (R1): `add_to_hand` of M #39.3 Dud; the Radiant face then also M #39.1 Auspicious Rock. Both are
//! their base faces: the text says no Radiant. A full hand burns what does not fit (§2.4, R317). The
//! designer's sad voice line is the presentation's. Built by S11 (#5261).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-106";

/// M #39.3 Dud.
const DUD: &str = "meditative-039-3";
/// M #39.1 Auspicious Rock.
const AUSPICIOUS_ROCK: &str = "meditative-039-1";

fn addict(adds: &'static [&'static str]) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            adds.iter()
                .map(|def_id| add_to_hand(json_as(json!({ "defId": def_id }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: addict(&[DUD]),
        radiant: addict(&[DUD, AUSPICIOUS_ROCK]),
    }
}

// FvH #106 Rock Addict — SPEC §8.14 row 106.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const KEEPER: &str = "stand-in (0) 1/1"; // a play always open, so no turn ends by itself

    /// The cards the Cry added, in the order they reached the hand.
    fn added(radiant: bool) -> Vec<CardInstance> {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, KEEPER], "library": filler(4) },
            "p2": { "hand": [KEEPER], "library": filler(4) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s.hand(P1).into_iter().filter(|card| card.def_id != KEEPER).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn its_cry_adds_a_dud() {
            let got = added(false);
            assert_eq!(crate::def_ids(&got), vec![DUD]);
            assert!(!got[0].radiant);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_cry_adds_a_dud_and_an_auspicious_rock_both_on_their_base_faces() {
            let got = added(true);
            assert_eq!(crate::def_ids(&got), vec![DUD, AUSPICIOUS_ROCK]);
            for card in nonempty(got, "the added cards") {
                assert!(!card.radiant, "{} is on its base face", card.def_id);
            }
        }
    }
}
