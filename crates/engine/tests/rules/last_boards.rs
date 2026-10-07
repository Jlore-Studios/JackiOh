//! B5 E30, last boards: a match setup input from outside the match (SPEC §8.7 C+ #29, §9.3, §10.1,
//! R417, R564), driven through `fixtures/lastBoards.ts`' Portal, the smallest script with C+ #29's
//! two faces. The real card's test (packages/cards/test/classic-plus/029-portal-to-the-past.test.ts)
//! covers the same cases again with the real catalog.
//!
//! Port of `packages/engine/test/lastBoards.test.ts`.

use serde::Serialize;

use jackioh_engine::subsystems::last_boards::{
    freeze_last_boards, last_board_candidates, last_board_for, rebuildable_from_id,
};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{put, slot};
use crate::rules::fixtures::last_boards::{
    FIELD_TRAP, LB_DECKS, PORTAL, PORTAL_RADIANT_CARDS, TRAP, act, portal_game, register_last_boards,
};

const FUSED: &str = "t-1:fx-1+fx-2";
const NESTED: &str = "t-2:(t-1:fx-1+fx-2)+fx-3";

fn decks() -> (Vec<String>, Vec<String>) {
    (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21))
}

fn entry(def_id: &str, radiant: bool) -> LastBoardEntry {
    LastBoardEntry {
        def_id: def_id.to_string(),
        radiant,
    }
}

fn created(last_boards: Option<LastBoardInput>) -> GameState {
    register_last_boards();
    create_game(&CreateGameArgs {
        seed: "lb".into(),
        decks: decks(),
        last_boards,
        ..Default::default()
    })
}

fn portal_of(state: &GameState) -> CardInstance {
    state
        .players
        .p1
        .hand
        .iter()
        .find(|entry| entry.def_id == PORTAL)
        .cloned()
        .expect("p1 holds no Portal")
}

fn play(state: &GameState, log: &mut Vec<Action>) -> GameState {
    let portal = portal_of(state);
    act(
        state,
        log,
        json!({ "type": "play", "playerId": "p1", "instanceId": portal.id }),
    )
}

fn answer_with(state: &GameState, log: &mut Vec<Action>, option: &str) -> GameState {
    let pending = state.pending.as_ref().expect("no prompt is open");
    let selection = json!([{ "pick": "mode", "option": option }]);
    act(
        state,
        log,
        json!({ "type": "answer", "playerId": pending.player_id, "choiceId": pending.id, "selection": selection }),
    )
}

fn json_of<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// TS `JSON.parse(JSON.stringify(state))`.
fn revive(state: &GameState) -> GameState {
    serde_json::from_value(json_of(state)).expect("a state revives from its JSON")
}

/// vitest's `toMatchObject`: every key the expected object names matches, recursively; an array
/// matches element for element and in length.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|found| matches_object(found, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len() && actual.iter().zip(expected).all(|(a, e)| matches_object(a, e))
        }
        _ => actual == expected,
    }
}

mod b5_e30_last_boards_as_a_create_game_input_r417 {
    use super::*;

    #[test]
    fn r417_each_seats_board_is_frozen_into_the_match_as_card_and_face_only() {
        // TS's third entry is `{ defId: "fx-5" }` with no face, which a `LastBoardEntry` cannot hold
        // (spec-gaps-part-25-3.md); it is written with the face the freeze gives it.
        let state = created(Some(json_as(json!([
            [{ "defId": "fx-3", "radiant": true, "damage": 4 }, { "defId": "fx-4", "radiant": false }],
            [{ "defId": "fx-5", "radiant": false }],
        ]))));
        assert_eq!(
            json_of(&state.last_boards),
            json!({
                "p1": [
                    { "defId": "fx-3", "radiant": true },
                    { "defId": "fx-4", "radiant": false },
                ],
                "p2": [{ "defId": "fx-5", "radiant": false }],
            })
        );
    }

