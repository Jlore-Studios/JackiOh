//! C+ #65.4 Golden Grape (SPEC §8.7 row 65.4). (1) Spell, Fruit, Token (printed Legendary).
//!   Base:    "Make a card on your side of the field or in your hand Radiant."
//!   Radiant: "Make a card on your side of the field or in your hand Radiant, and the cards next to it
//!            (beside it in its row, or beside it in your hand)."
//!
//! The pick travels in the play (R81): your Units, your backrow cards (a face-down one included, R33)
//! and your other hand cards. Make Radiant (§6.3) converts a field card in place with no Cry (§5.2, R22)
//! and leaves a Radiant card as it is; a change to a card someone may not read is cued to them redacted
//! whether or not the flag moved (R177).
//!
//! "Next to it" is read as the Spell resolves: §3.1's adjacency on the pick's own side and row
//! (an empty or dormant neighbour is nothing), or the hand cards at the indices either side of it.

pub const ID: &str = "classicplus-065-4";

// BUILD M9 Classic+ row C+ 65.4: the pick is declared with the play (R81), in your hand or a permanent
// you control (face-down included); it becomes Radiant, no change if it already is (§5.2); a hand or
// face-down card's change is reported to the opponent by a redacted `radiantSet` either way (R177);
// radiant also the cards next to it: its row neighbours on your side, or the neighbouring hand indices.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GOLDEN: &str = "classicplus-065-4";
    const FILLER: &str = "core-005";
    const TIMMY: &str = "core-011"; // Unit 3/3 → 6/6
    const DFENDER: &str = "core-001"; // Unit 0/7 → 0/14
    const FELINORS: &str = "core-012"; // Unit 3/4 → 6/9
    const SHEEPISH: &str = "core-041"; // Trap

    use crate::js;

    fn pick(s: &Scenario, card: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": s.card(card).id }])
    }

    fn radiant_sets(s: &Scenario, viewer: PlayerId) -> Vec<Value> {
        s.view(viewer)
            .events
            .iter()
            .filter_map(|event| match event {
                GameEvent::RadiantSet { instance_id, .. } => Some(json!({ "instanceId": instance_id })),
                _ => None,
            })
            .collect()
    }

    fn has_event(events: &[GameEvent], kind: &str) -> bool {
        events.iter().any(|event| event.event_type().as_str() == kind)
    }

    fn radiant_golden() -> Value {
        json!({ "def": GOLDEN, "radiant": true })
    }

    #[test]
    fn is_a_1_fruit_spell_token_printed_legendary_whose_pick_is_your_hand_or_your_side_of_the_field() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.id, GOLDEN);
        assert_eq!(js(&def.cost), json!(1));
        assert_eq!(js(&def.printed_rarity), json!("Legendary"));
        let decl = json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "ally", "of": ["unit", "backrow", "hand"] } }]);
        let CardScripts { base, radiant } = crate::scripts_of()[ID].clone();
        assert_eq!(js(&base.targets), decl);
        assert_eq!(js(&radiant.targets), decl);
    }

    mod base {
        use super::*;

        #[test]
        fn s5_2_a_card_in_your_hand_becomes_radiant() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [GOLDEN, TIMMY] }, "p2": { "hand": [FILLER] } }));
            let targets = pick(&s, TIMMY);
            s.play(GOLDEN, json!({ "targets": targets }));
            assert!(s.card(TIMMY).radiant);
            assert_eq!(s.card(TIMMY).zone.z(), ZoneName::Hand);
        }

        #[test]
        fn s5_2_r22_a_unit_of_yours_converts_in_place_radiant_stats_at_once_damage_kept_no_cry() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [GOLDEN, FILLER], "field": [{ "def": FELINORS, "damage": 2 }] },
                "p2": { "hand": [FILLER] }
            }));
            let targets = pick(&s, FELINORS);
            s.play(GOLDEN, json!({ "targets": targets }));
            s.expect_stats(FELINORS, json!({ "attack": 6, "maxHealth": 8, "health": 6 }));
            assert!(!has_event(s.last_events(), "summoned"));
        }

        #[test]
        fn r33_a_face_down_trap_of_yours_is_a_legal_pick_and_becomes_radiant_face_down() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [GOLDEN, FILLER], "backrow": [SHEEPISH] }, "p2": { "hand": [FILLER] } }));
            let targets = pick(&s, SHEEPISH);
            s.play(GOLDEN, json!({ "targets": targets }));
            assert!(s.card(SHEEPISH).radiant);
            assert_ne!(s.card(SHEEPISH).face_up, Some(true));
        }

        #[test]
        fn r81_the_opponent_s_cards_are_never_offered_and_a_play_naming_one_is_refused() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [GOLDEN, FILLER] }, "p2": { "hand": [FILLER], "field": [TIMMY] } }));
            let offered = serde_json::to_string(&legal_actions(s.state(), P1)).unwrap();
            assert!(!offered.contains(&s.card(TIMMY).id));
            let targets = pick(&s, TIMMY);
            s.expect_refused(|s| s.play(GOLDEN, json!({ "targets": targets })));
        }

        #[test]
        fn r781001_an_immutable_card_in_play_is_no_target_and_in_hand_radiant_is_still_its_own_text() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [GOLDEN, TIMMY, FILLER], "field": [FELINORS] },
                "p2": { "hand": [FILLER] },
            }));
            for card in [FELINORS, TIMMY] {
                let id = s.card(card).id.clone();
                find_instance_mut(s.state_mut(), &id).expect("the card is in the game").granted_keywords =
                    vec![json_as(json!({ "kind": "Immutable" }))];
            }
            // A Spell can't target an Immutable card in play (R781001)…
            let on_field = pick(&s, FELINORS);
            s.expect_refused(|s| s.play(GOLDEN, json!({ "targets": on_field })));
            // …but in a hand it may, and Immutable never blocks Radiant, the card's own text (§6.1).
            let held = pick(&s, TIMMY);
            s.play(GOLDEN, json!({ "targets": held }));
            assert!(s.card(TIMMY).radiant);
            assert!(!s.card(FELINORS).radiant);
        }

        #[test]
        fn s6_3_a_card_that_is_already_radiant_is_left_as_it_is() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [GOLDEN], "field": [{ "def": TIMMY, "radiant": true }] }, "p2": { "hand": [FILLER] } }));
            let targets = pick(&s, TIMMY);
            s.play(GOLDEN, json!({ "targets": targets }));
            assert!(s.card(TIMMY).radiant);
            assert!(!has_event(s.last_events(), "radiantSet"));
        }

        #[test]
        fn r177_a_hand_card_s_change_reaches_the_opponent_redacted_and_so_does_a_no_change_on_a_radiant_one() {
            crate::register_all();
            let mut fresh = scenario(json!({ "p1": { "hand": [GOLDEN, TIMMY] }, "p2": { "hand": [FILLER] } }));
            let targets = pick(&fresh, TIMMY);
            fresh.play(GOLDEN, json!({ "targets": targets }));
            assert_eq!(radiant_sets(&fresh, P2), vec![json!({ "instanceId": "hidden" })]);
            assert_eq!(radiant_sets(&fresh, P1), vec![json!({ "instanceId": fresh.card(TIMMY).id })]);

            let mut already = scenario(json!({ "p1": { "hand": [GOLDEN, { "def": TIMMY, "radiant": true }] }, "p2": { "hand": [FILLER] } }));
            let targets = pick(&already, TIMMY);
            already.play(GOLDEN, json!({ "targets": targets }));
            assert_eq!(radiant_sets(&already, P2), vec![json!({ "instanceId": "hidden" })]);
        }

        #[test]
        fn r177_r33_a_face_down_card_s_change_reaches_the_opponent_redacted() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [GOLDEN, FILLER], "backrow": [{ "def": SHEEPISH, "radiant": true }] },
                "p2": { "hand": [FILLER] }
            }));
            let targets = pick(&s, SHEEPISH);
            s.play(GOLDEN, json!({ "targets": targets }));
            assert_eq!(radiant_sets(&s, P2), vec![json!({ "instanceId": "hidden" })]);
            assert!(!serde_json::to_string(&s.view(P2)).unwrap().contains(SHEEPISH));
        }

        #[test]
        fn the_base_face_changes_only_the_pick_its_neighbours_stay_as_they_are() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [GOLDEN, FILLER], "field": [DFENDER, TIMMY, FELINORS] }, "p2": { "hand": [FILLER] } }));
            let targets = pick(&s, TIMMY);
            s.play(GOLDEN, json!({ "targets": targets }));
            assert_eq!(
                [s.card(DFENDER).radiant, s.card(TIMMY).radiant, s.card(FELINORS).radiant],
                [false, true, false]
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s3_1_a_unit_and_its_row_neighbours_on_your_side_lanes_n_1_and_n_1_become_radiant() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [radiant_golden(), FILLER],
                    "field": [
                        { "def": DFENDER, "lane": 1 },
                        { "def": TIMMY, "lane": 2 },
                        { "def": FELINORS, "lane": 3 },
                        { "def": "core-019", "lane": 4 }
                    ]
                },
                "p2": { "hand": [FILLER], "field": [{ "def": "core-043", "lane": 2 }] }
            }));
            let targets = pick(&s, TIMMY);
            s.play(GOLDEN, json!({ "targets": targets }));
            assert_eq!(
                [s.card(DFENDER).radiant, s.card(TIMMY).radiant, s.card(FELINORS).radiant],
                [true, true, true]
            );
            assert!(!s.card("core-019").radiant);
            assert!(!s.card("core-043").radiant);
        }

        #[test]
        fn s3_1_an_empty_neighbouring_zone_is_nothing_the_edge_lane_has_one_neighbour() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [radiant_golden(), FILLER], "field": [{ "def": TIMMY, "lane": 1 }, { "def": FELINORS, "lane": 3 }] },
                "p2": { "hand": [FILLER] }
            }));
            let targets = pick(&s, TIMMY);
            s.play(GOLDEN, json!({ "targets": targets }));
            assert!(s.card(TIMMY).radiant);
            assert!(!s.card(FELINORS).radiant);
        }

        #[test]
        fn s3_1_a_backrow_pick_s_neighbours_are_its_backrow_neighbours_not_the_units_in_front() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [radiant_golden(), FILLER],
                    "field": [{ "def": TIMMY, "lane": 2 }],
                    "backrow": [{ "def": SHEEPISH, "lane": 2 }, { "def": "core-060", "lane": 3 }]
                },
                "p2": { "hand": [FILLER] }
            }));
            let targets = pick(&s, SHEEPISH);
            s.play(GOLDEN, json!({ "targets": targets }));
            assert!(s.card(SHEEPISH).radiant);
            assert!(s.card("core-060").radiant);
            assert!(!s.card(TIMMY).radiant);
        }

        #[test]
        fn r13_a_card_dormant_under_a_stack_pile_is_no_neighbour_only_the_top_of_the_pile_beside_it() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [radiant_golden(), FILLER],
                    "field": [{ "def": TIMMY, "lane": 1 }, { "def": DFENDER, "lane": 2 }, { "def": FELINORS, "lane": 2, "stack": true }]
                },
                "p2": { "hand": [FILLER] }
            }));
            let targets = pick(&s, TIMMY);
            s.play(GOLDEN, json!({ "targets": targets }));
            assert!(s.card(FELINORS).radiant);
            assert!(!s.card(DFENDER).radiant);
        }

        #[test]
        fn in_your_hand_the_cards_at_the_neighbouring_indices_become_radiant_and_no_others() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [radiant_golden(), DFENDER, TIMMY, FELINORS, "core-019"] }, "p2": { "hand": [FILLER] } }));
            // The Grape leaves the hand as it is played, so the hand reads D-fender, Timmy, Felinors, Menace.
            let targets = pick(&s, TIMMY);
            s.play(GOLDEN, json!({ "targets": targets }));
            assert_eq!(
                [
                    s.card(DFENDER).radiant,
                    s.card(TIMMY).radiant,
                    s.card(FELINORS).radiant,
                    s.card("core-019").radiant
                ],
                [true, true, true, false]
            );
        }

        #[test]
        fn the_first_hand_card_has_one_neighbour() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [radiant_golden(), DFENDER, TIMMY, FELINORS] }, "p2": { "hand": [FILLER] } }));
            let targets = pick(&s, DFENDER);
            s.play(GOLDEN, json!({ "targets": targets }));
            assert_eq!(
                [s.card(DFENDER).radiant, s.card(TIMMY).radiant, s.card(FELINORS).radiant],
                [true, true, false]
            );
        }

        #[test]
        fn r177_every_hand_change_reaches_the_opponent_redacted() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [radiant_golden(), DFENDER, TIMMY, FELINORS] }, "p2": { "hand": [FILLER] } }));
            let targets = pick(&s, TIMMY);
            s.play(GOLDEN, json!({ "targets": targets }));
            assert_eq!(
                radiant_sets(&s, P2),
                vec![json!({ "instanceId": "hidden" }), json!({ "instanceId": "hidden" }), json!({ "instanceId": "hidden" })]
            );
        }
    }
}
