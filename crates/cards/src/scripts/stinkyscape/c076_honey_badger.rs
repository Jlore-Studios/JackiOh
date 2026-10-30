//! SPEC §8.10 Stinkyscape 76 Honey Badger, a card of the Stinkyscape set.
//!   Base:    "Starship Piece, Rush\nWhenever this attacks, gain a random Stinkyscape Resource." (4/6)
//!   Radiant: "Starship Piece, Rush\nWhenever this attacks, gain a random Radiant Stinkyscape
//!            Resource." (8/12)
//!
//! S #75 Badger's line ("whenever this attacks" is the `whenAttacks` hook, before any damage: declared
//! or forced attacks, R53, R2285000, never a defense or an attack called off, R44) with Rush (§6.1), a
//! printed keyword, so it may attack an enemy Unit, never the hero, the turn it lands and gain then.
//! Starship Piece: its death fuses a copy into its owner's Dormant Starship (R1852000). Radiant: a
//! Radiant Resource.

use jackioh_engine::effects::gain_resources;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-076";

/// "Gain a random Stinkyscape Resource": one, printed in the words.
const RESOURCES: i32 = 1;

fn face(radiant: bool) -> Script {
    Script {
        when_attacks: Some(hook(move |_ctx| {
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

// S 76 Honey Badger — SPEC §8.10 row 76, BUILD M12 row S 76: "with Rush, attacks an enemy Unit the turn
// it lands and gains one random Resource, never the hero that turn; radiant 8/12, Radiant Resources".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    const VANILLA: &str = "core-008";

    fn landing(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4), "field": [VANILLA] },
        }))
    }

    #[test]
    fn rush_attacks_a_unit_the_turn_it_lands_and_gains() {
        let mut s = landing(false);
        s.play(ID, json!({}));
        let target = s.unit_at(P2, 1);
        s.attack(ID, &target);
        let in_hand = crate::resources_in_hand(&s, P1);
        assert_eq!(in_hand.len(), 1);
        assert!(!in_hand[0].radiant);
    }

    #[test]
    fn not_the_hero_that_turn() {
        let mut s = landing(false);
        s.play(ID, json!({}));
        s.expect_refused(|s| s.attack(ID, "hero"));
        assert!(crate::resources_in_hand(&s, P1).is_empty());
    }

    #[test]
    fn radiant_radiant_resources() {
        let mut s = landing(true);
        s.play(ID, json!({}));
        let target = s.unit_at(P2, 1);
        s.attack(ID, &target);
        let in_hand = crate::resources_in_hand(&s, P1);
        assert_eq!(in_hand.len(), 1);
        assert!(in_hand[0].radiant);
    }
}
