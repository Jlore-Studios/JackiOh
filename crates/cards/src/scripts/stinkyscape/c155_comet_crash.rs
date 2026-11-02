//! SPEC §8.10 Stinkyscape 155 Comet Crash, a card of the Stinkyscape set.
//!   Base:    "Crater, Brittle 1\nDeath: Gain {resources|random Stinkyscape Resource|random Stinkyscape
//!            Resources}." (3)
//!   Radiant: "Crater, Brittle 1\nDeath: Gain {resources|random Radiant Stinkyscape Resource|random
//!            Radiant Stinkyscape Resources}." (3)
//!
//! as it enters, any way it enters, and the Lock outlasts it (R1850006, R688); Brittle 1 crumbles it at
//! its controller's first start of turn two or more turns after it arrived (§6.1, R385, R638). Death, a
//! backrow card's (§6.2), on any destroy, Brittle's included: `gain_resources` of `resources` random
//! Resources into its controller's hand (R1840000). Radiant: Radiant Resources. The number is the
//! declared `resources` (R386).

use jackioh_engine::effects::gain_resources;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-155";

fn face(radiant: bool) -> Script {
    Script {
        death: Some(hook(move |ctx| {
            vec![gain_resources(json_as(
                json!({ "count": param(&*ctx, "resources"), "radiant": radiant }),
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

// S 155 Comet Crash — SPEC §8.10 row 155, BUILD M12 row S 155.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    const NETHER: &str = "core-088"; // (4) Spell: destroy all permanents

    fn crash(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn zone(lane: i32) -> ZoneSlot {
        ZoneSlot {
            player: P1,
            row: Row::Backrow,
            lane,
        }
    }

    fn crumbled(s: &Scenario) -> bool {
        s.events()
            .iter()
            .any(|event| matches!(event, GameEvent::Crumbled { def_id, .. } if def_id == ID))
    }

    #[test]
    fn r1850006_locks_its_zone() {
        let mut s = crash(false);
        s.play(ID, json!({ "zone": 2 }));
        assert!(is_locked(s.state(), zone(2)));
        assert!(!is_locked(s.state(), zone(1)));
        assert!(crate::resources_in_hand(&s, P1).is_empty(), "no Death yet");
    }

    #[test]
    fn crumbles_at_t_plus_2_gains_3_zone_stays_locked() {
        let mut s = crash(false);
        s.play(ID, json!({ "zone": 2 }));
        s.end_turn();
        s.expect_in_zone(ID, "field");
        s.end_turn();
        assert!(
            crumbled(&s),
            "Brittle 1 crumbles it at its controller's next start of turn"
        );
        s.expect_in_zone(ID, "graveyard");
        let in_hand = crate::resources_in_hand(&s, P1);
        assert_eq!(in_hand.len(), 3);
        assert!(in_hand.iter().all(|card| !card.radiant));
        assert!(is_locked(s.state(), zone(2)), "the Lock outlasts it");
    }

    #[test]
    fn destroyed_sooner_it_gains_too() {
        let mut s = scenario(json!({
            "p1": { "hand": [NETHER, FILLER], "library": filler(4), "backrow": [ID] },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(NETHER, json!({}));
        s.expect_in_zone(ID, "graveyard");
        assert!(!crumbled(&s));
        assert_eq!(crate::resources_in_hand(&s, P1).len(), 3);
    }

    #[test]
    fn resources_reads_through_param() {
        let mut s = scenario(json!({
            "p1": { "hand": [NETHER, FILLER], "library": filler(4), "backrow": [ID] },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        set_param(s.card_mut(ID), "resources", 5);
        s.play(NETHER, json!({}));
        assert_eq!(crate::resources_in_hand(&s, P1).len(), 5);
    }

    #[test]
    fn radiant_radiant_resources() {
        let mut s = crash(true);
        s.play(ID, json!({ "zone": 1 }));
        assert!(is_locked(s.state(), zone(1)));
        s.end_turn();
        s.end_turn();
        let in_hand = crate::resources_in_hand(&s, P1);
        assert_eq!(in_hand.len(), 3);
        assert!(in_hand.iter().all(|card| card.radiant));
    }
}
