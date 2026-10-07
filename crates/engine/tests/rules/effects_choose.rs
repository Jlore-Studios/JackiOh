//! The effects that ask the controller something (BUILD M3-T1 "every effect has its own test file";
//! SPEC §6.3, §10.6, §5.1, R50, R81). `prompts.test.ts` covers the prompt machinery — one at a
//! time, the serializable resume, chaining. This file covers the six factories in
//! `effects/choose.ts` and the two scope readers beside them: what each declares, what it puts in
//! `state.pending`, what it emits, and what it does when there is nothing to offer.
//!
//! Port of `packages/engine/test/effects-choose.test.ts`.

use std::collections::BTreeSet;

use jackioh_engine::effects::{
    TargetScope, choose_cell, choose_from_hand, choose_mode, choose_target, chosen_options, discover_from_catalog,
    discover_from_graveyard, targets_in_scope,
};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{plain, stacker, taunter};
use crate::rules::fixtures::harness::{in_hand, new_game, put, slot};

/// TS `nextIndex = 1300`, bumped once per `def` call in declaration order (caller 1301 … fruit-c 1306).
fn def(name: &str, type_: &str, index: u32, extra: Value) -> CardDef {
    let mut def = json!({
        "id": format!("ec-pick-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (choose)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    });
    if let (Some(def), Some(extra)) = (def.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            def.insert(key.clone(), value.clone());
        }
    }
    json_as(def)
}

fn fruit(name: &str, index: u32) -> CardDef {
    def(
        name,
        "Unit",
        index,
        json!({
            "tags": ["Fruit"],
            "base": { "attack": 1, "health": 1, "keywords": [], "text": name },
            "radiant": { "attack": 2, "health": 2, "keywords": [], "text": name },
        }),
    )
}

/// The unit whose script does the asking.
fn caller() -> CardDef {
    def(
        "caller",
        "Unit",
        1301,
        json!({
            "base": { "attack": 2, "health": 3, "keywords": [], "text": "caller" },
            "radiant": { "attack": 4, "health": 6, "keywords": [], "text": "caller" },
        }),
    )
}
/// A backrow card, for a backrow scope and for `excludeSelf` on one.
fn field_card() -> CardDef {
    def("field", "Field Spell", 1302, json!({}))
}
/// A Discover pool of exactly four, one of which is the card that generates it (§5.1).
fn fruit_generator() -> CardDef {
    fruit("fruit-generator", 1303)
}
fn fruit_a() -> CardDef {
    fruit("fruit-a", 1304)
}
fn fruit_b() -> CardDef {
    fruit("fruit-b", 1305)
}
fn fruit_c() -> CardDef {
    fruit("fruit-c", 1306)
}

fn defs() -> Vec<CardDef> {
    vec![caller(), field_card(), fruit_generator(), fruit_a(), fruit_b(), fruit_c()]
}

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    state.turn = 3;
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    state
}

/// TS `ctxFor(state, self, extra)`: a context for p1 over a fresh sink (an rng at the state's cursor,
/// never written back), `extra`'s fields laid over it, handed to `f`. `self_` is read back from the
/// state as it stands now (TS held the live object).
fn with_ctx<R>(
    state: &mut GameState,
    self_: Option<&CardInstance>,
    extra: impl FnOnce(&mut EffectContext<'_>),
    f: impl FnOnce(&mut EffectContext<'_>) -> R,
) -> R {
    let self_ = self_.map(|card| find_instance(state, &card.id).cloned().unwrap_or_else(|| card.clone()));
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    let mut ctx = make_context(&mut sink, self_.as_ref(), HookOptions { controller: Some(PlayerId::P1), ..Default::default() });
    extra(&mut ctx);
    f(&mut ctx)
}

/// TS `run(ctxFor(state, self, extra), effects)`: each effect applied in turn; the context's events.
fn run_with(
    state: &mut GameState,
    self_: Option<&CardInstance>,
    extra: impl FnOnce(&mut EffectContext<'_>),
    effects: Vec<Effect>,
) -> Vec<GameEvent> {
    with_ctx(state, self_, extra, |ctx| {
        for effect in &effects {
            (effect.apply)(ctx);
        }
        ctx.events.clone()
    })
}

fn run(state: &mut GameState, self_: Option<&CardInstance>, effects: Vec<Effect>) -> Vec<GameEvent> {
    run_with(state, self_, |_ctx| {}, effects)
}

/// Only one prompt may be open at a time (§10.1), so a probe clears the last one first.
fn clear_prompt(state: &mut GameState) {
    state.pending = None;
}

fn ids(selections: &[Selection]) -> Vec<String> {
    selections
        .iter()
        .filter_map(|selection| match selection {
            Selection::Instance { instance_id } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

fn option_selections(state: &GameState) -> Vec<Selection> {
    state.pending.as_ref().map(|pending| pending.options.iter().map(|option| option.selection.clone()).collect()).unwrap_or_default()
}

fn option_keys(state: &GameState) -> Vec<String> {
    state.pending.as_ref().map(|pending| pending.options.iter().map(|option| option.key.clone()).collect()).unwrap_or_default()
}

fn option_labels(state: &GameState) -> Vec<String> {
    state.pending.as_ref().map(|pending| pending.options.iter().map(|option| option.label.clone()).collect()).unwrap_or_default()
}

/// The `mode` options' picks (TS `option.selection.pick === "mode" ? [option.selection.option] : []`).
fn mode_options(state: &GameState) -> Vec<String> {
    option_selections(state)
        .into_iter()
        .filter_map(|selection| match selection {
            Selection::Mode { option } => Some(option),
            _ => None,
        })
        .collect()
}

fn sorted(mut items: Vec<String>) -> Vec<String> {
    items.sort();
    items
}

fn scope(value: Value) -> TargetScope {
    json_as(value)
}

/// Jest's `toMatchObject`: every key `expected` names is in `actual` with a matching value (objects
/// recursively, arrays element by element and of the same length, anything else equal).
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

fn expect_pending_matches(state: &GameState, expected: Value) {
    let actual = serde_json::to_value(&state.pending).unwrap();
    assert!(matches_object(&actual, &expected), "pending {actual} does not match {expected}");
}

mod target_scopes_s10_6_r13 {
    use super::*;

    #[test]
    fn offers_every_active_unit_by_default_the_chooser_s_side_first_in_lane_order() {
        // §10.6.
        let mut state = game("scope-default");
        let mine_a = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mine_b = put(&mut state, &taunter.id, slot(PlayerId::P1, Row::Units, 3), json!({}));
        let theirs = put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, 2), json!({}));

        // §3.2 and R13: a card dormant under a Stack is not on the field, so it is never offered.
        let buried = put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, 1), json!({}));
        let mut top = new_instance(&mut state, &stacker.id, PlayerId::P2, Zone::Hand { player: PlayerId::P2 });
        assert!(place_on_field(&mut state, &mut top, slot(PlayerId::P2, Row::Units, 1), json_as(json!({ "stack": true }))));

        let self_ = Some(&mine_a);
        let offered = with_ctx(&mut state, self_, |_ctx| {}, |ctx| targets_in_scope(ctx, None));
        assert_eq!(ids(&offered), vec![mine_a.id.clone(), mine_b.id.clone(), top.id.clone(), theirs.id.clone()]);
        assert!(!ids(&offered).contains(&buried.id));
    }

    #[test]
    fn narrows_a_scope_by_side_and_offers_heroes_and_backrow_cards_when_it_names_them() {
        // §10.6.
        let mut state = game("scope-sides");
        let mine = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let theirs = put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, 1), json!({}));
        let my_backrow = put(&mut state, &field_card().id, slot(PlayerId::P1, Row::Backrow, 2), json!({}));
        let their_backrow = put(&mut state, &field_card().id, slot(PlayerId::P2, Row::Backrow, 5), json!({}));
        let self_ = Some(&mine);
        with_ctx(&mut state, self_, |_ctx| {}, |ctx| {
            assert_eq!(ids(&targets_in_scope(ctx, Some(&scope(json!({ "side": "ally" }))))), vec![mine.id.clone()]);
            assert_eq!(ids(&targets_in_scope(ctx, Some(&scope(json!({ "side": "enemy" }))))), vec![theirs.id.clone()]);
            assert_eq!(
                targets_in_scope(ctx, Some(&scope(json!({ "of": ["hero"] })))),
                vec![Selection::Hero { player: PlayerId::P1 }, Selection::Hero { player: PlayerId::P2 }]
            );
            assert_eq!(
                ids(&targets_in_scope(ctx, Some(&scope(json!({ "of": ["backrow"] }))))),
                vec![my_backrow.id.clone(), their_backrow.id.clone()]
            );
            // Within one side the kinds come in the order the scope lists them in the module: units,
            // backrow, then the hero.
            assert_eq!(
                targets_in_scope(ctx, Some(&scope(json!({ "side": "enemy", "of": ["unit", "backrow", "hero"] })))),
                vec![
                    Selection::Instance { instance_id: theirs.id.clone() },
                    Selection::Instance { instance_id: their_backrow.id.clone() },
                    Selection::Hero { player: PlayerId::P2 },
                ]
            );
        });
    }

    #[test]
    fn exclude_self_drops_the_card_running_the_script_from_a_unit_and_from_a_backrow_scope() {
        // §10.6.
        let mut state = game("scope-exclude-self");
        let self_card = put(&mut state, &caller().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let other = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        let self_backrow = put(&mut state, &field_card().id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        let other_backrow = put(&mut state, &field_card().id, slot(PlayerId::P1, Row::Backrow, 2), json!({}));

        let unit_self = Some(&self_card);
        with_ctx(&mut state, unit_self, |_ctx| {}, |ctx| {
            assert_eq!(
                ids(&targets_in_scope(ctx, Some(&scope(json!({ "side": "ally", "excludeSelf": true }))))),
                vec![other.id.clone()]
            );
            assert_eq!(
                ids(&targets_in_scope(ctx, Some(&scope(json!({ "side": "ally" }))))),
                vec![self_card.id.clone(), other.id.clone()]
            );
        });

        let backrow_self = Some(&self_backrow);
        with_ctx(&mut state, backrow_self, |_ctx| {}, |ctx| {
            assert_eq!(
                ids(&targets_in_scope(ctx, Some(&scope(json!({ "side": "ally", "of": ["backrow"], "excludeSelf": true }))))),
                vec![other_backrow.id.clone()]
            );
        });
        // With no instance running the script the flag excludes nothing (a Spell resolving).
        with_ctx(&mut state, None, |_ctx| {}, |ctx| {
            assert_eq!(
                ids(&targets_in_scope(ctx, Some(&scope(json!({ "side": "ally", "excludeSelf": true }))))),
                vec![self_card.id.clone(), other.id.clone()]
            );
        });
    }

    #[test]
    fn r81_reads_a_prompt_s_mode_pick_out_of_ctx_targets_and_a_play_s_modes_out_of_ctx_modes() {
        let mut state = game("chosen-options");
        let self_card = put(&mut state, &caller().id, slot(PlayerId::P1, Row::Units, 1), json!({}));

        // A Discover's answer arrives as a `mode` selection carrying a def id (§10.6).
        let answered = with_ctx(
            &mut state,
            Some(&self_card),
            |ctx| {
                ctx.targets = vec![
                    Selection::Mode { option: fruit_a().id },
                    Selection::Instance { instance_id: self_card.id.clone() },
                    Selection::Hero { player: PlayerId::P1 },
                    Selection::None,
                ];
                ctx.modes = vec!["left".to_string()];
            },
            |ctx| chosen_options(ctx),
        );
        // The prompt's picks come first, then the play's modes; nothing else is read.
        assert_eq!(answered, vec![fruit_a().id, "left".to_string()]);
        let plain_ctx = with_ctx(&mut state, Some(&self_card), |_ctx| {}, |ctx| chosen_options(ctx));
        assert_eq!(plain_ctx, Vec::<String>::new());
        let with_modes = with_ctx(
            &mut state,
            Some(&self_card),
            |ctx| ctx.modes = vec!["burn".to_string(), "freeze".to_string()],
            |ctx| chosen_options(ctx),
        );
        assert_eq!(with_modes, vec!["burn".to_string(), "freeze".to_string()]);
    }
}

mod the_choose_effects_s6_3_s10_6_m3_t1 {
    use super::*;

    #[test]
    fn choose_one_opens_a_mode_prompt_that_resumes_the_step_it_names() {
        // §6.3.
        let mut state = game("choose-mode");
        let self_card = put(&mut state, &caller().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let effect = choose_mode(json_as(json!({ "options": ["burn", "heal"], "step": "picked" })));
        assert_eq!(effect.kind, "chooseMode");

        let events = run(&mut state, Some(&self_card), vec![effect]);
        expect_pending_matches(
            &state,
            json!({
                "playerId": "p1",
                "kind": "mode",
                "prompt": "Choose one",
                "min": 1,
                "max": 1,
                "resume": {
                    "defId": caller().id, "hook": "resume", "step": "picked", "radiant": false,
                    "instanceId": self_card.id, "data": {},
                },
            }),
        );
        assert_eq!(
            serde_json::to_value(&state.pending.as_ref().expect("a prompt").options).unwrap(),
            json!([
                { "key": "mode:burn", "label": "burn", "selection": { "pick": "mode", "option": "burn" } },
                { "key": "mode:heal", "label": "heal", "selection": { "pick": "mode", "option": "heal" } },
            ])
        );
        let opened: Vec<&GameEvent> =
            events.iter().filter(|event| event.event_type() == GameEventType::PromptOpened).collect();
        let choice_id = state.pending.as_ref().map(|pending| pending.id.clone()).unwrap_or_default();
        assert_eq!(
            serde_json::to_value(opened).unwrap(),
            json!([{ "type": "promptOpened", "player": "p1", "choiceId": choice_id, "kind": "mode" }])
        );

        // Its own prompt text, the radiant face and the data a chain has captured all carry through.
        clear_prompt(&mut state);
        find_instance_mut(&mut state, &self_card.id).expect("the caller").radiant = true;
        run_with(
            &mut state,
            Some(&self_card),
            |ctx| ctx.data = IndexMap::from([("mode".to_string(), json!("burn"))]),
            vec![choose_mode(json_as(json!({
                "options": ["a"], "step": "again", "prompt": "Which one?", "data": { "round": 2 }
            })))],
        );
        expect_pending_matches(
            &state,
            json!({
                "prompt": "Which one?",
                "resume": { "step": "again", "radiant": true, "data": { "mode": "burn", "round": 2 } },
            }),
        );
    }

    #[test]
    fn choose_target_offers_the_scope_it_names_labelled_and_fizzles_on_an_empty_scope_s6_3() {
        // §10.6.
        let mut state = game("choose-target");
        let self_card = put(&mut state, &caller().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let enemy = put(&mut state, &taunter.id, slot(PlayerId::P2, Row::Units, 1), json!({}));
        let enemy_backrow = put(&mut state, &field_card().id, slot(PlayerId::P2, Row::Backrow, 3), json!({}));
        let effect =
            choose_target(json_as(json!({ "step": "zap", "scope": { "side": "enemy", "of": ["unit", "backrow", "hero"] } })));
        assert_eq!(effect.kind, "chooseTarget");

        run(&mut state, Some(&self_card), vec![effect]);
        expect_pending_matches(&state, json!({ "kind": "target", "prompt": "Choose a target", "min": 1, "max": 1 }));
        // Each option is labelled with the card's name, or, for a hero, whose it is to the chooser, and keyed by what
        // it selects, so two cards of one name are still two keys (§10.6: the key is what is sent back).
        assert_eq!(
            serde_json::to_value(&state.pending.as_ref().expect("a prompt").options).unwrap(),
            json!([
                {
                    "key": format!("instance:{}", enemy.id),
                    "label": taunter.name,
                    "selection": { "pick": "instance", "instanceId": enemy.id },
                },
                {
                    "key": format!("instance:{}", enemy_backrow.id),
                    "label": field_card().name,
                    "selection": { "pick": "instance", "instanceId": enemy_backrow.id },
                },
                { "key": "hero:p2", "label": "Enemy hero", "selection": { "pick": "hero", "player": "p2" } },
            ])
        );

        // An empty scope opens no prompt and emits nothing: the effect fizzles and the card resolves.
        clear_prompt(&mut state);
        let mut empty_board = game("choose-target-empty");
        let lonely = put(&mut empty_board, &caller().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let quiet = run(
            &mut empty_board,
            Some(&lonely),
            vec![choose_target(json_as(json!({
                "step": "zap", "scope": { "side": "enemy", "of": ["unit"] }, "prompt": "Zap what?"
            })))],
        );
        assert!(empty_board.pending.is_none());
        assert_eq!(quiet, Vec::<GameEvent>::new());
    }

    #[test]
    fn a_target_or_cell_prompt_names_heroes_and_cells_to_its_chooser_your_or_enemy_never_by_seat() {
        // §10.6.
        let mut state = game("choose-relative");
        let mine = put(&mut state, &caller().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let theirs = put(&mut state, &caller().id, slot(PlayerId::P2, Row::Units, 1), json!({}));

        run(
            &mut state,
            Some(&mine),
            vec![choose_target(json_as(json!({ "step": "zap", "scope": { "side": "any", "of": ["hero"] } })))],
        );
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(PlayerId::P1));
        assert_eq!(option_labels(&state), vec!["Your hero".to_string(), "Enemy hero".to_string()]);
        assert_eq!(option_keys(&state), vec!["hero:p1".to_string(), "hero:p2".to_string()]);

        // The same prompt held by p2 reads the other way round; the keys name the seats and do not move.
        clear_prompt(&mut state);
        run_with(
            &mut state,
            Some(&theirs),
            |ctx| ctx.controller = PlayerId::P2,
            vec![choose_target(json_as(json!({ "step": "zap", "scope": { "side": "any", "of": ["hero"] } })))],
        );
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(PlayerId::P2));
        assert_eq!(option_labels(&state), vec!["Enemy hero".to_string(), "Your hero".to_string()]);
        assert_eq!(option_keys(&state), vec!["hero:p1".to_string(), "hero:p2".to_string()]);

        clear_prompt(&mut state);
        run(
            &mut state,
            Some(&mine),
            vec![choose_cell(json_as(json!({ "step": "cell", "cells": { "rows": ["units"], "exceptLanes": [2, 3, 4, 5] } })))],
        );
        assert_eq!(option_labels(&state), vec!["Your Unit lane 1".to_string(), "Enemy Unit lane 1".to_string()]);
        assert_eq!(option_keys(&state), vec!["zone:p1:units:1".to_string(), "zone:p2:units:1".to_string()]);

        // A cell prompt the enemy holds names the cells to them: p1's are "Enemy", p2's are "Your".
        clear_prompt(&mut state);
        run(
            &mut state,
            Some(&mine),
            vec![choose_cell(json_as(json!({
                "step": "cell", "by": "enemy", "cells": { "rows": ["units"], "exceptLanes": [2, 3, 4, 5] }
            })))],
        );
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(PlayerId::P2));
        assert_eq!(option_labels(&state), vec!["Enemy Unit lane 1".to_string(), "Your Unit lane 1".to_string()]);
    }

    #[test]
    fn c26_choose_from_hand_offers_your_own_hand_only_and_asks_for_what_the_hand_can_give() {
        // #26.
        let mut state = game("choose-hand");
        let self_card = put(&mut state, &caller().id, slot(PlayerId::P1, Row::Units, 1), json!({}));

        // An empty hand opens nothing at all.
        let empty = run(&mut state, Some(&self_card), vec![choose_from_hand(json_as(json!({ "step": "picked" })))]);
        assert!(state.pending.is_none());
        assert_eq!(empty, Vec::<GameEvent>::new());

        let mine = in_hand(&mut state, &plain.id, PlayerId::P1, 3);
        in_hand(&mut state, &taunter.id, PlayerId::P2, 2);
        run(
            &mut state,
            Some(&self_card),
            vec![choose_from_hand(json_as(json!({ "step": "picked", "count": 2, "prompt": "Discard two" })))],
        );
        expect_pending_matches(
            &state,
            json!({
                "kind": "hand",
                "prompt": "Discard two",
                "min": 2,
                "max": 2,
                "resume": { "step": "picked", "instanceId": self_card.id },
            }),
        );
        assert_eq!(ids(&option_selections(&state)), mine.iter().map(|card| card.id.clone()).collect::<Vec<_>>());
        assert_eq!(option_keys(&state), mine.iter().map(|card| format!("instance:{}", card.id)).collect::<Vec<_>>());
        for theirs in &state.players[PlayerId::P2].hand {
            assert!(!ids(&option_selections(&state)).contains(&theirs.id));
        }

        // A count above the hand size asks for the whole hand rather than for an impossible number.
        clear_prompt(&mut state);
        run(&mut state, Some(&self_card), vec![choose_from_hand(json_as(json!({ "step": "picked", "count": 9 })))]);
        expect_pending_matches(&state, json!({ "min": 3, "max": 3 }));

        // With no count stated it asks for one, under the default prompt text.
        clear_prompt(&mut state);
        run(&mut state, Some(&self_card), vec![choose_from_hand(json_as(json!({ "step": "picked" })))]);
        expect_pending_matches(&state, json!({ "prompt": "Choose a card in your hand", "min": 1, "max": 1 }));
    }

    #[test]
    fn discover_offers_three_from_the_pool_it_names_never_the_card_that_generated_it_s5_1() {
        // §6.3.
        let mut state = game("discover-catalog");
        let self_card = put(&mut state, &fruit_generator().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let effect = discover_from_catalog(json_as(json!({ "step": "chosen", "query": { "tags": ["Fruit"] } })));
        assert_eq!(effect.kind, "discoverFromCatalog");

        run(&mut state, Some(&self_card), vec![effect]);
        expect_pending_matches(
            &state,
            json!({
                "kind": "discover",
                "prompt": "Discover a card",
                "min": 1,
                "max": 1,
                "resume": { "step": "chosen", "instanceId": self_card.id },
            }),
        );
        let offered = mode_options(&state);
        // The Fruit pool is four cards and one of them is the generator, so it is exactly the other
        // three — no repeats, and never itself.
        assert_eq!(sorted(offered.clone()), sorted(vec![fruit_a().id, fruit_b().id, fruit_c().id]));
        assert!(!offered.contains(&fruit_generator().id));
        assert_eq!(option_keys(&state), offered.iter().map(|id| format!("mode:{id}")).collect::<Vec<_>>());

        // The same seed and the same pool offer them in the same order (§10.7).
        let mut twin = game("discover-catalog");
        let twin_self = put(&mut twin, &fruit_generator().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        run(
            &mut twin,
            Some(&twin_self),
            vec![discover_from_catalog(json_as(json!({ "step": "chosen", "query": { "tags": ["Fruit"] } })))],
        );
        assert_eq!(option_keys(&twin), option_keys(&state));

        // A narrower count offers fewer, and a pool with nothing in it opens no prompt (it fizzles).
        clear_prompt(&mut state);
        run(
            &mut state,
            Some(&self_card),
            vec![discover_from_catalog(json_as(json!({
                "step": "chosen", "query": { "tags": ["Fruit"] }, "count": 2, "prompt": "Pick a fruit"
            })))],
        );
        assert_eq!(state.pending.as_ref().map(|pending| pending.options.len()), Some(2));
        assert_eq!(state.pending.as_ref().map(|pending| pending.prompt.clone()), Some("Pick a fruit".to_string()));

        clear_prompt(&mut state);
        let fizzle = run(
            &mut state,
            Some(&self_card),
            vec![discover_from_catalog(json_as(json!({ "step": "chosen", "query": { "cost": 99 } })))],
        );
        assert!(state.pending.is_none());
        assert_eq!(fizzle, Vec::<GameEvent>::new());

        // With no query the pool is the whole catalog, and with no instance running the script there
        // is no generating card to exclude (§5.1's exclusion is the card's own index).
        clear_prompt(&mut state);
        run(&mut state, None, vec![discover_from_catalog(json_as(json!({ "step": "chosen" })))]);
        let wide = mode_options(&state);
        assert_eq!(wide.len(), 3);
        assert_eq!(wide.iter().collect::<BTreeSet<_>>().len(), 3);
        for id in &wide {
            assert_eq!(registered_catalog().get(id).map(|def| def.token), Some(false));
        }
        let resume = serde_json::to_value(&state.pending.as_ref().expect("a prompt").resume).unwrap();
        assert!(matches_object(&resume, &json!({ "defId": "", "step": "chosen" })));
        assert!(state.pending.as_ref().expect("a prompt").resume.instance_id.is_none());
    }

    #[test]
    fn r247_a_discover_of_numbers_offers_the_same_three_cards_by_their_indices_and_the_view_names_none_of_them() {
        let mut by_card = game("discover-numbers");
        let self_card = put(&mut by_card, &fruit_generator().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        run(
            &mut by_card,
            Some(&self_card),
            vec![discover_from_catalog(json_as(json!({ "step": "chosen", "query": { "tags": ["Fruit"] } })))],
        );
        let cards = mode_options(&by_card);

        // The same seed and pool, offered as numbers: the draw is untouched, only what each option is.
        let mut by_number = game("discover-numbers");
        let number_self = put(&mut by_number, &fruit_generator().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        run(
            &mut by_number,
            Some(&number_self),
            vec![discover_from_catalog(json_as(json!({
                "step": "chosen", "query": { "tags": ["Fruit"] }, "offer": "index"
            })))],
        );
        let indices: Vec<String> = cards
            .iter()
            .map(|id| registered_catalog().get(id).map(|def| def.index.clone()).unwrap_or_default())
            .collect();
        assert_eq!(
            option_selections(&by_number),
            indices.iter().map(|index| Selection::Mode { option: index.clone() }).collect::<Vec<_>>()
        );
        assert_eq!(option_keys(&by_number), indices.iter().map(|index| format!("mode:{index}")).collect::<Vec<_>>());
        assert_eq!(option_labels(&by_number), indices);

        // §10.8: the chooser's view carries each number and no definition behind it.
        let Some(PendingView::ForYou(pending)) = view_for(&by_number, PlayerId::P1).pending else {
            panic!("p1 holds the prompt");
        };
        assert_eq!(
            serde_json::to_value(&pending.options).unwrap(),
            Value::Array(indices.iter().map(|index| json!({ "key": format!("mode:{index}"), "label": index })).collect())
        );
    }

    #[test]
    fn r50_discover_from_the_graveyard_offers_the_cards_actually_in_your_own_graveyard() {
        let mut state = game("discover-graveyard");
        let self_card = put(&mut state, &caller().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let effect = discover_from_graveyard(json_as(json!({ "step": "chosen" })));
        assert_eq!(effect.kind, "discoverFromGraveyard");

        // An empty graveyard opens nothing.
        let empty = run(&mut state, Some(&self_card), vec![effect.clone()]);
        assert!(state.pending.is_none());
        assert_eq!(empty, Vec::<GameEvent>::new());

        let mine: Vec<CardInstance> = [plain.id.clone(), taunter.id.clone(), field_card().id]
            .iter()
            .map(|def_id| {
                let card = new_instance(&mut state, def_id, PlayerId::P1, Zone::Graveyard { player: PlayerId::P1 });
                state.players[PlayerId::P1].graveyard.push(card.clone());
                card
            })
            .collect();
        let theirs = new_instance(&mut state, &plain.id, PlayerId::P2, Zone::Graveyard { player: PlayerId::P2 });
        state.players[PlayerId::P2].graveyard.push(theirs.clone());

        run(&mut state, Some(&self_card), vec![effect]);
        expect_pending_matches(&state, json!({ "kind": "discover", "prompt": "Discover a card from your graveyard" }));
        let offered = ids(&option_selections(&state));
        // R50: the options are the actual instances there, so a spell token in the graveyard is
        // eligible, and the opponent's graveyard is never offered.
        assert_eq!(sorted(offered.clone()), sorted(mine.iter().map(|card| card.id.clone()).collect()));
        assert!(!offered.contains(&theirs.id));
        assert_eq!(
            option_labels(&state),
            offered
                .iter()
                .map(|id| {
                    let def_id = mine.iter().find(|card| card.id == *id).map(|card| card.def_id.clone()).unwrap_or_default();
                    registered_catalog().get(&def_id).map(|def| def.name.clone()).unwrap_or_default()
                })
                .collect::<Vec<_>>()
        );

        clear_prompt(&mut state);
        run(
            &mut state,
            Some(&self_card),
            vec![discover_from_graveyard(json_as(json!({ "step": "chosen", "count": 2, "prompt": "Raise two" })))],
        );
        assert_eq!(state.pending.as_ref().map(|pending| pending.options.len()), Some(2));
        assert_eq!(state.pending.as_ref().map(|pending| pending.prompt.clone()), Some("Raise two".to_string()));
    }
}
