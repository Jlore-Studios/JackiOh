//! Declared numbers (docs/classic-sets.md B3.4 rule 5, R386): `param(ctx, key)` in a card script, the
//! pure `paramValue` the view and a `preview` hook read, the default steps and the bounds, a number
//! KY's Constant set and the steps after it, a fused card's ingredients each reading their own
//! declaration (R102), a number tuned on the Radiant face only (R749), and a card resolving with the
//! number as it stands.
//!
//! Port of `packages/engine/test/params.test.ts`. TS handed `param` plain objects shaped like a
//! context (`{ state, self, radiant, defId, data }`); here each is an `EffectContext` with those
//! fields set. A TS `throw` is a panic with the same message (SURFACE §4.4.9), caught by `panic_text`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use jackioh_engine::effects::tune::TuneDirection;
use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};

use crate::rules::fixtures::harness::{in_hand, put, sink_for, slot};
use crate::rules::fixtures::instance_data::{body, instance_game, nerfer, numbered, radiant_number};

fn game() -> GameState {
    let mut state = instance_game("params", None);
    state.turn = 3;
    state.active = P1;
    state
}

/// The one card `in_hand` added (TS `const [card] = inHand(…)`).
fn one(cards: Vec<CardInstance>) -> CardInstance {
    cards.into_iter().next().expect("no card")
}

/// The card as it stands in `state` now (TS held the live object).
fn live(state: &GameState, card: &CardInstance) -> CardInstance {
    find_instance(state, &card.id)
        .cloned()
        .unwrap_or_else(|| panic!("no card {}", card.id))
}

/// The card in `state`, to write through (TS wrote to the live object).
fn live_mut<'a>(state: &'a mut GameState, card: &CardInstance) -> &'a mut CardInstance {
    find_instance_mut(state, &card.id).unwrap_or_else(|| panic!("no card {}", card.id))
}

/// TS `paramValue(state, card, key)`, on the card as it stands.
fn value(state: &GameState, card: &CardInstance, key: &str) -> i32 {
    param_value(state, Some(&live(state, card)), key, ParamValueOptions::default())
}

/// TS `{ [PART_KEY]: path }`.
fn part(path: &[usize]) -> IndexMap<String, Value> {
    let mut data = IndexMap::new();
    data.insert(PART_KEY.to_string(), json!(path));
    data
}

/// The message a TS `throw` (a Rust panic) carried.
fn panic_text(run: impl FnOnce() -> i32) -> String {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)) {
        Ok(value) => panic!("expected a throw, got {value}"),
        Err(payload) => payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|text| text.to_string()))
            .unwrap_or_default(),
    }
}

/// TS `{ state, self: null, radiant, defId? }`: a context with no card of its own.
fn bare_context<'a>(state: &'a mut GameState, radiant: bool, def_id: Option<&str>) -> EffectContext<'a> {
    let mut ctx = EffectContext::new(sink_for(state), P1);
    ctx.self_ = None;
    ctx.radiant = radiant;
    ctx.def_id = def_id.map(str::to_string);
    ctx
}

/// B3.4 rule 5: declared numbers (R386)
mod r386_b3_4_rule_5_declared_numbers {
    use super::*;

    #[test]
    fn r386_a_number_reads_its_face_s_printed_value_until_the_card_s_tuning_moves_it() {
        let mut state = game();
        let card = one(in_hand(&mut state, &numbered.id, P1, 1));
        assert_eq!(value(&state, &card, "damage"), 2);
        live_mut(&mut state, &card).radiant = true;
        assert_eq!(value(&state, &card, "damage"), 4);
        assert_eq!(
            param_value(
                &state,
                Some(&live(&state, &card)),
                "damage",
                ParamValueOptions {
                    radiant: Some(false),
                    ..ParamValueOptions::default()
                }
            ),
            2
        );
        step_param(live_mut(&mut state, &card), "damage", 1);
        let mut numbers = IndexMap::new();
        numbers.insert("damage".to_string(), 1);
        assert_eq!(
            live(&state, &card).tuning,
            Some(Tuning {
                numbers: Some(numbers),
                ..Tuning::default()
            })
        );
        assert_eq!(value(&state, &card, "damage"), 5);
        step_param(live_mut(&mut state, &card), "damage", -1);
        // Steps that cancel out leave the card untuned (§9.3: it hashes as a card never touched).
        assert_eq!(live(&state, &card).tuning, None);
    }