    #[test]
    fn r417_no_last_boards_or_two_empty_ones_store_nothing_the_game_hashes_as_one_created_without() {
        let without = created(None);
        assert!(without.last_boards.is_none());
        assert_eq!(hash_state(&created(Some((vec![], vec![])))), hash_state(&without));
        assert_eq!(
            hash_state(&created(Some((vec![entry("nope", false)], vec![])))),
            hash_state(&without)
        );
    }

    #[test]
    fn r564_an_entry_this_match_cannot_rebuild_from_its_id_is_dropped_as_the_match_is_created() {
        register_last_boards();
        let catalog = registered_catalog();
        assert!(rebuildable_from_id("fx-1", catalog));
        assert!(rebuildable_from_id("fx-token-rush", catalog));
        assert!(rebuildable_from_id(FUSED, catalog));
        assert!(rebuildable_from_id(NESTED, catalog));
        assert!(!rebuildable_from_id("core-999", catalog));
        assert!(!rebuildable_from_id("t-3", catalog));
        assert!(!rebuildable_from_id("t-1:fx-1+nope", catalog));
        assert!(!rebuildable_from_id("t-1:fx-1", catalog));
        // R468: a digest names its list only to the process that minted it, so it never rebuilds.
        assert!(!rebuildable_from_id("t-4:#0123456789abcdef", catalog));
        assert!(!rebuildable_from_id("t-5:(t-4:#0123456789abcdef)+fx-1", catalog));

        // TS's p1 list also holds `null` and `{ radiant: true }` (no `defId`), which a
        // `LastBoardInput` cannot hold (spec-gaps-part-25-3.md); the entries it can hold are kept.
        let input: LastBoardInput = (
            vec![entry("nope", true), entry(FUSED, true)],
            vec![entry("t-4:#0123456789abcdef", false)],
        );
        assert_eq!(
            json_of(freeze_last_boards(Some(&input), catalog)),
            json!({ "p1": [{ "defId": FUSED, "radiant": true }] })
        );
    }

    #[test]
    fn r417_the_frozen_boards_survive_json() {
        let state = created(Some((vec![entry(FUSED, false)], vec![entry("fx-9", true)])));
        let revived = revive(&state);
        assert_eq!(revived, state);
        assert_eq!(hash_state(&revived), hash_state(&state));
    }

    #[test]
    fn r417_view_for_never_sends_a_last_board_to_either_seat() {
        // Two cards no deck holds, so nothing else in a view can name them.
        let state = created(Some((vec![entry(TRAP, true)], vec![entry(FIELD_TRAP, true)])));
        for viewer in [P1, P2] {
            let text = serde_json::to_string(&view_for(&state, viewer)).expect("a view serialises");
            assert!(!text.contains(TRAP));
            assert!(!text.contains(FIELD_TRAP));
            assert!(!text.contains("lastBoard"));
        }
    }
}

mod b5_e30_the_reader_the_server_calls_as_a_game_ends_r417 {
    use super::*;

