//! C+ #29 Portal to the Past (SPEC §8.7 row 29, R417, R564, R3837003, R3837004). (3) Spell.
//!   Base:    Discover a card from the board your last game ended with; it arrives costing (0).
//!   Radiant: {cards} different random cards of that board straight to your hand, each costing (0).
//! The board is the caster's last board, a setup input frozen into the match (B5 E30,
//! `subsystems/last_boards`). Since issue #3837 one that gives no card (hotseat, the AI's seat, a first
//! game, a board of dropped entries, R564, or of Portal alone, R387) falls back to the non-token Units
//! and Field Spells costing (3) or (4) (R3837003, R3837004): the base face Discovers one, the Radiant
//! face adds {cards} different ones, each costing (0) and on its base face. Through v0.6.0g such a board
//! gave nothing (R129); `script_v0_6_0g` keeps that for its replays (R767000).
//! R2739000: the base face sets `secret_option_counts`, since how many cards its Discover offers is read
//! off a board no view shows, so the other seat is never told it.

use jackioh_engine::effects::{
    add_from_last_board, add_random_from_catalog, add_random_from_last_board, add_to_hand, discover_from_catalog,
    discover_from_last_board,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-029";

/// The step the Discover's answer re-enters (§10.6).
const PICKED: &str = "picked";
/// R3837003: the step the fallback Discover's answer re-enters.
const FALLBACK_PICKED: &str = "fallbackPicked";
/// R3837004: the fallback pool's lowest and highest cost, "(3) or (4)".
const FALLBACK_MIN_COST: i32 = 3;
const FALLBACK_MAX_COST: i32 = 4;

/// R2476000: in its owner's hand, the different cards it can give from the board the caster's last game
/// ended with, after the words both faces print: the candidates `discover_from_last_board` and
/// `add_random_from_last_board` draw from (R564), never its own definitions (R387). 0 is the fallback
/// (R3837003: hotseat, a first game). The board is the caster's own, so only they see the count.
fn preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if ctx.zone != ConditionZone::Hand {
            return vec![];
        }
        let own = self_def_ids(Some(ctx.state), &ctx.self_.def_id);
        let count = subsystems::last_boards::last_board_candidates(ctx.state, ctx.controller, &own).len();
        vec![count_preview("the board your last game ended with", count as i32)]
    })
}

/// R3837004: the fallback pool, by query and never card by card: the non-token (the default) Units and
/// Field Spells of the sets that ship (R380, R1420) whose cost a query reads (R65) is (3) or (4).
fn fallback_query() -> Value {
    json!({
        "type": ["Unit", "Field Spell"],
        "costRange": { "min": FALLBACK_MIN_COST, "max": FALLBACK_MAX_COST },
    })
}

/// R3837003: whether the caster's last board gives a card, read as the last-board verbs read it: its
/// candidates (R564) less this card's own definitions (R387).
fn last_board_gives(ctx: &EffectContext<'_>) -> bool {
    let own = match &ctx.self_ {
        Some(this) => Some(this.def_id.clone()),
        None => ctx.def_id.clone(),
    };
    let exclude = own.map(|own| self_def_ids(Some(&*ctx.state), &own)).unwrap_or_default();
    !subsystems::last_boards::last_board_candidates(&*ctx.state, ctx.controller, &exclude).is_empty()
}

/// "It costs (0)", "Each costs (0)": the declared number `setCost` (R386), less being better. `fallback`
/// is whether a board that gives no card falls back to the (3) or (4) Cost pool (R3837003).
fn portal(fallback: bool) -> CardScripts {
    let base = Script {
        cry: Some(hook(move |ctx| {
            if fallback && !last_board_gives(ctx) {
                return vec![discover_from_catalog(json_as(json!({
                    "step": FALLBACK_PICKED,
                    "query": fallback_query(),
                    "prompt": "Discover a (3) or (4) Cost Unit or Field Spell",
                })))];
            }
            vec![discover_from_last_board(json_as(json!({ "step": PICKED })))]
        })),
        resume: IndexMap::from([
            (
                PICKED,
                hook(|ctx| vec![add_from_last_board(json_as(json!({ "costOverride": param(&*ctx, "setCost") })))]),
            ),
            (
                FALLBACK_PICKED,
                // A Discover's pick arrives as a `mode` selection carrying a catalog id (§10.6).
                hook(|ctx| {
                    let Some(picked) = chosen_options(ctx).into_iter().next() else {
                        return vec![];
                    };
                    vec![add_to_hand(json_as(json!({
                        "defId": picked,
                        "costOverride": param(&*ctx, "setCost"),
                    })))]
                }),
            ),
        ]),
        preview: Some(preview()),
        secret_option_counts: true,
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(move |ctx| {
            if fallback && !last_board_gives(ctx) {
                // R3563000: different cards, as the run's draws are unique.
                return vec![add_random_from_catalog(json_as(json!({
                    "query": fallback_query(),
                    "count": param(&*ctx, "cards"),
                    "costOverride": param(&*ctx, "setCost"),
                })))];
            }
            vec![add_random_from_last_board(json_as(json!({
                "count": param(&*ctx, "cards"),
                "costOverride": param(&*ctx, "setCost"),
            })))]
        })),
        preview: Some(preview()),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

pub fn script() -> CardScripts {
    portal(true)
}

/// R767000: through v0.6.0g a board that gave no card gave nothing (R129); `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_6_0g() -> CardScripts {
    portal(false)
}

