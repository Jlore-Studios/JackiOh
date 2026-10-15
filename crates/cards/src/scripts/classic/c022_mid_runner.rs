//! C #22 Mid Runner (SPEC §8.6 row 22). (1) Unit, Human, Common, 2/1 → 4/2.
//!   Base:    "Cry: If this is in midlane, Tribute it. If you had {threshold} or more mana when you
//!            played this, bounce {bounces|random enemy permanent|random enemy permanents}." — 4, 2
//!   Radiant: the same text, bounce 3; the doubled stats are the designer's word (R275, `docs/radiant-audit.md`).
//!
//! Two independent checks, read as the Cry begins, in the text's order. Midlane comes from the lane
//! count (R685); there the card Tributes itself: §6.3's Sacrifice, a death that bypasses Indestructible.
//! "When you played this" is the mana before paying (§10.5 step 1), `ctx.manaBeforePlay` (a cast's too,
//! R70); a Cry run any other way (E13) reads its controller's mana as it runs. The bounce takes that
//! many DIFFERENT random enemy permanents (R60: unit-pile tops, R13, and backrow, face-down included)
//! to their controllers' hands (R747): a full hand burns it (R4, R317), a unit token ceases to exist
//! (R11), and no view of theirs names a bounced face-down card (R97). R195: the hand glow uses the same test.

use jackioh_engine::effects::{ForEachCardArgs, bounce, cards_in_scope, for_each_card, sacrifice};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-022";

/// "{threshold} or more mana": the one test both the Cry and the glow make.
fn enough_mana(ctx: &impl ParamContext, mana: i32) -> bool {
    mana >= param(ctx, "threshold")
}

/// The mana its controller had as the play began (§10.5 step 1), or, for a Cry not played now, now.
fn mana_when_played(ctx: &EffectContext<'_>) -> i32 {
    ctx.mana_before_play
        .unwrap_or_else(|| unspent_mana_of(&*ctx.state, ctx.controller))
}

