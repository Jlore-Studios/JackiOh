//! C #24 Book of Knowledge (SPEC §8.6 row 24). (1) Spell, Book, Epic.
//!   Base:    "Draw {least}-{most}." — draw 2 or 3, 50/50
//!   Radiant: "Draw {least}-{most}." — draw 4, 5 or 6, 25/50/25
//!   Engine:  "One roll of the count, then that many draws; the hand cap applies (R4). Tunes: least 2 ↑
//!            (Radiant 4), most 3 ↑ (Radiant 6)."
//!
//! §2.4 and R58: "Draw N" is N separate draws, each with its own cast-on-draw chain, and each one
//! fatigues on an empty deck, burns into a full hand (R4, R317) or stops at a draw limit (B5 E3,
//! R457). `effects/draw` is that pipeline, so this file only names how many.
//!
//! Since #2657 the count is rolled as the Spell resolves: `least`, plus one coin flip of the match rng
//! for each card between `least` and `most`, so 2 or 3 at 50/50 and 4, 5 or 6 at 25/50/25. Both ends
//! are declared numbers (R386), read through `param`, and a Nerf that crossed them rolls between the
//! two. Through v0.5.9e it drew its declared `draw`, 3 or 6, with no roll (`script_v0_5_9e`, kept,
//! R767000). Both faces run one script.

use jackioh_engine::effects::draw;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-024";

