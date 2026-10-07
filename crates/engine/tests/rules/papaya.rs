//! Classic+ #62 KY's Papaya's curve targeting (SPEC §8.7 row 62, R422; docs/classic-sets.md B5 E32),
//! through the fixture Spell in `fixtures/papaya.ts`: the grid from the caster's seat, the curve in
//! exact rationals, the cells on it, the one-cell-at-a-time prompts (E18's `cell` kind), the exile of
//! the top card at each cell, the Radiant face's enemy rows, what the other seat sees (R97, R177), a
//! random cast's answers (R452), and a pause that survives JSON and a game that replays (§9.2, §9.3).
//!
//! Port of `packages/engine/test/papaya.test.ts`.

use serde::Serialize;

use jackioh_engine::effects::cast_new;
use jackioh_engine::subsystems::papaya::{
    PAPAYA_LANES, PapayaPoint, cards_on_curve, cells_on_curve, curve_at, point_of_zone, zone_of_point,
};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{events_of_type, put, setup_catalog, sink_for, slot};
use crate::rules::fixtures::papaya::{
    body, curve, curve_quickdraw, field, register_papaya_fixtures, snare, token,
};
use crate::rules::fixtures::prompt_harness::{answer_keys, board, cast_now, must, open_as, round_trip};
use crate::rules::fixtures::prompts::register_prompt_fixtures;

fn game(seed: &str) -> GameState {
    let state = board(seed);
    register_papaya_fixtures();
    state
}

/// A grid cell, `{ x, y }`.
fn pt(x: i32, y: i32) -> PapayaPoint {
    PapayaPoint { x, y }
}

fn pts(cells: &[(i32, i32)]) -> Vec<PapayaPoint> {
    cells.iter().map(|&(x, y)| pt(x, y)).collect()
}

/// A curve's cells as `(x, y)` pairs, for comparing with TS's `{ x, y }` literals.
fn cells(points: Vec<PapayaPoint>) -> Vec<(i32, i32)> {
    points.iter().map(|point| (point.x, point.y)).collect()
}

/// The option key of a grid cell, from p1's seat (the caster in every test here).
fn key(x: i32, y: i32) -> String {
    let zone = zone_of_point(P1, pt(x, y));
    format!("zone:{}:{}:{}", zone.player, zone.row, zone.lane)
}

/// Cast the curve for p1 and answer these cells, then "done" when fewer than four were given.
/// (TS `draw`.)
fn draw_curve(state: &mut GameState, points: &[(i32, i32)], radiant: bool) {
    cast_now(state, &curve().id, P1, radiant);
    for &(x, y) in points {
        answer_keys(state, &[&key(x, y)]);
    }
    if (points.len() as i32) < PAPAYA_MAX_CELLS {
        answer_keys(state, &["none"]);
    }
    assert!(state.pending.is_none());
}

fn zone_of(selection: &Selection) -> ZoneRef {
    match selection {
        Selection::Zone { player, row, lane } => ZoneRef {
            player: *player,
            row: *row,
            lane: *lane,
        },
        _ => panic!("expected a cell"),
    }
}

fn exiled_ids(state: &GameState) -> Vec<String> {
    state
        .players
        .p1
        .exile
        .iter()
        .chain(state.players.p2.exile.iter())
        .map(|card| card.id.clone())
        .collect()
}

/// A card on every one of the 20 cells: units on both unit rows, Field Spells on both backrows.
fn full_board(state: &mut GameState) -> Vec<Vec<CardInstance>> {
    [P1, P2]
        .into_iter()
        .map(|player| {
            let mut cards: Vec<CardInstance> = slots_of(player, Row::Units)
                .into_iter()
                .map(|at| put(state, &body().id, at, Default::default()))
                .collect();
            cards.extend(
                slots_of(player, Row::Backrow)
                    .into_iter()
                    .map(|at| put(state, &field().id, at, Default::default())),
            );
            cards
        })
        .collect()
}

fn json_of<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

fn events_json(events: &[GameEvent], kind: GameEventType) -> Vec<Value> {
    events_of_type(events, kind).into_iter().map(json_of).collect()
}

