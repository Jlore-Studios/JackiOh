//! C #26 Rapid Draw (SPEC §8.6 row 26). (0) Spell, Common.
//!   Base:    "Draw {draw}. Then discard {discard|card|cards}." — draw 4, discard 4
//!   Radiant: "Draw {draw}. Then discard {discard|card|cards}." — draw 5, discard 4
//!   Engine:  "Four draws (Radiant five; the hand cap applies, R4), then 4 random discards (R682;
//!            fewer in hand, all of them). Tunes: draw 4 ↑; discard 4 ↓."
//!
//! The draws are §2.4's pipeline (`draw`): each its own cast-on-draw chain (R58), fatigue on an empty
//! deck, the hand cap burning the overflow (R4, R317), a draw limit stopping the rest (R457).
//!
//! "Then discard": R682 makes it random, read during resolution over the hand the draws left. The hand
//! is read as the list reaches the discard, not as the card is played — a draw that pauses (a
//! cast-on-draw card's own prompt) resumes into the rest of the list, and the discard then reads the
//! hand as it stands.
//!
//! "Fewer in hand, all of them": with no more cards in hand than the discard asks for the whole hand
//! goes, in hand order and drawing no rng. That half reads the hand lazily (`forEachCard`, as the
//! list reaches it); with more cards than that, the first half finds nothing to do and the random
//! discard takes exactly that many.
//!
//! The two numbers are the declared `draw` and `discard` (R386), read through `param`; the Radiant
//! face differs only in its `draw`, so both faces run this one script.

use jackioh_engine::effects::{ForEachCardArgs, discard, discard_random, draw, for_each_card};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-026";

/// The whole hand, when it holds no more cards than the discard asks for; otherwise nothing.
fn whole_hand_if_no_choice(ctx: &mut EffectContext<'_>) -> Vec<String> {
    if zone_count(&*ctx.state, ctx.controller, OffFieldZone::Hand) > param(&*ctx, "discard") {
        return vec![];
    }
    zone_cards(&*ctx.state, ctx.controller, OffFieldZone::Hand)
        .into_iter()
        .map(|card| card.id)
        .collect()
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![
                draw(json_as(json!({ "count": param(&*ctx, "draw") }))),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(whole_hand_if_no_choice),
                    each: Arc::new(|instance_id: &str| {
                        discard(json_as(json!({ "target": { "of": "instance", "instanceId": instance_id } })))
                    }),
                }),
                discard_random(json_as(json!({ "count": param(&*ctx, "discard") }))),
            ]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 5 draws are its declared `draw`, which `param` reads off the running face.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C #26 Rapid Draw — SPEC §8.6 row 26, BUILD M9 Classic row C 26: "Draw 4 (§2.4: burns, fatigue, a
