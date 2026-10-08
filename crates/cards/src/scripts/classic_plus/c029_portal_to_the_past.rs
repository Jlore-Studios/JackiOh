//! C+ #29 Portal to the Past (SPEC §8.7 row 29, R417, R564). (3) Spell.
//!   Base:    Discover a card from the board your last game ended with; it arrives costing (0).
//!   Radiant: {cards} different random cards of that board straight to your hand, each costing (0).
//! The board is the caster's last board, a setup input frozen into the match (B5 E30,
//! `subsystems/lastBoards`); an empty one (hotseat, a first game) gives nothing (R129).

use jackioh_engine::effects::{add_from_last_board, add_random_from_last_board, discover_from_last_board};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-029";

/// The step the Discover's answer re-enters (§10.6).
const PICKED: &str = "picked";

/// "It costs (0)", "Each costs (0)": the declared number `setCost` (R386), less being better.
pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| vec![discover_from_last_board(json_as(json!({ "step": PICKED })))])),
        resume: IndexMap::from([(
            PICKED,
            hook(|ctx| vec![add_from_last_board(json_as(json!({ "costOverride": param(&*ctx, "setCost") })))]),
        )]),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(|ctx| {
            vec![add_random_from_last_board(json_as(json!({
                "count": param(&*ctx, "cards"),
                "costOverride": param(&*ctx, "setCost"),
            })))]
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// C+ #29 Portal to the Past — SPEC §8.7 row 29, R417, R564; BUILD M9 Classic+ row C+ 29. The last
// board is a setup input frozen into the match (B5 E30); `scenario({ lastBoards })` hands it to
// `createGame` as the server does. The engine's own proofs (the reader, the freeze, the fold) are in
// packages/engine/test/lastBoards.test.ts.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PORTAL: &str = "classicplus-029";
    const FILLER: &str = "core-005"; // keeps a hand from auto-ending the turn (§2.5)
    const MENACE: &str = "core-019";
    const TOKEN: &str = "core-t-rush"; // a unit token (R11)
    const FUSED: &str = "t-1:core-012+core-025"; // R179
    /// TS `type Entry = { defId: string; radiant: boolean }`.
    type Entry = (&'static str, bool);
    const BOARD: [Entry; 5] = [
        ("core-012", false),
        (MENACE, false),
        ("core-025", true),
        (TOKEN, false),
        ("core-043", false),
    ];
    const BOARD_IDS: [&str; 5] = [BOARD[0].0, BOARD[1].0, BOARD[2].0, BOARD[3].0, BOARD[4].0];
    const OPPONENT_BOARD: [Entry; 1] = [("core-092", false)];

    /// A board as the JSON `lastBoards` takes: `[{ defId, radiant }]`.
    fn entries(board: &[Entry]) -> Value {
        Value::Array(board.iter().map(|(def_id, radiant)| json!({ "defId": def_id, "radiant": radiant })).collect())
    }

    fn game(own: &[Entry], radiant: bool, fillers: usize) -> Scenario {
        let mut hand = vec![json!({ "def": PORTAL, "radiant": radiant })];
        hand.extend((0..fillers).map(|_| json!(FILLER)));
        scenario(json!({
            "lastBoards": [entries(own), entries(&OPPONENT_BOARD)],
            "p1": { "hand": hand },
            "p2": { "hand": [FILLER] },
        }))
    }

    fn offered(state: &GameState) -> Vec<String> {
        state
            .pending
            .as_ref()
            .map(|pending| pending.options.as_slice())
            .unwrap_or(&[])
            .iter()
            .map(|option| match &option.selection {
                Selection::Mode { option } => option.clone(),
                _ => String::new(),
            })
            .collect()
    }

    /// The cards Portal made: everything in p1's hand but the fillers.
    fn made(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id != FILLER && card.def_id != PORTAL)
            .collect()
    }

    use crate::js;

    use crate::matches_object;

    fn unique(ids: impl IntoIterator<Item = String>) -> usize {
        ids.into_iter().collect::<IndexSet<String>>().len()
    }

    mod c_n29_portal_to_the_past {
        use super::*;

        #[test]
        fn is_a_3_spell_whose_radiant_count_is_the_declared_number_cards() {
            crate::register_all();
            let def = crate::card_def(ID);
            assert!(matches_object(&js(&def), &json!({ "id": PORTAL, "type": "Spell", "cost": 3 })));
            assert_eq!(
                js(&def.params),
                json!([
                    { "key": "cards", "base": 3, "radiant": 3, "better": "up", "step": 1, "min": 1 },
                    { "key": "setCost", "base": 0, "radiant": 0, "better": "down", "step": 1, "min": 0 }
                ])
            );
            let scripts = script();
            assert!(!scripts.base.resume.is_empty());
            assert!(scripts.radiant.resume.is_empty());
        }

        #[test]
        fn r386_a_degrade_makes_the_pick_cost_1_and_an_upgrade_finds_the_cost_at_its_floor_of_0() {
            crate::register_all();
            let mut s = game(&[("core-025", true)], false, 1);
            assert!(!crate::can_upgrade_number(&s, PORTAL, "setCost"));
            assert_eq!(crate::degrade_number(&mut s, PORTAL, "setCost"), 1);
            s.play(PORTAL, json!({}));
            s.answer(json!("mode:core-025"));
            assert!(matches_object(&js(&made(&s)), &json!([{ "defId": "core-025", "costOverride": 1 }])));
        }

        mod base_discover_a_card_from_the_board_your_last_game_ended_with_it_costs_0 {
            use super::*;

            #[test]
            fn r417_offers_3_different_cards_of_the_caster_s_own_last_board_never_the_opponent_s() {
                crate::register_all();
                let mut s = game(&BOARD, false, 1);
                s.play(PORTAL, json!({}));
                assert!(matches_object(&js(&s.state().pending), &json!({ "kind": "discover", "playerId": "p1" })));
                let options = offered(s.state());
                assert_eq!(options.len(), 3);
                assert_eq!(unique(options.clone()), 3);
                for def_id in &options {
                    assert!(BOARD_IDS.contains(&def_id.as_str()));
                }
            }

            #[test]
            fn r417_offers_them_all_when_the_board_holds_fewer_than_3_different_cards() {
                crate::register_all();
                let mut s = game(&[BOARD[0], BOARD[0], BOARD[1]], false, 1);
                s.play(PORTAL, json!({}));
                let mut options = offered(s.state());
                options.sort();
                assert_eq!(options, ["core-012", MENACE]);
            }

            #[test]
            fn r417_the_pick_arrives_as_a_new_card_the_caster_owns_on_its_entry_s_face_costing_0() {
                crate::register_all();
                let mut s = game(&[("core-025", true)], false, 1);
                s.play(PORTAL, json!({}));
                let radiant = s.state().pending.as_ref().and_then(|pending| pending.options.first()).and_then(|o| o.radiant);
                assert_eq!(radiant, Some(true));
                s.answer(json!("mode:core-025"));
                assert!(matches_object(
                    &js(&made(&s)),
                    &json!([{ "defId": "core-025", "radiant": true, "costOverride": 0, "owner": "p1", "controller": "p1" }])
                ));
                let hand = match s.view(P1).you.hand {
                    HandView::Cards(cards) => cards,
                    HandView::Count { .. } => panic!("§10.8: the viewer's own hand is a list of cards"),
                };
                assert_eq!(hand.iter().find(|card| card.def_id == "core-025").map(|card| card.cost), Some(0));
                assert!(s.state().pending.is_none());
                s.expect_in_zone(PORTAL, "graveyard");
            }

            #[test]
            fn r417_only_card_and_face_come_back_no_stats_buffs_or_damage() {
                crate::register_all();
                let mut s = game(&[(MENACE, false)], false, 1);
                s.play(PORTAL, json!({})).answer(json!(format!("mode:{MENACE}")));
                assert!(matches_object(
                    &js(&made(&s)),
                    &json!([{ "defId": MENACE, "radiant": false, "damage": 0, "buffs": { "attack": 0, "health": 0 } }])
                ));
            }

            #[test]
            fn r11_a_unit_token_on_that_board_comes_back_as_a_card_in_hand() {
                crate::register_all();
                let mut s = game(&[(TOKEN, false)], false, 1);
                s.play(PORTAL, json!({})).answer(json!(format!("mode:{TOKEN}")));
                assert!(matches_object(
                    &js(&made(&s)),
                    &json!([{ "defId": TOKEN, "costOverride": 0, "zone": { "z": "hand", "player": "p1" } }])
                ));
            }

            #[test]
            fn r179_a_fused_card_on_that_board_is_offered_by_its_id_and_rebuilt_by_the_engine() {
                crate::register_all();
                let mut s = game(&[(FUSED, false)], false, 1);
                s.play(PORTAL, json!({}));
                assert_eq!(offered(s.state()), [FUSED]);
                let first = match s.view(P1).pending {
                    Some(PendingView::ForYou(prompt)) => prompt.options.first().map(js),
                    _ => None,
                };
                assert!(matches_object(&js(&first), &json!({ "defId": FUSED })));
                s.answer(json!(format!("mode:{FUSED}")));
                assert!(matches_object(&js(&made(&s)), &json!([{ "defId": FUSED, "costOverride": 0 }])));
                assert_eq!(
                    js(&s.state().transient_defs.get(FUSED).and_then(|def| def.ingredients.clone())),
                    json!([{ "defId": "core-012" }, { "defId": "core-025" }])
                );
            }

            #[test]
            fn r564_two_copies_of_a_card_are_one_option_on_its_radiant_face_if_either_copy_was_radiant() {
                crate::register_all();
                let mut s = game(&[(MENACE, false), (MENACE, true)], false, 1);
                s.play(PORTAL, json!({}));
                let options = s.state().pending.as_ref().map(|pending| pending.options.clone()).unwrap_or_default();
                assert_eq!(options.len(), 1);
                assert!(matches_object(
                    &js(&options[0]),
                    &json!({ "key": format!("mode:{MENACE}"), "radiant": true })
                ));
            }

            #[test]
            fn r387_never_offers_portal_to_the_past_itself() {
                crate::register_all();
                let mut s = game(&[(PORTAL, false), (MENACE, false)], false, 1);
                s.play(PORTAL, json!({}));
                assert_eq!(offered(s.state()), [MENACE]);
            }

            #[test]
            fn r564_an_entry_the_match_s_catalog_lacks_was_dropped_when_the_match_was_created() {
                crate::register_all();
                let s = game(&[("core-999", false), (MENACE, false)], false, 1);
                assert_eq!(
                    js(&s.state().last_boards.as_ref().and_then(|boards| boards.p1.clone())),
                    json!([{ "defId": MENACE, "radiant": false }])
                );
            }

            #[test]
            fn r129_an_empty_last_board_hotseat_a_first_game_fizzles_no_prompt_no_random_draw_the_spell_still_played() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [PORTAL, FILLER] }, "p2": { "hand": [FILLER] } }));
                let cursor = s.state().rng_cursor;
                let played = s.state().counters.played;
                s.play(PORTAL, json!({}));
                assert!(s.state().pending.is_none());
                assert_eq!(s.state().rng_cursor, cursor);
                assert_eq!(s.state().counters.played, played + 1);
                assert!(made(&s).is_empty());
                s.expect_in_zone(PORTAL, "graveyard").expect_events(json!("cardPlayed"));
            }

            #[test]
            fn r97_r177_the_options_reach_the_chooser_only_the_opponent_sees_a_prompt_open_then_a_card_added_under_the_sentinel() {
                crate::register_all();
                let mut s = game(&BOARD, false, 1);
                s.play(PORTAL, json!({}));
                assert_eq!(js(&s.view(P2).pending), json!({ "forYou": false, "pendingFor": "p1" }));
                for def_id in BOARD_IDS {
                    assert!(!serde_json::to_string(&s.view(P2)).unwrap().contains(&format!("\"{def_id}\"")));
                }
                let pick = offered(s.state()).first().cloned().unwrap_or_default();
                s.answer(json!(format!("mode:{pick}")));
                let added: Vec<Value> = s
                    .view(P2)
                    .events
                    .iter()
                    .filter(|event| matches!(event, GameEvent::AddedToHand { .. }))
                    .map(js)
                    .collect();
                assert_eq!(
                    added,
                    vec![json!({ "type": "addedToHand", "player": "p1", "instanceId": "hidden", "defId": "hidden" })]
                );
                assert!(!serde_json::to_string(&s.view(P2)).unwrap().contains(&format!("\"{pick}\"")));
            }

            #[test]
            fn r317_the_pick_fills_a_hand_to_its_cap_of_10_and_fits() {
                crate::register_all();
                let mut s = game(&[(MENACE, false)], false, 9);
                s.play(PORTAL, json!({})).answer(json!(format!("mode:{MENACE}")));
                assert_eq!(s.hand(P1).len(), 10);
                assert_eq!(made(&s).len(), 1);
                assert!(!s.last_events().iter().any(|event| matches!(event, GameEvent::Burned { .. })));
            }

            #[test]
            fn s9_3_paused_mid_prompt_the_state_survives_json_and_resumes_to_the_same_card() {
                crate::register_all();
                let mut s = game(&BOARD, false, 1);
                s.play(PORTAL, json!({}));
                let revived: GameState = serde_json::from_str(&serde_json::to_string(s.state()).unwrap()).unwrap();
                assert_eq!(&revived, s.state());
                let option = offered(&revived).get(1).cloned().unwrap_or_default();
                let action: Action = json_as(json!({
                    "type": "answer",
                    "playerId": "p1",
                    "choiceId": revived.pending.as_ref().map(|pending| pending.id.clone()).unwrap_or_default(),
                    "selection": [{ "pick": "mode", "option": option }],
                    "nonce": "portal-round-trip",
                }));
                let result = reduce(&revived, &action);
                assert!(result.error.is_none());
                assert!(
                    result.state.players.p1.hand.iter().any(|card| card.def_id == option && card.cost_override == Some(0))
                );
                assert!(result.state.work.is_empty());
            }

            /// TS's `act`: one action of the fold test's log, its nonce numbered by the log so far.
            fn act(log: &mut Vec<Action>, state: &GameState, body: Value) -> GameState {
                let mut with_nonce = body.clone();
                with_nonce["nonce"] = json!(format!("portal-fold-{}", log.len()));
                let action: Action = json_as(with_nonce);
                let result = reduce(state, &action);
                if let Some(error) = &result.error {
                    panic!("{}: {error}", body["type"].as_str().unwrap_or_default());
                }
                log.push(action);
                result.state
            }

            #[test]
            fn s9_3_r417_in_a_real_game_it_folds_from_seed_decks_handicaps_lastboards_log_to_the_same_hash_through_a_json_pause() {
                crate::register_all();
                let p1_deck: Vec<&str> = vec![
                    PORTAL, "core-002", "core-005", "core-006", "core-008", "core-011", "core-012", "core-013", "core-015",
                    "core-016", "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043",
                    "core-044", "core-053", "core-055",
                ];
                let p2_deck: Vec<&str> = vec![
                    "core-015", "core-011", "core-004", "core-002", "core-005", "core-006", "core-008", "core-012",
                    "core-013", "core-016", "core-019", "core-020", "core-025", "core-026", "core-032", "core-036",
                    "core-043", "core-044", "core-053", "core-055",
                ];
                let mut own = BOARD.to_vec();
                own.push((FUSED, true));
                let last_boards = json!([entries(&own), entries(&OPPONENT_BOARD)]);
                // p2 plays under a practice handicap (R180), so every setup input travels together.
                let handicaps = json!({ "p2": js(&AI_TUTORIAL) });
                let decks = json!([p1_deck, p2_deck[..AI_TUTORIAL.deck_size as usize].to_vec()]);
                let mut log: Vec<Action> = Vec::new();

                let mut seed = String::new();
                let mut found: Option<GameState> = None;
                let mut at = 0;
                while at < 300 && found.is_none() {
                    seed = format!("portal-fold-{at}");
                    let args: CreateGameArgs = json_as(json!({
                        "seed": seed, "decks": decks, "handicaps": handicaps, "lastBoards": last_boards,
                    }));
                    let begun = begin_game(&create_game(&args)).state;
                    if begun.pending.is_none() && begun.players.p1.hand.iter().any(|card| card.def_id == PORTAL) {
                        found = Some(begun);
                    }
                    at += 1;
                }
                let mut state = found.expect("no seed deals p1 the Portal");
                for player in [P1, P2] {
                    let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
                    state = act(&mut log, &state, json!({ "type": "mulligan", "keep": keep, "playerId": player }));
                }
                // To p1's first turn with (3) mana, ending every turn on the way.
                while !(state.active == P1 && state.players.p1.mana.current >= 3) {
                    assert!(state.pending.is_none());
                    let active = state.active;
                    state = act(&mut log, &state, json!({ "type": "endTurn", "playerId": active }));
                }
                let portal = state
                    .players
                    .p1
                    .hand
                    .iter()
                    .find(|card| card.def_id == PORTAL)
                    .map(|card| card.id.clone())
                    .expect("p1 no longer holds the Portal");
                state = act(&mut log, &state, json!({ "type": "play", "playerId": "p1", "instanceId": portal }));
                assert_eq!(state.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Discover));

                let revived: GameState = serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
                assert_eq!(revived, state);
                let options = offered(&revived);
                let option = if options.iter().any(|option| option == FUSED) {
                    FUSED.to_string()
                } else {
                    options.first().cloned().unwrap_or_default()
                };
                let choice_id = revived.pending.as_ref().map(|pending| pending.id.clone()).unwrap_or_default();
                let done = act(
                    &mut log,
                    &revived,
                    json!({ "type": "answer", "playerId": "p1", "choiceId": choice_id, "selection": [{ "pick": "mode", "option": option }] }),
                );
                assert!(done.players.p1.hand.iter().any(|card| card.def_id == option && card.cost_override == Some(0)));

                let replayed = fold(&json_as(json!({
                    "seed": seed, "decks": decks, "handicaps": handicaps, "lastBoards": last_boards, "log": js(&log),
                })));
                assert!(replayed.errors.is_empty());
                assert_eq!(hash_state(&replayed.state), hash_state(&done));
                // Folded without its last boards it is another game: the boards are part of the match's inputs.
                let without = fold(&json_as(json!({ "seed": seed, "decks": decks, "handicaps": handicaps, "log": js(&log) })));
                assert_ne!(hash_state(&without.state), hash_state(&done));
            }
        }

        mod radiant_add_cards_random_cards_from_the_board_your_last_game_ended_with_each_costs_0 {
            use super::*;

            #[test]
            fn r417_adds_3_different_random_cards_of_the_board_straight_to_hand_with_no_prompt_each_costing_0() {
                crate::register_all();
                let mut s = game(&BOARD, true, 1);
                s.play(PORTAL, json!({}));
                assert!(s.state().pending.is_none());
                let cards = made(&s);
                assert_eq!(cards.len(), 3);
                assert_eq!(unique(cards.iter().map(|card| card.def_id.clone())), 3);
                for card in &cards {
                    assert!(BOARD_IDS.contains(&card.def_id.as_str()));
                    assert!(matches_object(
                        &js(card),
                        &json!({ "costOverride": 0, "owner": "p1", "radiant": card.def_id == "core-025" })
                    ));
                }
            }

            #[test]
            fn r417_adds_them_all_when_the_board_holds_fewer_different_cards_than_cards() {
                crate::register_all();
                let mut s = game(&[(MENACE, false)], true, 1);
                s.play(PORTAL, json!({}));
                let ids: Vec<String> = made(&s).iter().map(|card| card.def_id.clone()).collect();
                assert_eq!(ids, [MENACE]);
            }

            #[test]
            fn r129_an_empty_board_adds_nothing_and_draws_no_random_number() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": PORTAL, "radiant": true }, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                let cursor = s.state().rng_cursor;
                s.play(PORTAL, json!({}));
                assert_eq!(s.state().rng_cursor, cursor);
                assert!(made(&s).is_empty());
                s.expect_in_zone(PORTAL, "graveyard");
            }

            #[test]
            fn r386_the_count_is_param_ctx_cards_an_upgrade_adds_4_a_degrade_2() {
                crate::register_all();
                let mut up = game(&BOARD, true, 1);
                step_param(up.card_mut(PORTAL), "cards", 1);
                up.play(PORTAL, json!({}));
                assert_eq!(made(&up).len(), 4);

                let mut down = game(&BOARD, true, 1);
                step_param(down.card_mut(PORTAL), "cards", -1);
                down.play(PORTAL, json!({}));
                assert_eq!(made(&down).len(), 2);
            }

            #[test]
            fn r317_a_full_hand_burns_the_overflow_which_both_players_read() {
                crate::register_all();
                let mut s = game(&BOARD, true, 9);
                s.play(PORTAL, json!({}));
                assert_eq!(s.hand(P1).len(), 10);
                let burned = s.last_events().iter().filter(|event| matches!(event, GameEvent::Burned { .. })).count();
                assert_eq!(burned, 2);
                for event in s.view(P2).events.iter().filter(|entry| matches!(entry, GameEvent::Burned { .. })) {
                    assert!(matches_object(&js(event), &json!({ "owner": "p1" })));
                    if let GameEvent::Burned { def_id, .. } = event {
                        assert!(BOARD_IDS.contains(&def_id.as_str()));
                    }
                }
            }

            #[test]
            fn r97_the_opponent_reads_each_card_added_to_hand_as_the_sentinel() {
                crate::register_all();
                let mut s = game(&BOARD, true, 1);
                s.play(PORTAL, json!({}));
                let added: Vec<GameEvent> = s
                    .view(P2)
                    .events
                    .into_iter()
                    .filter(|event| matches!(event, GameEvent::AddedToHand { .. }))
                    .collect();
                assert_eq!(added.len(), 3);
                for event in &added {
                    assert!(matches_object(&js(event), &json!({ "player": "p1", "instanceId": "hidden", "defId": "hidden" })));
                }
            }
        }
    }
}