fn ids_of(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

/// The zone a card stands in now (TS read `card.zone.z` off the live object).
fn zone_name(state: &GameState, id: &str) -> Option<ZoneName> {
    find_instance(state, id).map(|card| card.zone.z())
}

fn answer_count(state: &GameState, player: PlayerId) -> usize {
    legal_actions(state, player)
        .iter()
        .filter(|action| matches!(action, ActionBody::Answer { .. }))
        .count()
}

/// `expect(() => run()).toThrow(/text/)`: TS threw, Rust panics with the same message.
fn panics_with(run: impl FnOnce(), text: &str) {
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run));
    let payload = caught.expect_err("expected a panic");
    let message = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|text| text.to_string()))
        .unwrap_or_default();
    assert!(message.contains(text), "panic message {message:?} does not contain {text:?}");
}

mod r422_the_grid_from_the_casters_seat {
    use super::*;

    #[test]
    fn r422_x_is_the_lane_less_one_on_both_sides_y_0_your_backrow_1_your_units_2_their_units_3_their_backrow() {
        assert_eq!(
            json_of(zone_of_point(P1, pt(0, 0))),
            json!({ "player": "p1", "row": "backrow", "lane": 1 })
        );
        assert_eq!(
            json_of(zone_of_point(P1, pt(4, 1))),
            json!({ "player": "p1", "row": "units", "lane": 5 })
        );
        assert_eq!(
            json_of(zone_of_point(P1, pt(2, 2))),
            json!({ "player": "p2", "row": "units", "lane": 3 })
        );
        assert_eq!(
            json_of(zone_of_point(P1, pt(4, 3))),
            json!({ "player": "p2", "row": "backrow", "lane": 5 })
        );
        // The other seat's grid is its own: its backrow is its row 0.
        assert_eq!(
            json_of(zone_of_point(P2, pt(0, 0))),
            json!({ "player": "p2", "row": "backrow", "lane": 1 })
        );
        assert_eq!(
            json_of(zone_of_point(P2, pt(1, 2))),
            json!({ "player": "p1", "row": "units", "lane": 2 })
        );
        for caster in [P1, P2] {
            for x in 0..PAPAYA_LANES {
                for y in 0..4 {
                    let back = point_of_zone(caster, zone_of_point(caster, pt(x, y)));
                    assert_eq!((back.x, back.y), (x, y));
                }
            }
        }
        panics_with(
            || {
                zone_of_point(P1, pt(5, 0));
            },
            "not a cell",
        );
        panics_with(
            || {
                zone_of_point(P1, pt(0, 4));
            },
            "not a cell",
        );
    }
}

mod r422_the_curve_in_exact_rationals {
    use super::*;

    #[test]
    fn r422_one_cell_is_a_constant_the_curve_crosses_its_whole_row() {
        assert_eq!(
            cells(cells_on_curve(&pts(&[(2, 1)]))),
            vec![(0, 1), (1, 1), (2, 1), (3, 1), (4, 1)]
        );
    }

    #[test]
    fn r422_two_cells_make_a_line_which_may_meet_a_third_cell_and_more() {
        let line = pts(&[(0, 0), (1, 1)]);
        // x = 4 would be row 4, off the grid.
        assert_eq!(
            cells(cells_on_curve(&line)),
            vec![(0, 0), (1, 1), (2, 2), (3, 3)]
        );
    }

    #[test]
    fn r422_between_lanes_the_curve_touches_nothing_y_x_2_meets_lanes_1_3_and_5_and_passes_2_and_4_by() {
        let half = pts(&[(0, 0), (2, 1)]);
        let at = curve_at(&half, 1);
        assert_eq!((at.num, at.den), (1, 2));
        let at = curve_at(&half, 3);
        assert_eq!((at.num, at.den), (3, 2));
        assert_eq!(
            cells(cells_on_curve(&half)),
            vec![(0, 0), (2, 1), (4, 2)]
        );
    }

    #[test]
    fn r422_the_lowest_degree_three_cells_in_a_line_make_that_line_not_a_parabola() {
        let collinear = pts(&[(0, 3), (1, 2), (3, 0)]);
        assert_eq!(
            cells(cells_on_curve(&collinear)),
            vec![(0, 3), (1, 2), (2, 1), (3, 0)]
        );
    }