// C+ #29 Portal to the Past — SPEC §8.7 row 29, BUILD M9 Classic+ row C+ 29. `scenario({ lastBoards })` hands
// the last board to `createGame` as the server does.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PORTAL: &str = "classicplus-029";
    const FILLER: &str = "core-005"; // Stockpile: keeps a hand from auto-ending the turn (§2.5)
    const MENACE: &str = "core-019";
    const TOKEN: &str = "core-t-rush"; // Rush Token: a unit token (R11)
    const FUSED: &str = "t-1:core-012+core-025"; // R179
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

    /// R3837004: a non-token Unit or Field Spell whose query cost (R65) is (3) or (4).
    fn in_fallback_pool(def_id: &str) -> bool {
        let def = crate::card_def(def_id);
        !def.token
            && matches!(def.type_, CardType::Unit | CardType::FieldSpell)
            && (3..=4).contains(&crate::query::query_cost(&def))
    }

    mod c_n29_portal_to_the_past {
        use super::*;

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
                let options = s.prompt_modes();
                assert_eq!(options.len(), 3);
                assert_eq!(must(s.state().pending.as_ref(), "an open prompt").options.len(), 3);
                assert_eq!(unique(options.clone()), 3);
                for def_id in &options {
                    assert!(BOARD_IDS.contains(&def_id.as_str()));
                }
            }

            /// R2476000: in hand, the different cards of the caster's last board it can give: two
            /// different cards on a board of three, and the Discover then offers those two.
            #[test]
            fn r2476000_in_hand_it_previews_the_different_cards_of_your_last_board() {
                crate::register_all();
                for radiant in [false, true] {
                    let s = game(&[BOARD[0], BOARD[0], BOARD[1]], radiant, 1);
                    let portal = s.card(PORTAL).id.clone();
                    assert_eq!(
                        crate::preview_seen(&s, P1, &portal),
                        vec![("the board your last game ended with".to_string(), 2, None)],
                        "radiant {radiant}"
                    );
                }
                let empty = game(&[], false, 1);
                let portal = empty.card(PORTAL).id.clone();
                assert_eq!(crate::preview_seen(&empty, P1, &portal), vec![("the board your last game ended with".to_string(), 0, None)]);
            }

            #[test]
            fn r417_offers_them_all_when_the_board_holds_fewer_than_3_different_cards() {
                crate::register_all();
                let mut s = game(&[BOARD[0], BOARD[0], BOARD[1]], false, 1);
                s.play(PORTAL, json!({}));
                let mut options = s.prompt_modes();
                options.sort();
                assert_eq!(options, ["core-012", MENACE]);
                assert_eq!(must(s.state().pending.as_ref(), "an open prompt").options.len(), 2);
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
                assert_eq!(s.prompt_modes(), [FUSED]);
                assert_eq!(must(s.state().pending.as_ref(), "an open prompt").options.len(), 1);
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
                assert_eq!(s.prompt_modes(), [MENACE]);
                assert_eq!(must(s.state().pending.as_ref(), "an open prompt").options.len(), 1);
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
            fn r3837003_r3837004_an_empty_last_board_discovers_one_of_3_units_or_field_spells_costing_3_or_4() {
                crate::register_all();
                // Hotseat and the AI's seat hand no last board; a first game hands an empty one.
                let none = scenario(json!({ "p1": { "hand": [PORTAL, FILLER] }, "p2": { "hand": [FILLER] } }));
                for mut s in [none, game(&[], false, 1)] {
                    let played = s.state().counters.played;
                    s.play(PORTAL, json!({}));
                    assert!(matches_object(&js(&s.state().pending), &json!({ "kind": "discover", "playerId": "p1" })));
                    let options = s.prompt_modes();
                    assert_eq!(options.len(), DISCOVER_OPTIONS as usize);
                    assert_eq!(unique(options.iter().cloned()), options.len());
                    for option in &options {
                        assert!(in_fallback_pool(option), "{option}");
                    }
                    s.answer(json!(format!("mode:{}", options[0])));
                    let cards = made(&s);
                    assert_eq!(cards.len(), 1);
                    assert!(matches_object(&js(&cards[0]), &json!({ "defId": options[0], "costOverride": 0, "owner": "p1" })));
                    assert!(!cards[0].radiant);
                    assert_eq!(s.state().counters.played, played + 1);
                    s.expect_in_zone(PORTAL, "graveyard");
                }
            }

            #[test]
            fn r3837003_a_board_of_dropped_entries_or_only_portal_falls_back() {
                crate::register_all();
                // R564 dropped the entry the match's catalog lacks (an older patch's card); R387 never
                // offers Portal itself. Neither board gives a card, so both fall back.
                for board in [[("core-999", false)], [(PORTAL, false)]] {
                    let mut s = game(&board, false, 1);
                    s.play(PORTAL, json!({}));
                    let options = s.prompt_modes();
                    assert_eq!(options.len(), DISCOVER_OPTIONS as usize, "{board:?}");
                    assert!(options.iter().all(|option| in_fallback_pool(option)), "{board:?}: {options:?}");

                    let mut radiant = game(&board, true, 1);
                    radiant.play(PORTAL, json!({}));
                    let cards = made(&radiant);
                    assert_eq!(cards.len(), 3, "{board:?}");
                    assert!(cards.iter().all(|card| in_fallback_pool(&card.def_id)), "{board:?}");
                }
            }

            #[test]
            fn r3837003_the_opponents_board_never_stands_in_for_yours() {
                crate::register_all();
                // `game` hands p2 the opponent's board, Felinor Fiender, a (2): never yours to Discover.
                let mut s = game(&[], false, 1);
                s.play(PORTAL, json!({}));
                let options = s.prompt_modes();
                assert!(!options.iter().any(|option| option == OPPONENT_BOARD[0].0));
                assert!(options.iter().all(|option| in_fallback_pool(option)));
            }

            #[test]
            fn r129_r767000_the_kept_v0_6_0g_script_still_fizzles_on_an_empty_board() {
                crate::under_kept(PORTAL, super::super::super::script_v0_6_0g(), || {
                    for radiant in [false, true] {
                        let mut s = scenario(json!({
                            "p1": { "hand": [{ "def": PORTAL, "radiant": radiant }, FILLER] },
                            "p2": { "hand": [FILLER] },
                        }));
                        let cursor = s.state().rng_cursor;
                        let played = s.state().counters.played;
                        s.play(PORTAL, json!({}));
                        assert!(s.state().pending.is_none());
                        assert_eq!(s.state().rng_cursor, cursor);
                        assert_eq!(s.state().counters.played, played + 1);
                        assert!(made(&s).is_empty());
                        s.expect_in_zone(PORTAL, "graveyard").expect_events(json!("cardPlayed"));
                    }
                });
            }

            #[test]
            fn r2739000_r97_r177_the_options_reach_the_chooser_only_the_opponent_sees_a_prompt_open_then_a_card_added_under_the_sentinel() {
                crate::register_all();
                let mut s = game(&BOARD, false, 1);
                s.play(PORTAL, json!({}));
                assert_eq!(js(&s.view(P2).pending), json!({ "forYou": false, "pendingFor": "p1" }));
                for def_id in BOARD_IDS {
                    assert!(!serde_json::to_string(&s.view(P2)).unwrap().contains(&format!("\"{def_id}\"")));
                }
                let pick = s.prompt_modes().first().cloned().unwrap_or_default();
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
                let option = prompt_modes_of(revived.pending.as_ref().expect("a prompt is open")).get(1).cloned().unwrap_or_default();
                let action: Action = json_as(json!({
                    "type": "answer",
                    "playerId": "p1",
                    "choiceId": revived.pending.as_ref().map(|pending| pending.id.clone()).unwrap_or_default(),
                    "selection": pick::mode(&option),
                    "nonce": "portal-round-trip",
                }));
                let result = reduce(&revived, &action);
                assert!(result.error.is_none());
                assert!(
                    result.state.players.p1.hand.iter().any(|card| card.def_id == option && card.cost_override == Some(0))
                );
                assert!(result.state.work.is_empty());
            }

            /// One action of the fold test's log, its nonce numbered by the log so far.
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
                    "core-044", "core-053", "core-055", "core-001", "core-003", "core-004", "core-009", "core-010",
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
                let options = prompt_modes_of(revived.pending.as_ref().expect("a prompt is open"));
                let option = if options.iter().any(|option| option == FUSED) {
                    FUSED.to_string()
                } else {
                    options.first().cloned().unwrap_or_default()
                };
                let choice_id = revived.pending.as_ref().map(|pending| pending.id.clone()).unwrap_or_default();
                let done = act(
                    &mut log,
                    &revived,
                    json!({ "type": "answer", "playerId": "p1", "choiceId": choice_id, "selection": pick::mode(&option) }),
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
            fn r3837003_r3837004_radiant_an_empty_last_board_adds_3_different_units_or_field_spells_costing_3_or_4() {
                let none = scenario(json!({
                    "p1": { "hand": [{ "def": PORTAL, "radiant": true }, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                for mut s in [none, game(&[], true, 1)] {
                    s.play(PORTAL, json!({}));
                    assert!(s.state().pending.is_none());
                    let cards = made(&s);
                    assert_eq!(cards.len(), 3);
                    assert_eq!(unique(cards.iter().map(|card| card.def_id.clone())), 3);
                    for card in &cards {
                        assert!(in_fallback_pool(&card.def_id), "{}", card.def_id);
                        assert!(matches_object(&js(card), &json!({ "costOverride": 0, "owner": "p1" })));
                        assert!(!card.radiant);
                    }
                    s.expect_in_zone(PORTAL, "graveyard");
                }
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
