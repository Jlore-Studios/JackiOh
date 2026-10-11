//! R1447: a view names no card its viewer may not read (SPEC §9.1, §10.8, R97). Two leaks the fuzz's
//! I6 found on `main` (super run 38145503152, seeds 5245 and 8178):
//!
//!   - a preview value's `ids` (R372, R280) named every card its count counted, the controller's own
//!     face-down traps included, to both seats. The ids keep only the cards the viewer reads where
//!     they lie (R466's readers); the number still counts them all, since the other seat sees backs;
//!   - a Heroic Power (#98, R43, R752) on a face-down card was listed in its hero's `powers`, its id and
//!     definition with it, though a face-down card has no ability to use. It is listed no more, for
//!     either seat, until the card is face-up.
//!
//! Test-only definitions, registered over the fixture catalog.

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{new_game, put, slot};

/// A test-only definition; `index` is its catalog index (3711 on).
fn def(name: &str, type_: &str, index: i32) -> CardDef {
    json_as(json!({
        "id": format!("vh-{name}"),
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
    def("trap", "Trap", 3711)
}
fn field_spell() -> CardDef {
    def("field-spell", "Field Spell", 3712)
}
/// A Field Spell whose preview counts every backrow card on the field and names them, as Classic+ #44's
/// "all permanents" names what it counts.
fn counter() -> CardDef {
    def("counter", "Field Spell", 3713)
}

fn counted(ctx: ConditionContext<'_>) -> Vec<PreviewValue> {
    let ids: Vec<String> = PLAYER_IDS
        .iter()
        .flat_map(|player| ctx.state.players[*player].backrow.iter().flatten())
        .map(|card| card.id.clone())
        .collect();
    vec![PreviewValue {
        label: "backrow cards".to_string(),
        value: i32::try_from(ids.len()).expect("a few cards"),
        display: None,
        ids: Some(ids),
    }]
}

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for card in [trap(), field_spell(), counter()] {
        catalog.insert(card.id.clone(), card);
    }
    register_catalog(catalog);
    let mut all = registered_scripts().clone();
    all.insert(
        counter().id,
        CardScripts {
            base: Script {
                preview: Some(condition_hook(counted)),
                ..Script::default()
            },
            radiant: Script {
                preview: Some(condition_hook(counted)),
                ..Script::default()
            },
        },
    );
    register_scripts(all);
    state.turn = 3;
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    state
}

/// The preview a public backrow entry carries, failing on a back or an empty zone.
fn preview_on(entry: Option<&Option<BackrowView>>) -> Vec<PreviewValue> {
    match entry {
        Some(Some(BackrowView::Public(card))) => card.preview.clone().expect("the card previews"),
        _ => panic!("expected a card the viewer reads"),
    }
}

/// Gives `id` the power #98 rolls, as a fusion with #98 carries it in its memory (R43).
fn give_power(state: &mut GameState, id: &str) {
    find_instance_mut(state, id)
        .expect("the card is on the field")
        .memory
        .insert("power".to_string(), json!("armor"));
}

mod r1447_a_view_names_no_card_its_viewer_may_not_read {
    use super::*;

    #[test]
    fn r1447_a_preview_names_only_the_cards_its_viewer_reads_and_still_counts_every_one() {
        let mut state = game("r1447-preview");
        let count = put(
            &mut state,
            &counter().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        let mine = put(
            &mut state,
            &trap().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        let theirs = put(
            &mut state,
            &trap().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        let open = put(
            &mut state,
            &field_spell().id,
            slot(PlayerId::P2, Row::Backrow, 2),
            json!({}),
        );

        // The controller reads its own face-down trap, never the other seat's.
        let own = preview_on(view_for(&state, PlayerId::P1).you.backrow.first());
        assert_eq!(own.len(), 1);
        assert_eq!(own[0].value, 4);
        assert_eq!(
            own[0].ids.as_deref(),
            Some(&[count.id.clone(), mine.id.clone(), open.id.clone()][..])
        );

        // The other seat reads the count's card, its own trap and the open Field Spell: the count is
        // the same, and the controller's face-down trap is not named.
        let other = preview_on(view_for(&state, PlayerId::P2).opponent.backrow.first());
        assert_eq!(other[0].value, 4);
        assert_eq!(
            other[0].ids.as_deref(),
            Some(&[count.id.clone(), theirs.id.clone(), open.id.clone()][..])
        );
        let serialized = serde_json::to_string(&view_for(&state, PlayerId::P2)).expect("serialises");
        assert!(
            !serialized.contains(&format!("\"{}\"", mine.id)),
            "p2's view names p1's face-down trap"
        );
    }

    #[test]
    fn r1447_a_heroic_power_on_a_face_down_card_is_listed_for_neither_seat_until_it_is_face_up() {
        let mut state = game("r1447-power");
        let hidden = put(
            &mut state,
            &trap().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        give_power(&mut state, &hidden.id);

        for viewer in [PlayerId::P1, PlayerId::P2] {
            let view = view_for(&state, viewer);
            let hero = if viewer == PlayerId::P2 {
                &view.you.hero
            } else {
                &view.opponent.hero
            };
            assert!(hero.power.is_none(), "{viewer} sees a face-down card's power");
            assert!(hero.powers.is_empty(), "{viewer} sees a face-down card's power");
            let serialized = serde_json::to_string(&view).expect("serialises");
            if viewer == PlayerId::P1 {
                assert!(
                    !serialized.contains(&format!("\"{}\"", hidden.id)),
                    "p1's view names p2's face-down trap"
                );
            }
        }

        // Face-up, the card's power is its controller's again, and public (R43).
        find_instance_mut(&mut state, &hidden.id)
            .expect("on the field")
            .face_up = Some(true);
        for viewer in [PlayerId::P1, PlayerId::P2] {
            let view = view_for(&state, viewer);
            let hero = if viewer == PlayerId::P2 {
                &view.you.hero
            } else {
                &view.opponent.hero
            };
            assert_eq!(
                hero.power.as_ref().map(|power| power.instance_id.clone()),
                Some(hidden.id.clone())
            );
        }
    }

    #[test]
    fn r1447_a_heroic_power_on_a_face_up_field_spell_is_listed_for_both_seats_as_before() {
        let mut state = game("r1447-open-power");
        let open = put(
            &mut state,
            &field_spell().id,
            slot(PlayerId::P1, Row::Backrow, 3),
            json!({}),
        );
        give_power(&mut state, &open.id);
        for viewer in [PlayerId::P1, PlayerId::P2] {
            let view = view_for(&state, viewer);
            let hero = if viewer == PlayerId::P1 {
                &view.you.hero
            } else {
                &view.opponent.hero
            };
            let power = hero.power.as_ref().expect("the power is listed");
            assert_eq!(power.instance_id, open.id);
            assert_eq!(power.name, "armor");
        }
    }
}