    #[test]
    fn r422_three_cells_make_a_parabola_in_exact_fractions_which_may_meet_a_fourth_cell() {
        let arch = pts(&[(0, 0), (1, 1), (3, 0)]);
        let at = curve_at(&arch, 2);
        assert_eq!((at.num, at.den), (1, 1));
        // y = −x²/2 + 3x/2: at lane 5 it is −2, off the grid.
        let at = curve_at(&arch, 4);
        assert_eq!((at.num, at.den), (-2, 1));
        assert_eq!(
            cells(cells_on_curve(&arch)),
            vec![(0, 0), (1, 1), (2, 1), (3, 0)]
        );
    }

    #[test]
    fn r422_four_cells_fix_a_cubic_which_may_hit_a_cell_of_the_fifth_lane() {
        let cubic = pts(&[(0, 0), (1, 2), (2, 1), (3, 0)]);
        // y = x³/2 − 3x² + 9x/2, which is 2 at x = 4.
        let at = curve_at(&cubic, 4);
        assert_eq!((at.num, at.den), (2, 1));
        assert_eq!(
            cells(cells_on_curve(&cubic)),
            vec![(0, 0), (1, 2), (2, 1), (3, 0), (4, 2)]
        );
    }

    #[test]
    fn r422_a_cubic_that_leaves_the_grid_in_the_fifth_lane_meets_nothing_there() {
        let zigzag = pts(&[(0, 0), (1, 1), (2, 0), (3, 1)]);
        let at = curve_at(&zigzag, 4);
        assert_eq!((at.num, at.den), (8, 1));
        assert_eq!(cells(cells_on_curve(&zigzag)), cells(zigzag));
    }

    #[test]
    fn r422_the_cells_are_1_to_4_grid_cells_in_different_lanes() {
        panics_with(
            || {
                cells_on_curve(&[]);
            },
            "1 to 4 grid cells in different lanes",
        );
        panics_with(
            || {
                cells_on_curve(&pts(&[(1, 0), (1, 2)]));
            },
            "different lanes",
        );
        panics_with(
            || {
                cells_on_curve(&pts(&[(0, 4)]));
            },
            "grid cells",
        );
    }
}

mod r422_the_cells_are_asked_one_board_cell_prompt_at_a_time {
    use super::*;

    #[test]
    fn r422_the_first_prompt_offers_all_20_cells_and_no_done_and_legal_actions_lists_each_of_them() {
        let mut state = game("papaya-first");
        cast_now(&mut state, &curve().id, P1, false);
        let first = open_as(&state, PromptKind::Cell, P1);
        assert_eq!(first.options.len(), 20);
        assert!(first
            .options
            .iter()
            .all(|option| matches!(option.selection, Selection::Zone { .. })));
        assert_eq!(answer_count(&state, P1), 20);
        // The chooser's own rows from the hero outward, then the enemy's: the grid's rows in order.
        let rows: Vec<i32> = first
            .options
            .iter()
            .map(|option| point_of_zone(P1, zone_of(&option.selection)).y)
            .collect();
        let expected: Vec<i32> = [0, 1, 2, 3]
            .into_iter()
            .flat_map(|y| std::iter::repeat_n(y, 5))
            .collect();
        assert_eq!(rows, expected);
    }

    #[test]
    fn r422_each_later_prompt_offers_the_4_cells_of_every_unused_lane_and_done_never_more_than_21_answers() {
        let mut state = game("papaya-later");
        cast_now(&mut state, &curve().id, P1, false);
        let picks = [(1, 2), (3, 0), (0, 3)];
        for (at, &(x, y)) in picks.iter().enumerate() {
            answer_keys(&mut state, &[&key(x, y)]);
            let pending = open_as(&state, PromptKind::Cell, P1);
            let lanes_left = PAPAYA_LANES as usize - (at + 1);
            assert_eq!(pending.options.len(), 4 * lanes_left + 1);
            assert_eq!(
                pending.options.last().map(|option| option.selection.clone()),
                Some(Selection::None)
            );
            let used: Vec<i32> = picks[..=at].iter().map(|&(x, _)| x + 1).collect();
            assert!(!pending.options.iter().any(|option| matches!(
                &option.selection,
                Selection::Zone { lane, .. } if used.contains(lane)
            )));
            assert!(prompt_answers(&pending).len() <= 21);
            assert!(prompt_answers(&pending).len() < MAX_PROMPT_ANSWERS);
            assert_eq!(answer_count(&state, P1), 4 * lanes_left + 1);
        }
    }

