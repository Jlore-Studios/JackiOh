//! C #32 Felinor Feelings (SPEC §8.6 row 32). Spell, Felinor, cost 0, Rare.
//!   Base:    "Steal an enemy permanent in a lane where you control a (1) Cost Unit."
//!   Radiant: "Summon a Felinor Token. Then steal an enemy permanent in a lane where you control a (1)
//!             Cost Unit."
//!
//! A lane (§3.1) counts when the top of its unit pile (a card dormant under a Stack is not on the
//! field, R13) is yours and costs exactly (1): R396's `costNow`, R65's cost where it stands, an X Unit
//! at the X it was played for (0 with none chosen). The enemy permanent may be the top of their pile
//! there or their backrow card, face-down included (the thief reads it from then on, R33; an option
//! carries only its id, R177).
//!
//! Base: a declared target (R81), filtered by `targetChecks` (§10.6). With no such target the play is
//! still legal and the steal fizzles (§8's conventions, R90). The steal is §6.3's (R15: its own lane
//! if free, else the first free zone; none, and it stays), an entry that leaves it summoning sick (R171).
//!
//! Radiant: first a Felinor Token in your leftmost open unit zone (R64), then the steal is a prompt, so
//! the token's lane counts. A full board summons no token; with no such permanent no prompt opens.

use indexmap::IndexSet;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-032";

/// §7's shared Felinor Token, a (1) Cost Unit.
const FELINOR_TOKEN: &str = "core-t-felinor";
/// The name of the lane rule in `targetChecks`, as the declaration's filter names it.
const LANE_RULE: &str = "felinorLane";
/// The continuation the Radiant face's prompt re-enters.
const STEAL: &str = "feelings-steal";

/// The lanes where `player` controls a Unit that costs exactly (1) (R396).
fn feeling_lanes(state: &GameState, player: PlayerId) -> IndexSet<i32> {
    let mut lanes = IndexSet::new();
    for unit in active_units_of(state, player).iter() {
        if let Zone::Field { lane, .. } = unit.zone
            && cost_now(state, unit) == 1
        {
            lanes.insert(lane);
        }
    }
    lanes
}

/// An enemy permanent standing in one of those lanes.
fn in_feeling_lane(state: &GameState, player: PlayerId, card: Option<&CardInstance>) -> bool {
    let Some(card) = card else { return false };
    let Zone::Field { lane, .. } = card.zone else { return false };
    if card.controller != opponent_of(player) {
        return false;
    }
    feeling_lanes(state, player).contains(&lane)
}

fn lane_rule() -> TargetCheck {
    target_check(|args| in_feeling_lane(args.state, args.player, args.candidate))
}

