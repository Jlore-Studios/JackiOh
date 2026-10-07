//! The Glitch Easter egg (issue #170; subsystems/glitch.ts, catalog.pickGenerated; R673–R679). Through a
//! fixture Glitch registered under the real id; the real card's test (packages/cards/test/classic/
//! t-glitch-glitch.test.ts) covers the same cases again on the real catalog.
//!
//! `pickGenerated`'s third argument (TS `GlitchOdds`, `{ systemPlays? }`) and `seatPlayedBy`'s first
//! are the state in Rust; where TS passed a bare object literal, this passes the test's state with
//! only that field set.
//!
//! Port of `packages/engine/test/glitch.test.ts`.

use jackioh_engine::catalog::{CatalogQueryArgs, find_def, pick_generated, query};
use jackioh_engine::effects::glitch;
use jackioh_engine::mana::play_cost;
use jackioh_engine::reduce::reduce;
use jackioh_engine::replay::hash_state;
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::subsystems::glitch::{count_system_play, seat_played_by, seats_swapped};
use jackioh_engine::testkit::*;
use jackioh_engine::view_for::{HIDDEN_ID, view_for};
use jackioh_engine::wire::PlayerId::{P1, P2};
use jackioh_engine::zones::{card_at, slots_of};

use crate::rules::fixtures::catalog::spell_def;
use crate::rules::fixtures::harness::{in_hand, new_game, put, slot};

fn json_of<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

/// `JSON.parse(JSON.stringify(state))`.
fn round_trip(state: &GameState) -> GameState {
    serde_json::from_value(json_of(state)).expect("a state survives JSON")
}

/// `{ ...spellDef(index, overrides), ...rest }`: the fixture Spell with `rest`'s fields laid over it.
fn spell_with(index: i32, overrides: Value, rest: Value) -> CardDef {
    let mut card = json_of(spell_def(index, overrides));
    if let (Some(card), Some(rest)) = (card.as_object_mut(), rest.as_object()) {
        for (key, value) in rest {
            card.insert(key.clone(), value.clone());
        }
    }
    json_as(card)
}

fn glitch_def() -> CardDef {
    spell_with(
        9001,
        json!({
            "id": GLITCH_DEF_ID,
            "index": "T-glitch",
            "name": "Glitch",
            "set": "Classic",
            "token": true,
            "rarity": "Token",
            "tags": ["Token"],
        }),
        json!({
            "cost": 0,
            "base": { "keywords": [], "text": "" },
            "radiant": { "keywords": [], "text": "" },
        }),
    )
}

/// A game at p1's turn 3 in its main phase, Glitch registered beside the fixture catalog.
fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    catalog.insert(GLITCH_DEF_ID.to_string(), glitch_def());
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    let face = Script {
        cry: Some(hook(|_ctx| vec![glitch()])),
        ..Script::default()
    };
    scripts.insert(
        GLITCH_DEF_ID.to_string(),
        CardScripts {
            base: face.clone(),
            radiant: face,
        },
    );
    register_scripts(scripts);
    state.phase = Phase::Main;
    state.turn = 3;
    state.active = P1;
    state.players.p1.turns_started = 2;
    state.players.p2.turns_started = 1;
    state.players.p1.mana = ManaState {
        current: 0,
        max: 2,
        ..state.players.p1.mana
    };
    state
}

/// The first seed whose Glitch draws `outcome` when p1 plays it (each outcome is one rng draw, R676).
fn seed_for(outcome: GlitchOutcome) -> String {
    for n in 0..200 {
        let seed = format!("glitch-{outcome}-{n}");
        let mut rng = create_rng(&seed, game(&seed).rng_cursor);
        let drawn = rng.int(GLITCH_OUTCOMES.len() as i32);
        if GLITCH_OUTCOMES[drawn as usize] == outcome {
            return seed;
        }
    }
    panic!("no seed draws {outcome}");
}

fn play_action(instance_id: &str, player: PlayerId, nonce: &str) -> Action {
    json_as(json!({ "type": "play", "playerId": player, "instanceId": instance_id, "nonce": nonce }))
}

fn play_glitch(state: &mut GameState) -> ReduceResult {
    let card = in_hand(state, GLITCH_DEF_ID, P1, 1).into_iter().next();
    let id = card.map(|card| card.id).unwrap_or_default();
    let result = reduce(state, &play_action(&id, P1, "glitch"));
    assert!(result.error.is_none());
    result
}

fn catalog_query(body: Value) -> Vec<CardDef> {
    let args: CatalogQueryArgs = json_as(body);
    query(&args).into_iter().cloned().collect()
}

/// The state as `pickGenerated`'s odds: TS's `{ systemPlays }` literal, the state with that field set.
fn odds(state: &GameState, system_plays: Option<i32>) -> GameState {
    let mut odds = state.clone();
    odds.system_plays = system_plays;
    odds
}

