//! SPEC §8.10 Stinkyscape 96 Ocean Tsunami, a card of the Stinkyscape set.
//!   Base:    "Cry: Discard your hand. Fill your hand with random Stinkyscape Resources." (6/6)
//!   Radiant: "Cry: Discard your hand. Fill your hand with random Radiant Stinkyscape Resources."
//!            (12/12)
//!
//! Cry (R1): §6.3's Discard of its controller's whole hand (`discard_hand`, no draw from the rng), each
//! Resource in it paying its Credits as it goes (R2250001), then `gain_resources { fill }`: as many
//! random Resources as the hand has room for under its cap, into the hand (R2250000). Summoned, it has
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

// S 96 Ocean Tsunami — SPEC §8.10 row 96, BUILD M12 row S 96 (R2250000, R2250001).
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::summon;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, run_as, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    const REKNITE: &str = "stinkyscape-t-reknite";

    fn tsunami(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER, FILLER, FILLER, REKNITE, REKNITE],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    #[test]
    fn r2250000_r2250001_discards_the_hand_its_resources_paying_then_fills_it_with_resources() {
        let mut s = tsunami(false);
        s.play(ID, json!({}));
        let graveyard = s.pile(P1, "graveyard");
        assert_eq!(graveyard.iter().filter(|card| card.def_id == FILLER).count(), 3);
        assert_eq!(graveyard.iter().filter(|card| card.def_id == REKNITE).count(), 2);
        assert!(
            s.state().players.p1.market_credits.unwrap_or(0) > 0,
            "the discarded Resources paid their Credits"
        );
        let hand = s.hand(P1);
        assert_eq!(hand.len(), HAND_CAP as usize, "filled to the cap");
        assert_eq!(crate::resources_in_hand(&s, P1).len(), HAND_CAP as usize);
        assert!(hand.iter().all(|card| !card.radiant));
    }

    #[test]
    fn summoned_nothing_runs() {
        let mut s = tsunami(false);
        run_as(&mut s, summon(json_as(json!({ "defId": ID }))), P1);
        assert_eq!(s.hand(P1).len(), 6, "the hand is kept");
        assert_eq!(crate::resources_in_hand(&s, P1).len(), 2, "nothing gained");
    }

    #[test]
    fn radiant_radiant_resources() {
        let mut s = tsunami(true);
        s.play(ID, json!({}));
        let hand = s.hand(P1);
        assert_eq!(hand.len(), HAND_CAP as usize);
        assert!(hand.iter().all(|card| card.radiant), "every Resource gained is Radiant");
    }
}