    #[test]
    fn r422_the_fourth_cell_ends_the_picking_papaya_max_cells_and_draws_the_cubic() {
        let mut state = game("papaya-four");
        let board = full_board(&mut state);
        let enemy = &board[1];
        cast_now(&mut state, &curve().id, P1, false);
        for (x, y) in [(0, 0), (1, 2), (2, 1), (3, 0)] {
            answer_keys(&mut state, &[&key(x, y)]);
        }
        assert!(state.pending.is_none());
        // The fifth lane's cell (4, 2) is the enemy's unit in lane 5.
        assert!(exiled_ids(&state).contains(&enemy[4].id));
        assert_eq!(exiled_ids(&state).len(), 5);
    }

    #[test]
    fn r97_r177_the_other_seat_sees_only_that_a_prompt_is_open_the_options_are_cells_never_cards() {
        let mut state = game("papaya-hidden");
        let trap = put(&mut state, &snare().id, slot(P2, Row::Backrow, 2), Default::default());
        cast_now(&mut state, &curve().id, P1, false);
        assert_eq!(
            json_of(view_for(&state, P2).pending),
            json!({ "forYou": false, "pendingFor": "p1" })
        );
        let pending = open_as(&state, PromptKind::Cell, P1);
        assert!(!serde_json::to_string(&pending.options)
            .expect("options serialise")
            .contains(&trap.id));
        assert!(!serde_json::to_string(&view_for(&state, P1).pending)
            .expect("a view serialises")
            .contains(&trap.id));
        for opened in events_json(&view_for(&state, P2).events, GameEventType::PromptOpened) {
            let mut keys: Vec<String> = opened
                .as_object()
                .map(|fields| fields.keys().cloned().collect())
                .unwrap_or_default();
            keys.sort();
            assert_eq!(keys, vec!["choiceId", "kind", "player", "type"]);
        }
    }
}

mod r422_every_card_on_the_curve_is_exiled {
    use super::*;

    #[test]
    fn r422_one_cells_constant_exiles_its_whole_row_and_nothing_else() {
        let mut state = game("papaya-row");
        let board = full_board(&mut state);
        let (own, enemy) = (&board[0], &board[1]);
        draw_curve(&mut state, &[(3, 2)], false);
        assert_eq!(sorted(exiled_ids(&state)), sorted(ids_of(&enemy[..5])));
        assert!(own
            .iter()
            .all(|card| zone_name(&state, &card.id) == Some(ZoneName::Field)));
    }

    #[test]
    fn r422_a_line_exiles_the_third_cell_it_meets_and_nothing_between_lanes() {
        let mut state = game("papaya-line");
        let board = full_board(&mut state);
        let (own, enemy) = (&board[0], &board[1]);
        // y = x/2: (0, 0) your backrow lane 1, (2, 1) your units lane 3, (4, 2) their units lane 5.
        draw_curve(&mut state, &[(0, 0), (2, 1)], false);
        assert_eq!(
            sorted(exiled_ids(&state)),
            sorted(vec![own[5].id.clone(), own[2].id.clone(), enemy[4].id.clone()])
        );
    }

    #[test]
    fn r422_a_face_down_trap_on_the_curve_is_exiled_and_the_exile_names_it_openly_to_both_players() {
        let mut state = game("papaya-trap");
        let trap = put(&mut state, &snare().id, slot(P2, Row::Backrow, 4), Default::default());
        assert_ne!(trap.face_up, Some(true));
        cast_now(&mut state, &curve().id, P1, false);
        answer_keys(&mut state, &[&key(0, 3)]);
        let answered = answer_keys(&mut state, &["none"]);
        assert_eq!(ids_of(&state.players.p2.exile), vec![trap.id.clone()]);
        let exiled: Vec<Value> = events_json(&answered.events, GameEventType::Exiled)
            .iter()
            .map(|event| event["instanceId"].clone())
            .collect();
        assert_eq!(exiled, vec![json!(trap.id)]);
        state.applied = vec![AppliedAction {
            nonce: "pp".into(),
            events: answered.events.clone(),
        }];
        for viewer in [P1, P2] {
            let seen: Vec<Value> = events_json(&view_for(&state, viewer).events, GameEventType::Exiled)
                .iter()
                .map(|event| Value::Array(vec![event["instanceId"].clone(), event["defId"].clone()]))
                .collect();
            assert_eq!(seen, vec![json!([trap.id, snare().id])]);
        }
    }