fn picked_id(def: Option<&CardDef>) -> Option<String> {
    def.map(|def| def.id.clone())
}

mod r673_glitchs_odds {
    use super::*;

    #[test]
    fn r673_with_no_system_play_a_pick_for_a_hand_draws_exactly_as_before_one_draw_the_same_card() {
        let state = game("odds-none");
        let pool = catalog_query(json!({ "type": "Unit" }));
        let pool: Vec<&CardDef> = pool.iter().collect();
        let mut a = create_rng("odds-none", 0);
        let mut b = create_rng("odds-none", 0);
        assert_eq!(
            picked_id(pick_generated(&mut a, &pool, Some(&odds(&state, Some(0))))),
            b.pick(&pool).map(|def| def.id.clone())
        );
        assert_eq!(
            picked_id(pick_generated(&mut a, &pool, Some(&odds(&state, None)))),
            b.pick(&pool).map(|def| def.id.clone())
        );
        assert_eq!(a.cursor(), b.cursor());
    }

    #[test]
    fn r673_after_n_system_plays_one_more_draw_makes_the_pick_glitch_when_it_falls_under_n_10000() {
        let state = game("odds-some");
        let pool = catalog_query(json!({ "type": "Unit" }));
        let pool: Vec<&CardDef> = pool.iter().collect();
        let three = odds(&state, Some(3));
        let mut glitched = 0;
        let mut kept = 0;
        let mut n = 0;
        while n < 4000 && (glitched == 0 || kept == 0) {
            let mut probe = create_rng(&format!("odds-{n}"), 0);
            let _ = probe.pick(&pool);
            let roll = probe.int(GLITCH_ODDS_DENOMINATOR);
            let mut rng = create_rng(&format!("odds-{n}"), 0);
            let def = picked_id(pick_generated(&mut rng, &pool, Some(&three)));
            assert_eq!(rng.cursor(), 2);
            if roll < 3 {
                assert_eq!(def.as_deref(), Some(GLITCH_DEF_ID));
                glitched += 1;
            } else {
                assert_ne!(def.as_deref(), Some(GLITCH_DEF_ID));
                kept += 1;
            }
            n += 1;
        }
        assert!(kept > 0);
        // 10000 out of 10000: every pick is Glitch.
        let every = odds(&state, Some(GLITCH_ODDS_DENOMINATOR));
        assert_eq!(
            picked_id(pick_generated(&mut create_rng("all", 0), &pool, Some(&every))).as_deref(),
            Some(GLITCH_DEF_ID)
        );
    }

    #[test]
    fn r673_counts_every_play_of_a_in_the_system_card_by_either_player_and_nothing_else() {
        let mut state = game("count");
        let mut def_ids: Vec<&str> = SYSTEM_CARD_DEF_IDS.to_vec();
        def_ids.push("fx-1");
        for def_id in def_ids {
            let card = new_instance(&mut state, def_id, P2, Zone::Resolving { player: P2 });
            count_system_play(&mut state, &card);
        }
        assert_eq!(state.system_plays, Some(SYSTEM_CARD_DEF_IDS.len() as i32));
    }

    #[test]
    fn r674_no_pool_holds_glitch_not_even_one_that_takes_every_token_naming_it_by_id_still_finds_it() {
        game("pools");
        let ids = |body: Value| -> Vec<String> { catalog_query(body).into_iter().map(|def| def.id).collect() };
        assert!(!ids(json!({ "withTokens": true })).contains(&GLITCH_DEF_ID.to_string()));
        assert!(!ids(json!({ "tags": ["Token"] })).contains(&GLITCH_DEF_ID.to_string()));
        assert_eq!(ids(json!({ "defId": GLITCH_DEF_ID })), vec![GLITCH_DEF_ID.to_string()]);
    }
}

mod r675_glitch_is_always_playable_on_its_owners_turn {
    use super::*;

    #[test]
    fn r675_costs_0_whatever_modifies_it_so_it_is_played_with_no_mana() {
        let mut state = game("price");
        let card = in_hand(&mut state, GLITCH_DEF_ID, P1, 1)
            .into_iter()
            .next()
            .expect("no Glitch");
        let held = find_instance_mut(&mut state, &card.id).expect("no Glitch");
        held.cost_mod = 5;
        held.cost_override = Some(3);
        let card = held.clone();
        assert_eq!(play_cost(&state, &card), 0);
        assert!(reduce(&state, &play_action(&card.id, P1, "free")).error.is_none());
    }

    #[test]
    fn r675_is_not_playable_on_the_other_players_turn() {
        let mut state = game("their-turn");
        let card = in_hand(&mut state, GLITCH_DEF_ID, P2, 1).into_iter().next();
        let id = card.map(|card| card.id).unwrap_or_default();
        assert!(reduce(&state, &play_action(&id, P2, "no")).error.is_some());
    }
}