    #[test]
    fn r749_a_number_tuned_on_radiant_reads_its_printed_value_on_the_base_face_whatever_its_steps_and_steps_on_the_radiant_face()
     {
        let mut state = game();
        let card = one(in_hand(&mut state, &radiant_number.id, P1, 1));
        // The base face prints no number: no change finds one to move, KY's Constant lists none, and a
        // step recorded anyway leaves it at its printed 1.
        assert!(steppable_params(&state, &live(&state, &card), TuneDirection::Upgrade).is_empty());
        assert!(steppable_params(&state, &live(&state, &card), TuneDirection::Degrade).is_empty());
        assert!(
            !numbers_on(&state, &live(&state, &card))
                .iter()
                .any(|entry| entry.id == "param:times")
        );
        step_param(live_mut(&mut state, &card), "times", 1);
        assert_eq!(value(&state, &card, "times"), 1);
        set_param(live_mut(&mut state, &card), "times", 3);
        assert_eq!(value(&state, &card, "times"), 1);
        // The Radiant face prints it, so it is tuned as any number is.
        let shining = one(in_hand(&mut state, &radiant_number.id, P1, 1));
        live_mut(&mut state, &shining).radiant = true;
        assert_eq!(value(&state, &shining, "times"), 2);
        assert_eq!(
            steppable_params(&state, &live(&state, &shining), TuneDirection::Upgrade)
                .iter()
                .map(|entry| entry.delta)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            numbers_on(&state, &live(&state, &shining))
                .iter()
                .find(|entry| entry.id == "param:times")
                .map(|entry| entry.value),
            Some(2)
        );
        step_param(live_mut(&mut state, &shining), "times", 1);
        assert_eq!(value(&state, &shining, "times"), 3);
    }

    #[test]
    fn r386_the_default_step_is_1_up_to_5_2_from_6_to_12_and_a_quarter_rounded_above_a_declared_step_wins() {
        let def = def_of(Some(&game()), &numbered.id).clone();
        let by_key = |key: &str| -> Param {
            def.params
                .as_ref()
                .and_then(|params| params.iter().find(|p| p.key == key))
                .cloned()
                .unwrap_or_else(|| panic!("{key}"))
        };
        assert_eq!(PARAM_DEFAULT_STEP.small.step, 1);
        assert_eq!(param_step(&by_key("damage"), 2), 1);
        assert_eq!(param_step(&by_key("big"), 8), 2);
        assert_eq!(param_step(&by_key("big"), 16), 4);
        assert_eq!(param_step(&by_key("huge"), 20), 5);
        let stepped = Param {
            step: Some(3),
            ..by_key("huge")
        };
        assert_eq!(param_step(&stepped, 20), 3);
    }

    #[test]
    fn r386_an_amount_never_drops_below_1_or_its_declared_min_and_never_rises_above_its_max() {
        let mut state = game();
        let card = one(in_hand(&mut state, &numbered.id, P1, 1));
        step_param(live_mut(&mut state, &card), "damage", -10);
        assert_eq!(value(&state, &card, "damage"), TUNE_MIN_AMOUNT);
        step_param(live_mut(&mut state, &card), "threshold", -10);
        assert_eq!(value(&state, &card, "threshold"), 2);
        step_param(live_mut(&mut state, &card), "huge", 100);
        assert_eq!(value(&state, &card, "huge"), 44);
    }

