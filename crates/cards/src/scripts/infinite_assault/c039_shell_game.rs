//! IA #39 Shell Game (SPEC §8.13 row 39), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Set a Figment. Then draw a card for each face-down card you have, up to three."
//!   Radiant: "Set a Figment. Then draw a card for each face-down card you have, up to five."
//!
//! Its own Figment lands first, so the count reads it too (R4586001): a face-down card of its
//! controller's — one set by its Cloaked counts, a Revealed one does not. The draw is one card
//! per face-down card, up to the face's cap. With none, no card is drawn (R61).
//!
//! Engine: a Figment `summon`, then `draw` per face-down card (R4586001). Built by C2 (#4586).

use jackioh_engine::prelude::*;

use crate::scripts::infinite_assault::c025_1_figment::set_figments;

pub const ID: &str = "infiniteassault-039";

/// The most cards the draw counts on the base face.
const BASE_CAP: i32 = 3;
/// The most cards the draw counts on the Radiant face.
const RADIANT_CAP: i32 = 5;

fn shell(cap: i32) -> Script {
    Script {
        cry: Some(hook(move |_| {
            let mut effects = set_figments(1, false);
            effects.push(for_each_card(ForEachCardArgs {
                cards: Arc::new(move |c: &mut EffectContext<'_>| {
                    face_down_cards(&*c.state, c.controller)
                        .into_iter()
                        .take(cap.max(0) as usize)
                        .map(|card| card.id)
                        .collect()
                }),
                each: Arc::new(|_| draw(json_as(json!({ "count": 1 })))),
            }));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: shell(BASE_CAP),
        radiant: shell(RADIANT_CAP),
    }
}

// IA #39 Shell Game — SPEC §8.13 row 39 (R4586001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FIGMENT: &str = "infiniteassault-025-1";
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn cast(radiant: bool, seed: &str, backrow: Value) -> (Scenario, usize) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE],
                "backrow": backrow,
                "library": crate::filler(8),
            },
            "p2": { "hand": [STOCKPILE], "library": crate::filler(8) },
        }));
        let before = s.hand(P1).len();
        s.play(ID, json!({}));
        (s, before)
    }

    mod base {
        use super::*;

        #[test]
        fn r4586001_the_figment_counts_itself_one_card_drawn() {
            let (s, before) = cast(false, "shell-alone", json!([]));
            assert_eq!(s.hand(P1).len(), before, "played one, drew one");
        }

        #[test]
        fn r4586001_three_face_down_cards_draw_three_and_no_more() {
            let (s, before) = cast(
                false,
                "shell-cap",
                json!([FIGMENT, FIGMENT, FIGMENT, FIGMENT]),
            );
            // Its own Figment makes five; the draw stops at three.
            assert_eq!(s.hand(P1).len(), before + 2, "played one, drew three");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586001_five_face_down_cards_draw_five() {
            let (s, before) = cast(
                true,
                "shell-radiant",
                json!([FIGMENT, FIGMENT, FIGMENT, FIGMENT]),
            );
            assert_eq!(s.hand(P1).len(), before + 4, "played one, drew five");
        }
    }
}