    #[test]
    fn r11_a_unit_token_on_the_curve_ceases_to_exist_instead_of_reaching_the_exile_pile() {
        let mut state = game("papaya-token");
        let made = put(&mut state, &token().id, slot(P2, Row::Units, 1), Default::default());
        draw_curve(&mut state, &[(0, 2)], false);
        // TS: `made.zone.z` is "gone"; a card that ceased to exist is in no zone of the state.
        assert!(find_instance(&state, &made.id).is_none());
        assert!(state.players.p2.exile.is_empty());
    }

    #[test]
    fn s3_2_r13_the_top_of_a_stack_pile_is_exiled_and_the_card_beneath_resumes_not_exiled_by_the_same_curve() {
        let mut state = game("papaya-stack");
        let beneath = put(&mut state, &body().id, slot(P2, Row::Units, 2), Default::default());
        let mut top = new_instance(&mut state, &body().id, P2, Zone::Hand { player: P2 });
        assert!(place_on_field(
            &mut state,
            &mut top,
            slot(P2, Row::Units, 2),
            PlaceOnFieldOptions { stack: Some(true) }
        ));
        draw_curve(&mut state, &[(1, 2)], false);
        assert_eq!(zone_name(&state, &top.id), Some(ZoneName::Exile));
        assert_eq!(
            card_at(&state, slot(P2, Row::Units, 2)).map(|card| card.id.clone()),
            Some(beneath.id.clone())
        );
        assert_eq!(zone_name(&state, &beneath.id), Some(ZoneName::Field));
    }

    #[test]
    fn r422_the_cells_are_offered_empty_or_not_and_a_curve_over_empty_cells_exiles_nothing() {
        let mut state = game("papaya-empty");
        draw_curve(&mut state, &[(0, 1), (4, 3)], false);
        assert!(exiled_ids(&state).is_empty());
    }

    #[test]
    fn r422_radiant_exiles_only_the_enemys_cards_on_the_curve_rows_2_and_3() {
        let mut state = game("papaya-radiant");
        let board = full_board(&mut state);
        let (own, enemy) = (&board[0], &board[1]);
        // y = x: (0, 0) and (1, 1) are yours, (2, 2) and (3, 3) the enemy's.
        draw_curve(&mut state, &[(0, 0), (1, 1)], true);
        assert_eq!(
            sorted(exiled_ids(&state)),
            sorted(vec![enemy[2].id.clone(), enemy[8].id.clone()])
        );
        assert!(own
            .iter()
            .all(|card| zone_name(&state, &card.id) == Some(ZoneName::Field)));
    }

    #[test]
    fn r422_cards_on_curve_reads_the_board_once_the_tops_of_the_piles_in_lane_order() {
        let mut state = game("papaya-read");
        let board = full_board(&mut state);
        let own = &board[0];
        // TS passes `{ state, controller: "p1" }`, the two context fields it reads.
        let ids = cards_on_curve(&state, P1, &pts(&[(0, 1)]), false);
        assert_eq!(ids, ids_of(&own[..5]));
        assert!(cards_on_curve(&state, P1, &pts(&[(0, 1)]), true).is_empty());
        assert!(cards_on_curve(&state, P1, &[], false).is_empty());
    }
}

mod r452_r113_9_3_random_casts_pauses_and_replays {
    use super::*;

