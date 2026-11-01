//! T-AI-2 Scaling Law (SPEC §8.7 row T-AI-2; BUILD M9 row T-AI-2). (2) Unit, AI, Token, 2/2 → 4/4.
//!   Base:    "Has +1/+1 for each AI generated card you've played this game."
//!   Radiant: "Has +2/+2 for each AI generated card you've played this game."
//!
//! A §10.4 layer-2 `setStat` over its controller's per-game count of AI-tagged plays (B5 E4,
//! `played_this_game_with_tag`): casts count (R70), a cast on draw not (R2179002), itself once played (step 4 counts a play before its
//! stats are read), a fused card carrying the tag once; the count outlives the cards. AI cards declare
//! no numbers (B8), so the per-card bonus is this file's.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-t-ai-02";

/// "+1/+1" (Radiant "+2/+2") for each AI generated card played.
const PER_AI_CARD: i32 = 1;
const RADIANT_PER_AI_CARD: i32 = 2;

/// R2476000: in its owner's hand, where hand stats leave layer 2 to the field (R243), the AI generated
/// cards its play would count, after "for each AI generated card you've played this game": those played
/// so far and, being one, itself (it counts itself once played). On the field its stats show it, so
/// it previews nothing there. Plays are public.
fn preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if ctx.zone != ConditionZone::Hand {
            return vec![];
        }
        let itself = i32::from(tags_of(ctx.state, ctx.self_).contains(&Tag::Ai));
        let count = played_this_game_with_tag(ctx.state, ctx.controller, Tag::Ai) + itself;
        vec![count_preview("for each AI generated card you've played this game", count)]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        set_stat: Some(read_hook(|args: HookArgs<'_>| {
            let per = if args.radiant { RADIANT_PER_AI_CARD } else { PER_AI_CARD };
            let bonus = per * played_this_game_with_tag(args.state, args.self_.controller, Tag::Ai);
            SetStat {
                attack: Some(bonus),
                max_health: Some(bonus),
            }
        })),
        preview: Some(preview()),
        ..Script::default()
    };
    CardScripts {
        // The same script: the hook reads the running face.
        radiant: base.clone(),
        base,
    }
}