fn cry(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(self_) = ctx.live_self().cloned() else {
        return vec![];
    };
    // R685: midlane is computed from the lane count, never hardcoded, through the engine helper (M3-T1).
    let lane = slot_of(&*ctx.state, &self_).map_or(-1, |slot| slot.lane);
    let in_midlane = midlane_lanes_of(&*ctx.state, ctx.controller).contains(&lane)
        && matches!(self_.zone, Zone::Field { row: Row::Units, .. });
    let bounces = param(&*ctx, "bounces");
    let mana = mana_when_played(ctx);
    let mut effects = Vec::new();
    if in_midlane {
        effects.push(sacrifice(json_as(json!({ "target": { "of": "self" } }))));
    }
    if enough_mana(&*ctx, mana) {
        effects.push(for_each_card(ForEachCardArgs {
            // R60: different cards, drawn as the Cry reaches this clause.
            cards: Arc::new(move |c: &mut EffectContext<'_>| -> Vec<String> {
                let scope: BoardScope = json_as(json!({ "side": "enemy", "rows": ["units", "backrow"] }));
                let permanents = cards_in_scope(c, &scope);
                c.sink
                    .rng
                    .shuffle(&permanents)
                    .into_iter()
                    .take(bounces.max(0) as usize)
                    .map(|card| card.id)
                    .collect()
            }),
            each: Arc::new(|instance_id: &str| {
                bounce(json_as(json!({ "target": { "of": "instance", "instanceId": instance_id } })))
            }),
        }));
    }
    effects
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(cry)),
        // R195: in hand, whether playing it now would bounce.
        condition_met: Some(condition_hook(|ctx| {
            ctx.zone == ConditionZone::Hand && enough_mana(&ctx, unspent_mana_of(ctx.state, ctx.controller))
        })),
        ..Script::default()
    };
    // The same script: the Radiant face is the base text on doubled stats (docs/radiant-audit.md).
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C #22 Mid Runner — SPEC §8.6 row 22, BUILD M9 Classic row C 22: both checks may pass in one Cry;
// `conditionMet` answers in hand whether your mana is 4 or more now (R195); radiant 4/2 returns 3;
// its tuned numbers (mana threshold, bounces) read through `param()` (R386).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const RUNNER: &str = "classic-022";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9.
    const TEMPO: &str = "core-011"; // (1) Unit 3/3.
    const RUSH_TOKEN: &str = "core-t-rush";
    const BIG_FELINOR: &str = "core-043"; // (4) Unit 3/10, under the Stack pile below.
    const FIENDER: &str = "core-092"; // Felinor Fiender: Stack.
    const PAWN: &str = "core-096"; // (1) Trap; answers only a lethal attack.
    const FIELD_SPELL: &str = "core-073"; // (2) Field Spell.
    const FILLER: &str = "core-005";
    const ANCHOR: &str = "core-010";

    fn bounced_ids(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Bounced { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn enemy_board_ids(s: &Scenario) -> Vec<String> {
        let units = (1..=5).filter_map(|lane| s.unit(P2, lane).map(|card| card.id));
        let backrow = (1..=5).filter_map(|lane| s.backrow(P2, lane).map(|card| card.id));
        units.chain(backrow).collect()
    }

    fn distinct(ids: &[String]) -> usize {
        ids.iter().collect::<IndexSet<_>>().len()
    }

    mod c_n22_mid_runner {
        use super::*;

        #[test]
        fn runs_one_script_on_both_faces_and_midlane_of_5_lanes_is_lane_3_r685() {
            crate::register_all();
            let scripts = script();
            assert_eq!(crate::card_def(ID).id, RUNNER);
            // The Radiant face is the base script itself.
            assert!(Arc::ptr_eq(
                scripts.radiant.cry.as_ref().unwrap(),
                scripts.base.cry.as_ref().unwrap()
            ));
            assert_eq!(midlane_lanes(5), vec![3]);
        }

        #[test]
        fn r685_computes_midlane_from_the_lane_count_odd_counts_center_even_counts_both_centers() {
            crate::register_all();
            assert_eq!(midlane_lanes(5), vec![3]);
            assert_eq!(midlane_lanes(3), vec![2]);
            assert_eq!(midlane_lanes(1), vec![1]);
            assert_eq!(midlane_lanes(4), vec![2, 3]);
            assert_eq!(midlane_lanes(6), vec![3, 4]);
        }

        mod base {
            use super::*;

            #[test]
            fn is_a_2_1() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "field": [RUNNER], "hand": [ANCHOR] } }));
                s.expect_stats(RUNNER, json!({ "attack": 2, "health": 1 }));
            }

            #[test]
            fn played_into_lane_3_it_tributes_itself_a_death() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [RUNNER, ANCHOR], "mana": 3 },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA] },
                }));

                s.play(RUNNER, json!({ "zone": 3 }));

                s.expect_in_zone(RUNNER, "graveyard");
                assert!(s
                    .events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Destroyed { def_id, .. } if def_id == RUNNER)));
                assert_eq!(bounced_ids(&s), Vec::<String>::new());
            }

            #[test]
            fn played_anywhere_else_it_stays() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [RUNNER, ANCHOR], "mana": 3 }, "p2": { "hand": [ANCHOR] } }));

                s.play(RUNNER, json!({ "zone": 2 }));

                s.expect_in_zone(RUNNER, "field");
                assert_eq!(s.unit(P1, 2).map(|card| card.def_id), Some(RUNNER.to_string()));
            }

            #[test]
            fn with_4_mana_before_paying_it_bounces_two_different_random_enemy_permanents_to_their_controller_s_hand() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [RUNNER, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, MENACE, TEMPO], "backrow": [FIELD_SPELL] },
                }));
                let before = enemy_board_ids(&s);

                s.play(RUNNER, json!({ "zone": 1 }));

                let bounced = bounced_ids(&s);
                assert_eq!(bounced.len(), 2);
                assert_eq!(distinct(&bounced), 2);
                let hand: Vec<String> = s.hand(P2).into_iter().map(|card| card.id).collect();
                for id in &bounced {
                    assert!(before.contains(id));
                    assert!(hand.contains(id));
                }
                assert_eq!(enemy_board_ids(&s).len(), 2);
            }

            #[test]
            fn with_3_mana_before_paying_it_bounces_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [RUNNER, ANCHOR], "mana": 3 },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, MENACE] },
                }));

                s.play(RUNNER, json!({ "zone": 1 }));

                assert_eq!(bounced_ids(&s), Vec::<String>::new());
            }

            #[test]
            fn s10_5_step_1_it_reads_the_mana_before_paying_the_price_this_play_paid_a_runner_made_to_cost_2_still_had_4() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": RUNNER, "costMod": 1 }, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, MENACE] },
                }));

                s.play(RUNNER, json!({ "zone": 1 }));

                s.expect_mana(P1, 2);
                assert_eq!(bounced_ids(&s).len(), 2);
            }

            #[test]
            fn r60_fewer_enemy_permanents_than_two_it_bounces_what_there_is_and_nothing_on_an_empty_board() {
                crate::register_all();
                let mut one = scenario(json!({ "p1": { "hand": [RUNNER, ANCHOR] }, "p2": { "hand": [ANCHOR], "field": [VANILLA] } }));
                one.play(RUNNER, json!({ "zone": 1 }));
                assert_eq!(bounced_ids(&one).len(), 1);
                one.expect_in_zone(VANILLA, "hand");

                let mut none = scenario(json!({ "p1": { "hand": [RUNNER, ANCHOR] }, "p2": { "hand": [ANCHOR] } }));
                none.play(RUNNER, json!({ "zone": 1 }));
                assert_eq!(bounced_ids(&none), Vec::<String>::new());
            }

            #[test]
            fn r11_a_bounced_unit_token_ceases_to_exist() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [RUNNER, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [RUSH_TOKEN, RUSH_TOKEN] },
                }));
                let tokens = [s.unit(P2, 1), s.unit(P2, 2)];

                s.play(RUNNER, json!({ "zone": 1 }));

                for token in tokens {
                    let token = token.expect("two tokens");
                    s.expect_in_zone(&token, "gone");
                }
                let hand: Vec<String> = s.hand(P2).into_iter().map(|card| card.def_id).collect();
                assert_eq!(hand, vec![ANCHOR]);
            }

            #[test]
            fn r13_only_the_top_of_an_enemy_stack_pile_is_a_permanent_it_is_bounced_and_the_card_beneath_resumes_and_stays() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [RUNNER, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [BIG_FELINOR, { "def": FIENDER, "stack": true }] },
                }));

                s.play(RUNNER, json!({ "zone": 1 }));

                let fiender = s.card(FIENDER).id.clone();
                assert_eq!(bounced_ids(&s), vec![fiender]);
                s.expect_in_zone(FIENDER, "hand");
                assert_eq!(s.unit(P2, 1).map(|card| card.def_id), Some(BIG_FELINOR.to_string()));
            }

            #[test]
            fn r4_r317_a_full_hand_burns_the_bounced_card_into_its_owner_s_graveyard() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [RUNNER, ANCHOR] },
                    "p2": {
                        "hand": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                        "field": [VANILLA],
                    },
                }));

                s.play(RUNNER, json!({ "zone": 1 }));

                s.expect_in_zone(VANILLA, "graveyard");
                assert!(s.events().iter().any(|event| event.event_type() == GameEventType::Burned));
            }

            #[test]
            fn both_checks_may_pass_in_one_cry_in_lane_3_with_4_mana_it_dies_and_bounces_two() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [RUNNER, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, MENACE, TEMPO] },
                }));

                s.play(RUNNER, json!({ "zone": 3 }));

                s.expect_in_zone(RUNNER, "graveyard");
                assert_eq!(bounced_ids(&s).len(), 2);
            }

            #[test]
            fn r97_a_bounced_face_down_trap_is_never_named_in_your_view() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [RUNNER, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "backrow": [{ "def": PAWN, "faceUp": false }] },
                }));

                s.play(RUNNER, json!({ "zone": 1 }));

                s.expect_in_zone(PAWN, "hand");
                assert!(!serde_json::to_string(&s.view(P1)).unwrap().contains(PAWN));
                assert!(serde_json::to_string(&s.view(P2)).unwrap().contains(PAWN));
            }

            #[test]
            fn r386_a_degrade_of_the_threshold_to_5_stops_4_mana_bouncing_an_upgrade_to_3_lets_3_mana_bounce() {
                crate::register_all();
                let mut harder = scenario(json!({
                    "p1": { "hand": [RUNNER, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, MENACE] },
                }));
                step_param(harder.card_mut(RUNNER), "threshold", 1);
                harder.play(RUNNER, json!({ "zone": 1 }));
                assert_eq!(bounced_ids(&harder), Vec::<String>::new());

                let mut easier = scenario(json!({
                    "p1": { "hand": [RUNNER, ANCHOR], "mana": 3 },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, MENACE] },
                }));
                step_param(easier.card_mut(RUNNER), "threshold", -1);
                easier.play(RUNNER, json!({ "zone": 1 }));
                assert_eq!(bounced_ids(&easier).len(), 2);
            }

            #[test]
            fn r386_an_upgrade_of_bounces_makes_it_bounce_3() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [RUNNER, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, MENACE, TEMPO, VANILLA] },
                }));
                step_param(s.card_mut(RUNNER), "bounces", 1);

                s.play(RUNNER, json!({ "zone": 1 }));

                assert_eq!(distinct(&bounced_ids(&s)), 3);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r275_is_a_4_2_with_the_same_text_the_designer_s_word_docs_radiant_audit_md() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "field": [{ "def": RUNNER, "radiant": true }], "hand": [ANCHOR] } }));
                s.expect_stats(RUNNER, json!({ "attack": 4, "health": 2 }));
                let def = crate::card_def(ID);
                assert_eq!(def.radiant.text, def.base.text);
            }

            #[test]
            fn in_lane_3_it_tributes_itself_and_with_4_mana_it_bounces_three() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": RUNNER, "radiant": true }, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA, MENACE, TEMPO] },
                }));

                s.play(RUNNER, json!({ "zone": 3 }));

                s.expect_in_zone(RUNNER, "graveyard");
                assert_eq!(distinct(&bounced_ids(&s)), 3);
            }

            #[test]
            fn with_3_mana_it_stays_out_of_lane_3_and_bounces_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": RUNNER, "radiant": true }, ANCHOR], "mana": 3 },
                    "p2": { "hand": [ANCHOR], "field": [VANILLA] },
                }));

                s.play(RUNNER, json!({ "zone": 5 }));

                s.expect_in_zone(RUNNER, "field");
                assert_eq!(bounced_ids(&s), Vec::<String>::new());
            }
        }
    }
}
