//! SPEC §8.14 FvH #98.1 Yarma Prime ((3) Unit, Felinor, Prime, Token, printed Legendary, 4/12 → 8/24),
//! the token FvH #98 Yarma shuffles. Built by S10 (#5260).
//!   Base:    "Cry: Shuffle {portals|Dream Portal|Dream Portals} into your deck. / Death: Draw
//!            {draw|card|cards}." (portals 7, draw 2)
//!   Radiant: "Cry: Shuffle {portals|Radiant Dream Portal|Radiant Dream Portals} into your deck. / Death:
//!            Draw {draw|Dream Portal|Dream Portals} from your deck." (portals 7, draw 2)
//!
//! As Meditative #45.1 is Meditative #45's. Cry (R1): `shuffle_into` of the declared `portals` FvH #98.2
//! Dream Portals (R386), Radiant on the Radiant face, a full deck taking what fits (R80, R316). Death,
//! base: `draw` of the declared `draw`. Death, Radiant: `draw_from_library` of a Dream Portal `draw` times,
//! the nearest the top first; with fewer in the deck it draws what there is and takes no fatigue; each is
//! a real draw, so it casts on draw (R58). Tunes: portals 7 ↑, draw 2 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-098-1";

/// FvH #98.2 Dream Portal.
const DREAM_PORTAL: &str = "felinorsvshumans-098-2";

fn shuffle_portals(radiant: bool) -> Hook {
    hook(move |ctx| {
        vec![shuffle_into(json_as(json!({
            "defId": DREAM_PORTAL,
            "count": param(&*ctx, "portals"),
            "radiant": radiant,
        })))]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(shuffle_portals(false)),
        death: Some(hook(|ctx| vec![draw(json_as(json!({ "count": param(&*ctx, "draw") })))])),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(shuffle_portals(true)),
        death: Some(hook(|ctx| {
            (0..param(&*ctx, "draw").max(0))
                .map(|_| draw_from_library(json_as(json!({ "defId": DREAM_PORTAL }))))
                .collect()
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// FvH #98.1 Yarma Prime — SPEC §8.14 row 98.1 (R1, R58, R80, R316, R386).
#[cfg(test)]
mod tests {
    use super::{DREAM_PORTAL, ID};
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;

    const HIT_JOB: &str = "core-016"; // Hit Job: the removal
    /// A textless card to fill a deck with.
    const FILLER: &str = "stand-in (9) Spell";

    /// The Dream Portals its Cry shuffles, and the cards its Death draws.
    const PORTALS: usize = 7;
    const DRAW: usize = 2;

    /// p1's library cards of `def_id`, each one's Radiant flag.
    fn in_library(s: &Scenario, def_id: &str) -> Vec<bool> {
        s.pile(P1, "library")
            .into_iter()
            .filter(|card| card.def_id == def_id)
            .map(|card| card.radiant)
            .collect()
    }

    fn drawn_by(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Drawn { player: drawer, def_id, .. } if *drawer == player => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn played(radiant: bool) -> Scenario {
        let mut s = weapon_board(
            "fvh098-1-cry",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "library": filler(4), "mana": 10 }),
            json!({}),
        );
        s.play(ID, json!({}));
        s
    }

    /// p1's Prime on the field over `library`, destroyed by p1's own Hit Job.
    fn killed(radiant: bool, library: Value) -> Scenario {
        let mut s = weapon_board(
            "fvh098-1-death",
            json!({
                "field": [{ "def": ID, "radiant": radiant }],
                "hand": [HIT_JOB],
                "library": library,
                "mana": 10,
            }),
            json!({}),
        );
        let prime = s.unit_at(P1, 1);
        s.play(HIT_JOB, pick::targets(pick::instance(&prime)));
        // The Prime is a token: it dies out of the game.
        s.expect_in_zone(&prime, "gone");
        s
    }

    fn portal(radiant: bool) -> Value {
        json!({ "def": DREAM_PORTAL, "radiant": radiant })
    }

    mod base {
        use super::*;

        #[test]
        fn its_cry_shuffles_7_dream_portals_into_your_deck() {
            let s = played(false);
            assert_eq!(in_library(&s, DREAM_PORTAL), vec![false; PORTALS]);
        }

        #[test]
        fn its_death_draws_2_from_the_top() {
            let s = killed(false, json!([FILLER, FILLER, FILLER]));
            assert_eq!(drawn_by(&s, P1), vec![FILLER; DRAW]);
            assert_eq!(s.pile(P1, "library").len(), 3 - DRAW);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_cry_shuffles_7_radiant_dream_portals_into_your_deck() {
            let s = played(true);
            assert_eq!(in_library(&s, DREAM_PORTAL), vec![true; PORTALS]);
        }

        #[test]
        fn r58_its_death_draws_2_dream_portals_from_your_deck_each_cast_on_draw() {
            let s = killed(true, json!([FILLER, portal(true), FILLER, portal(true), portal(true)]));
            assert_eq!(drawn_by(&s, P1), vec![DREAM_PORTAL; DRAW]);
            assert_eq!(in_library(&s, DREAM_PORTAL), [true]);
            assert_eq!(in_library(&s, FILLER).len(), 2);
            // Each Portal cast on draw summons a Felinor: two in all, the Prime gone.
            assert_eq!(s.last_count("summoned"), DRAW);
        }

        #[test]
        fn with_fewer_dream_portals_it_draws_what_there_is_and_takes_no_fatigue() {
            let s = killed(true, json!([FILLER, portal(true)]));
            assert_eq!(drawn_by(&s, P1), [DREAM_PORTAL]);
            assert_eq!(s.last_count("fatigue"), 0);
            assert_eq!(in_library(&s, FILLER).len(), 1);
        }
    }
}
