//! #33 Unstable Clone Machine (SPEC §8.2 row 33): Field Spell, "After you play a card, shuffle 3
//! copies of it into your library"; radiant "3 Radiant copies" (R275), which restates only which copies
//! are Radiant, so the rest is kept (§8 Conventions).
//!
//! R34 token cards are copied too (no token filter); R11 lets a unit-token card sit in a library.
//! R57 a copy is a fresh instance carrying only the radiant flag (and `statsOverride`, see ENGINE GAP).
//! R80 a library never takes a copy past `LIBRARY_CAP` (60), so no cap check here; R316 reports the
//!     refusal as `libraryOverflow`, `copyOf` naming the played card for a view to judge it by: a Trap
//!     set face-down stays unnamed.
//! R70 a cast is a play too. R17 it answers `cardResolved` (§10.5 step 7), never `cardPlayed`, or a
//!     Stockpile could draw its own copies; the event carries the resolved face (R34, R57), since #41
//!     Sheepish transforms a played Unit at step 4. R119: a permanent is on the field before its own
//!     `cardResolved` (the play places it at step 4), so it would copy itself; it starts counting from
//!     the next play.
//! ENGINE GAP: `shuffleInto` has no `statsOverride`, so a Fused or Crafted body copies at printed stats,
//! which R57 says it should keep.

pub const ID: &str = "core-033";

