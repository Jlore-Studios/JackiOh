//! Two marks v0.1.1 adds to a backrow card's view (SPEC §10.8), each read by the client and decided
//! by the engine alone (CLAUDE.md rule 7):
//!
//!   R371  `unrevealed: true` on the controller's own view of a Trap or Field Trap that is still
//!         face-down, so the board can say "your opponent can't see this card" without working out
//!         R33 itself. It is the same answer `backrowIsPublic` gives the other seat, so the mark and
//!         the back cannot disagree: present exactly when the other player sees a back.
//!   R372  `counters.gradeLetter`, the letter #93 Combo-Index's grade counter stands for (E..S), and
//!         a preview value's `display`, the word a value prints as, carried through `previewOf`
//!         untouched. The Core card's own values are proved in packages/cards
//!         (test/093-combo-index.test.ts).
//!
//! Test-only definitions, registered over the fixture catalog as viewFor.test.ts and
//! preview.test.ts register theirs, and put back afterwards.
//!
//! Port of `packages/engine/test/view-marks.test.ts`. TS's `beforeAll`/`afterAll` saved the registries
//! and put them back after the file; each Rust test runs on its own thread with its own testkit
//! override (SURFACE §8), so nothing registered here outlives its test and there is nothing to restore.

use jackioh_engine::effects::steal;
use jackioh_engine::subsystems::combo_index::GRADES;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{new_game, put, sink_for, slot};

/// TS `def(name, type)`; `index` is the one TS's `nextIndex` counter gave it (3701 on).
fn def(name: &str, type_: &str, index: i32) -> CardDef {
    json_as(json!({
        "id": format!("vm-{name}"),
        "index": index.to_string(),
        "name": name,
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 2,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    }))
}

fn trap() -> CardDef {
    def("trap", "Trap", 3701)
}
fn field_trap() -> CardDef {
    def("field-trap", "Field Trap", 3702)
}
fn field_spell() -> CardDef {
    def("field-spell", "Field Spell", 3703)
}
/// A Field Spell whose preview names its value by a word, as #93's grade does.
fn worded() -> CardDef {
    def("worded", "Field Spell", 3704)
}

fn worded_values() -> Vec<PreviewValue> {
    vec![
        PreviewValue {
            label: "Grade".to_string(),
            value: 4,
            display: Some("B".to_string()),
            ids: None,
        },
        PreviewValue {
            label: "N".to_string(),
            value: 4,
            display: None,
            ids: None,
        },
        // An empty `display` is no word at all: the number prints.
        PreviewValue {
            label: "M".to_string(),
            value: 2,
            display: Some(String::new()),
            ids: None,
        },
    ]
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(
        worded().id,
        CardScripts {
            base: Script {
                preview: Some(condition_hook(|_ctx| worded_values())),
                ..Script::default()
            },
            radiant: Script {
                preview: Some(condition_hook(|_ctx| worded_values())),
                ..Script::default()
            },
        },
    );
    scripts
}

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for card in [trap(), field_trap(), field_spell(), worded()] {
        catalog.insert(card.id.clone(), card);
    }
    register_catalog(catalog);
    let mut all = registered_scripts().clone();
    all.extend(scripts());
    register_scripts(all);
    state.turn = 3;
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    state
}

/// The public half of `BackrowView`, failing on a back or an empty zone.
fn public_card(entry: Option<&Option<BackrowView>>) -> PublicBackrowView {
    match entry {
        Some(Some(BackrowView::Public(card))) => card.clone(),
        _ => panic!("expected a card the viewer reads"),
    }
}

/// A backrow entry as its JSON (`entry?.faceDown`, `"unrevealed" in entry`).
fn entry_json(entry: &impl serde::Serialize) -> Value {
    serde_json::to_value(entry).expect("a view serialises")
}

/// `toMatchObject`: every key of `expected` is in `actual` with a matching value.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|found| matches_object(found, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len()
                && actual
                    .iter()
                    .zip(expected)
                    .all(|(found, value)| matches_object(found, value))
        }
        _ => actual == expected,
    }
}

mod r371_the_controllers_view_marks_a_face_down_trap_as_unrevealed {
    use super::*;

