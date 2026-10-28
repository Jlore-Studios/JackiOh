//! SPEC §8.10 Stinkyscape 96 Ocean Tsunami, a card of the held Stinkyscape set (R1420).
//!   Base:    "Cry: Discard your hand. Fill your Resource tray with random Stinkyscape Resources." (6/6)
//!   Radiant: "Cry: Discard your hand. Fill your Resource tray with random Radiant Stinkyscape
//!            Resources." (12/12)
//!
//! Cry (R1): §6.3's Discard of its controller's whole hand (`discard_hand`, no draw from the rng), then
//! `gain_resources { fill }` (R1840004): as many random Resources as the tray has room for, so none is
//! sold (R1840001); the tray is no hand, so the discard never reaches it (R1840000). Summoned, it has
//! no Cry. Radiant: Radiant Resources.

use jackioh_engine::effects::{discard_hand, gain_resources};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-096";

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![
                discard_hand(DiscardHandArgs::default()),
                gain_resources(json_as(json!({ "fill": true, "radiant": radiant }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S 96 Ocean Tsunami — SPEC §8.10 row 96, BUILD M12 row S 96: "played with 3 cards in hand and 2
// Resources in the tray, discards the 3 and gains 6 Resources, the tray full at 8 and nothing sold;
// summoned, nothing; radiant 12/12 and Radiant Resources".
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::summon;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{def_ids, filler, run_as, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    /// Two Resources already in the tray.
    const REKNITE: &str = "stinkyscape-t-reknite";

    fn tsunami(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER, FILLER, FILLER],
                "library": filler(4),
                "tray": [REKNITE, REKNITE],
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn sold(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::ResourceSold { .. }))
            .count()
    }

    #[test]
    fn with_3_in_hand_and_2_in_the_tray_discards_3_gains_6_sells_none() {
        let mut s = tsunami(false);
        s.play(ID, json!({}));
        assert!(s.hand(P1).is_empty(), "the hand is discarded");
        let graveyard = s.pile(P1, "graveyard");
        assert_eq!(graveyard.iter().filter(|card| card.def_id == FILLER).count(), 3);
        let tray = s.pile(P1, "tray");
        assert_eq!(tray.len(), RESOURCE_TRAY_CAP as usize, "filled to the cap");
        assert_eq!(
            def_ids(&tray[..2]),
            vec![REKNITE; 2],
            "the two already there stay first"
        );
        assert!(tray[2..].iter().all(|card| !card.radiant));
        assert_eq!(sold(&s), 0, "nothing sold");
    }

    #[test]
    fn summoned_nothing_runs() {
        let mut s = tsunami(false);
        run_as(&mut s, summon(json_as(json!({ "defId": ID }))), P1);
        assert_eq!(s.hand(P1).len(), 4, "the hand is kept");
        assert_eq!(s.pile(P1, "tray").len(), 2, "nothing gained");
    }

    #[test]
    fn radiant_radiant_resources() {
        let mut s = tsunami(true);
        s.play(ID, json!({}));
        let tray = s.pile(P1, "tray");
        assert_eq!(tray.len(), RESOURCE_TRAY_CAP as usize);
        assert!(
            tray[2..].iter().all(|card| card.radiant),
            "the six gained are Radiant"
        );
        assert_eq!(sold(&s), 0);
    }
}