// #33 Unstable Clone Machine — SPEC §8.2 row 33, BUILD M4-T4 must-pass row 33: "After each play,
// library +3 fresh copies with the radiant flag preserved; token spells copied (R34); nothing is
// added to a 60-card library (R80)". Radiant (R275): all three are Radiant whatever the played card
// was, and R80 still caps the library.
// R119: a Clone Machine a play's own resolution put onto the field (#98's Recruit) starts counting
// from the next play, as the played Clone Machine itself does.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    /// The harness's import-time `registerAll()`: the engine's testkit cannot name the cards crate.
    fn scn(opts: Value) -> Scenario {
        crate::register_all();
        scenario(opts)
    }

    /// The copies of one def sitting in a library.
    fn copies_in(cards: &[CardInstance], def_id: &str) -> Vec<CardInstance> {
        cards.iter().filter(|card| card.def_id == def_id).cloned().collect()
    }

    fn full_library() -> Vec<&'static str> {
        vec!["15"; 60]
    }

    fn distinct_ids(cards: &[CardInstance]) -> usize {
        cards.iter().map(|card| card.id.clone()).collect::<IndexSet<_>>().len()
    }

    fn view_json(s: &Scenario, seat: &str) -> Value {
        serde_json::to_value(s.view(seat)).expect("a view is JSON")
    }

    fn view_events_of_type(view: &Value, event_type: &str) -> Vec<Value> {
        view["events"]
            .as_array()
            .expect("a view carries its events")
            .iter()
            .filter(|event| event["type"] == event_type)
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn after_you_play_a_unit_3_fresh_copies_of_it_are_shuffled_into_your_library() {
            let mut s = scn(json!({
                "seed": "clone-unit",
                "p1": { "hand": ["15"], "backrow": ["33"], "field": ["43"], "library": [] },
                "p2": { "field": ["15"] },
            }));

            s.play("15", json!({}));

            let copies = copies_in(&s.pile("p1", "library"), "core-015");
            assert_eq!(copies.len(), 3);
            // R57: fresh instances — no damage, no buffs, not radiant, distinct ids.
            assert!(copies.iter().all(|card| card.damage == 0));
            assert!(copies.iter().all(|card| card.buffs.attack == 0 && card.buffs.health == 0));
            assert!(copies.iter().all(|card| !card.radiant));
            assert_eq!(distinct_ids(&copies), 3);
            s.expect_events(json!(["cardPlayed", "shuffledIn", "shuffledIn", "shuffledIn"]));
        }

        #[test]
        fn after_you_play_a_spell_3_copies_of_the_spell_are_shuffled_in() {
            let mut s = scn(json!({
                "seed": "clone-spell",
                "p1": { "hand": ["31"], "backrow": ["33"], "field": ["43"], "library": [] },
                "p2": { "field": ["15"] },
            }));

            s.play("31", json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            assert_eq!(copies_in(&s.pile("p1", "library"), "core-031").len(), 3);
        }

        #[test]
        fn r57_the_played_cards_radiant_flag_is_preserved_on_all_3_copies() {
            let mut s = scn(json!({
                "seed": "clone-radiant-source",
                "p1": { "hand": ["15"], "backrow": ["33"], "field": ["43"], "library": [] },
                "p2": { "field": ["15"] },
            }));
            s.card_mut("15").radiant = true;

            s.play("15", json!({}));

            let copies = copies_in(&s.pile("p1", "library"), "core-015");
            assert_eq!(copies.len(), 3);
            assert!(copies.iter().all(|card| card.radiant));
        }

        #[test]
        fn r34_a_token_spell_is_copied_like_any_other_card() {
            let mut s = scn(json!({
                "seed": "clone-token",
                "p1": { "hand": ["93.1"], "backrow": ["33"], "field": ["43"], "library": [] },
                "p2": { "field": ["15"] },
            }));
            let token_def_id = s.card("93.1").def_id.clone();

            // Combo-Fodder declares a target (R81), so the play carries one.
            s.play("93.1", json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            assert_eq!(copies_in(&s.pile("p1", "library"), &token_def_id).len(), 3);
        }

        #[test]
        fn r80_nothing_is_added_to_a_60_card_library() {
            let mut s = scn(json!({
                "seed": "clone-cap",
                "p1": { "hand": ["31"], "backrow": ["33"], "field": ["43"], "library": full_library() },
                "p2": { "field": ["15"] },
            }));
            assert_eq!(s.pile("p1", "library").len(), 60);

            s.play("31", json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            assert_eq!(s.pile("p1", "library").len(), 60);
            assert_eq!(copies_in(&s.pile("p1", "library"), "core-031").len(), 0);
        }

        #[test]
        fn copies_go_in_after_each_play_so_two_plays_leave_6() {
            let mut s = scn(json!({
                "seed": "clone-two-plays",
                "p1": { "hand": ["15", "31"], "backrow": ["33"], "field": ["43"], "library": [] },
                "p2": { "field": ["15"] },
            }));

            s.play("15", json!({}));
            s.play("31", json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            assert_eq!(copies_in(&s.pile("p1", "library"), "core-015").len(), 3);
            assert_eq!(copies_in(&s.pile("p1", "library"), "core-031").len(), 3);
        }

        #[test]
        fn r119_a_permanent_does_not_answer_its_own_arrival_it_starts_counting_from_the_next_play() {
            let mut s = scn(json!({
                "seed": "clone-self",
                "p1": { "hand": ["33"], "field": ["43"], "library": [] },
                "p2": { "field": ["15"] },
            }));

            s.play("33", json!({}));

            assert_eq!(s.pile("p1", "library").len(), 0);
        }

        #[test]
        fn you_play_a_card_is_the_controllers_own_plays_not_the_opponents() {
            let mut s = scn(json!({
                "seed": "clone-opponent",
                "active": "p2",
                "p1": { "backrow": ["33"], "field": ["43"], "library": [] },
                "p2": { "hand": ["15"], "field": ["15"], "library": [] },
            }));

            s.play("15", json!({}));

            assert_eq!(s.pile("p1", "library").len(), 0);
            assert_eq!(s.pile("p2", "library").len(), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r275_all_3_copies_of_a_non_radiant_card_are_radiant() {
            let mut s = scn(json!({
                "seed": "clone-radiant",
                "p1": { "hand": ["15"], "backrow": [{ "def": "33", "radiant": true }], "field": ["43"], "library": [] },
                "p2": { "field": ["15"] },
            }));

            s.play("15", json!({}));

            let copies = copies_in(&s.pile("p1", "library"), "core-015");
            assert_eq!(copies.len(), 3);
            assert!(copies.iter().all(|card| card.radiant));
            // R57: still fresh instances, three of them.
            assert!(copies.iter().all(|card| card.damage == 0));
            assert_eq!(distinct_ids(&copies), 3);
            // The played card itself is untouched: only the copies are Radiant.
            assert_eq!(s.unit("p1", 2).map(|card| card.radiant), Some(false));
        }

        #[test]
        fn r275_a_spells_copies_are_radiant_too() {
            let mut s = scn(json!({
                "seed": "clone-radiant-spell",
                "p1": { "hand": ["31"], "backrow": [{ "def": "33", "radiant": true }], "field": ["43"], "library": [] },
                "p2": { "field": ["15"] },
            }));

            s.play("31", json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            let copies = copies_in(&s.pile("p1", "library"), "core-031");
            assert_eq!(copies.len(), 3);
            assert!(copies.iter().all(|card| card.radiant));
        }

        #[test]
        fn a_radiant_card_played_into_a_radiant_copier_makes_3_radiant_copies() {
            let mut s = scn(json!({
                "seed": "clone-radiant-both",
                "p1": {
                    "hand": [{ "def": "15", "radiant": true }],
                    "backrow": [{ "def": "33", "radiant": true }],
                    "field": ["43"],
                    "library": [],
                },
                "p2": { "field": ["15"] },
            }));

            s.play("15", json!({}));

            let copies = copies_in(&s.pile("p1", "library"), "core-015");
            assert_eq!(copies.len(), 3);
            assert!(copies.iter().all(|card| card.radiant));
        }

        #[test]
        fn r80_the_radiant_form_adds_nothing_to_a_60_card_library_either() {
            let mut s = scn(json!({
                "seed": "clone-radiant-cap",
                "p1": {
                    "hand": ["31"],
                    "backrow": [{ "def": "33", "radiant": true }],
                    "field": ["43"],
                    "library": full_library(),
                },
                "p2": { "field": ["15"] },
            }));

            s.play("31", json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            assert_eq!(s.pile("p1", "library").len() as i32, LIBRARY_CAP);
            assert_eq!(copies_in(&s.pile("p1", "library"), "core-031").len(), 0);
        }

        #[test]
        fn r80_a_library_one_short_of_the_cap_takes_one_copy_and_it_is_radiant() {
            let mut s = scn(json!({
                "seed": "clone-radiant-near-cap",
                "p1": {
                    "hand": ["31"],
                    "backrow": [{ "def": "33", "radiant": true }],
                    "field": ["43"],
                    "library": full_library()[1..].to_vec(),
                },
                "p2": { "field": ["15"] },
            }));
            assert_eq!(s.pile("p1", "library").len() as i32, LIBRARY_CAP - 1);

            s.play("31", json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            assert_eq!(s.pile("p1", "library").len() as i32, LIBRARY_CAP);
            let copies = copies_in(&s.pile("p1", "library"), "core-031");
            assert_eq!(copies.len(), 1);
            assert_eq!(copies.first().map(|card| card.radiant), Some(true));
        }
    }

    mod r316_what_a_full_library_turns_away {
        use super::*;

        #[test]
        fn r316_reports_each_of_the_three_copies_a_60_card_library_refuses_as_not_created_naming_the_played_card_to_both_seats()
        {
            let mut s = scn(json!({
                "seed": "clone-overflow",
                "p1": { "hand": ["31"], "backrow": ["33"], "field": ["43"], "library": full_library() },
                "p2": { "field": ["15"] },
            }));

            s.play("31", json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            let refused: Vec<Value> = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::LibraryOverflow { .. }))
                .map(|event| {
                    let event = serde_json::to_value(event).expect("an event is JSON");
                    json!([event["player"], event["defId"], event["outcome"]])
                })
                .collect();
            assert_eq!(
                refused,
                vec![
                    json!(["p1", "core-031", "notCreated"]),
                    json!(["p1", "core-031", "notCreated"]),
                    json!(["p1", "core-031", "notCreated"]),
                ]
            );
            // The spell was played face-up, so its copies read openly on both seats.
            for viewer in ["p1", "p2"] {
                let seen: Vec<Value> = view_events_of_type(&view_json(&s, viewer), "libraryOverflow")
                    .iter()
                    .map(|event| event["defId"].clone())
                    .collect();
                assert_eq!(seen, vec![json!("core-031"), json!("core-031"), json!("core-031")], "{viewer}");
            }
        }

        #[test]
        fn r316_keeps_the_copies_of_a_trap_set_face_down_as_secret_as_the_trap_the_other_seat_reads_the_sentinel() {
            let mut s = scn(json!({
                "seed": "clone-overflow-trap",
                "p1": { "hand": ["41"], "backrow": ["33"], "field": ["43"], "library": full_library() },
                "p2": { "field": ["15"] },
            }));

            s.play("41", json!({ "zone": 2 }));

            let mine = view_events_of_type(&view_json(&s, "p1"), "libraryOverflow");
            let theirs = view_events_of_type(&view_json(&s, "p2"), "libraryOverflow");
            let mine_defs: Vec<Value> = mine.iter().map(|event| event["defId"].clone()).collect();
            assert_eq!(mine_defs, vec![json!("core-041"), json!("core-041"), json!("core-041")]);
            assert_eq!(theirs.len(), 3);
            for event in &theirs {
                assert_eq!(
                    event,
                    &json!({
                        "type": "libraryOverflow",
                        "player": "p1",
                        "instanceId": "hidden",
                        "defId": "hidden",
                        "outcome": "notCreated",
                    })
                );
            }
            // Nothing in the other seat's view names the trap.
            let theirs_text = serde_json::to_string(&s.view("p2")).expect("a view is JSON");
            assert!(!theirs_text.contains("core-041"));
        }
    }

    mod r119_a_permanent_does_not_answer_the_play_that_put_it_onto_the_field {
        use super::*;

        #[test]
        fn r119_a_clone_machine_a_played_cards_recruit_put_on_the_field_does_not_answer_that_play() {
            // Classic #60 Pile On's Radiant face: "Recruit every permanent in your deck", as it resolves.
            let mut s = scn(json!({
                "seed": "edge-r8-hp-clone",
                "p1": {
                    "hand": [{ "def": "classic-060", "radiant": true }, "core-008"],
                    "library": ["core-033", "core-005", "core-005"],
                    "mana": 8,
                },
                "p2": {
                    "hand": ["core-008"],
                    "library": ["core-008", "core-008", "core-008", "core-008", "core-008", "core-008"],
                },
            }));

            s.play("classic-060", json!({}));

            // The Recruit put the Clone Machine on p1's backrow while Pile On's play was resolving.
            let backrow: Vec<String> = (1..=5)
                .filter_map(|lane| s.backrow("p1", lane).map(|card| card.def_id))
                .collect();
            assert!(backrow.contains(&"core-033".to_string()));
            // R119: "does not fire on the play that put it onto the field: it starts counting from the next
            // play". No copies of Pile On are shuffled in.
            assert_eq!(copies_in(&s.pile("p1", "library"), "classic-060").len(), 0);
        }
    }

    mod r311_the_owners_library_list {
        use super::*;

        #[test]
        fn r311_lists_the_3_copies_with_the_face_they_went_in_with_radiant_on_the_radiant_face() {
            let mut s = scn(json!({
                "seed": "clone-r311",
                "p1": { "hand": ["15"], "backrow": [{ "def": "33", "radiant": true }], "field": ["43"], "library": ["25"] },
                "p2": { "field": ["15"] },
            }));

            s.play("15", json!({}));

            // The copies are of a card p1 played in the open, so p1 knows what went in (never where).
            let mine = view_json(&s, "p1");
            let list = &mine["you"]["ownLibrary"];
            assert_eq!(list["unknown"], json!(0));
            let cards = list["cards"].as_array().expect("the owner's library list has cards");
            // The copies are effect-made, so R943 marks them Created; the dealt card carries no mark.
            // Re-pinned for v0.3.5: the Meditative set ships, so R943's Created mark is live.
            assert!(cards.contains(&json!({ "defId": "core-015", "radiant": true, "count": 3, "created": true })));
            assert!(cards.contains(&json!({ "defId": "core-025", "radiant": false, "count": 1 })));
            // p2 reads p1's library as a count and nothing else.
            let theirs = view_json(&s, "p2");
            assert!(theirs["opponent"]["ownLibrary"].is_null());
            assert_eq!(theirs["opponent"]["libraryCount"], json!(4));
        }
    }

    #[test]
    fn r386_an_upgrade_shuffles_4_copies_and_a_degrade_2() {
        for (upgrade, copies) in [(true, 4), (false, 2)] {
            let mut s = scn(json!({
                "seed": "clone-tuned",
                "p1": { "hand": ["15"], "backrow": ["33"], "field": ["43"], "library": ["25"] },
                "p2": { "field": ["15"] },
            }));
            let moved = if upgrade {
                crate::upgrade_number(&mut s, "core-033", "copies")
            } else {
                crate::degrade_number(&mut s, "core-033", "copies")
            };
            assert_eq!(moved, copies);
            s.play("15", json!({}));
            assert_eq!(copies_in(&s.pile("p1", "library"), "core-015").len(), copies as usize);
        }
    }
}
