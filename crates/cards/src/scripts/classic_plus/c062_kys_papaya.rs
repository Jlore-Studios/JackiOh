//! C+ #62 KY's Papaya (SPEC §8.7 row 62, R422; BUILD M9 row C+ 62). (1) Spell, Fruit, KY, Epic.
//!   Base:    "Draw a curve y = ax³ + bx² + cx + d across the board, … Exile every card on the curve."
//!   Radiant: "… Exile every enemy card on the curve."
//!
//! The curve is the engine's (E32): the player picks 1 to 4 cells in different
//! lanes, one board-cell prompt at a time, the curve is the lowest-degree polynomial through them in
//! exact rationals, and the top card at every cell on it is exiled. The picks are board cells, not
//! cards, so they keep the cell chrome: the Discover rule is for card selections, and twenty cells
//! are past its five-option limit either way. The running face decides the rows: `papayaAnswered`
//! reads `ctx.radiant` and keeps to the enemy's rows 2 and 3 on the Radiant one.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-062";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| subsystems::papaya_begin())),
        resume: IndexMap::from([(subsystems::PAPAYA_STEP, hook(subsystems::papaya_answered))]),
        ..Script::default()
    };
    // The same script: the Radiant face's "every enemy card" is the running face, read by the subsystem.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C+ #62 KY's Papaya — SPEC §8.7 row 62, R422, BUILD M9 row C+ 62: one board-cell prompt at a time (20