// T-AI-2 Scaling Law — SPEC §8.7 row T-AI-2, BUILD M9 row T-AI-2.
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::{CastNewArgs, CastNewDef, cast_new};
    use jackioh_engine::testkit::*;

    const SCALING: &str = "classicplus-t-ai-02";
    const HALLUCINATION: &str = "classicplus-t-ai-03"; // (0) Spell, AI.
    const ASSISTANT: &str = "classicplus-t-ai-01"; // (1) Unit, AI.
    const FILLER: &str = "core-005";

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A direct engine call on the scenario's live state, its rng rebuilt from (seed, cursor) and the
    /// cursor written back afterwards.
    fn direct<R>(s: &mut Scenario, run: impl FnOnce(&mut EngineSink<'_>) -> R) -> R {
        let mut events: Vec<GameEvent> = Vec::new();
        let state = s.state_mut();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let out = {
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            run(&mut sink)
        };
        state.rng_cursor = rng.cursor();
        out
    }

    mod t_ai_2_scaling_law {
        use super::*;

        mod base {
            use super::*;

            /// R2476000: in hand, the AI cards its play would count, itself included: 1, then 2 after a
            /// Hallucination; played, it is 2/2 plus 2.
            #[test]
            fn r2476000_in_hand_it_previews_the_ai_cards_its_play_would_count() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [SCALING, HALLUCINATION, FILLER] }, "p2": { "hand": [FILLER] } }));
                let scaling = s.card(SCALING).id.clone();
                let label = "for each AI generated card you've played this game".to_string();
                assert_eq!(crate::preview_seen(&s, P1, &scaling), vec![(label.clone(), 1, None)]);
                s.play(HALLUCINATION, json!({}));
                assert_eq!(crate::preview_seen(&s, P1, &scaling), vec![(label, 2, None)]);
                s.play(SCALING, json!({ "zone": 1 }));
                let unit = s.unit_at(P1, 1);
                s.expect_stats(&unit, json!({ "attack": 4, "health": 4, "maxHealth": 4 }));
                assert!(crate::preview_seen(&s, P1, &scaling).is_empty());
            }

            #[test]
            fn s10_4_it_counts_itself_once_played_3_3_as_the_games_first_ai_card() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [SCALING, FILLER] }, "p2": { "hand": [FILLER] } }));
                s.play(SCALING, json!({ "zone": 1 }));
                assert_eq!(played_this_game_with_tag(s.state(), P1, Tag::Ai), 1);
                let unit = s.unit_at(P1, 1);
                s.expect_stats(&unit, json!({ "attack": 3, "health": 3, "maxHealth": 3 }));
            }

            #[test]
            fn plus_1_plus_1_for_each_ai_generated_card_played_after_it_and_the_count_outlives_the_cards() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [SCALING, HALLUCINATION, HALLUCINATION, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(SCALING, json!({ "zone": 1 }));
                let unit = s.unit_at(P1, 1);
                s.play(HALLUCINATION, json!({}));
                s.expect_stats(&unit, json!({ "attack": 4, "health": 4 }));
                s.play(HALLUCINATION, json!({}));
                s.expect_stats(&unit, json!({ "attack": 5, "health": 5 }));
                assert_eq!(s.pile(P1, "graveyard").iter().filter(|card| card.def_id == HALLUCINATION).count(), 2);
            }

            #[test]
            fn counts_the_ai_cards_played_before_it_arrived() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [HALLUCINATION, ASSISTANT, SCALING, FILLER], "mana": 10 },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(HALLUCINATION, json!({}));
                s.play(ASSISTANT, json!({ "zone": 2 }));
                s.play(SCALING, json!({ "zone": 1 }));
                let unit = s.unit_at(P1, 1);
                s.expect_stats(&unit, json!({ "attack": 5, "health": 5 }));
            }

            #[test]
            fn r70_a_cast_ai_card_counts() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [SCALING, FILLER] }, "p2": { "hand": [FILLER] } }));
                s.play(SCALING, json!({ "zone": 1 }));
                direct(&mut s, |sink| {
                    {
                        let mut ctx = make_context(
                            sink,
                            None,
                            HookOptions {
                                controller: Some(P1),
                                ..Default::default()
                            },
                        );
                        apply_effects(
                            &[cast_new(CastNewArgs {
                                def: CastNewDef::from(HALLUCINATION),
                                radiant: None,
                                how: Default::default(),
                            })],
                            &mut ctx,
                        );
                    }
                    settle(sink, SettleOptions::default());
                });
                let unit = s.unit_at(P1, 1);
                s.expect_stats(&unit, json!({ "attack": 4, "health": 4 }));
            }

            #[test]
            fn the_opponents_ai_cards_dont_count() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [FILLER], "field": [SCALING] },
                    "p2": { "hand": [HALLUCINATION, HALLUCINATION, FILLER] },
                }));
                let unit = s.unit_at(P1, 1);
                s.expect_stats(&unit, json!({ "attack": 2, "health": 2 }));
                s.play(HALLUCINATION, json!({}));
                s.expect_stats(&unit, json!({ "attack": 2, "health": 2 }));
                assert_eq!(played_this_game_with_tag(s.state(), P2, Tag::Ai), 1);
            }

            #[test]
            fn an_ai_slop_fusion_of_two_ai_cards_counts_once_s10_5_step_4_counts_each_tag_of_a_play_once() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [SCALING, HALLUCINATION, HALLUCINATION, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(SCALING, json!({ "zone": 1 }));
                let hallucinations: Vec<CardInstance> =
                    s.hand(P1).into_iter().filter(|card| card.def_id == HALLUCINATION).collect();
                let into = hallucinations.first().cloned().expect("a Hallucination to fuse into");
                let other = hallucinations.get(1).cloned().expect("a second Hallucination");
                let fused = direct(&mut s, |sink| {
                    subsystems::fuse(
                        sink,
                        json_as(json!({ "ingredients": [other], "into": into, "handPrice": "fused" })),
                    )
                });
                let fused = match fused {
                    Some(card) => card,
                    None => panic!("the fusion"),
                };
                s.play(fused.id.as_str(), json!({}));
                assert_eq!(played_this_game_with_tag(s.state(), P1, Tag::Ai), 2);
                let unit = s.unit_at(P1, 1);
                s.expect_stats(&unit, json!({ "attack": 4, "health": 4 }));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn a_4_4_with_plus_2_plus_2_for_each_6_6_as_the_games_first_8_8_after_one_more() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": SCALING, "radiant": true }, HALLUCINATION, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(SCALING, json!({ "zone": 1 }));
                let unit = s.unit_at(P1, 1);
                s.expect_stats(&unit, json!({ "attack": 6, "health": 6 }));
                s.play(HALLUCINATION, json!({}));
                s.expect_stats(&unit, json!({ "attack": 8, "health": 8 }));
            }
        }
    }
}