    fn board() -> GameState {
        let mut state = created(None);
        put(
            &mut state,
            "fx-1",
            slot(P1, Row::Units, 1),
            json!({ "radiant": true }),
        );
        put(&mut state, "fx-2", slot(P1, Row::Units, 2), Default::default());
        // A Stack pile: the dormant card beneath is not on the field (R13).
        let mut top = new_instance(&mut state, "fx-3", P1, Zone::Hand { player: P1 });
        assert!(
            place_on_field(
                &mut state,
                &mut top,
                slot(P1, Row::Units, 2),
                PlaceOnFieldOptions { stack: Some(true) }
            ),
            "stack"
        );
        put(&mut state, TRAP, slot(P1, Row::Backrow, 1), Default::default());
        // A Unit carried on p1's trap (R446).
        let mut carried = new_instance(&mut state, "fx-4", P1, Zone::Hand { player: P1 });
        assert!(
            place_on_field(
                &mut state,
                &mut carried,
                slot(P1, Row::Backrow, 1),
                PlaceOnFieldOptions { stack: Some(true) }
            ),
            "carry"
        );
        put(&mut state, "fx-21", slot(P2, Row::Units, 3), Default::default());
        put(
            &mut state,
            TRAP,
            slot(P2, Row::Backrow, 2),
            json!({ "radiant": true }),
        );
        let fired = put(
            &mut state,
            FIELD_TRAP,
            slot(P2, Row::Backrow, 3),
            Default::default(),
        );
        find_instance_mut(&mut state, &fired.id)
            .expect("the fired Field Trap")
            .face_up = Some(true);
        // p2's trap that p1 controls now (a steal, R33): p1 reads it, p2 no longer does.
        let mut stolen = new_instance(&mut state, FIELD_TRAP, P2, Zone::Hand { player: P2 });
        assert!(
            place_on_field(
                &mut state,
                &mut stolen,
                slot(P1, Row::Backrow, 4),
                PlaceOnFieldOptions::default()
            ),
            "stolen"
        );
        // Damage and buffs are the card's state, never its entry.
        let hurt = state.players.p2.units[2]
            .as_mut()
            .and_then(|pile| pile.first_mut())
            .expect("p2 unit");
        hurt.damage = 1;
        hurt.buffs = AttackHealth { attack: 3, health: 3 };
        state
    }

    #[test]
    fn r417_every_card_on_the_field_both_sides_each_seat_minus_the_other_sides_face_down_cards_r33() {
        let state = board();
        assert_eq!(
            last_board_for(&state, P1),
            vec![
                entry("fx-1", true),
                entry("fx-3", false),
                entry("fx-4", false),
                entry(TRAP, false),
                entry(FIELD_TRAP, false),
                entry("fx-21", false),
                entry(FIELD_TRAP, false),
            ]
        );
        assert_eq!(
            last_board_for(&state, P2),
            vec![
                entry("fx-1", true),
                entry("fx-3", false),
                entry("fx-4", false),
                entry("fx-21", false),
                entry(TRAP, true),
                entry(FIELD_TRAP, false),
            ]
        );
    }

    #[test]
    fn r417_a_seats_board_is_exactly_what_its_own_view_shows_of_the_field() {
        let state = board();
        for seat in [P1, P2] {
            let view = view_for(&state, seat);
            let mut sides = [&view.you, &view.opponent];
            sides.sort_by(|a, b| a.player.as_str().cmp(b.player.as_str()));
            let shown: Vec<String> = sides
                .iter()
                .flat_map(|side| {
                    let mut ids: Vec<String> = side
                        .units
                        .iter()
                        .flatten()
                        .map(|unit| unit.def_id.clone())
                        .collect();
                    for (lane, card) in side.backrow.iter().enumerate() {
                        if let Some(carried) = side
                            .carried
                            .as_ref()
                            .and_then(|carried| carried.get(lane))
                            .and_then(|carried| carried.as_ref())
                        {
                            ids.push(carried.def_id.clone());
                        }
                        if let Some(BackrowView::Public(card)) = card {
                            ids.push(card.def_id.clone());
                        }
                    }
                    ids
                })
                .collect();
            let board: Vec<String> = last_board_for(&state, seat)
                .into_iter()
                .map(|entry| entry.def_id)
                .collect();
            assert_eq!(board, shown);
        }
    }

    #[test]
    fn r417_an_empty_field_is_an_empty_board() {
        assert!(last_board_for(&created(None), P1).is_empty());
    }
}

mod b5_e30_c_29s_verbs_over_the_frozen_board_r417_r564 {
    use super::*;

    // p2's deck holds fx-21 to fx-40, so none of these can reach p1's hand any other way.
    fn p1_board() -> Vec<LastBoardEntry> {
        vec![
            entry("fx-25", false),
            entry("fx-26", false),
            entry("fx-25", true),
            entry("fx-27", false),
            entry("fx-token-rush", false),
            entry(PORTAL, false),
        ]
    }