/// "Draw `least`-`most`": `least`, and one flip of the match rng for each card past it up to `most`.
fn roll(rng: &mut Rng, least: i32, most: i32) -> i32 {
    let (lo, hi) = if least <= most { (least, most) } else { (most, least) };
    lo + (lo..hi).map(|_| rng.int(2)).sum::<i32>()
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            let (least, most) = (param(&*ctx, "least"), param(&*ctx, "most"));
            let count = roll(&mut *ctx.rng, least, most);
            vec![draw(json_as(json!({ "count": count })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 4 to 6 is its declared `least` and `most`, which `param` reads
    // off the running face.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// R767000: through v0.5.9e it drew its declared `draw`, 3 or 6, with no roll; `KEPT_SCRIPTS` folds
/// those replays under it, the definitions they carry declaring `draw`.
pub fn script_v0_5_9e() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| vec![draw(json_as(json!({ "count": param(&*ctx, "draw") })))])),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C #24 Book of Knowledge — SPEC §8.6 row 24, BUILD M9 Classic row C 24: "Draw 2 or 3 (50/50): one roll,
// then that many draws, each with its own cast-on-draw chain (R58), fatigue from an empty deck, a full
// hand burning (R317), a draw limit stopping the rest (§2.4); the drawn cards are never named in the
// opponent's view; radiant: draw 4 to 6 (25/50/25); its tuned numbers (least, most) read through
// `param()` (R386)".
//
// The pipeline cases set both ends to one count, so they draw it with no roll. The steal case puts C #4
// Palantir in the opponent's backrow: the base face steals a (1) Cost Spell with no "you may" (balance
// patch 1, mandatory) and casts it itself (patch v0.3.10, R831001), so the Book draws for the opponent
// and never for you while Palantir stands. (The draw-limit half, B5 E3, is proved in C #4's own test
// file.)
#[cfg(test)]
mod tests {
    use super::*;
    use crate::def_ids;
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

    fn count(s: &Scenario, kind: GameEventType) -> usize {
        s.events().iter().filter(|event| event.event_type() == kind).count()
    }

    /// Both ends of the Book in hand set to `n`, so it draws `n` with no roll.
    fn fixed(s: &mut Scenario, n: i32) {
        set_param(s.card_mut(BOOK), "least", n);
        set_param(s.card_mut(BOOK), "most", n);
    }

    /// How many cards the Book drew over `seeds` seeds of the given face, each count's tally.
    fn counts(radiant: bool, seeds: usize) -> IndexMap<usize, usize> {
        let mut seen: IndexMap<usize, usize> = IndexMap::new();
        for n in 0..seeds {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": format!("c024-roll-{radiant}-{n}"),
                "p1": { "hand": [{ "def": BOOK, "radiant": radiant }], "library": [A, B, C, D, E, F, G] },
                "p2": { "hand": [FILLER] },
            }));
            let cursor = s.state().rng_cursor;
            s.play(BOOK, json!({}));
            let drawn = s.hand(P1).len();
            *seen.entry(drawn).or_default() += 1;
            // One flip per card past the low end, and no other draw.
            let flips = if radiant { 2 } else { 1 };
            assert_eq!(s.state().rng_cursor, cursor + flips, "seed {n}");
        }
        seen
    }

    mod c_n24_book_of_knowledge {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn set_to_3_it_draws_3_from_the_top_one_draw_each() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [BOOK], "library": [A, B, C, D] }, "p2": { "hand": [FILLER] } }));
                fixed(&mut s, 3);

                s.play(BOOK, json!({}));

                assert_eq!(def_ids(&s.hand(P1)), vec![A, B, C]);
                assert_eq!(def_ids(&s.pile(P1, "library")), vec![D]);
                assert_eq!(count(&s, GameEventType::Drawn), 3);
            }

            #[test]
            fn r58_each_draw_has_its_own_cast_on_draw_chain_a_cn_virus_casts_and_its_draw_repeats() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [BOOK], "library": [VIRUS, A, B, C, D] }, "p2": { "hand": [FILLER] } }));
                fixed(&mut s, 3);

                s.play(BOOK, json!({}));

                assert_eq!(def_ids(&s.hand(P1)), vec![A, B, C]);
                s.expect_health(P1, 29);
                assert_eq!(def_ids(&s.pile(P1, "library")), vec![D]);
            }

            #[test]
            fn s2_4_an_empty_deck_fatigues_one_card_then_fatigue_1_and_2() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [BOOK], "library": [A] }, "p2": { "hand": [FILLER] } }));
                fixed(&mut s, 3);

                s.play(BOOK, json!({}));

                assert_eq!(def_ids(&s.hand(P1)), vec![A]);
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
                fixed(&mut s, 3);

                s.play(BOOK, json!({}));

                assert_eq!(s.hand(P1).len(), 10);
                assert_eq!(def_ids(&s.hand(P1)).last().map(String::as_str), Some(A));
                assert_eq!(count(&s, GameEventType::Burned), 2);
                let graveyard = def_ids(&s.pile(P1, "graveyard"));
                assert!(graveyard.contains(&B.to_string()) && graveyard.contains(&C.to_string()));
                let theirs = serde_json::to_string(&s.view(P2)).unwrap();
                assert!(theirs.contains(B));
                assert!(theirs.contains(C));
                assert!(!theirs.contains(A));
            }

            #[test]
            fn r831001_palantir_s_base_face_steals_the_book_with_no_prompt_and_its_3_draws_are_the_opponent_s() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [BOOK], "library": [A, B, C] },
                    "p2": { "hand": [FILLER], "backrow": [{ "def": PALANTIR, "faceUp": true }], "library": [D, E, F] },
                }));
                fixed(&mut s, 3);
                let book = s.card(BOOK).clone();

                // The base steal is mandatory (balance patch 1): no prompt opens.
                s.play(BOOK, json!({}));
                assert!(s.state().pending.is_none());
                s.expect_in_zone(PALANTIR, "graveyard");
                // Cast by its new owner, it ends in their graveyard and draws them 3.
                let stolen = s.card(book.id.as_str());
                assert_eq!(stolen.owner, P2);
                assert_eq!(stolen.zone, Zone::Graveyard { player: P2 });
                assert_eq!(def_ids(&s.hand(P2)), [FILLER, D, E, F]);
                assert_eq!(s.pile(P1, "library").len(), 3);
                assert_eq!(def_ids(&s.hand(P1)), Vec::<String>::new());
            }

            #[test]
            fn r97_the_drawn_cards_are_never_named_in_the_opponent_s_view() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [BOOK], "library": [A, B, C] }, "p2": { "hand": [FILLER] } }));
                fixed(&mut s, 3);

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
            fn r386_an_upgrade_of_both_ends_makes_it_draw_3_or_4() {
                crate::register_all();
                let mut seen: IndexSet<usize> = IndexSet::new();
                for n in 0..24 {
                    let mut s = scenario(json!({
                        "seed": format!("c024-up-{n}"),
                        "p1": { "hand": [BOOK], "library": [A, B, C, D, E] },
                        "p2": { "hand": [FILLER] },
                    }));
                    step_param(s.card_mut(BOOK), "least", 1);
                    step_param(s.card_mut(BOOK), "most", 1);
                    s.play(BOOK, json!({}));
                    seen.insert(s.hand(P1).len());
                }
                let mut seen: Vec<usize> = seen.into_iter().collect();
                seen.sort();
                assert_eq!(seen, vec![3, 4]);
            }

            #[test]
            fn draws_2_or_3_at_50_50_one_flip_of_the_rng() {
                let seen = counts(false, 200);
                let (two, three) = (seen.get(&2).copied().unwrap_or(0), seen.get(&3).copied().unwrap_or(0));
                assert_eq!(two + three, 200, "2 or 3 only: {seen:?}");
                assert!(two > 70 && three > 70, "about half each: {seen:?}");
            }

            #[test]
            fn a_nerf_that_crossed_the_ends_rolls_between_them() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "c024-crossed",
                    "p1": { "hand": [BOOK], "library": [A, B, C, D, E] },
                    "p2": { "hand": [FILLER] },
                }));
                set_param(s.card_mut(BOOK), "least", 4);
                s.play(BOOK, json!({}));
                assert!((3..=4).contains(&s.hand(P1).len()));
            }

            #[test]
            fn r767000_the_script_kept_for_v0_5_9e_draws_its_declared_draw_with_no_roll() {
                let old = |def: &mut CardDef| {
                    let declared: Vec<Param> = serde_json::from_value(json!([{
                        "key": "draw", "base": 3, "radiant": 6, "better": "up", "step": 1, "min": 1
                    }]))
                    .expect("v0.5.9e's draw");
                    def.params.get_or_insert_with(Vec::new).extend(declared);
                };
                crate::under_kept_as(BOOK, script_v0_5_9e(), old, || {
                    let mut s = scenario(json!({ "p1": { "hand": [BOOK], "library": [A, B, C, D] }, "p2": { "hand": [FILLER] } }));
                    let cursor = s.state().rng_cursor;
                    s.play(BOOK, json!({}));
                    assert_eq!(def_ids(&s.hand(P1)), vec![A, B, C]);
                    assert_eq!(s.state().rng_cursor, cursor, "no roll");
                });
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn draws_4_5_or_6_at_25_50_25_two_flips_of_the_rng() {
                let seen = counts(true, 200);
                let [four, five, six] = [4, 5, 6].map(|n| seen.get(&n).copied().unwrap_or(0));
                assert_eq!(four + five + six, 200, "4 to 6 only: {seen:?}");
                assert!(five > four && five > six && four > 20 && six > 20, "25/50/25: {seen:?}");
            }

            #[test]
            fn s2_4_six_draws_from_a_deck_of_four_four_cards_then_fatigue_1_and_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": BOOK, "radiant": true }], "library": [A, B, C, D] },
                    "p2": { "hand": [FILLER] },
                }));
                fixed(&mut s, 6);

                s.play(BOOK, json!({}));

                assert_eq!(def_ids(&s.hand(P1)), vec![A, B, C, D]);
                s.expect_health(P1, 27);
            }

            #[test]
            fn r386_its_ends_read_through_param() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": BOOK, "radiant": true }], "library": [A, B, C, D, E, F, G] },
                    "p2": { "hand": [FILLER] },
                }));
                fixed(&mut s, 7);

                s.play(BOOK, json!({}));

                assert_eq!(def_ids(&s.hand(P1)), vec![A, B, C, D, E, F, G]);
            }
        }
    }
}