// cells, then the unused lanes' cells and "done"), x the lane and y the row from the caster's seat, the
// lowest-degree curve in exact rationals, every card on it exiled (a pile's top; face-down openly; tokens
// cease; nothing between lanes); cells never cards (R177); a random cast answers itself (R452); radiant only enemy cards.
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::{cast_new, lock};
    use jackioh_engine::testkit::*;

    const PAPAYA: &str = "classicplus-062";
    const UNIT: &str = "core-008"; // Mr. Vanilla, a plain 4/4.
    const FIELD: &str = "core-006"; // Mana Well, a Field Spell.
    const TRAP: &str = "core-041"; // Sheepish, set face-down.
    const TOKEN: &str = "core-t-rush";
    const FILLER: &str = "core-005";

    use crate::scenario;

    /// A card on every cell: Mr. Vanilla in each unit zone, a Mana Well in each backrow zone.
    fn full_side() -> Value {
        json!({
            "hand": [FILLER],
            "field": [UNIT, UNIT, UNIT, UNIT, UNIT],
            "backrow": [FIELD, FIELD, FIELD, FIELD, FIELD],
        })
    }

    fn full(radiant: bool) -> Scenario {
        let mut p1 = full_side();
        p1["hand"] = json!([{ "def": PAPAYA, "radiant": radiant }, FILLER]);
        scenario(json!({ "p1": p1, "p2": full_side() }))
    }

    /// The option key of the cell (x, y) from `caster`'s seat: y 0 their backrow, 1 their units, 2 the
    /// other seat's units, 3 the other seat's backrow.
    fn cell_for(x: i32, y: usize, caster: PlayerId) -> String {
        let other = caster.opponent();
        let (player, row) = [(caster, "backrow"), (caster, "units"), (other, "units"), (other, "backrow")][y];
        format!("zone:{player}:{row}:{}", x + 1)
    }

    fn cell(x: i32, y: usize) -> String {
        cell_for(x, y, PlayerId::P1)
    }

    fn curve(s: &mut Scenario, cells: &[(i32, usize)]) {
        s.play(PAPAYA, json!({}));
        for &(x, y) in cells {
            s.answer(json!(cell(x, y)));
        }
        if cells.len() < 4 {
            s.answer(json!("none"));
        }
        assert!(s.state().pending.is_none());
    }

    fn exiled(s: &Scenario) -> Vec<String> {
        let mut ids: Vec<String> = s.pile(PlayerId::P1, "exile").into_iter().map(|card| card.id).collect();
        ids.extend(s.pile(PlayerId::P2, "exile").into_iter().map(|card| card.id));
        ids
    }

    fn at(s: &Scenario, player: PlayerId, row: &str, lane: i32) -> String {
        let card = if row == "units" { s.unit(player, lane) } else { s.backrow(player, lane) };
        match card {
            Some(card) => card.id,
            None => panic!("nothing in {player} {row} {lane}"),
        }
    }

    fn sorted(mut ids: Vec<String>) -> Vec<String> {
        ids.sort();
        ids
    }

    /// Runs `effects` on a hand-built sink at the state's rng cursor, settling when asked; returns its events.
    fn by_hand(
        s: &mut Scenario,
        effects: Vec<Effect>,
        self_: Option<CardInstance>,
        controller: PlayerId,
        then_settle: bool,
    ) -> Vec<GameEvent> {
        let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            {
                let mut ctx = make_context(
                    &mut sink,
                    self_.as_ref(),
                    HookOptions { controller: Some(controller), ..HookOptions::default() },
                );
                apply_effects(&effects, &mut ctx);
            }
            if then_settle {
                settle(&mut sink, SettleOptions::default());
            }
        }
        s.state_mut().rng_cursor = rng.cursor();
        events
    }

    #[test]
    fn is_one_script_on_both_faces_the_subsystem_reads_the_running_face() {
        crate::register_all();
        assert_eq!(crate::card_def(super::ID).id, PAPAYA);
        let scripts = super::script();
        assert!(std::sync::Arc::ptr_eq(
            scripts.radiant.cry.as_ref().expect("a Cry"),
            scripts.base.cry.as_ref().expect("a Cry"),
        ));
        assert!(!scripts.base.resume.is_empty());
    }

    mod base {
        use super::*;

        #[test]
        fn r422_the_cells_come_one_board_cell_prompt_at_a_time_20_cells_then_the_unused_lanes_cells_and_done_never_more_than_21_answers_every_one_in_legal_actions() {
            let mut s = full(false);
            s.play(PAPAYA, json!({}));
            let mut sizes: Vec<usize> = Vec::new();
            for (x, y) in [(2, 1), (0, 3), (4, 0)] {
                let pending = s.state().pending.clone().unwrap_or_else(|| panic!("a cell prompt"));
                assert_eq!(pending.kind, PromptKind::Cell);
                let answers = legal_actions(s.state(), PlayerId::P1)
                    .into_iter()
                    .filter(|action| matches!(action, ActionBody::Answer { .. }))
                    .count();
                assert_eq!(answers, pending.options.len());
                sizes.push(pending.options.len());
                s.answer(json!(cell(x, y)));
            }
            sizes.push(s.state().pending.as_ref().map_or(0, |pending| pending.options.len()));
            assert_eq!(sizes, vec![20, 17, 13, 9]);
            assert!(sizes.iter().copied().max().unwrap_or(0) <= 21);
            let pending = s.state().pending.as_ref();
            assert_eq!(
                pending.and_then(|pending| pending.options.last()).map(|option| option.selection.clone()),
                Some(Selection::None)
            );
            assert_eq!(
                pending.map(|pending| {
                    pending.options.iter().any(|option| option.key.ends_with(":3") || option.key.ends_with(":1"))
                }),
                Some(false)
            );
        }

        #[test]
        fn r422_the_cells_are_offered_empty_occupied_or_locked_alike() {
            let mut s = scenario(json!({ "p1": { "hand": [PAPAYA, FILLER] }, "p2": { "hand": [FILLER] } }));
            by_hand(
                &mut s,
                vec![lock(json_as(json!({ "zone": { "of": "lane", "player": "enemy", "row": "units", "lane": 2 } })))],
                None,
                PlayerId::P1,
                false,
            );
            s.play(PAPAYA, json!({}));
            let keys: Vec<String> = s
                .state()
                .pending
                .as_ref()
                .map(|pending| pending.options.iter().map(|option| option.key.clone()).collect())
                .unwrap_or_default();
            assert!(keys.contains(&"zone:p2:units:2".to_string()));
            assert_eq!(s.state().pending.as_ref().map(|pending| pending.options.len()), Some(20));
        }

        #[test]
        fn r422_x_is_the_lane_from_your_own_lane_1_and_y_the_row_from_your_own_backrow_on_either_seat() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [FILLER], "field": [UNIT, UNIT], "backrow": [FIELD] },
                "p2": { "hand": [PAPAYA, FILLER], "field": [UNIT] },
            }));
            s.play(PAPAYA, json!({}));
            // p2's y = 2 row is p1's unit row: the constant through (1, 2) exiles p1's units in lanes 1 and 2.
            let lanes = vec![at(&s, PlayerId::P1, "units", 1), at(&s, PlayerId::P1, "units", 2)];
            s.answer(json!(cell_for(1, 2, PlayerId::P2)));
            s.answer(json!("none"));
            assert_eq!(sorted(exiled(&s)), sorted(lanes));
            assert!(s.unit(PlayerId::P2, 1).is_some());
            assert!(s.backrow(PlayerId::P1, 1).is_some());
        }

        #[test]
        fn r422_one_cell_is_a_constant_its_whole_row_is_exiled_and_nothing_else() {
            let mut s = full(false);
            let row: Vec<String> = (1..=5).map(|lane| at(&s, PlayerId::P2, "backrow", lane)).collect();
            curve(&mut s, &[(2, 3)]);
            assert_eq!(sorted(exiled(&s)), sorted(row));
            assert!(s.state().players.p1.exile.is_empty());
        }

        #[test]
        fn r422_two_cells_make_a_line_which_meets_a_third_cell_y_x_through_lanes_1_to_4() {
            let mut s = full(false);
            let line = vec![
                at(&s, PlayerId::P1, "backrow", 1),
                at(&s, PlayerId::P1, "units", 2),
                at(&s, PlayerId::P2, "units", 3),
                at(&s, PlayerId::P2, "backrow", 4),
            ];
            curve(&mut s, &[(0, 0), (1, 1)]);
            assert_eq!(sorted(exiled(&s)), sorted(line));
            // Lane 5 would be row 4, off the board.
            assert!(s.unit(PlayerId::P1, 5).is_some());
        }

        #[test]
        fn r422_between_lanes_the_curve_touches_nothing_y_x_2_exiles_lanes_1_3_and_5_only() {
            let mut s = full(false);
            let hits = vec![
                at(&s, PlayerId::P1, "backrow", 1),
                at(&s, PlayerId::P1, "units", 3),
                at(&s, PlayerId::P2, "units", 5),
            ];
            curve(&mut s, &[(0, 0), (2, 1)]);
            assert_eq!(sorted(exiled(&s)), sorted(hits));
        }

        #[test]
        fn r422_three_cells_in_a_line_are_that_line_the_lowest_degree_through_them() {
            let mut s = full(false);
            // (0, 3), (1, 2), (3, 0) lie on y = 3 − x, which also meets (2, 1); a parabola would not.
            let line = vec![
                at(&s, PlayerId::P2, "backrow", 1),
                at(&s, PlayerId::P2, "units", 2),
                at(&s, PlayerId::P1, "units", 3),
                at(&s, PlayerId::P1, "backrow", 4),
            ];
            curve(&mut s, &[(0, 3), (1, 2), (3, 0)]);
            assert_eq!(sorted(exiled(&s)), sorted(line));
        }

        #[test]
        fn r422_four_cells_fix_the_cubic_the_fourth_ends_the_picking_and_it_may_hit_a_cell_of_the_fifth_lane() {
            let mut s = full(false);
            // y = x³/2 − 3x² + 9x/2 through (0, 0), (1, 2), (2, 1), (3, 0) is 2 at x = 4: p2's unit in lane 5.
            let hits = vec![
                at(&s, PlayerId::P1, "backrow", 1),
                at(&s, PlayerId::P2, "units", 2),
                at(&s, PlayerId::P1, "units", 3),
                at(&s, PlayerId::P1, "backrow", 4),
                at(&s, PlayerId::P2, "units", 5),
            ];
            curve(&mut s, &[(0, 0), (1, 2), (2, 1), (3, 0)]);
            assert_eq!(sorted(exiled(&s)), sorted(hits));
            s.expect_events(json!(["cardPlayed", "promptOpened", "exiled", "cardResolved"]));
        }

        #[test]
        fn r422_face_down_cards_on_the_curve_are_exiled_and_the_exile_names_them_openly() {
            let mut s = scenario(json!({
                "p1": { "hand": [PAPAYA, FILLER] },
                "p2": { "hand": [FILLER], "backrow": [TRAP, TRAP] },
            }));
            let traps = vec![at(&s, PlayerId::P2, "backrow", 1), at(&s, PlayerId::P2, "backrow", 2)];
            assert_ne!(s.backrow(PlayerId::P2, 1).and_then(|card| card.face_up), Some(true));
            curve(&mut s, &[(0, 3)]);
            let pile: Vec<String> = s.pile(PlayerId::P2, "exile").into_iter().map(|card| card.id).collect();
            assert_eq!(sorted(pile), sorted(traps.clone()));
            for viewer in [PlayerId::P1, PlayerId::P2] {
                let shown: Vec<(String, String)> = s
                    .view(viewer)
                    .events
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::Exiled { instance_id, def_id, .. } => Some((instance_id.clone(), def_id.clone())),
                        _ => None,
                    })
                    .collect();
                assert_eq!(sorted(shown.iter().map(|(id, _)| id.clone()).collect()), sorted(traps.clone()));
                assert!(shown.iter().all(|(_, def_id)| def_id == TRAP));
            }
        }

        #[test]
        fn s6_3_exile_is_no_destroy_an_indestructible_unit_and_a_reborn_unit_on_the_curve_are_exiled_and_nothing_comes_back() {
            let mut s = scenario(json!({
                "p1": { "hand": [PAPAYA, FILLER] },
                "p2": { "hand": [FILLER], "field": ["core-066", "core-003"] },
            }));
            let rock = s.unit(PlayerId::P2, 1).expect("The Rock");
            let defender = s.unit(PlayerId::P2, 2).expect("a Reborn unit");
            curve(&mut s, &[(0, 2)]);
            s.expect_in_zone(&rock, "exile").expect_in_zone(&defender, "exile");
            assert!(s.unit(PlayerId::P2, 1).is_none());
            assert!(s.unit(PlayerId::P2, 2).is_none());
        }

        #[test]
        fn r11_tokens_on_the_curve_cease_to_exist() {
            let mut s = scenario(json!({
                "p1": { "hand": [PAPAYA, FILLER] },
                "p2": { "hand": [FILLER], "field": [TOKEN, UNIT] },
            }));
            let token = s.unit(PlayerId::P2, 1).expect("a token");
            let unit = s.unit(PlayerId::P2, 2).expect("a unit");
            curve(&mut s, &[(0, 2)]);
            s.expect_in_zone(&token, "gone").expect_in_zone(&unit, "exile");
        }

        #[test]
        fn s3_2_r13_the_top_of_a_stack_pile_is_exiled_and_the_card_beneath_resumes_not_exiled_by_the_same_curve() {
            let mut s = scenario(json!({
                "p1": { "hand": [PAPAYA, FILLER] },
                "p2": { "hand": [FILLER], "field": [{ "def": "core-043", "lane": 1 }, { "def": "core-092", "stack": true }] },
            }));
            let top = s.unit(PlayerId::P2, 1).expect("the top of the pile");
            curve(&mut s, &[(0, 2)]);
            s.expect_in_zone(&top, "exile");
            assert_eq!(s.unit(PlayerId::P2, 1).map(|card| card.def_id), Some("core-043".to_string()));
        }

        #[test]
        fn r422_a_curve_over_empty_cells_exiles_nothing_and_the_spell_still_resolves_to_the_graveyard() {
            let mut s = scenario(json!({ "p1": { "hand": [PAPAYA, FILLER] }, "p2": { "hand": [FILLER] } }));
            curve(&mut s, &[(0, 1), (4, 2)]);
            assert_eq!(exiled(&s), Vec::<String>::new());
            s.expect_in_zone(PAPAYA, "graveyard");
        }

        #[test]
        fn r177_the_prompts_offer_cells_never_cards_the_other_seat_sees_only_that_a_prompt_is_open_then_the_exiles() {
            let mut s = scenario(json!({
                "p1": { "hand": [PAPAYA, FILLER] },
                "p2": { "hand": [FILLER], "backrow": [TRAP] },
            }));
            let trap = at(&s, PlayerId::P2, "backrow", 1);
            s.play(PAPAYA, json!({}));
            let elsewhere = json!({ "forYou": false, "pendingFor": "p1" });
            assert_eq!(serde_json::to_value(&s.view(PlayerId::P2).pending).unwrap(), elsewhere);
            assert_eq!(
                s.state().pending.as_ref().map(|pending| {
                    pending.options.iter().all(|option| matches!(option.selection, Selection::Zone { .. }))
                }),
                Some(true)
            );
            assert!(!serde_json::to_string(&s.view(PlayerId::P1).pending).unwrap().contains(&trap));
            assert!(!serde_json::to_string(&s.view(PlayerId::P2).pending).unwrap().contains("zone"));
            s.answer(json!(cell(0, 3)));
            assert_eq!(serde_json::to_value(&s.view(PlayerId::P2).pending).unwrap(), elsewhere);
            s.answer(json!("none"));
            assert!(
                s.view(PlayerId::P2)
                    .events
                    .iter()
                    .any(|event| matches!(event, GameEvent::Exiled { instance_id, .. } if *instance_id == trap))
            );
        }

        #[test]
        fn r452_a_random_cast_answers_the_same_prompts_at_random_nothing_pauses_and_the_curves_cards_are_exiled() {
            for seed in 1..=4 {
                let mut s = scenario(json!({
                    "seed": format!("papaya-random-{seed}"),
                    "p1": full_side(),
                    "p2": full_side(),
                }));
                let mut lane_of: IndexMap<String, i32> = IndexMap::new();
                for player in [PlayerId::P1, PlayerId::P2] {
                    for lane in [1, 2, 3, 4, 5] {
                        lane_of.insert(at(&s, player, "units", lane), lane);
                        lane_of.insert(at(&s, player, "backrow", lane), lane);
                    }
                }
                let events = by_hand(
                    &mut s,
                    vec![cast_new(json_as(json!({ "def": PAPAYA, "random": true })))],
                    None,
                    PlayerId::P1,
                    true,
                );
                assert!(s.state().pending.is_none());
                assert!(!events.iter().any(|event| matches!(event, GameEvent::PromptOpened { .. })));
                // A full board: at least the cell the curve was drawn through, and never two cards of one lane.
                let lanes: Vec<Option<i32>> = events
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::Exiled { instance_id, .. } => Some(lane_of.get(instance_id).copied()),
                        _ => None,
                    })
                    .collect();
                assert!(!lanes.is_empty());
                assert_eq!(lanes.iter().collect::<IndexSet<_>>().len(), lanes.len());
                assert!(lanes.iter().all(|lane| lane.is_some()));
            }
        }

        #[test]
        fn r113_paused_mid_answer_the_cells_so_far_survive_json_and_the_copy_answers_to_the_same_hash() {
            let mut s = full(false);
            s.play(PAPAYA, json!({}));
            s.answer(json!(cell(0, 0)));
            s.answer(json!(cell(1, 1)));
            let paused = s.state().clone();
            let copy: GameState = serde_json::from_value(serde_json::to_value(&paused).unwrap()).unwrap();
            assert_eq!(copy, paused);
            fn answer(state: &GameState, body: ActionBody, nonce: &str) -> GameState {
                let result = reduce(state, &Action::new(body, PlayerId::P1, nonce));
                if let Some(error) = result.error {
                    panic!("{error}");
                }
                result.state
            }
            let mut live = paused;
            let mut revived = copy;
            for (index, key) in [cell(3, 2), "none".to_string()].iter().enumerate() {
                let pending = live.pending.clone().unwrap_or_else(|| panic!("a prompt"));
                let option = pending
                    .options
                    .iter()
                    .find(|held| held.key == *key)
                    .cloned()
                    .unwrap_or_else(|| panic!("option {key}"));
                let body = ActionBody::Answer {
                    choice_id: pending.id.clone(),
                    selection: vec![option.selection.clone()],
                };
                live = answer(&live, body.clone(), &format!("rt-{index}"));
                revived = answer(&revived, body, &format!("rt-{index}"));
            }
            assert!(live.pending.is_none());
            assert_eq!(hash_state(&revived), hash_state(&live));
        }

        #[test]
        fn s9_2_a_game_that_draws_a_curve_replays_from_its_log_to_the_same_hash() {
            crate::register_all();
            let units = ["core-001", "core-002", "core-004", "core-007", "core-008", "core-009", "core-011", "core-012"];
            let more = ["core-013", "core-015", "core-019", "core-020", "core-022", "core-025", "core-030", "core-032"];
            let mut deck: Vec<String> = units.iter().chain(more.iter()).map(|id| id.to_string()).collect();
            deck.extend(["core-037", "core-043", "core-045", PAPAYA].iter().map(|id| id.to_string()));
            let mut found: Option<(String, GameState)> = None;
            let mut n = 0;
            while n < 300 && found.is_none() {
                let seed = format!("papaya-replay-{n}");
                n += 1;
                let begun = begin_game(&create_game(&CreateGameOptions {
                    seed: seed.clone(),
                    decks: (deck.clone(), deck.clone()),
                    ..CreateGameOptions::default()
                }))
                .state;
                if begun.players.p1.hand.iter().any(|card| card.def_id == PAPAYA) && begun.pending.is_none() {
                    found = Some((seed, begun));
                }
            }
            let (seed, mut state) = found.unwrap_or_else(|| panic!("no seed deals p1 the Papaya"));
            let mut log: Vec<Action> = Vec::new();
            let mut count: u32 = 0;
            fn act(state: &GameState, body: ActionBody, player: PlayerId, count: &mut u32, log: &mut Vec<Action>) -> GameState {
                *count += 1;
                let action = Action::new(body, player, format!("pr{count}"));
                let result = reduce(state, &action);
                if let Some(error) = result.error {
                    panic!("{error}");
                }
                log.push(action);
                result.state
            }
            for player in [PlayerId::P1, PlayerId::P2] {
                let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
                state = act(&state, ActionBody::Mulligan { keep }, player, &mut count, &mut log);
            }
            let papaya = state
                .players
                .p1
                .hand
                .iter()
                .find(|card| card.def_id == PAPAYA)
                .map(|card| card.id.clone())
                .unwrap_or_default();
            state = act(
                &state,
                json_as(json!({ "type": "play", "instanceId": papaya })),
                PlayerId::P1,
                &mut count,
                &mut log,
            );
            for key in [cell(0, 3), cell(2, 1), "none".to_string()] {
                let pending = state.pending.clone();
                let option = pending
                    .as_ref()
                    .and_then(|pending| pending.options.iter().find(|held| held.key == key).cloned());
                let (Some(pending), Some(option)) = (pending, option) else {
                    panic!("option {key}");
                };
                state = act(
                    &state,
                    ActionBody::Answer { choice_id: pending.id, selection: vec![option.selection] },
                    PlayerId::P1,
                    &mut count,
                    &mut log,
                );
            }
            assert!(state.pending.is_none());
            let replayed = fold(&json_as(json!({ "seed": seed, "decks": [deck, deck], "log": log })));
            assert!(replayed.errors.is_empty());
            assert_eq!(hash_state(&replayed.state), hash_state(&state));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r422_only_enemy_cards_on_the_curve_are_exiled_y_x_takes_their_2_2_and_3_3_not_your_0_0_and_1_1() {
            let mut s = full(true);
            let enemy = vec![at(&s, PlayerId::P2, "units", 3), at(&s, PlayerId::P2, "backrow", 4)];
            curve(&mut s, &[(0, 0), (1, 1)]);
            assert_eq!(sorted(exiled(&s)), sorted(enemy));
            assert!(s.state().players.p1.exile.is_empty());
        }

        #[test]
        fn r422_a_constant_through_your_own_row_exiles_nothing_through_theirs_the_whole_row() {
            let mut own = full(true);
            curve(&mut own, &[(1, 1)]);
            assert_eq!(exiled(&own), Vec::<String>::new());
            let mut theirs = full(true);
            let row: Vec<String> = (1..=5).map(|lane| at(&theirs, PlayerId::P2, "units", lane)).collect();
            curve(&mut theirs, &[(1, 2)]);
            assert_eq!(sorted(exiled(&theirs)), sorted(row));
        }

        #[test]
        fn r422_the_radiant_face_asks_the_same_prompts() {
            let mut s = full(true);
            s.play(PAPAYA, json!({}));
            assert_eq!(s.state().pending.as_ref().map(|pending| pending.options.len()), Some(20));
            s.answer(json!(cell(0, 0)));
            assert_eq!(s.state().pending.as_ref().map(|pending| pending.options.len()), Some(17));
        }
    }
}