    #[test]
    fn r371_a_face_down_trap_and_field_trap_read_in_full_by_their_controller_carry_unrevealed_true() {
        let mut state = game("r371-own");
        put(
            &mut state,
            &trap().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        put(
            &mut state,
            &field_trap().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );

        let mine = view_for(&state, PlayerId::P1).you.backrow;
        assert!(matches_object(
            &entry_json(&public_card(mine.first())),
            &json!({ "faceDown": false, "defId": trap().id, "unrevealed": true })
        ));
        assert!(matches_object(
            &entry_json(&public_card(mine.get(1))),
            &json!({ "faceDown": false, "defId": field_trap().id, "unrevealed": true })
        ));

        // Exactly where the other seat sees a back: the mark and R33's marker are one answer.
        let theirs = view_for(&state, PlayerId::P2).opponent.backrow;
        assert_eq!(entry_json(&theirs[0])["faceDown"], json!(true));
        assert_eq!(entry_json(&theirs[1])["faceDown"], json!(true));
        assert!(
            !serde_json::to_string(&theirs)
                .expect("serialises")
                .contains("unrevealed")
        );
    }

    #[test]
    fn r371_a_field_spell_and_a_field_trap_that_has_fired_are_public_so_neither_carries_the_mark() {
        let mut state = game("r371-public");
        put(
            &mut state,
            &field_spell().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        let fired = put(
            &mut state,
            &field_trap().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        find_instance_mut(&mut state, &fired.id)
            .expect("the fired trap")
            .face_up = Some(true);

        for viewer in [PlayerId::P1, PlayerId::P2] {
            let view = view_for(&state, viewer);
            let row = if viewer == PlayerId::P1 {
                &view.you.backrow
            } else {
                &view.opponent.backrow
            };
            for entry in [row.first(), row.get(1)] {
                assert!(entry_json(&public_card(entry)).get("unrevealed").is_none());
            }
        }
        let fired = find_instance(&state, &fired.id).cloned().expect("the fired trap");
        assert!(!is_face_down(&state, &fired));
    }

    #[test]
    fn r371_r33_the_mark_follows_control_a_stolen_face_down_trap_is_unrevealed_for_its_thief() {
        let mut state = game("r371-steal");
        let hidden = put(
            &mut state,
            &trap().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        assert_eq!(
            public_card(view_for(&state, PlayerId::P2).you.backrow.first()).unrevealed,
            Some(true)
        );

        let cursor = {
            let mut sink = sink_for(&mut state);
            {
                let mut ctx = make_context(
                    &mut sink,
                    None,
                    HookOptions {
                        controller: Some(PlayerId::P1),
                        ..Default::default()
                    },
                );
                (steal(json_as(json!({ "instanceId": hidden.id }))).apply)(&mut ctx);
            }
            sink.rng.cursor()
        };
        state.rng_cursor = cursor;

        assert_eq!(
            find_instance(&state, &hidden.id).map(|card| card.controller),
            Some(PlayerId::P1)
        );
        assert!(matches_object(
            &entry_json(&public_card(view_for(&state, PlayerId::P1).you.backrow.first())),
            &json!({ "defId": trap().id, "unrevealed": true })
        ));
        assert_eq!(
            entry_json(&view_for(&state, PlayerId::P2).opponent.backrow[0])["faceDown"],
            json!(true)
        );
    }
}

mod r372_a_grades_letter_and_a_worded_value_travel_in_the_view {
    use super::*;

    #[test]
    fn r372_counters_grade_letter_names_each_grade_1_6_as_e_d_c_b_a_s_on_both_seats() {
        let mut state = game("r372-letters");
        let card = put(
            &mut state,
            &field_spell().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        for (at, letter) in GRADES.iter().enumerate() {
            let grade = at as i32 + 1;
            find_instance_mut(&mut state, &card.id)
                .expect("the card")
                .counters
                .grade = Some(grade);
            assert_eq!(
                entry_json(&public_card(view_for(&state, PlayerId::P1).you.backrow.first()).counters),
                json!({ "grade": grade, "gradeLetter": letter })
            );
            assert_eq!(
                entry_json(&public_card(view_for(&state, PlayerId::P2).opponent.backrow.first()).counters),
                json!({ "grade": grade, "gradeLetter": letter })
            );
        }
        assert_eq!(
            serde_json::to_value(GRADES.to_vec()).expect("serialises"),
            json!(["E", "D", "C", "B", "A", "S"])
        );
    }

    #[test]
    fn r372_a_card_with_no_grade_counter_carries_neither_number_nor_letter() {
        let mut state = game("r372-none");
        put(
            &mut state,
            &field_spell().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        assert_eq!(
            entry_json(&public_card(view_for(&state, PlayerId::P1).you.backrow.first()).counters),
            json!({})
        );
    }

    #[test]
    fn r372_preview_of_carries_a_values_display_word_and_drops_an_empty_one() {
        let mut state = game("r372-display");
        put(
            &mut state,
            &worded().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        for viewer in [PlayerId::P1, PlayerId::P2] {
            let view = view_for(&state, viewer);
            let side = if viewer == PlayerId::P1 {
                &view.you
            } else {
                &view.opponent
            };
            let entry = public_card(side.backrow.first());
            assert_eq!(
                serde_json::to_value(&entry.preview).expect("serialises"),
                json!([
                    { "label": "Grade", "value": 4, "display": "B" },
                    { "label": "N", "value": 4 },
                    { "label": "M", "value": 2 },
                ])
            );
        }
    }
}
