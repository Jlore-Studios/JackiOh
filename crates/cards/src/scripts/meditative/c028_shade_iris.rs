//! M #28 Shade-iris (SPEC §8.8 row 28): (2) Unit, Common, 7/4 → 14/8.
//!
//! Base and Radiant: "Cry: Shuffle {curses|Ancient Curse|Ancient Curses} into your opponent's deck."
//! Engine: `shuffle_into({ defId: "meditative-028-1", count: param("curses"), player: opponent })`
//! (§6.3 Shuffle into, R80's library cap, a public `shuffledIn`), the same shape as Core #90
//! CN-Viral Injection. The curses belong to the deck's owner and are Created cards (ME-CREATED).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-028";

/// M #28.1 Ancient Curse, the token this card shuffles (§7).
const CURSE: &str = "meditative-028-1";

fn iris() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![shuffle_into(json_as(json!({
                "defId": CURSE,
                "count": param(&*ctx, "curses"),
                "player": "enemy",
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // Both faces share the one script: only the declared `curses` differs (1, radiant 2).
    let base = iris();
    let radiant = iris();
    CardScripts { base, radiant }
}

// M #28 Shade-iris — SPEC §8.8 row 28, BUILD M10 row M 28: "Cry (played or cast): one Ancient Curse
// (M 28.1) shuffled into the opponent's deck at a random position, theirs, recorded in their list
// (R311), refused by a full deck (R80); curses reads through `param()`; radiant 14/8 and two
// Curses".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const IRIS: &str = "meditative-028";
    const FILLER: &str = "core-005";

    /// R80's cap, restated from `engine/src/config.rs` so a change to it fails here by name.
    const LIBRARY_CAP: usize = 60;

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    fn library_of(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.state().players[player].library.clone()
    }

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    /// p1 plays Shade-iris (base unless `radiant_face`) at `seed`.
    fn played(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": IRIS, "radiant": radiant_face }, FILLER],
                "library": filler(3),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(IRIS, json!({}));
        s
    }

    mod m28_shade_iris {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn cry_shuffles_one_curse_into_the_opponent_s_deck_owned_by_them() {
                let mut s = played("iris-enemy-library", false);

                assert_eq!(library_of(&s, P2).len(), 5);
                let curse = library_of(&s, P2).into_iter().find(|card| card.def_id == CURSE);
                let curse = curse.expect("a curse in the opponent's library");
                assert_eq!(curse.owner, P2);
                assert_eq!(curse.controller, P2);
                assert!(!curse.radiant);
                // Nothing was added to the caster's own library.
                assert_eq!(library_of(&s, P1).len(), 3);
                assert!(!def_ids(&library_of(&s, P1)).contains(&CURSE.to_string()));
                s.expect_events(json!(["cardPlayed", "shuffledIn"]));
            }

            #[test]
            fn r311_the_victim_s_list_names_the_curse() {
                let s = played("iris-r311", false);

                // The play was public and its text names the card, so p2 knows what went in; never
                // where.
                let own = crate::js(&s.view(Some(P2)).you.own_library);
                let cards = own["cards"].as_array().cloned().unwrap_or_default();
                assert!(cards.contains(&json!({ "defId": CURSE, "radiant": false, "count": 1 })));
                assert_eq!(own["unknown"], json!(0));
                // The caster reads p2's library as a count and nothing else.
                assert!(s.view(Some(P1)).opponent.own_library.is_none());
                assert_eq!(s.view(Some(P1)).opponent.library_count, 5);
            }

            #[test]
            fn r80_a_full_deck_refuses_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "iris-library-cap",
                    "p1": { "hand": [IRIS, FILLER], "library": filler(3) },
                    "p2": { "hand": [FILLER], "library": filler(LIBRARY_CAP) },
                }));

                s.play(IRIS, json!({}));

                assert_eq!(library_of(&s, P2).len(), LIBRARY_CAP);
                assert!(!def_ids(&library_of(&s, P2)).contains(&CURSE.to_string()));
                // §8 Conventions: a fizzled clause does not un-play the card, and a Unit that
                // resolved stands on the field.
                s.expect_in_zone(IRIS, "field");
            }

            #[test]
            fn curses_reads_through_param() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "iris-param",
                    "p1": { "hand": [IRIS, FILLER], "library": filler(3) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                assert_eq!(crate::upgrade_number(&mut s, IRIS, "curses"), 2);

                s.play(IRIS, json!({}));

                assert_eq!(
                    library_of(&s, P2).iter().filter(|card| card.def_id == CURSE).count(),
                    2
                );
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn shuffles_two_curses_and_is_14_8() {
                let mut s = played("iris-radiant", true);

                assert_eq!(
                    library_of(&s, P2).iter().filter(|card| card.def_id == CURSE).count(),
                    2
                );
                let me = s.unit(P1, 1).expect("Shade-iris on the field");
                s.expect_stats(&me, json!({ "attack": 14, "health": 8, "maxHealth": 8 }));
            }
        }
    }
}