fn steal_chosen() -> Effect {
    steal(json_as(json!({ "target": { "of": "chosen" } })))
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "enemy", "of": ["unit", "backrow"], "check": LANE_RULE }),
        )],
        target_checks: IndexMap::from([(LANE_RULE, lane_rule())]),
        cry: Some(hook(|_ctx| vec![steal_chosen()])),
        ..Script::default()
    };

    let radiant = Script {
        cry: Some(hook(|_ctx| {
            let mut choose: ChooseTargetWhereArgs = json_as(json!({
                "step": STEAL,
                "scope": { "side": "enemy", "of": ["unit", "backrow"] },
                "prompt": "Steal an enemy permanent in a lane where you control a (1) Cost Unit",
            }));
            choose.where_ = Some(Arc::new(|ctx, card| in_feeling_lane(&*ctx.state, ctx.controller, card)));
            vec![summon(json_as(json!({ "defId": FELINOR_TOKEN }))), choose_target_where(choose)]
        })),
        resume: IndexMap::from([(STEAL, hook(|_ctx| vec![steal_chosen()]))]),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FEELINGS: &str = "classic-032";
    const TIMMY: &str = "core-011"; // (1) Unit 3/3.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const POINTMASTER: &str = "core-020"; // (2) Unit 7/1.
    const MENACE: &str = "core-019"; // (3) Unit 9/9.
    const BEAR: &str = "core-060"; // (1) Trap.
    const BILLY: &str = "classicplus-069"; // (X) Unit.
    const TOKEN: &str = "core-t-felinor";
    const FILLER: &str = "core-010";

    fn spread(mut base: Value, over: Value) -> Value {
        if let (Some(into), Some(more)) = (base.as_object_mut(), over.as_object()) {
            for (key, value) in more {
                into.insert(key.clone(), value.clone());
            }
        }
        base
    }

    /// Each `SideSetup` spread over the hand Feelings needs.
    fn setup(p1: Value, p2: Value, radiant_face: bool) -> Scenario {
        scenario(json!({
            "p1": spread(json!({ "hand": [{ "def": FEELINGS, "radiant": radiant_face }, FILLER] }), p1),
            "p2": spread(json!({ "hand": [FILLER] }), p2),
        }))
    }

    /// `at(card)` as the play's literal.
    fn at(card: &CardInstance) -> Value {
        json!([{ "pick": "instance", "instanceId": card.id }])
    }

    /// `at(card)` as the selections a prompt carries.
    fn at_selection(card: &CardInstance) -> Vec<Selection> {
        vec![Selection::Instance {
            instance_id: card.id.clone(),
        }]
    }

    fn must_unit(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        match s.unit(player, lane) {
            Some(unit) => unit,
            None => panic!("no unit in {player} lane {lane}"),
        }
    }

    /// The targets `legalActions` offers this play, by instance id.
    fn offered(s: &Scenario) -> Vec<String> {
        let card = s.card(FEELINGS).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .flat_map(|action| match action {
                ActionBody::Play { instance_id, targets, .. } if instance_id == card => targets
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|selection| match selection {
                        Selection::Instance { instance_id } => Some(instance_id),
                        _ => None,
                    })
                    .collect::<Vec<String>>(),
                _ => Vec::new(),
            })
            .collect()
    }

    /// The open prompt's options, as selections.
    fn pending_selections(s: &Scenario) -> Option<Vec<Selection>> {
        s.state()
            .pending
            .as_ref()
            .map(|pending| pending.options.iter().map(|option| option.selection.clone()).collect())
    }

    mod c_n32_felinor_feelings {
        use super::*;

        #[test]
        fn has_no_declared_numbers_the_base_face_declares_its_target_the_radiant_face_asks_later() {
            crate::register_all();
            let def = crate::card_def(FEELINGS);
            assert_eq!(def.id, FEELINGS);
            assert!(def.params.is_none());
            let scripts = script();
            assert_eq!(
                scripts
                    .base
                    .targets
                    .first()
                    .and_then(|decl| decl.filter.as_ref())
                    .and_then(|filter| filter.check.clone()),
                Some("felinorLane".to_string())
            );
            assert!(scripts.radiant.targets.is_empty());
        }

        mod base {
            use super::*;

            #[test]
            fn r81_steals_the_declared_enemy_unit_in_a_lane_where_you_control_a_1_cost_unit() {
                crate::register_all();
                let mut s = setup(
                    json!({ "field": [{ "def": TIMMY, "lane": 2 }] }),
                    json!({ "field": [{ "def": VANILLA, "lane": 2 }] }),
                    false,
                );
                let vanilla = must_unit(&s, P2, 2);

                s.play(FEELINGS, json!({ "targets": at(&vanilla) }));

                assert_eq!(s.card(&vanilla).controller, P1);
                assert!(s.unit(P2, 2).is_none());
            }

            #[test]
            fn r15_the_stolen_unit_takes_its_own_lane_when_free_else_your_first_free_zone() {
                crate::register_all();
                // Lane 2 on p1's side holds the Timmy, so the Vanilla lands in p1's lane 1.
                let mut s = setup(
                    json!({ "field": [{ "def": TIMMY, "lane": 2 }] }),
                    json!({ "field": [{ "def": VANILLA, "lane": 2 }] }),
                    false,
                );
                let vanilla = must_unit(&s, P2, 2);

                s.play(FEELINGS, json!({ "targets": at(&vanilla) }));

                assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(vanilla.id.clone()));
            }

            #[test]
            fn s10_6_only_permanents_in_such_lanes_are_offered_another_lane_s_is_refused() {
                crate::register_all();
                let mut s = setup(
                    json!({ "field": [{ "def": TIMMY, "lane": 2 }, { "def": POINTMASTER, "lane": 3 }] }),
                    json!({ "field": [{ "def": VANILLA, "lane": 2 }, { "def": MENACE, "lane": 3 }] }),
                    false,
                );
                let vanilla = must_unit(&s, P2, 2);
                let menace = must_unit(&s, P2, 3);

                assert!(offered(&s).contains(&vanilla.id));
                assert!(!offered(&s).contains(&menace.id));
                s.expect_refused(|s| s.play(FEELINGS, json!({ "targets": at(&menace) })));
            }

            #[test]
            fn r65_a_unit_s_cost_changes_count_a_2_cost_unit_made_1_opens_its_lane() {
                crate::register_all();
                let mut s = setup(
                    json!({ "field": [{ "def": POINTMASTER, "lane": 3, "costMod": -1 }] }),
                    json!({ "field": [{ "def": MENACE, "lane": 3 }] }),
                    false,
                );
                let menace = must_unit(&s, P2, 3);

                s.play(FEELINGS, json!({ "targets": at(&menace) }));

                assert_eq!(s.card(&menace).controller, P1);
            }

            #[test]
            fn r396_an_x_unit_costs_the_x_it_was_played_for_and_0_with_none_chosen() {
                crate::register_all();
                let mut s = setup(
                    json!({ "field": [{ "def": BILLY, "lane": 1, "statsOverride": { "attack": 3, "health": 3 } }] }),
                    json!({ "field": [{ "def": MENACE, "lane": 1 }] }),
                    false,
                );
                let billy = must_unit(&s, P1, 1);
                let menace = must_unit(&s, P2, 1);

                find_instance_mut(s.state_mut(), &billy.id).expect("Billy is on the field").x = None;
                assert!(!offered(&s).contains(&menace.id));
                find_instance_mut(s.state_mut(), &billy.id).expect("Billy is on the field").x = Some(1);
                assert!(offered(&s).contains(&menace.id));
            }

            #[test]
            fn a_backrow_card_in_that_lane_may_be_taken_face_down_included_and_lands_in_your_backrow() {
                crate::register_all();
                let mut s = setup(
                    json!({ "field": [{ "def": TIMMY, "lane": 4 }] }),
                    json!({ "backrow": [{ "def": BEAR, "faceUp": false, "lane": 4 }] }),
                    false,
                );
                let bear = s.card(BEAR).clone();

                s.play(FEELINGS, json!({ "targets": at(&bear) }));

                assert_eq!(s.backrow(P1, 4).map(|card| card.id), Some(bear.id.clone()));
                assert_eq!(s.card(&bear).controller, P1);
            }

            #[test]
            fn r33_a_stolen_face_down_trap_is_read_by_you_from_then_on_and_no_longer_by_its_owner() {
                crate::register_all();
                let mut s = setup(
                    json!({ "field": [{ "def": TIMMY, "lane": 4 }] }),
                    json!({ "backrow": [{ "def": BEAR, "faceUp": false, "lane": 4 }] }),
                    false,
                );
                let bear = s.card(BEAR).clone();
                let before = serde_json::to_string(&s.view(P1).opponent.backrow).expect("views serialise");
                assert!(!before.contains(BEAR));

                s.play(FEELINGS, json!({ "targets": at(&bear) }));

                let mine = serde_json::to_string(&s.view(P1).you.backrow).expect("views serialise");
                assert!(mine.contains(BEAR));
                let theirs = serde_json::to_string(&s.view(P2).opponent.backrow).expect("views serialise");
                assert!(!theirs.contains(BEAR));
            }

            #[test]
            fn r177_a_face_down_option_is_offered_by_its_id_alone_the_play_s_targets_name_no_hidden_card() {
                crate::register_all();
                let s = setup(
                    json!({ "field": [{ "def": TIMMY, "lane": 4 }] }),
                    json!({ "backrow": [{ "def": BEAR, "faceUp": false, "lane": 4 }] }),
                    false,
                );
                let bear = s.card(BEAR).clone();

                assert_eq!(offered(&s), vec![bear.id.clone()]);
                // The option is the bare id: nothing p1 is shown names the card beneath it.
                assert!(offered(&s).iter().all(|id| *id == bear.id));
                assert!(!serde_json::to_string(&s.view(P1)).expect("views serialise").contains(BEAR));
            }

            #[test]
            fn r13_a_1_cost_unit_of_yours_dormant_under_a_stack_pile_opens_no_lane() {
                crate::register_all();
                let mut s = setup(
                    json!({ "field": [{ "def": TIMMY, "lane": 2 }, { "def": POINTMASTER, "stack": true }] }),
                    json!({ "field": [{ "def": MENACE, "lane": 2 }] }),
                    false,
                );

                assert!(offered(&s).is_empty());
                let menace = must_unit(&s, P2, 2);
                s.expect_refused(|s| s.play(FEELINGS, json!({ "targets": at(&menace) })));
            }

            #[test]
            fn r13_an_enemy_stack_pile_offers_its_top_never_the_card_dormant_beneath_it() {
                crate::register_all();
                let mut s = setup(
                    json!({ "field": [{ "def": TIMMY, "lane": 2 }] }),
                    json!({ "field": [{ "def": VANILLA, "lane": 2 }, { "def": MENACE, "stack": true }] }),
                    false,
                );
                let dormant = s.card(VANILLA).clone();
                let top = must_unit(&s, P2, 2);

                assert_eq!(offered(&s), vec![top.id.clone()]);
                s.expect_refused(|s| s.play(FEELINGS, json!({ "targets": at(&dormant) })));
            }

            #[test]
            fn r90_with_no_such_target_the_play_is_still_legal_and_the_steal_fizzles() {
                crate::register_all();
                let mut s = setup(
                    json!({ "field": [{ "def": POINTMASTER, "lane": 1 }] }),
                    json!({ "field": [{ "def": MENACE, "lane": 1 }] }),
                    false,
                );

                s.play(FEELINGS, json!({}));

                assert_eq!(must_unit(&s, P2, 1).controller, P2);
                s.expect_in_zone(FEELINGS, "graveyard");
            }

            #[test]
            fn r15_with_no_free_zone_on_your_side_the_stolen_card_stays_with_its_owner() {
                crate::register_all();
                let mut s = setup(
                    json!({ "field": [TIMMY, VANILLA, VANILLA, VANILLA, VANILLA] }),
                    json!({ "field": [{ "def": MENACE, "lane": 1 }] }),
                    false,
                );
                let menace = must_unit(&s, P2, 1);

                s.play(FEELINGS, json!({ "targets": at(&menace) }));

                assert_eq!(s.card(&menace).controller, P2);
                assert_eq!(s.unit(P2, 1).map(|unit| unit.id), Some(menace.id.clone()));
            }

            #[test]
            fn r171_the_steal_is_an_entry_the_stolen_unit_is_summoning_sick() {
                crate::register_all();
                let mut s = setup(
                    json!({ "field": [{ "def": TIMMY, "lane": 2 }] }),
                    json!({ "field": [{ "def": POINTMASTER, "lane": 2 }] }),
                    false,
                );
                let point = must_unit(&s, P2, 2);

                s.play(FEELINGS, json!({ "targets": at(&point) }));

                s.expect_refused(|s| s.attack(&point, "hero"));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn summons_a_felinor_token_first_then_asks_so_the_token_s_lane_counts() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "field": [{ "def": MENACE, "lane": 1 }] }), true);
                let menace = must_unit(&s, P2, 1);

                s.play(FEELINGS, json!({}));

                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(TOKEN.to_string()));
                assert_eq!(s.state().pending.as_ref().map(|pending| pending.player_id), Some(P1));
                assert_eq!(pending_selections(&s), Some(at_selection(&menace)));
                s.answer(at(&menace));

                assert_eq!(s.card(&menace).controller, P1);
                assert_eq!(s.unit(P1, 2).map(|unit| unit.id), Some(menace.id.clone()));
            }

            #[test]
            fn s9_3_the_open_steal_prompt_survives_a_json_round_trip_and_resumes_through_reduce() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "field": [{ "def": MENACE, "lane": 1 }] }), true);
                let menace = must_unit(&s, P2, 1);
                s.play(FEELINGS, json!({}));
                let paused = s.state().clone();
                let revived: GameState =
                    serde_json::from_value(serde_json::to_value(&paused).expect("the state serialises"))
                        .expect("the state parses back");
                assert_eq!(revived, paused);

                let choice_id = revived.pending.as_ref().map(|pending| pending.id.clone()).unwrap_or_default();
                let result = reduce(
                    &revived,
                    &Action::new(
                        ActionBody::Answer {
                            choice_id,
                            selection: at_selection(&menace),
                        },
                        P1,
                        "feelings-round-trip",
                    ),
                );

                assert!(result.error.is_none());
                assert!(result.state.pending.is_none());
                assert!(result.state.work.is_empty());
                assert_eq!(find_instance(&result.state, &menace.id).map(|card| card.controller), Some(P1));
            }

            #[test]
            fn offers_only_permanents_in_such_lanes() {
                crate::register_all();
                let mut s = setup(
                    json!({ "field": [{ "def": POINTMASTER, "lane": 1 }] }),
                    json!({ "field": [{ "def": MENACE, "lane": 2 }, { "def": VANILLA, "lane": 3 }] }),
                    true,
                );

                s.play(FEELINGS, json!({}));

                // The token lands in lane 2 (lane 1 is taken): Menace's lane counts, Vanilla's does not.
                assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id), Some(TOKEN.to_string()));
                assert_eq!(pending_selections(&s), Some(at_selection(&must_unit(&s, P2, 2))));
            }

            #[test]
            fn a_full_board_summons_no_token_and_the_prompt_reads_the_lanes_as_they_are() {
                crate::register_all();
                let mut s = setup(
                    json!({ "field": [POINTMASTER, POINTMASTER, { "def": TIMMY, "lane": 3 }, POINTMASTER, POINTMASTER] }),
                    json!({ "field": [{ "def": MENACE, "lane": 3 }] }),
                    true,
                );

                s.play(FEELINGS, json!({}));

                assert!(!s.events().iter().any(|event| matches!(event, GameEvent::Summoned { .. })));
                assert_eq!(pending_selections(&s), Some(at_selection(&must_unit(&s, P2, 3))));
                let menace = must_unit(&s, P2, 3);
                s.answer(at(&menace));
                // No free zone: it stays with its owner (R15).
                assert_eq!(must_unit(&s, P2, 3).controller, P2);
            }

            #[test]
            fn with_no_such_permanent_it_asks_nothing() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "field": [{ "def": MENACE, "lane": 4 }] }), true);

                s.play(FEELINGS, json!({}));

                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(TOKEN.to_string()));
                assert!(s.state().pending.is_none());
                assert_eq!(must_unit(&s, P2, 4).controller, P2);
            }

            #[test]
            fn r177_a_face_down_option_in_the_prompt_names_no_card_in_your_view() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "backrow": [{ "def": BEAR, "faceUp": false, "lane": 1 }] }), true);

                s.play(FEELINGS, json!({}));

                assert!(s.state().pending.is_some());
                let pending = serde_json::to_string(&s.view(P1).pending).expect("views serialise");
                assert!(pending.contains(&format!("\"{}\"", s.card(BEAR).id)));
                assert!(!serde_json::to_string(&s.view(P1)).expect("views serialise").contains(BEAR));
                let bear = s.card(BEAR).clone();
                s.answer(at(&bear));
                assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some(BEAR.to_string()));
            }
        }
    }
}
