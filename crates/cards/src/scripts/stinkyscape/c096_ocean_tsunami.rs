//! SPEC §8.10 Stinkyscape 96 Ocean Tsunami, a card of the Stinkyscape set.
//!   Base:    "Tribute 1. Cry: Discard your hand. Fill your hand with random Stinkyscape Resources."
//!            (6/6, cost 4)
//!   Radiant: "Tribute 1. Cry: Discard your hand. Fill your hand with random Radiant Stinkyscape
//!            Resources." (12/12)
//!
//! Tribute 1 since issue #3837 (§6.3, as C #45 Nature Titan's): one of your units, or one Sheep Token,
//! sacrificed as the play is paid, so before the Cry. Through v0.6.0g it had none, which
//! `script_v0_6_0g` keeps for those replays (R767000).
//!
//! Cry (R1): §6.3's Discard of its controller's whole hand (`discard_hand`, no draw from the rng), each
//! Resource in it paying its Credits as it goes (R2250001), then `gain_resources { fill }`: as many
//! random Resources as the hand has room for under its cap, into the hand (R2250000). Summoned, it has
//! no Cry. Radiant: Radiant Resources.

use jackioh_engine::effects::{discard_hand, gain_resources};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-096";

/// "Tribute 1" (§6.3): one of your units, or one Sheep Token.
const TRIBUTE_COST: i32 = 1;

fn face(radiant: bool, tribute: Option<i32>) -> Script {
    Script {
        static_flags: tribute.map(|n| StaticFlags {
            tribute: Some(n),
            ..StaticFlags::default()
        }),
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
        base: face(false, Some(TRIBUTE_COST)),
        radiant: face(true, Some(TRIBUTE_COST)),
    }
}

/// R767000: no Tribute through v0.6.0g; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_0g() -> CardScripts {
    CardScripts {
        base: face(false, None),
        radiant: face(true, None),
    }
}

// S 96 Ocean Tsunami — SPEC §8.10 row 96, BUILD M12 row S 96 (R2250000, R2250001).
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::summon;
    use jackioh_engine::testkit::*;

    use super::{ID, script_v0_6_0g};
    use crate::{filler, run_as, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    const REKNITE: &str = "stinkyscape-t-reknite";
    /// The unit its Tribute 1 takes.
    const BODY: &str = "stand-in (1) 1/1";

    fn tsunami(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER, FILLER, FILLER, REKNITE, REKNITE],
                "field": [BODY],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// Issue #3837: Tribute 1, and with no unit to pay it the play is refused (R101).
    #[test]
    fn issue_3837_with_no_unit_to_tribute_it_cant_be_played() {
        for radiant in [false, true] {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(4) },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            s.expect_refused_with(|s| s.play(ID, json!({})), "Tribute");
            s.expect_in_zone(ID, "hand");
        }
    }

    #[test]
    fn issue_3837_the_tribute_dies_before_the_cry_discards() {
        for radiant in [false, true] {
            let mut s = tsunami(radiant);
            let body = s.unit(P1, 1).expect("the body").id.clone();
            s.play(ID, json!({ "tributes": [BODY] }));
            s.expect_in_zone(&body, "graveyard").expect_in_zone(ID, "field");
            let kinds: Vec<GameEventType> = s.last_events().iter().map(|event| event.event_type()).collect();
            let died = kinds.iter().position(|kind| *kind == GameEventType::Destroyed);
            let discarded = kinds.iter().position(|kind| *kind == GameEventType::Discarded);
            assert!(died.is_some() && discarded.is_some(), "radiant {radiant}");
            assert!(died < discarded, "radiant {radiant}");
        }
    }

    #[test]
    fn r767000_the_kept_v0_6_0g_script_needs_no_tribute() {
        crate::under_kept(ID, script_v0_6_0g(), || {
            let mut s = tsunami(false);
            s.play(ID, json!({}));
            s.expect_in_zone(ID, "field");
            assert!(s.unit(P1, 1).is_some_and(|unit| unit.def_id == BODY), "the body stays");
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        });
    }

    /// #3142: it costs (4) on both faces, where it cost (3).
    #[test]
    fn it_costs_4_on_both_faces() {
        for radiant in [false, true] {
            let s = tsunami(radiant);
            assert_eq!(effective_cost(s.state(), s.card(ID), Default::default()), 4, "radiant {radiant}");
        }
    }

    #[test]
    fn r2250000_r2250001_discards_the_hand_its_resources_paying_then_fills_it_with_resources() {
        let mut s = tsunami(false);
        s.play(ID, json!({ "tributes": [BODY] }));
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
        s.play(ID, json!({ "tributes": [BODY] }));
        let hand = s.hand(P1);
        assert_eq!(hand.len(), HAND_CAP as usize);
        assert!(hand.iter().all(|card| card.radiant), "every Resource gained is Radiant");
    }
}
