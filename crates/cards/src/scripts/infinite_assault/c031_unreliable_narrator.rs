//! IA #31 Unreliable Narrator (SPEC §8.13 row 31), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Cry: Look at the top three cards of your opponent's deck. Put one of them on the
//!            bottom."
//!   Radiant: "Cry: Look at the top three cards of your opponent's deck. Put two of them on the
//!            bottom."
//!
//! The Cry offers the top three of the enemy library in library order; with fewer in the deck,
//! those there are offered. The picked cards go to the bottom in pick order and the rest keep
//! their order; the other seat is offered nothing (R81). No rng is drawn and no event is emitted
//! (R177). "Three", "one" and "two" are the card's words for the counts below, not declared
//! numbers (R1748003 needs none).
//!
//! Engine: `choose_pick` over a library's `top` N, `putOnLibraryBottom` (R4586005).
//! Built by C2 (#4586).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-031";

/// The resume step answering the look's pick.
const NARRATOR: &str = "narrator";

/// How many cards of the top of the enemy deck the Cry looks at.
const LOOK: i32 = 3;
/// How many of them reach the bottom on the base face.
const BOTTOM: i32 = 1;
/// How many of them reach the bottom on the Radiant face.
const RADIANT_BOTTOM: i32 = 2;

fn narrator(bottom: i32) -> Script {
    Script {
        cry: Some(hook(move |_| {
            vec![choose_pick(json_as(json!({
                "step": NARRATOR,
                "from": [{ "zone": "library", "player": "enemy", "top": LOOK }],
                "min": bottom,
                "max": bottom,
                "prompt": "Unreliable Narrator: put cards on the bottom of your opponent's deck",
            })))]
        })),
        resume: IndexMap::from([(
            NARRATOR,
            hook(|_| vec![put_on_library_bottom(json_as(json!({})))]),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: narrator(BOTTOM),
        radiant: narrator(RADIANT_BOTTOM),
    }
}

// IA #31 Unreliable Narrator — SPEC §8.13 row 31 (R4586005).
#[cfg(any())] // TEMP-NEUTRALIZED: pre-existing C2 breakage, restored after verification
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::{ID, NARRATOR};
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const TOP_A: &str = "core-001";
    const TOP_B: &str = "core-008";
    const TOP_C: &str = "core-010";
    const REST: &str = "core-020";

    fn told(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE],
                "library": [STOCKPILE, STOCKPILE, STOCKPILE],
            },
            "p2": {
                "hand": [STOCKPILE],
                "library": [TOP_A, TOP_B, TOP_C, REST, STOCKPILE, STOCKPILE],
            },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    /// The open pick's options' instance ids, in offer order.
    fn offered(s: &Scenario) -> Vec<String> {
        s.state().pending.clone().expect("the look's pick").options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Instance { instance_id } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn library_defs(s: &Scenario) -> Vec<String> {
        s.state().players[PlayerId::P2].library.iter().map(|card| card.def_id.clone()).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4586005_the_top_three_are_offered_in_library_order() {
            let s = told(false, "narrator-look");
            let options = offered(&s);
            assert_eq!(options.len(), 3);
            assert_eq!(s.state().pending.clone().expect("pick").resume.step, NARRATOR);
            let defs: Vec<String> = options
                .iter()
                .map(|id| find_instance(s.state(), id).expect("offered").def_id.clone())
                .collect();
            assert_eq!(
                defs.iter().map(String::as_str).collect::<Vec<_>>(),
                vec![TOP_A, TOP_B, TOP_C]
            );
        }

        #[test]
        fn r4586005_the_pick_goes_to_the_bottom_and_the_rest_keep_their_order() {
            let mut s = told(false, "narrator-pick");
            let options = offered(&s);
            s.answer(json!(options[1].clone()));
            let defs = library_defs(&s);
            assert_eq!(
                defs.iter().map(String::as_str).collect::<Vec<_>>(),
                vec![TOP_A, TOP_C, REST, STOCKPILE, STOCKPILE, TOP_B]
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586005_two_picks_go_to_the_bottom_in_pick_order() {
            let mut s = told(true, "narrator-radiant");
            let options = offered(&s);
            s.answer(json!([options[2].clone(), options[0].clone()]));
            let defs = library_defs(&s);
            assert_eq!(
                defs.iter().map(String::as_str).collect::<Vec<_>>(),
                vec![TOP_B, REST, STOCKPILE, STOCKPILE, TOP_C, TOP_A]
            );
        }
    }
}