mod r676_glitchs_outcome_and_the_reset {
    use super::*;

    #[test]
    fn r676_draws_its_outcome_from_the_match_rng_and_says_which_publicly() {
        for &outcome in GLITCH_OUTCOMES {
            let mut state = game(&seed_for(outcome));
            let events = play_glitch(&mut state).events;
            let glitched: Vec<Value> = events
                .iter()
                .map(json_of)
                .filter(|event| event["type"] == json!("glitched"))
                .collect();
            assert_eq!(glitched, vec![json!({ "type": "glitched", "player": "p1", "outcome": outcome })]);
        }
    }

    #[test]
    fn r676_a_reset_deals_the_match_again_from_its_decks_setup_mulligans_open_fresh_ids_the_nonce_log_kept() {
        let mut before = game(&seed_for(GlitchOutcome::Reset));
        before.system_plays = Some(2);
        let old_ids: IndexSet<String> = before
            .players
            .p1
            .library
            .iter()
            .chain(before.players.p2.library.iter())
            .map(|card| card.id.clone())
            .collect();
        let ReduceResult { state, events, .. } = play_glitch(&mut before);
        assert_eq!(state.turn, SETUP_TURN);
        assert_eq!(state.phase, Phase::Mulligan);
        assert!(state.mulligan.as_ref().map(|seats| &seats.p1).is_some());
        assert!(state.system_plays.is_none());
        assert!(state.reset_owed.is_none());
        assert_eq!(state.resets, Some(1));
        assert_eq!(state.opening, before.opening);
        assert!(state.applied.iter().any(|entry| entry.nonce == "glitch"));
        let dealt: Vec<&CardInstance> = state.players.p1.hand.iter().chain(state.players.p1.library.iter()).collect();
        assert_eq!(dealt.len(), before.opening.as_ref().map(|opening| opening.decks.0.len()).unwrap_or(0));
        assert!(!dealt.iter().any(|card| old_ids.contains(&card.id)));
        assert!(events.iter().any(|event| json_of(event)["type"] == json!("glitched")));
    }

    #[test]
    fn r676_a_reset_is_pure_the_same_state_and_action_give_the_same_state_and_a_json_copy_plays_the_same() {
        let mut start = game(&seed_for(GlitchOutcome::Reset));
        in_hand(&mut start, GLITCH_DEF_ID, P1, 1);
        let card = start.players.p1.hand.last().cloned();
        let action = play_action(&card.map(|card| card.id).unwrap_or_default(), P1, "pure");
        let a = reduce(&clone_state(&start), &action).state;
        let b = reduce(&round_trip(&start), &action).state;
        assert_eq!(hash_state(&a), hash_state(&b));
    }
}

mod r677_the_seat_swap {
    use super::*;

    #[test]
    fn r677_swaps_which_seat_each_account_plays_and_only_that_an_even_number_of_swaps_is_none() {
        let mut start = game(&seed_for(GlitchOutcome::Swap));
        let state = play_glitch(&mut start).state;
        assert_eq!(state.seat_swaps, Some(1));
        assert!(seats_swapped(&state));
        assert_eq!(seat_played_by(&state, P1), P2);
        assert_eq!(seat_played_by(&state, P2), P1);
        // TS `seatPlayedBy({ seatSwaps: 2 }, "p1")`: a state whose seats were swapped twice.
        let mut twice = state.clone();
        twice.seat_swaps = Some(2);
        assert_eq!(seat_played_by(&twice, P1), P1);
    }
}

mod r678_other_games_boards {
    use super::*;

    #[test]
    fn r678_replaces_both_fields_with_the_frozen_boards_units_to_the_unit_zones_and_the_rest_to_the_backrow() {
        let seed = seed_for(GlitchOutcome::Boards);
        let mut state = game(&seed);
        put(&mut state, "fx-1", slot(P1, Row::Units, 1), json!({}));
        put(&mut state, "fx-2", slot(P2, Row::Units, 3), json!({}));
        let decks = state
            .opening
            .as_ref()
            .map(|opening| opening.decks.clone())
            .unwrap_or_default();
        let frozen = create_game(&json_as(json!({
            "seed": seed,
            "decks": decks,
            "glitchBoards": [
                [{ "defId": "fx-7", "radiant": true }, { "defId": "fx-8", "radiant": false }],
                [{ "defId": "not-a-card", "radiant": false }],
            ],
        })));
        state.glitch_boards = frozen.glitch_boards.clone();
        let after = play_glitch(&mut state).state;
        let units = |player: PlayerId| -> Vec<CardInstance> {
            slots_of(player, Row::Units)
                .into_iter()
                .filter_map(|at| card_at(&after, at).cloned())
                .collect()
        };
        assert_eq!(
            units(P1)
                .iter()
                .map(|card| json!([card.def_id, card.radiant, card.owner]))
                .collect::<Vec<_>>(),
            vec![json!(["fx-7", true, "p1"]), json!(["fx-8", false, "p1"])]
        );
        // An entry this match cannot rebuild was dropped when it was frozen (R564), so p2's field is empty.
        assert!(units(P2).is_empty());
        assert_eq!(
            after.players.p1.graveyard.iter().map(|card| card.def_id.clone()).collect::<Vec<_>>(),
            vec![GLITCH_DEF_ID.to_string()]
        );
    }
}

