//! SPEC §8.10 Stinkyscape 84.1 Metal Scraps, a token of the held Stinkyscape set (R1420).
//!   Base:    "Cast on draw: Gain a random Stinkyscape Resource."
//!   Radiant: "Cast on draw: Gain a random Radiant Stinkyscape Resource."
//!
//! Cast on draw (§6.2, `StaticFlags.cast_on_draw`, M #30.1 Love Bomb's shape): drawn, it is cast at no
//! cost and counted as played (R70), its owner then drawing again under R58's chain cap. It is a token,
//! in no deck at setup (§2.6), and only S #84 Power Cell shuffles it in. Its cast is `gain_resources`
//! of one random Resource into its caster's tray (R1840000, R1840002's roll), a ninth sold at once
//! (R1840001); a Spell token, it then goes to the graveyard. Radiant: a Radiant Resource.

use jackioh_engine::effects::gain_resources;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-084-1";

/// "Gain a random Stinkyscape Resource": one, printed in the words.
const RESOURCES: i32 = 1;

fn face(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |_ctx| {
            vec![gain_resources(json_as(
                json!({ "count": RESOURCES, "radiant": radiant }),
            ))]
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

// S 84.1 Metal Scraps — SPEC §8.10 row 84.1, BUILD M12 row S 84.1: "drawn, it is cast at no cost, gains
// one random Resource into the tray and its owner draws again; a token, in no deck at setup (§2.6);
// radiant a Radiant Resource".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    /// A Metal Scraps on top of p1's library, so the next draw casts it.
    fn scraps_on_top(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [FILLER], "library": [{ "def": ID, "radiant": radiant }, FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    #[test]
    fn r58_drawn_it_is_cast_gains_and_draws_again() {
        let mut s = scraps_on_top(false);
        s.start_turn();
        let tray = s.pile(P1, "tray");
        assert_eq!(tray.len(), 1, "one Resource gained");
        assert!(card_def(&tray[0].def_id).resource.is_some());
        assert!(!tray[0].radiant);
        s.expect_in_zone(ID, "graveyard");
        assert!(
            s.hand(P1).iter().all(|card| card.def_id != ID),
            "never in the hand"
        );
        assert_eq!(
            s.hand(P1).len(),
            2,
            "the draw repeated: a Stockpile beside the first"
        );
        assert_eq!(
            s.state().players[P1].mana.current,
            s.state().players[P1].mana.max,
            "cast free"
        );
    }

    #[test]
    fn radiant_a_radiant_resource() {
        let mut s = scraps_on_top(true);
        s.start_turn();
        let tray = s.pile(P1, "tray");
        assert_eq!(tray.len(), 1);
        assert!(tray[0].radiant);
    }
}