    #[test]
    fn r386_a_number_set_outright_is_the_value_from_then_on_and_a_later_step_counts_from_it() {
        let mut state = game();
        let card = one(in_hand(&mut state, &numbered.id, P1, 1));
        step_param(live_mut(&mut state, &card), "big", 2);
        set_param(live_mut(&mut state, &card), "big", 3);
        assert_eq!(value(&state, &card, "big"), 3);
        let mut set = IndexMap::new();
        set.insert("big".to_string(), 3);
        assert_eq!(
            live(&state, &card).tuning,
            Some(Tuning {
                set: Some(set),
                ..Tuning::default()
            })
        );
        step_param(live_mut(&mut state, &card), "big", 1);
        // The step is read off the face's printed number (8: 2), so one step from 3 is 5.
        assert_eq!(value(&state, &card, "big"), 5);
    }

    #[test]
    fn r386_param_ctx_key_reads_the_running_card_on_the_face_that_runs_a_card_that_has_ceased_to_exist_reads_its_printed_value() {
        let mut state = game();
        let card = one(in_hand(&mut state, &numbered.id, P1, 1));
        step_param(live_mut(&mut state, &card), "damage", 2);
        let now = live(&state, &card);
        {
            let mut sink = sink_for(&mut state);
            let mut ctx = make_context(&mut sink, Some(&now), HookOptions::default());
            assert_eq!(param(&ctx, "damage"), 4);
            // TS `{ ...ctx, radiant: true }`.
            ctx.radiant = true;
            assert_eq!(param(&ctx, "damage"), 6);
            ctx.radiant = false;
            let message = panic_text(|| param(&ctx, "nope"));
            assert!(message.contains("declares no number \"nope\""), "{message}");
        }
        {
            let ctx = bare_context(&mut state, true, Some(&numbered.id));
            assert_eq!(param(&ctx, "damage"), 4);
        }
        {
            let ctx = bare_context(&mut state, false, None);
            let message = panic_text(|| param(&ctx, "damage"));
            assert!(message.contains("no card"), "{message}");
        }
    }

    #[test]
    fn r102_on_a_fused_card_each_ingredient_s_text_reads_its_own_declaration_and_the_card_declares_the_union() {
        let mut state = game();
        let kept = put(&mut state, &body.id, slot(P1, Row::Units, 1), json!({}));
        let a = one(in_hand(&mut state, &numbered.id, P1, 1));
        let b = one(in_hand(&mut state, &nerfer.id, P1, 1));
        let fused = {
            let mut sink = sink_for(&mut state);
            fuse(
                &mut sink,
                FuseArgs {
                    ingredients: vec![a, b],
                    target: Some(kept),
                    ..FuseArgs::default()
                },
            )
        }
        .expect("expected a fusion");
        assert_eq!(
            params_of(&state, &fused.def_id)
                .iter()
                .map(|p| p.key.clone())
                .collect::<Vec<_>>(),
            vec!["damage", "threshold", "big", "huge", "times"]
        );
        let view = params_view(&state, &live(&state, &fused));
        {
            let mut sink = sink_for(&mut state);
            let mut ctx = make_context(&mut sink, Some(&fused), HookOptions::default());
            // Ingredient 0 is the numbered spell, 1 Book of Nerf's shape, and the kept body comes last (R77).
            ctx.data = part(&[0]);
            assert_eq!(param(&ctx, "damage"), 2);
            ctx.data = part(&[1]);
            assert_eq!(param(&ctx, "times"), 3);
            let message = panic_text(|| param(&ctx, "damage"));
            assert!(message.contains("declares no number"), "{message}");
        }
        let mut expected = IndexMap::new();
        for (key, number) in [("damage", 2), ("threshold", 3), ("big", 8), ("huge", 20), ("times", 3)] {
            expected.insert(key.to_string(), number);
        }
        assert_eq!(view, Some(expected));
    }