mod r679_the_void {
    use super::*;

    #[test]
    fn r679_ends_the_game_with_no_winner_and_reason_voided() {
        let mut start = game(&seed_for(GlitchOutcome::Void));
        let ReduceResult { state, events, .. } = play_glitch(&mut start);
        assert_eq!(json_of(state.result), json!({ "winner": "draw", "reason": "voided" }));
        assert_eq!(
            events.last().map(json_of),
            Some(json!({ "type": "gameOver", "winner": "draw", "reason": "voided" }))
        );
    }

    #[test]
    fn r679_the_effect_alone_from_any_context_ends_the_game_the_same_way() {
        let mut state = game(&seed_for(GlitchOutcome::Void));
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..Default::default()
                },
            );
            (glitch().apply)(&mut ctx);
        }
        assert_eq!(state.result.map(|result| json_of(result.reason)), Some(json!("voided")));
        assert_eq!(find_def(None, GLITCH_DEF_ID).map(json_of), Some(json_of(glitch_def())));
    }
}

mod r764_a_glitchs_reset_or_boards_leaves_no_public_trace_of_the_cards_it_took_unseen {
    use super::*;

    fn trap_def() -> CardDef {
        spell_with(
            9002,
            json!({ "id": "fx-glitch-trap", "index": "T-trap", "name": "Fx Trap" }),
            json!({ "type": "Trap" }),
        )
    }

    /// The `player`'s events that p1's view names, as `[instanceId, defId]`, apart from p1's own Glitch.
    fn named_by(state: &GameState, player: PlayerId, ty: &str) -> Vec<Value> {
        let view = json_of(view_for(state, P1));
        let event = view["events"]
            .as_array()
            .and_then(|events| {
                events
                    .iter()
                    .find(|entry| entry["type"] == json!(ty) && entry.get("player") == Some(&json!(player)))
            })
            .cloned();
        match event {
            Some(event) if event.get("instanceId").is_some() => {
                vec![event["instanceId"].clone(), event.get("defId").cloned().unwrap_or(Value::Null)]
            }
            _ => vec![],
        }
    }

    #[test]
    fn r764_boards_a_face_down_trap_the_board_took_still_reads_as_the_sentinel_in_the_events_that_named_it() {
        let mut state = game(&seed_for(GlitchOutcome::Boards));
        let mut catalog = registered_catalog().clone();
        catalog.insert(trap_def().id, trap_def());
        register_catalog(catalog);
        let trap = put(&mut state, &trap_def().id, slot(P2, Row::Backrow, 1), json!({}));
        state.applied = vec![json_as(json!({
            "nonce": "before",
            "events": [{ "type": "cardPlayed", "player": "p2", "instanceId": trap.id, "defId": trap.def_id, "costPaid": 1 }],
        }))];
        // The control: while it stands face-down, p1 reads the sentinel.
        assert_eq!(named_by(&state, P2, "cardPlayed"), vec![json!(HIDDEN_ID), json!(HIDDEN_ID)]);

        let after = play_glitch(&mut state).state;
        assert!(card_at(&after, slot(P2, Row::Backrow, 1)).is_none());
        assert_eq!(named_by(&after, P2, "cardPlayed"), vec![json!(HIDDEN_ID), json!(HIDDEN_ID)]);
    }

    #[test]
    fn r764_reset_a_card_the_other_player_drew_in_the_old_game_still_reads_as_the_sentinel_after_the_match_began_again() {
        let mut state = game(&seed_for(GlitchOutcome::Reset));
        let drawn = in_hand(&mut state, "fx-1", P2, 1)
            .into_iter()
            .next()
            .expect("no hand card");
        state.applied = vec![json_as(json!({
            "nonce": "before",
            "events": [{ "type": "drawn", "player": "p2", "instanceId": drawn.id, "defId": drawn.def_id, "turnDraw": 1 }],
        }))];
        assert_eq!(named_by(&state, P2, "drawn"), vec![json!(HIDDEN_ID), json!(HIDDEN_ID)]);

        let after = play_glitch(&mut state).state;
        assert_eq!(after.resets, Some(1));
        assert_eq!(named_by(&after, P2, "drawn"), vec![json!(HIDDEN_ID), json!(HIDDEN_ID)]);
    }
}