    #[test]
    fn r452_a_random_cast_answers_every_cell_prompt_at_random_nothing_pauses_and_its_curve_exiles_what_lies_on_it() {
        for seed in 1..=6 {
            let mut state = game(&format!("papaya-random-{seed}"));
            let lanes: IndexMap<String, i32> = full_board(&mut state)
                .iter()
                .flat_map(|cards| {
                    cards
                        .iter()
                        .enumerate()
                        .map(|(at, card)| (card.id.clone(), (at % 5) as i32 + 1))
                })
                .collect();
            let (cursor, events) = {
                let mut sink = sink_for(&mut state);
                {
                    let mut ctx = make_context(
                        &mut sink,
                        None,
                        HookOptions {
                            controller: Some(P1),
                            ..Default::default()
                        },
                    );
                    apply_effects(
                        &[cast_new(json_as(json!({ "def": curve().id, "random": true })))],
                        &mut ctx,
                    );
                }
                settle(&mut sink, SettleOptions::default());
                (sink.rng.cursor(), sink.events.clone())
            };
            state.rng_cursor = cursor;
            assert!(state.pending.is_none());
            assert!(events_of_type(&events, GameEventType::PromptOpened).is_empty());
            let exiled = events_json(&events, GameEventType::Exiled);
            // A full board: the curve passes through at least the cell it was drawn through.
            assert!(!exiled.is_empty());
            assert!(exiled.len() <= PAPAYA_LANES as usize);
            // A curve is a function of the lane: never two cards of one lane.
            let hit: Vec<i32> = exiled
                .iter()
                .map(|event| {
                    *must(
                        lanes.get(event["instanceId"].as_str().unwrap_or_default()),
                        "an exiled card's lane",
                    )
                })
                .collect();
            assert_eq!(hit.iter().collect::<IndexSet<_>>().len(), hit.len());
        }
    }

    #[test]
    fn r113_a_pause_between_cells_survives_json_and_the_copy_answers_to_the_very_same_state() {
        let mut state = game("papaya-json");
        full_board(&mut state);
        cast_now(&mut state, &curve().id, P1, false);
        answer_keys(&mut state, &[&key(0, 3)]);
        answer_keys(&mut state, &[&key(2, 2)]);
        let mut copy = round_trip(&state);
        assert_eq!(copy, state);
        for target in [&mut state, &mut copy] {
            answer_keys(target, &[&key(4, 1)]);
            answer_keys(target, &["none"]);
        }
        assert!(copy.pending.is_none());
        assert_eq!(hash_state(&copy), hash_state(&state));
        assert_eq!(exiled_ids(&copy).len(), exiled_ids(&state).len());
    }

    #[test]
    fn s9_2_a_game_that_draws_a_curve_replays_from_its_log_to_the_same_hash() {
        setup_catalog();
        register_prompt_fixtures();
        register_papaya_fixtures();
        let seed = "papaya-replay";
        let mut first_deck = vanilla_deck(DECK_SIZE - 1, 1);
        first_deck.push(curve_quickdraw().id);
        let decks = (first_deck, vanilla_deck(DECK_SIZE, 21));
        let mut log: Vec<Action> = Vec::new();
        let mut n = 0;
        let mut act = |current: &GameState, input: Value| -> GameState {
            n += 1;
            let mut action = input;
            action["nonce"] = json!(format!("pp{n}"));
            let action: Action = json_as(action);
            let result = reduce(current, &action);
            if let Some(error) = result.error {
                panic!("{error}");
            }
            log.push(action);
            result.state
        };
        let mut state = begin_game(&create_game(&CreateGameArgs {
            seed: seed.into(),
            decks: decks.clone(),
            ..Default::default()
        }))
        .state;
        for player in [P1, P2] {
            let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
            state = act(
                &state,
                json!({ "type": "mulligan", "playerId": player, "keep": keep }),
            );
        }
        let papaya = must(
            state.players.p1.hand.iter().find(|card| card.def_id == curve_quickdraw().id),
            "the quickdraw curve",
        )
        .clone();
        state = act(
            &state,
            json!({ "type": "play", "playerId": "p1", "instanceId": papaya.id }),
        );
        for selection in [
            json!({ "pick": "zone", "player": "p1", "row": "units", "lane": 1 }),
            json!({ "pick": "zone", "player": "p2", "row": "units", "lane": 3 }),
            json!({ "pick": "none" }),
        ] {
            let choice = must(state.pending.as_ref(), "a cell prompt").id.clone();
            state = act(
                &state,
                json!({ "type": "answer", "playerId": "p1", "choiceId": choice, "selection": [selection] }),
            );
        }
        assert!(state.pending.is_none());
        let folded = fold(&FoldArgs {
            seed: seed.into(),
            decks,
            log,
            ..FoldArgs::default()
        });
        assert!(folded.errors.is_empty());
        assert_eq!(hash_state(&folded.state), hash_state(&state));
    }
}