    #[test]
    fn r102_a_part_path_that_names_no_ingredient_is_an_error_not_the_numbers_of_the_card_it_stopped_at() {
        let mut state = game();
        let kept = put(&mut state, &body.id, slot(P1, Row::Units, 1), json!({}));
        let a = one(in_hand(&mut state, &numbered.id, P1, 1));
        let b = one(in_hand(&mut state, &nerfer.id, P1, 1));
        let fused = {
            let mut sink = sink_for(&mut state);
            fuse(
                &mut sink,
                FuseArgs {
                    ingredients: vec![a, b],
                    target: Some(kept),
                    ..FuseArgs::default()
                },
            )
        }
        .expect("expected a fusion");
        let mut sink = sink_for(&mut state);
        let mut ctx = make_context(&mut sink, Some(&fused), HookOptions::default());
        // Three ingredients: index 3 is past them, and ingredient 0 is a catalog card with no ingredients of its own.
        ctx.data = part(&[3]);
        let past = panic_text(|| param(&ctx, "damage"));
        assert!(past.contains("no ingredient 3 on the part path 3"), "{past}");
        ctx.data = part(&[0, 0]);
        let nested = panic_text(|| param(&ctx, "damage"));
        assert!(nested.contains("no ingredient 0 on the part path 0.0"), "{nested}");
    }

    #[test]
    fn r386_a_card_that_declares_no_number_carries_no_params_in_its_view() {
        let mut state = game();
        put(&mut state, &body.id, slot(P1, Row::Units, 1), json!({}));
        let card = one(in_hand(&mut state, &numbered.id, P1, 1));
        let view = view_for(&state, P1);
        assert_eq!(view.you.units[0].as_ref().and_then(|unit| unit.params.clone()), None);
        let HandView::Cards(hand) = &view.you.hand else {
            panic!("own hand is a list");
        };
        let mut expected = IndexMap::new();
        for (key, number) in [("damage", 2), ("threshold", 3), ("big", 8), ("huge", 20)] {
            expected.insert(key.to_string(), number);
        }
        assert_eq!(
            hand.iter()
                .find(|c| c.instance_id == card.id)
                .and_then(|c| c.params.clone()),
            Some(expected)
        );
    }
}

/// TS's module `let nonce`.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn act(state: &GameState, action: ActionInput) -> GameState {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let result = reduce(state, &action.with_nonce(format!("pm{nonce}")));
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result.state
}

/// B3.4 rule 5: a card resolves with its number as it stands (R386)
mod r386_b3_4_rule_5_a_card_resolves_with_its_number_as_it_stands {
    use super::*;

    #[test]
    fn r386_a_spell_that_reads_param_ctx_damage_deals_its_tuned_amount() {
        let mut state = begin_game(&instance_game("param-play", None)).state;
        let keep: Vec<String> = state.players.p1.hand.iter().map(|c| c.id.clone()).collect();
        state = act(
            &state,
            ActionInput {
                body: ActionBody::Mulligan { keep },
                player_id: P1,
            },
        );
        let keep: Vec<String> = state.players.p2.hand.iter().map(|c| c.id.clone()).collect();
        state = act(
            &state,
            ActionInput {
                body: ActionBody::Mulligan { keep },
                player_id: P2,
            },
        );
        let card = one(in_hand(&mut state, &numbered.id, P1, 1));
        step_param(live_mut(&mut state, &card), "damage", 3);
        state.players.p1.mana.current = 4;
        state.players.p1.mana.max = 4;
        let before = state.players.p2.hero.health;
        let play = legal_actions(&state, P1)
            .into_iter()
            .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == card.id))
            .expect("expected a play");
        state = act(
            &state,
            ActionInput {
                body: play,
                player_id: P1,
            },
        );
        assert_eq!(state.players.p2.hero.health, before - 5);
    }
}