// draw limit), then discard 4 cards at random (R682; 4 or fewer in hand → all of them), with no prompt;
// radiant: draw 5, discard 4; its tuned numbers (draw, discard) read through `param()` (R386)".
//
// The draw-limit case puts C #4 Palantir in the opponent's backrow ("Aura: Your opponent can't draw
// more than 1 card each turn", B5 E3, R457).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const RAPID: &str = "classic-026";
    const PALANTIR: &str = "classic-004"; // Field Spell; Aura: Your opponent can't draw more than 1 card each turn.
    const FILLER: &str = "core-005"; // Stockpile
    const VIRUS: &str = "core-090-1"; // CN-Virus: Cast on draw: take 1 damage.
    // Library cards nobody else holds, so a view can be searched for their ids.
    const A: &str = "core-019";
    const B: &str = "core-011";
    const C: &str = "core-001";
    const D: &str = "core-020";
    const E: &str = "core-012";
    const F: &str = "core-045";

    fn hand_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player).into_iter().map(|card| card.def_id).collect()
    }

    fn grave_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.pile(player, "graveyard").into_iter().map(|card| card.def_id).collect()
    }

    fn discarded_defs(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Discarded { def_id, .. } => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn count(s: &Scenario, kind: GameEventType) -> usize {
        s.events().iter().filter(|event| event.event_type() == kind).count()
    }

    /// TS `expect(actual).toEqual(expect.arrayContaining(expected))`.
    fn contains_all(actual: &[String], expected: &[&str]) -> bool {
        expected.iter().all(|wanted| actual.iter().any(|have| have == wanted))
    }

    mod c_n26_rapid_draw {
        use super::*;

        #[test]
        fn runs_one_script_on_both_faces_with_no_resume_step_r682_random_no_prompt() {
            crate::register_all();
            let scripts = script();
            assert_eq!(crate::card_def(ID).id, RAPID);
            // TS `expect(radiant).toBe(base)`: the Radiant face is the base script itself.
            assert!(Arc::ptr_eq(
                scripts.radiant.cry.as_ref().unwrap(),
                scripts.base.cry.as_ref().unwrap()
            ));
            assert!(scripts.base.resume.is_empty());
        }

        mod base {
            use super::*;

            #[test]
            fn draws_4_then_r682_discards_4_at_random_with_no_prompt() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [RAPID, FILLER, FILLER], "library": [A, B, C, D, E] }, "p2": { "hand": [FILLER] } }));

                s.play(RAPID, json!({}));

                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(P1).len(), 2);
                let discarded = discarded_defs(&s);
                assert_eq!(discarded.len(), 4);
                for def_id in &discarded {
                    assert!([FILLER, A, B, C, D].contains(&def_id.as_str()));
                }
                let mut expected: Vec<&str> = discarded.iter().map(String::as_str).collect();
                expected.push(RAPID);
                assert!(contains_all(&grave_defs(&s, P1), &expected));
            }

            #[test]
            fn r682_the_random_discards_come_from_the_match_rng_the_same_game_discards_the_same_cards() {
                crate::register_all();
                let opts = json!({ "p1": { "hand": [RAPID, FILLER, FILLER], "library": [A, B, C, D, E] }, "p2": { "hand": [FILLER] } });
                let mut first = scenario(opts.clone());
                first.play(RAPID, json!({}));
                let mut second = scenario(opts);
                second.play(RAPID, json!({}));
                assert_eq!(discarded_defs(&first), discarded_defs(&second));
            }

            #[test]
            fn r682_with_4_or_fewer_cards_in_hand_it_discards_all_of_them_and_asks_nothing() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [RAPID], "library": [A, B, C, D, E] }, "p2": { "hand": [FILLER] } }));

                s.play(RAPID, json!({}));

                assert!(s.state().pending.is_none());
                assert_eq!(hand_defs(&s, P1), Vec::<String>::new());
                assert_eq!(discarded_defs(&s), vec![A, B, C, D]);
            }

            #[test]
            fn r58_each_draw_has_its_own_cast_on_draw_chain_and_the_discard_then_reads_the_hand_the_draws_left() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [RAPID, FILLER], "library": [VIRUS, A, B, C, D, E] }, "p2": { "hand": [FILLER] } }));

                s.play(RAPID, json!({}));

                // The CN-Virus cast itself in the first draw's chain, which then drew A.
                s.expect_health(P1, 29);
                let library: Vec<String> = s.pile(P1, "library").into_iter().map(|card| card.def_id).collect();
                assert_eq!(library, vec![E]);
                // Four drawn into a five-card hand, then four random discards: one card left, no prompt.
                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(P1).len(), 1);
                let discarded = discarded_defs(&s);
                assert_eq!(discarded.len(), 4);
                for def_id in &discarded {
                    assert!([FILLER, A, B, C, D].contains(&def_id.as_str()));
                }
            }

            #[test]
            fn s2_4_fatigue_a_deck_of_2_draws_2_and_fatigues_twice_and_the_2_cards_in_hand_are_discarded() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [RAPID], "library": [A, B] }, "p2": { "hand": [FILLER] } }));

                s.play(RAPID, json!({}));

                s.expect_health(P1, 27);
                assert_eq!(discarded_defs(&s), vec![A, B]);
                assert!(s.state().pending.is_none());
            }

            #[test]
            fn r4_r317_a_full_hand_burns_the_overflow_then_the_discard_takes_4_at_random() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [RAPID, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                        "library": [A, B, C, D],
                    },
                    "p2": { "hand": [FILLER] },
                }));

                s.play(RAPID, json!({}));

                // Three burned on the draw, four discarded at random: six cards left, no prompt.
                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(P1).len(), 6);
                assert_eq!(count(&s, GameEventType::Burned), 3);
                assert!(contains_all(&grave_defs(&s, P1), &[B, C, D]));
                assert_eq!(discarded_defs(&s).len(), 4);
            }

            #[test]
            fn s2_4_b5_e3_a_draw_limit_stops_the_rest_under_the_opponent_s_palantir_it_draws_1_and_the_discard_takes_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [RAPID], "library": [A, B, C, D] },
                    "p2": { "hand": [FILLER], "backrow": [{ "def": PALANTIR, "faceUp": true }] },
                }));

                s.play(RAPID, json!({}));

                assert_eq!(count(&s, GameEventType::DrawLimited), 3);
                assert_eq!(discarded_defs(&s), vec![A]);
                let library: Vec<String> = s.pile(P1, "library").into_iter().map(|card| card.def_id).collect();
                assert_eq!(library, vec![B, C, D]);
            }

            #[test]
            fn r97_the_card_left_in_hand_stays_hidden_from_the_opponent_while_the_discards_are_public() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [RAPID, FILLER], "library": [A, B, C, D] }, "p2": { "hand": [FILLER] } }));

                s.play(RAPID, json!({}));

                assert!(s.state().pending.is_none());
                // Four drawn, four discarded at random: the one card left is hidden, the discards are not.
                let left = s.hand(P1).first().map(|card| card.id.clone());
                let theirs = serde_json::to_string(&s.view(P2)).unwrap();
                assert!(!theirs.contains(left.as_deref().unwrap_or("no-card-in-hand")));
                for card in s.pile(P1, "graveyard") {
                    assert!(theirs.contains(&card.id));
                }
                let mine = serde_json::to_string(&s.view(P1)).unwrap();
                for id in [A, B, C, D, FILLER] {
                    assert!(mine.contains(id));
                }
            }

            #[test]
            fn r386_an_upgrade_draws_5_a_degrade_of_discard_takes_5_an_upgrade_of_it_3() {
                crate::register_all();
                let mut draw_up = scenario(json!({ "p1": { "hand": [RAPID], "library": [A, B, C, D, E, F] }, "p2": { "hand": [FILLER] } }));
                step_param(draw_up.card_mut(RAPID), "draw", 1);
                draw_up.play(RAPID, json!({}));
                assert!(draw_up.state().pending.is_none());
                assert_eq!(draw_up.hand(P1).len(), 1);
                assert_eq!(discarded_defs(&draw_up).len(), 4);

                let mut discard_up = scenario(json!({ "p1": { "hand": [RAPID], "library": [A, B, C, D, E] }, "p2": { "hand": [FILLER] } }));
                step_param(discard_up.card_mut(RAPID), "discard", -1);
                discard_up.play(RAPID, json!({}));
                assert_eq!(discarded_defs(&discard_up).len(), 3);

                let mut discard_down = scenario(json!({
                    "p1": { "hand": [RAPID, FILLER, FILLER], "library": [A, B, C, D, E] },
                    "p2": { "hand": [FILLER] },
                }));
                step_param(discard_down.card_mut(RAPID), "discard", 1);
                discard_down.play(RAPID, json!({}));
                assert_eq!(discarded_defs(&discard_down).len(), 5);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn draws_5_then_discards_4_at_random() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": RAPID, "radiant": true }], "library": [A, B, C, D, E, F] },
                    "p2": { "hand": [FILLER] },
                }));

                s.play(RAPID, json!({}));

                assert!(s.state().pending.is_none());
                assert_eq!(s.hand(P1).len(), 1);
                assert_eq!(discarded_defs(&s).len(), 4);
            }

            #[test]
            fn r682_a_deck_of_3_leaves_3_in_hand_all_discarded_without_a_prompt() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": RAPID, "radiant": true }], "library": [A, B, C] },
                    "p2": { "hand": [FILLER] },
                }));

                s.play(RAPID, json!({}));

                assert!(s.state().pending.is_none());
                assert_eq!(discarded_defs(&s), vec![A, B, C]);
            }
        }
    }
}