    const P1_CARDS: [&str; 4] = ["fx-25", "fx-26", "fx-27", "fx-token-rush"];

    fn p2_board() -> Vec<LastBoardEntry> {
        vec![entry(TRAP, false)]
    }

    #[test]
    fn r564_the_candidates_are_each_different_card_once_radiant_if_any_entry_was_never_the_generating_card_r387()
     {
        let state = created(Some((p1_board(), p2_board())));
        assert_eq!(
            last_board_candidates(&state, P1, &[PORTAL.to_string()]),
            vec![
                entry("fx-25", true),
                entry("fx-26", false),
                entry("fx-27", false),
                entry("fx-token-rush", false),
            ]
        );
        assert_eq!(last_board_candidates(&state, P2, &[]), p2_board());
    }

    #[test]
    fn r417_base_discover_3_different_cards_of_the_casters_own_board_shown_to_the_caster_only_10_8_r81() {
        let mut run = portal_game("lb-discover", Some((p1_board(), p2_board())));
        let state = play(&run.state, &mut run.log);
        let pending = state.pending.as_ref();
        assert_eq!(pending.map(|pending| pending.kind), Some(PromptKind::Discover));
        assert_eq!(pending.map(|pending| pending.player_id), Some(P1));
        let offered: Vec<String> = pending
            .map(|pending| {
                pending
                    .options
                    .iter()
                    .map(|option| match &option.selection {
                        Selection::Mode { option } => option.clone(),
                        _ => String::new(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(offered.len(), 3);
        assert_eq!(offered.iter().collect::<IndexSet<_>>().len(), 3);
        for def_id in &offered {
            assert!(P1_CARDS.contains(&def_id.as_str()));
        }
        // The other seat learns that a prompt is open and whose, and nothing of its options (R81, R177).
        assert_eq!(
            json_of(view_for(&state, P2).pending),
            json!({ "forYou": false, "pendingFor": "p1" })
        );
        assert!(
            !serde_json::to_string(&view_for(&state, P2))
                .expect("a view serialises")
                .contains("fx-token-rush")
        );
    }

    #[test]
    fn r417_the_pick_arrives_as_a_new_card_the_caster_owns_on_its_entrys_face_costing_0_the_other_seat_reads_the_sentinel_r97()
     {
        let mut run = portal_game("lb-pick", Some((vec![entry("fx-25", true)], p2_board())));
        let mut state = play(&run.state, &mut run.log);
        assert_eq!(
            state.pending.as_ref().map(|pending| pending
                .options
                .iter()
                .map(|option| option.radiant)
                .collect::<Vec<_>>()),
            Some(vec![Some(true)])
        );
        state = answer_with(&state, &mut run.log, "fx-25");
        let made = state
            .players
            .p1
            .hand
            .iter()
            .find(|card| card.def_id == "fx-25")
            .expect("the picked card in hand");
        assert!(matches_object(
            &json_of(made),
            &json!({ "owner": "p1", "controller": "p1", "radiant": true, "costOverride": 0, "damage": 0 })
        ));
        assert!(state.pending.is_none());
        let seen: Vec<Value> = view_for(&state, P2)
            .events
            .iter()
            .filter(|event| event.event_type() == GameEventType::AddedToHand)
            .map(json_of)
            .collect();
        assert!(matches_object(
            seen.last().expect("an addedToHand event"),
            &json!({ "instanceId": HIDDEN_ID, "defId": HIDDEN_ID })
        ));
    }

    #[test]
    fn r129_an_empty_last_board_fizzles_no_prompt_no_random_draw_and_the_spell_still_counts_as_played() {
        let mut run = portal_game("lb-empty", None);
        let before = run.state.rng_cursor;
        let played = run.state.counters.played;
        let state = play(&run.state, &mut run.log);
        assert!(state.pending.is_none());
        assert_eq!(state.rng_cursor, before);
        assert_eq!(state.counters.played, played + 1);
        assert!(
            state
                .players
                .p1
                .graveyard
                .iter()
                .any(|card| card.def_id == PORTAL)
        );
    }

    #[test]
    fn r179_a_fused_entry_is_rebuilt_from_its_id_definition_and_scripts_nested_fusions_included() {
        let mut run = portal_game("lb-fused", Some((vec![entry(NESTED, false)], vec![])));
        let mut state = play(&run.state, &mut run.log);
        state = answer_with(&state, &mut run.log, NESTED);
        let made = state.players.p1.hand.iter().find(|card| card.def_id == NESTED);
        assert_eq!(made.and_then(|card| card.cost_override), Some(0));
        // Summed faces: fx-1, fx-2 and fx-3 are 2/2 each, so 6/6, and 12/12 Radiant.
        assert!(matches_object(
            &json_of(state.transient_defs.get(NESTED)),
            &json!({ "base": { "attack": 6, "health": 6 }, "radiant": { "attack": 12, "health": 12 }, "type": "Unit" })
        ));
        assert!(matches_object(
            &json_of(state.transient_defs.get(FUSED)),
            &json!({ "base": { "attack": 4, "health": 4 } })
        ));
        // TS `registeredScripts()[NESTED]` is defined: Rust registers no fused scripts (SURFACE §6.6),
        // it composes them on lookup from the definition above (spec-gaps-part-25-3.md).
        let _composed = script_of(&state, NESTED);
    }

    #[test]
    fn r417_radiant_adds_3_different_random_cards_straight_to_hand_each_costing_0() {
        let mut run = portal_game("lb-radiant", Some((p1_board(), p2_board())));
        let portal = portal_of(&run.state);
        find_instance_mut(&mut run.state, &portal.id)
            .expect("the Portal")
            .radiant = true;
        let state = play(&run.state, &mut run.log);
        assert!(state.pending.is_none());
        let added: Vec<&CardInstance> = state
            .players
            .p1
            .hand
            .iter()
            .filter(|card| card.cost_override == Some(0))
            .collect();
        assert_eq!(added.len(), PORTAL_RADIANT_CARDS as usize);
        assert_eq!(
            added
                .iter()
                .map(|card| card.def_id.clone())
                .collect::<IndexSet<_>>()
                .len(),
            PORTAL_RADIANT_CARDS as usize
        );
        for card in &added {
            assert!(P1_CARDS.contains(&card.def_id.as_str()));
        }
    }

    #[test]
    fn r417_paused_mid_prompt_the_state_survives_json_and_folds_from_seed_decks_last_boards_log_to_the_same_hash()
     {
        let mut first = p1_board();
        first.push(entry(FUSED, true));
        let boards: LastBoardInput = (first, p2_board());
        let mut run = portal_game("lb-fold", Some(boards.clone()));
        let paused = play(&run.state, &mut run.log);
        let revived = revive(&paused);
        assert_eq!(revived, paused);
        let option = match revived
            .pending
            .as_ref()
            .and_then(|pending| pending.options.first())
            .map(|option| &option.selection)
        {
            Some(Selection::Mode { option }) => option.clone(),
            _ => panic!("no mode option"),
        };
        let done = answer_with(&revived, &mut run.log, &option);

        let replayed = fold(&FoldArgs {
            seed: run.seed.clone(),
            decks: LB_DECKS.clone(),
            last_boards: Some(boards),
            log: run.log.clone(),
            ..FoldArgs::default()
        });
        assert!(replayed.errors.is_empty());
        assert_eq!(hash_state(&replayed.state), hash_state(&done));
        // Folded without them, it is another game.
        let without = fold(&FoldArgs {
            seed: run.seed.clone(),
            decks: LB_DECKS.clone(),
            log: run.log.clone(),
            ..FoldArgs::default()
        });
        assert_ne!(hash_state(&without.state), hash_state(&done));
    }
}
