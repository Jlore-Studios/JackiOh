//! C #24 Book of Knowledge (SPEC §8.6 row 24). (1) Spell, Book, Epic.
//!   Base:    "Draw {draw}." — draw 3
//!   Radiant: "Draw {draw}." — draw 6
//!   Engine:  "Three draws; the hand cap applies (R4). Tunes: draw 3 ↑."
//!
//! §2.4 and R58: "Draw N" is N separate draws, each with its own cast-on-draw chain, and each one
//! fatigues on an empty deck, burns into a full hand (R4, R317) or stops at a draw limit (B5 E3,
//! R457). `effects/draw` is that pipeline, so this file only names how many.
//!
//! The count is the declared number `draw` (R386), 3 or 6, read through `param`; both faces run this
//! one script.

use jackioh_engine::effects::draw;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-024";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| vec![draw(json_as(json!({ "count": param(&*ctx, "draw") })))])),
        ..Script::default()
    };
    // The same script: the Radiant face's 6 is its declared `draw`, which `param` reads off the running face.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C #24 Book of Knowledge — SPEC §8.6 row 24, BUILD M9 Classic row C 24: "Draw 3: three draws, each
// with its own cast-on-draw chain (R58), fatigue from an empty deck, a full hand burning (R317), a
// draw limit stopping the rest (§2.4); the drawn cards are never named in the opponent's view;
// radiant: draw 6; its tuned number (draw) reads through `param()` (R386)".
//
// The steal case puts C #4 Palantir in the opponent's backrow: the base face steals a Book with
// no "you may" (balance patch 1, mandatory), so the Book never resolves while Palantir stands.
// (The draw-limit half, B5 E3, is proved in C #4's own test file.)
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOOK: &str = "classic-024";
    const PALANTIR: &str = "classic-004"; // Field Spell; Aura: Your opponent can't draw more than 1 card each turn.
    const VIRUS: &str = "core-090-1"; // CN-Virus: Cast on draw: take 1 damage.
    const FILLER: &str = "core-005";
    // Library cards nobody else holds, so the opponent's view can be searched for their ids.
    const A: &str = "core-019";
    const B: &str = "core-011";
    const C: &str = "core-001";
    const D: &str = "core-020";
    const E: &str = "core-012";
    const F: &str = "core-045";
    const G: &str = "core-032";

    fn hand_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player).into_iter().map(|card| card.def_id).collect()
    }

    fn library_defs(s: &Scenario) -> Vec<String> {
        s.pile(P1, "library").into_iter().map(|card| card.def_id).collect()
    }

    fn count(s: &Scenario, kind: GameEventType) -> usize {
        s.events().iter().filter(|event| event.event_type() == kind).count()
    }

    mod c_n24_book_of_knowledge {
        use super::*;

        #[test]
        fn runs_one_script_on_both_faces() {
            crate::register_all();
            let scripts = script();
            assert_eq!(crate::card_def(ID).id, BOOK);
            // TS `expect(radiant).toBe(base)`: the Radiant face is the base script itself.
            assert!(Arc::ptr_eq(
                scripts.radiant.cry.as_ref().unwrap(),
                scripts.base.cry.as_ref().unwrap()
            ));
        }

        mod base {
            use super::*;

            #[test]
            fn draws_3_from_the_top_one_draw_each() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [BOOK], "library": [A, B, C, D] }, "p2": { "hand": [FILLER] } }));

                s.play(BOOK, json!({}));

                assert_eq!(hand_defs(&s, P1), vec![A, B, C]);
                assert_eq!(library_defs(&s), vec![D]);
                assert_eq!(count(&s, GameEventType::Drawn), 3);
            }

            #[test]
            fn r58_each_draw_has_its_own_cast_on_draw_chain_a_cn_virus_casts_and_its_draw_repeats() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [BOOK], "library": [VIRUS, A, B, C, D] }, "p2": { "hand": [FILLER] } }));

                s.play(BOOK, json!({}));

                assert_eq!(hand_defs(&s, P1), vec![A, B, C]);
                s.expect_health(P1, 29);
                assert_eq!(library_defs(&s), vec![D]);
            }

            #[test]
            fn s2_4_an_empty_deck_fatigues_one_card_then_fatigue_1_and_2() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [BOOK], "library": [A] }, "p2": { "hand": [FILLER] } }));

                s.play(BOOK, json!({}));

                assert_eq!(hand_defs(&s, P1), vec![A]);
                assert_eq!(count(&s, GameEventType::Fatigue), 2);
                s.expect_health(P1, 27);
            }

            #[test]
            fn r4_r317_a_full_hand_burns_the_overflow_into_the_graveyard_which_both_players_read() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [BOOK, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                        "library": [A, B, C],
                    },
                    "p2": { "hand": [FILLER] },
                }));

                s.play(BOOK, json!({}));

                assert_eq!(s.hand(P1).len(), 10);
                assert_eq!(hand_defs(&s, P1).last().map(String::as_str), Some(A));
                assert_eq!(count(&s, GameEventType::Burned), 2);
                let graveyard: Vec<String> = s.pile(P1, "graveyard").into_iter().map(|card| card.def_id).collect();
                assert!(graveyard.contains(&B.to_string()) && graveyard.contains(&C.to_string()));
                let theirs = serde_json::to_string(&s.view(P2)).unwrap();
                assert!(theirs.contains(B));
                assert!(theirs.contains(C));
                assert!(!theirs.contains(A));
            }

            #[test]
            fn palantir_s_base_face_steals_a_book_with_no_prompt_tributed_at_once_nothing_resolves() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [BOOK], "library": [A, B, C] },
                    "p2": { "hand": [FILLER], "backrow": [{ "def": PALANTIR, "faceUp": true }] },
                }));
                let book = s.card(BOOK).clone();

                // The base steal is mandatory (balance patch 1): no prompt opens.
                s.play(BOOK, json!({}));
                assert!(s.state().pending.is_none());
                s.expect_in_zone(PALANTIR, "graveyard");
                let stolen = s.card(book.id.as_str());
                assert_eq!(stolen.owner, P2);
                assert_eq!(stolen.zone, Zone::Hand { player: P2 });
                assert_eq!(s.pile(P1, "library").len(), 3);
                assert_eq!(hand_defs(&s, P1), Vec::<String>::new());
            }

            #[test]
            fn r97_the_drawn_cards_are_never_named_in_the_opponent_s_view() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [BOOK], "library": [A, B, C] }, "p2": { "hand": [FILLER] } }));

                s.play(BOOK, json!({}));

                let theirs = serde_json::to_string(&s.view(P2)).unwrap();
                for id in [A, B, C] {
                    assert!(!theirs.contains(id));
                }
                assert_eq!(serde_json::to_value(&s.view(P2).opponent.hand).unwrap(), json!({ "count": 3 }));
                let mine = serde_json::to_string(&s.view(P1)).unwrap();
                for id in [A, B, C] {
                    assert!(mine.contains(id));
                }
            }

            #[test]
            fn r386_an_upgrade_makes_it_draw_4_a_degrade_2() {
                crate::register_all();
                let mut up = scenario(json!({ "p1": { "hand": [BOOK], "library": [A, B, C, D, E] }, "p2": { "hand": [FILLER] } }));
                step_param(up.card_mut(BOOK), "draw", 1);
                up.play(BOOK, json!({}));
                assert_eq!(hand_defs(&up, P1), vec![A, B, C, D]);

                let mut down = scenario(json!({ "p1": { "hand": [BOOK], "library": [A, B, C, D, E] }, "p2": { "hand": [FILLER] } }));
                step_param(down.card_mut(BOOK), "draw", -1);
                down.play(BOOK, json!({}));
                assert_eq!(hand_defs(&down, P1), vec![A, B]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn draws_6() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": BOOK, "radiant": true }], "library": [A, B, C, D, E, F, G] },
                    "p2": { "hand": [FILLER] },
                }));

                s.play(BOOK, json!({}));

                assert_eq!(hand_defs(&s, P1), vec![A, B, C, D, E, F]);
                assert_eq!(library_defs(&s), vec![G]);
            }

            #[test]
            fn s2_4_six_draws_from_a_deck_of_four_four_cards_then_fatigue_1_and_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": BOOK, "radiant": true }], "library": [A, B, C, D] },
                    "p2": { "hand": [FILLER] },
                }));

                s.play(BOOK, json!({}));

                assert_eq!(hand_defs(&s, P1), vec![A, B, C, D]);
                s.expect_health(P1, 27);
            }

            #[test]
            fn r386_an_upgrade_steps_the_radiant_6_to_7() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": BOOK, "radiant": true }], "library": [A, B, C, D, E, F, G] },
                    "p2": { "hand": [FILLER] },
                }));
                step_param(s.card_mut(BOOK), "draw", 1);

                s.play(BOOK, json!({}));

                assert_eq!(hand_defs(&s, P1), vec![A, B, C, D, E, F, G]);
            }
        }
    }
}
