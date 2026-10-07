//! Degrade and Upgrade (docs/classic-sets.md B3.4; R386, R440, R442) and KY's Constant's number set
//! outright (Classic+ #41): every row of the menu with its bounds, the draw (the row, then the item),
//! Immutable, "N times", random picks over hidden piles and their cues, R242's order, what the views
//! show and hide (R177, R311), and how the changes ride the card — through leaving the field (R78),
//! onto a copy (R57), into a Fuse (R102), and not through a Transform.
//!
//! Port of `packages/engine/test/effects-tune.test.ts`.

use jackioh_engine::effects::{
    TuneDirection, applicable_changes, degrade, set_number, shuffle_copies_of_self, shuffle_into,
    summon_copy, transform, tune_once, upgrade,
};
use jackioh_engine::testkit::*;
use serde::Serialize;

use super::fixtures::combat as combat_fx;
use super::fixtures::harness::{events_of_type, in_hand, put, set_library, slot};
use super::fixtures::instance_data as instance_fx;

fn game(seed: &str) -> GameState {
    let mut state = instance_fx::instance_game(seed, None);
    state.turn = 3;
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    state
}

/// TS `HookOptions & { self?: CardInstance | null }`.
#[derive(Default)]
struct RunOptions {
    self_: Option<CardInstance>,
    controller: Option<PlayerId>,
    targets: Option<Vec<Selection>>,
}

/// Apply one effect as `resolve.ts` does and hand back its events; the rng cursor is kept. TS's
/// `sinkFor(state)` is the sink built here: its rng starts at the state's cursor, as reduce does.
fn run(state: &mut GameState, effect: Effect, options: RunOptions) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            options.self_.as_ref(),
            HookOptions {
                controller: Some(options.controller.unwrap_or(PlayerId::P1)),
                targets: options.targets,
                ..HookOptions::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

/// One application at a time, as `tuneOnce` makes it. TS handed it the live card; Rust reads the
/// card by id as it stands now.
fn once(state: &mut GameState, id: &str, direction: TuneDirection) -> Vec<GameEvent> {
    let card = live(state, id);
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(PlayerId::P1),
                ..HookOptions::default()
            },
        );
        tune_once(&mut ctx, &card, direction, true);
    }
    state.rng_cursor = rng.cursor();
    events
}

/// TS `sinkFor(state)` for a call that does not write the cursor back.
fn with_sink<R>(state: &mut GameState, f: impl FnOnce(&mut EngineSink<'_>) -> R) -> R {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    f(&mut sink)
}

fn changes(events: &[GameEvent]) -> Vec<TuningChange> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Degraded { change, .. } | GameEvent::Upgraded { change, .. } => Some(change.clone()),
            _ => None,
        })
        .collect()
}

/// `changes(events).map((change) => change.kind)`.
fn change_kinds(events: &[GameEvent]) -> Vec<String> {
    changes(events)
        .iter()
        .map(|change| to_json(change)["kind"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// A `degraded` or `upgraded` event's fields.
struct Tuned {
    instance_id: String,
    hidden_from: Option<Vec<PlayerId>>,
    change: TuningChange,
}

/// `eventsOfType(events, "degraded")` / `"upgraded"`.
fn tuned(events: &[GameEvent], type_: GameEventType) -> Vec<Tuned> {
    events
        .iter()
        .filter(|event| event.event_type() == type_)
        .filter_map(|event| match event {
            GameEvent::Degraded {
                instance_id,
                hidden_from,
                change,
                ..
            }
            | GameEvent::Upgraded {
                instance_id,
                hidden_from,
                change,
                ..
            } => Some(Tuned {
                instance_id: instance_id.clone(),
                hidden_from: hidden_from.clone(),
                change: change.clone(),
            }),
            _ => None,
        })
        .collect()
}

/// TS `card(state, defId, player = "p1")`: a fresh card of that definition in the player's hand.
fn card(state: &mut GameState, def_id: &str) -> CardInstance {
    in_hand(state, def_id, PlayerId::P1, 1)
        .into_iter()
        .next()
        .expect("no card")
}

/// TS `string[]` for the harness's `setLibrary`, from the ids as written.
fn owned(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| id.to_string()).collect()
}

/// TS held the live instance and read it after an effect; Rust reads the card again by id.
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id)
        .cloned()
        .unwrap_or_else(|| panic!("no card {id} in the state"))
}

/// TS wrote through the live instance; Rust writes through the card found by id.
fn live_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id} in the state"))
}

/// `applicableChanges(state, card, direction)`, as the rows' names.
fn rows(state: &GameState, id: &str, direction: TuneDirection) -> Value {
    to_json(applicable_changes(state, &live(state, id), direction))
}

/// `card.tuning?.x?.[key]`.
fn x_tuning(card: &CardInstance, key: &str) -> Option<i32> {
    card.tuning
        .as_ref()
        .and_then(|tuning| tuning.x.as_ref())
        .and_then(|x| x.get(key))
        .copied()
}

fn tuning(literal: Value) -> Option<Tuning> {
    Some(json_as(literal))
}

fn param(state: &GameState, id: &str, key: &str) -> i32 {
    param_value(state, Some(&live(state, id)), key, Default::default())
}

/// `numbersOn(state, card)` as JSON (`{ id, ref, label, value }` each).
fn numbers(state: &GameState, id: &str) -> Vec<Value> {
    numbers_on(state, &live(state, id)).iter().map(to_json).collect()
}

fn has_rush(keywords: &[Keyword]) -> bool {
    keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Rush)
}

fn to_json<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// TS `toMatchObject`: every key `expected` names is in `actual` with a matching value (objects
/// recursively, arrays element by element); `actual` may carry more.
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

/// The viewer's own hand in a view, as JSON entries (TS asserted it is a list).
fn own_hand(view: &PlayerView) -> Vec<Value> {
    let hand = to_json(&view.you.hand);
    assert!(hand.is_array(), "own hand is a list");
    hand.as_array().cloned().unwrap_or_default()
}

fn first_event_of(events: &[GameEvent], type_: GameEventType) -> Option<GameEvent> {
    events.iter().find(|event| event.event_type() == type_).cloned()
}

mod b3_4_the_menu_row_by_row_r386 {
    use super::*;

    #[test]
    fn r386_cost_a_degrade_adds_1_up_to_4_an_upgrade_takes_1_down_to_0_each_by_cost_mod() {
        let mut state = game("tune");
        let spell = card(&mut state, &instance_fx::constant.id);
        assert_eq!(rows(&state, &spell.id, TuneDirection::Degrade), json!(["cost"]));
        let degraded = run(
            &mut state,
            degrade(json_as(json!({ "instanceId": spell.id, "times": 4 }))),
            RunOptions::default(),
        );
        // 1 → 2 → 3 → 4, and at (4) nothing is left to change: the fourth application is cued `none`
        // because the card is in a hand the other player may not read (R440).
        assert_eq!(
            to_json(changes(&degraded)),
            json!([
                { "kind": "cost", "delta": 1 },
                { "kind": "cost", "delta": 1 },
                { "kind": "cost", "delta": 1 },
                { "kind": "none" },
            ])
        );
        assert_eq!(live(&state, &spell.id).cost_mod, 3);
        assert_eq!(
            effective_cost(&state, &live(&state, &spell.id), Default::default()),
            TUNE_COST_CAP
        );
        assert_eq!(rows(&state, &spell.id, TuneDirection::Degrade), json!([]));

        let upgraded = run(
            &mut state,
            upgrade(json_as(json!({ "instanceId": spell.id, "times": 5 }))),
            RunOptions::default(),
        );
        assert_eq!(
            change_kinds(&upgraded),
            vec!["cost", "cost", "cost", "cost", "none"]
        );
        assert_eq!(
            effective_cost(&state, &live(&state, &spell.id), Default::default()),
            0
        );
    }

    #[test]
    fn r386_cost_is_never_an_x_cost_cards_its_row_is_its_x_instead() {
        let mut state = game("tune");
        let bolt = card(&mut state, &instance_fx::x_bolt.id);
        assert_eq!(rows(&state, &bolt.id, TuneDirection::Degrade), json!(["x"]));
        assert_eq!(rows(&state, &bolt.id, TuneDirection::Upgrade), json!(["x"]));
    }

    #[test]
    fn r386_stats_a_degrades_4_split_floors_attack_at_0_and_current_health_at_1_the_floors_share_lost() {
        for seed in 1..=25 {
            let mut state = game(&format!("stats-{seed}"));
            let unit = put(
                &mut state,
                &instance_fx::dear_body.id,
                slot(PlayerId::P1, Row::Units, 1),
                json!({}),
            );
            live_mut(&mut state, &unit.id).damage = 1;
            // (4) already: the stats are the only row, so each application is one draw, the split.
            assert_eq!(rows(&state, &unit.id, TuneDirection::Degrade), json!(["stats"]));
            let before = unit_view(&state, &live(&state, &unit.id));
            let change = changes(&once(&mut state, &unit.id, TuneDirection::Degrade))
                .into_iter()
                .next();
            let Some(TuningChange::Stats {
                attack: d_attack,
                health: d_health,
            }) = change
            else {
                panic!("expected a stats change");
            };
            let after = unit_view(&state, &live(&state, &unit.id));
            assert_eq!(after.attack, before.attack + d_attack);
            assert_eq!(after.health, before.health + d_health);
            assert!(after.attack >= 0);
            assert!(after.health >= 1);
            // A 2/1 (2/2 with 1 damage) has 2 attack and 0 health above the floors to give.
            assert!(-d_attack <= 2);
            assert_eq!(d_health, 0);
            assert_eq!(live(&state, &unit.id).damage, 1);
        }
    }

    #[test]
    fn r386_stats_an_upgrades_4_split_moves_max_health_on_the_field_and_the_face_a_hand_card_will_enter_with()
    {
        for seed in 1..=10 {
            let mut state = game(&format!("up-{seed}"));
            let unit = put(
                &mut state,
                &instance_fx::dear_body.id,
                slot(PlayerId::P1, Row::Units, 1),
                json!({}),
            );
            live_mut(&mut state, &unit.id).damage = 1;
            let change = changes(&once(&mut state, &unit.id, TuneDirection::Upgrade))
                .into_iter()
                .next();
            if let Some(TuningChange::Stats {
                attack: d_attack,
                health: d_health,
            }) = change
            {
                assert_eq!(d_attack + d_health, 4);
                assert_eq!(
                    unit_view(&state, &live(&state, &unit.id)).max_health,
                    2 + d_health
                );
                assert_eq!(unit_view(&state, &live(&state, &unit.id)).health, 1 + d_health);
            }
            let held = card(&mut state, &instance_fx::dear_body.id);
            let in_hand_change = changes(&once(&mut state, &held.id, TuneDirection::Upgrade))
                .into_iter()
                .next();
            if let Some(TuningChange::Stats {
                attack: d_attack,
                health: d_health,
            }) = in_hand_change
            {
                assert_eq!(
                    to_json(stats_with_buffs(&state, &live(&state, &held.id))),
                    json!({ "attack": 2 + d_attack, "maxHealth": 2 + d_health })
                );
            }
        }
    }

    #[test]
    fn r386_keyword_a_degrade_takes_one_the_card_has_of_its_own_never_a_harmful_one_nor_one_its_position_lends()
     {
        for seed in 1..=25 {
            let mut state = game(&format!("kw-{seed}"));
            let unit = put(
                &mut state,
                &instance_fx::shackled.id,
                slot(PlayerId::P1, Row::Units, 1),
                json!({}),
            );
            {
                let card = live_mut(&mut state, &unit.id);
                card.position = Some(Position::Def);
                card.cost_mod = TUNE_COST_CAP - 1; // no cost row
                card.damage = 1;
                card.tuning = tuning(json!({ "attack": -2 })); // 0/1: no stats row either
            }
            assert_eq!(rows(&state, &unit.id, TuneDirection::Degrade), json!(["keyword"]));
            let change = changes(&once(&mut state, &unit.id, TuneDirection::Degrade))
                .into_iter()
                .next();
            assert_eq!(
                to_json(change),
                json!({ "kind": "keyword", "keyword": { "kind": "Rush" }, "added": false })
            );
            let kinds: Vec<KeywordKind> = unit_view(&state, &live(&state, &unit.id))
                .keywords
                .iter()
                .map(Keyword::kind)
                .collect();
            assert!(kinds.contains(&KeywordKind::CantAttack));
            assert!(kinds.contains(&KeywordKind::Taunt));
            assert!(!kinds.contains(&KeywordKind::Rush));
            // Nothing left of its own but the harmful one: the row is gone.
            assert_eq!(rows(&state, &unit.id, TuneDirection::Degrade), json!([]));
        }
    }

    #[test]
    fn r386_keyword_a_removed_keyword_goes_whether_it_was_printed_added_by_an_upgrade_or_granted() {
        let mut state = game("tune");
        let unit = put(
            &mut state,
            &instance_fx::shackled.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        {
            let card = live_mut(&mut state, &unit.id);
            card.granted_keywords = vec![Keyword::Rush, Keyword::Pierce];
            card.tuning = tuning(json!({ "addKeywords": [{ "kind": "Rush" }] }));
        }
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(PlayerId::P1),
                    ..HookOptions::default()
                },
            );
            // Draw until the keyword row takes Rush, then read what is left.
            for _ in 0..40 {
                let now = live(&*ctx.state, &unit.id);
                if !has_rush(&unit_view(&*ctx.state, &now).keywords) {
                    break;
                }
                tune_once(&mut ctx, &now, TuneDirection::Degrade, true);
            }
        }
        let now = live(&state, &unit.id);
        assert!(!has_rush(&unit_view(&state, &now).keywords));
        assert!(
            now.tuning
                .as_ref()
                .and_then(|tuning| tuning.remove_keywords.as_ref())
                .is_some_and(|removed| removed.contains(&KeywordKind::Rush))
        );
        assert!(
            !now.tuning
                .as_ref()
                .and_then(|tuning| tuning.add_keywords.clone())
                .unwrap_or_default()
                .contains(&Keyword::Rush)
        );
        assert!(!now.granted_keywords.contains(&Keyword::Rush));
    }

    #[test]
    fn r386_keyword_an_upgrade_adds_one_from_r21_pool_that_a_unit_lacks_never_to_a_spell_or_a_vanilla_unit() {
        for seed in 1..=15 {
            let mut state = game(&format!("add-{seed}"));
            let unit = put(
                &mut state,
                &instance_fx::free_body.id,
                slot(PlayerId::P1, Row::Units, 1),
                json!({}),
            );
            assert_eq!(
                rows(&state, &unit.id, TuneDirection::Upgrade),
                json!(["stats", "keyword"])
            );
            let change = changes(&once(&mut state, &unit.id, TuneDirection::Upgrade))
                .into_iter()
                .next();
            if let Some(TuningChange::Keyword { keyword, added }) = change {
                assert!(added);
                let now = live(&state, &unit.id);
                assert_eq!(
                    now.tuning.as_ref().and_then(|tuning| tuning.add_keywords.clone()),
                    Some(vec![keyword.clone()])
                );
                assert!(unit_view(&state, &now).keywords.contains(&keyword));
            }
        }
        let mut state = game("tune");
        let constant = card(&mut state, &instance_fx::constant.id);
        assert_eq!(
            rows(&state, &constant.id, TuneDirection::Upgrade),
            json!(["cost"])
        );
        let silenced = put(
            &mut state,
            &instance_fx::free_body.id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        );
        live_mut(&mut state, &silenced.id).vanilla = true;
        assert_eq!(
            rows(&state, &silenced.id, TuneDirection::Upgrade),
            json!(["stats"])
        );
    }

    #[test]
    fn r386_x_printed_armor_lucky_and_spell_damage_move_by_1_more_being_better_never_below_1() {
        let mut state = game("tune");
        let unit = card(&mut state, &instance_fx::numbered_body.id);
        live_mut(&mut state, &unit.id).cost_mod = TUNE_COST_CAP - 1;
        assert_eq!(
            rows(&state, &unit.id, TuneDirection::Degrade),
            json!(["stats", "keyword", "x"])
        );
        let keyword_numbers = |state: &GameState| -> Value {
            let mut out = serde_json::Map::new();
            for number in numbers(state, &unit.id) {
                if number["ref"]["kind"] == "keyword" {
                    out.insert(
                        number["label"].as_str().unwrap_or_default().to_string(),
                        number["value"].clone(),
                    );
                }
            }
            Value::Object(out)
        };
        assert_eq!(
            keyword_numbers(&state),
            json!({ "Armor": 2, "Lucky": 1, "Spell Damage": 1 })
        );
        // The X row alone, by hand: Armor is the only one a Degrade can move.
        live_mut(&mut state, &unit.id).tuning = tuning(json!({ "x": { "Armor": -1 } }));
        assert_eq!(
            keyword_numbers(&state),
            json!({ "Armor": 1, "Lucky": 1, "Spell Damage": 1 })
        );
        assert!(
            stats_of_keywords(&state, &live(&state, &unit.id)).contains(&json!({ "kind": "Armor", "n": 1 }))
        );
        live_mut(&mut state, &unit.id).tuning = tuning(json!({ "x": { "Lucky": 2, "Spell Damage": 1 } }));
        assert_eq!(
            keyword_numbers(&state),
            json!({ "Armor": 2, "Lucky": 3, "Spell Damage": 2 })
        );
    }

    #[test]
    fn r386_x_echo_activate_and_tribute_read_through_their_tuning_tribute_is_less_is_better() {
        let mut state = game("tune");
        let echo = card(&mut state, &instance_fx::echo_bolt.id);
        let active = card(&mut state, &instance_fx::activator.id);
        let tribute = card(&mut state, &instance_fx::tributer.id);
        let value_of = |state: &GameState, id: &str, key: &str| -> Option<i64> {
            numbers(state, id)
                .iter()
                .find(|number| number["label"] == key)
                .and_then(|number| number["value"].as_i64())
        };
        assert_eq!(
            [
                value_of(&state, &echo.id, "Echo"),
                value_of(&state, &active.id, "Activate"),
                value_of(&state, &tribute.id, "Tribute"),
            ],
            [Some(1), Some(2), Some(2)]
        );

        // Upgrade's X row: one better each — Echo 2, Activate 3, Tribute 1.
        for (id, key) in [
            (&echo.id, "Echo"),
            (&active.id, "Activate"),
            (&tribute.id, "Tribute"),
        ] {
            for _ in 0..60 {
                if x_tuning(&live(&state, id), key).unwrap_or(0) != 0 {
                    break;
                }
                once(&mut state, id, TuneDirection::Upgrade);
            }
        }
        assert_eq!(
            [
                value_of(&state, &echo.id, "Echo"),
                value_of(&state, &active.id, "Activate"),
                value_of(&state, &tribute.id, "Tribute"),
            ],
            [Some(2), Some(3), Some(1)]
        );
        // TS called `printedEcho(echo)` without the state; the Rust port takes it, and a hand card copies
        // no Echo, so the number is the card's own either way.
        assert_eq!(printed_echo(&live(&state, &echo.id), &state), 2);
        assert_eq!(tuned_count(&live(&state, &tribute.id), "Tribute", 2), 1);
        // Tribute 1 is as good as it gets: no Upgrade X item is left on it, and a Degrade raises it.
        assert_eq!(x_tuning(&live(&state, &tribute.id), "Tribute"), Some(-1));
    }

    #[test]
    fn r386_x_an_x_cost_cards_x_counts_1_more_or_less_when_it_resolves_never_below_1() {
        let mut state = game("tune");
        let spell = card(&mut state, &instance_fx::x_bolt.id);
        run(
            &mut state,
            degrade(json_as(json!({ "instanceId": spell.id, "times": 2 }))),
            RunOptions::default(),
        );
        assert_eq!(
            to_json(live(&state, &spell.id).tuning.and_then(|tuning| tuning.x)),
            json!({ "X": -2 })
        );
        live_mut(&mut state, &spell.id).x = Some(3);
        assert_eq!(x_of(&live(&state, &spell.id)), 1);
        live_mut(&mut state, &spell.id).x = Some(5);
        assert_eq!(x_of(&live(&state, &spell.id)), 3);
        // The context a resolution runs in reads it (`resolve.makeContext`).
        let spell = live(&state, &spell.id);
        let x = with_sink(&mut state, |sink| {
            make_context(sink, Some(&spell), HookOptions::default()).x
        });
        assert_eq!(x, 3);
    }

    #[test]
    fn r386_x_a_buff_billy_on_the_field_is_its_x_in_stats_and_an_upgrade_of_its_x_grows_it_b2_7() {
        let mut state = game("tune");
        let mut unit = new_instance(
            &mut state,
            &instance_fx::billy.id,
            PlayerId::P1,
            Zone::Hand { player: PlayerId::P1 },
        );
        unit.x = Some(2);
        place_on_field(
            &mut state,
            &mut unit,
            slot(PlayerId::P1, Row::Units, 1),
            PlaceOnFieldOptions::default(),
        );
        assert!(matches_object(
            &to_json(unit_view(&state, &live(&state, &unit.id))),
            &json!({ "attack": 6, "maxHealth": 6 }),
        ));
        live_mut(&mut state, &unit.id).tuning = tuning(json!({ "x": { "X": 1 } }));
        assert!(matches_object(
            &to_json(unit_view(&state, &live(&state, &unit.id))),
            &json!({ "attack": 9, "maxHealth": 9 }),
        ));
        // On the field its X has resolved: neither a Degrade nor an Upgrade has an X to move (SPEC §8.7 row 69).
        {
            let card = live_mut(&mut state, &unit.id);
            card.x = Some(1);
            card.tuning = None;
        }
        let x = json!("x");
        assert!(
            !rows(&state, &unit.id, TuneDirection::Degrade)
                .as_array()
                .is_some_and(|rows| rows.contains(&x))
        );
        assert!(
            !rows(&state, &unit.id, TuneDirection::Upgrade)
                .as_array()
                .is_some_and(|rows| rows.contains(&x))
        );
        assert_eq!(
            rows(&state, &unit.id, TuneDirection::Upgrade),
            json!(["stats", "keyword"])
        );
    }

    #[test]
    fn r386_x_a_brittle_count_in_force_moves_itself_never_below_1_a_printed_one_not_yet_started_moves_the_number_it_starts_at()
     {
        let mut state = game("tune");
        let unit = put(
            &mut state,
            &instance_fx::brittle_unit.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        live_mut(&mut state, &unit.id).cost_mod = TUNE_COST_CAP - 1;
        assert_eq!(
            live(&state, &unit.id).brittle.map(|brittle| brittle.count),
            Some(2)
        );
        for _ in 0..60 {
            if live(&state, &unit.id).brittle.map(|brittle| brittle.count) != Some(2) {
                break;
            }
            once(&mut state, &unit.id, TuneDirection::Degrade);
        }
        assert_eq!(
            live(&state, &unit.id).brittle.map(|brittle| brittle.count),
            Some(1)
        );
        let held = card(&mut state, &instance_fx::brittle_unit.id);
        for _ in 0..60 {
            if x_tuning(&live(&state, &held.id), "Brittle").is_some() {
                break;
            }
            once(&mut state, &held.id, TuneDirection::Upgrade);
        }
        assert_eq!(x_tuning(&live(&state, &held.id), "Brittle"), Some(1));
        // TS placed `held`, the one object that is still in the hand too (`placeOnField` takes nothing
        // out of the hand), and read `held.brittle`. Rust's hand keeps its own copy, so the object TS
        // read is the copy `place_on_field` leaves as it landed.
        let mut held_now = live(&state, &held.id);
        assert!(place_on_field(
            &mut state,
            &mut held_now,
            slot(PlayerId::P1, Row::Units, 2),
            PlaceOnFieldOptions::default(),
        ));
        assert_eq!(held_now.brittle.map(|brittle| brittle.count), Some(3));
    }

    #[test]
    fn r386_number_a_declared_number_steps_its_own_way_up_or_down_by_its_step_within_its_bounds() {
        let mut state = game("tune");
        let spell = card(&mut state, &instance_fx::numbered.id);
        {
            let card = live_mut(&mut state, &spell.id);
            card.cost_mod = -card.cost_mod - 2; // a (0): no cost row for an Upgrade
        }
        assert_eq!(rows(&state, &spell.id, TuneDirection::Upgrade), json!(["number"]));
        let mut seen: IndexSet<String> = IndexSet::new();
        for _ in 0..40 {
            if let Some(TuningChange::Number { key, .. }) =
                changes(&once(&mut state, &spell.id, TuneDirection::Upgrade))
                    .into_iter()
                    .next()
            {
                seen.insert(key);
            }
        }
        let mut seen: Vec<String> = seen.into_iter().collect();
        seen.sort();
        assert_eq!(seen, vec!["big", "damage", "huge", "threshold"]);
        // Less is better for the threshold, and it never goes below its min (2).
        assert_eq!(param(&state, &spell.id, "threshold"), 2);
        // The default steps: 2 for 8 (6–12), a quarter for 20 (5); `huge` stops at its max.
        assert_eq!((param(&state, &spell.id, "big") - 8) % 2, 0);
        let huge = param(&state, &spell.id, "huge");
        assert!((huge - 20) % 5 == 0 || huge == 44);
        assert!(huge <= 44);
    }

    #[test]
    fn r386_number_a_degrade_never_takes_an_amount_below_1() {
        let mut state = game("tune");
        let spell = card(&mut state, &instance_fx::numbered.id);
        run(
            &mut state,
            degrade(json_as(json!({ "instanceId": spell.id, "times": 60 }))),
            RunOptions::default(),
        );
        assert_eq!(param(&state, &spell.id, "damage"), 1);
        assert!(param(&state, &spell.id, "threshold") >= 3);
    }
}

mod b3_4_rules_1_and_2_the_draw_immutable_n_times_r386_r442 {
    use super::*;

    #[test]
    fn r386_an_immutable_card_is_never_changed_and_nothing_is_drawn_for_it() {
        let mut state = game("tune");
        let unit = put(
            &mut state,
            &instance_fx::stoic.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let cursor = state.rng_cursor;
        let events = run(
            &mut state,
            degrade(json_as(json!({ "instanceId": unit.id, "times": 3 }))),
            RunOptions::default(),
        );
        assert!(events.is_empty());
        assert_eq!(state.rng_cursor, cursor);
        assert_eq!(live(&state, &unit.id).tuning, None);
        assert_eq!(live(&state, &unit.id).cost_mod, 0);
    }

    #[test]
    fn r440_an_immutable_card_in_a_hand_is_still_cued_once_per_application_with_the_change_none() {
        let mut state = game("tune");
        let held = card(&mut state, &instance_fx::stoic.id);
        let events = run(
            &mut state,
            upgrade(json_as(json!({ "instanceId": held.id, "times": 2 }))),
            RunOptions::default(),
        );
        assert_eq!(
            to_json(&events),
            json!([
                { "type": "upgraded", "instanceId": held.id, "defId": instance_fx::stoic.id, "change": { "kind": "none" }, "hiddenFrom": ["p2"] },
                { "type": "upgraded", "instanceId": held.id, "defId": instance_fx::stoic.id, "change": { "kind": "none" }, "hiddenFrom": ["p2"] },
            ])
        );
    }

    #[test]
    fn r442_one_row_one_item_nothing_is_drawn_two_rows_one_draw_picks_the_row_then_the_row_draws_its_own() {
        let mut state = game("tune");
        let spell = card(&mut state, &instance_fx::constant.id);
        let cursor = state.rng_cursor;
        once(&mut state, &spell.id, TuneDirection::Degrade);
        assert_eq!(state.rng_cursor, cursor);

        let unit = put(
            &mut state,
            &instance_fx::free_body.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        assert_eq!(
            rows(&state, &unit.id, TuneDirection::Upgrade),
            json!(["stats", "keyword"])
        );
        let before = state.rng_cursor;
        once(&mut state, &unit.id, TuneDirection::Upgrade);
        // The row, then the split k or the keyword out of the pool: two draws either way.
        assert_eq!(state.rng_cursor, before + 2);
    }

    #[test]
    fn r386_n_times_is_n_applications_each_reported() {
        let mut state = game("tune");
        let unit = put(
            &mut state,
            &instance_fx::body.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let events = run(
            &mut state,
            upgrade(json_as(json!({ "instanceId": unit.id, "times": 5 }))),
            RunOptions::default(),
        );
        assert_eq!(events_of_type(&events, GameEventType::Upgraded).len(), 5);
        assert!(
            tuned(&events, GameEventType::Upgraded)
                .iter()
                .all(|event| event.hidden_from.is_none() && event.change != TuningChange::None)
        );
    }

    #[test]
    fn r386_a_named_target_reaches_a_card_on_the_field_in_a_hand_or_in_a_deck_one_that_has_ceased_to_exist_is_not_changed()
     {
        let mut state = game("tune");
        let constant_id = instance_fx::constant.id.clone();
        let top = set_library(&mut state, PlayerId::P1, &owned(&[constant_id.as_str()]))
            .into_iter()
            .next()
            .expect("no card");
        let events = run(
            &mut state,
            degrade(json_as(json!({ "target": { "of": "chosen" } }))),
            RunOptions {
                targets: Some(vec![Selection::Instance {
                    instance_id: top.id.clone(),
                }]),
                ..RunOptions::default()
            },
        );
        assert_eq!(live(&state, &top.id).cost_mod, 1);
        assert_eq!(
            tuned(&events, GameEventType::Degraded)
                .first()
                .map(|event| event.hidden_from.clone()),
            Some(Some(vec![PlayerId::P1, PlayerId::P2]))
        );
        live_mut(&mut state, &top.id).zone = Zone::Gone { player: PlayerId::P1 };
        state.players.p1.library = vec![];
        assert!(
            run(
                &mut state,
                degrade(json_as(json!({ "target": { "of": "chosen" } }))),
                RunOptions {
                    targets: Some(vec![Selection::Instance {
                        instance_id: top.id.clone(),
                    }]),
                    ..RunOptions::default()
                },
            )
            .is_empty()
        );
    }
}

mod b3_4_scopes_and_random_picks_r60_r129_r242_r440 {
    use super::*;

    #[test]
    fn r440_a_random_pick_over_a_deck_picks_n_different_cards_cues_each_for_both_players_in_the_decks_order()
    {
        let mut state = game("tune");
        let constant_id = instance_fx::constant.id.clone();
        let deck = set_library(&mut state, PlayerId::P2, &owned(&[constant_id.as_str(); 10]));
        let events = tuned(
            &run(
                &mut state,
                degrade(json_as(
                    json!({ "scope": { "side": "enemy", "zones": ["library"] }, "random": 4 }),
                )),
                RunOptions::default(),
            ),
            GameEventType::Degraded,
        );
        assert_eq!(events.len(), 4);
        assert_eq!(
            events
                .iter()
                .map(|event| &event.instance_id)
                .collect::<IndexSet<_>>()
                .len(),
            4
        );
        assert!(
            events
                .iter()
                .all(|event| event.hidden_from == Some(vec![PlayerId::P1, PlayerId::P2]))
        );
        let order: Vec<i64> = events
            .iter()
            .map(|event| {
                deck.iter()
                    .position(|card| card.id == event.instance_id)
                    .map_or(-1, |at| at as i64)
            })
            .collect();
        let mut sorted = order.clone();
        sorted.sort();
        assert_eq!(order, sorted);
    }

    #[test]
    fn r129_a_pick_of_at_least_as_many_cards_as_there_are_takes_them_all_and_draws_nothing_for_the_pick() {
        let mut state = game("tune");
        let constant_id = instance_fx::constant.id.clone();
        set_library(
            &mut state,
            PlayerId::P2,
            &owned(&[constant_id.as_str(), constant_id.as_str()]),
        );
        let cursor = state.rng_cursor;
        let events = tuned(
            &run(
                &mut state,
                degrade(json_as(
                    json!({ "scope": { "side": "enemy", "zones": ["library"] }, "random": 4 }),
                )),
                RunOptions::default(),
            ),
            GameEventType::Degraded,
        );
        assert_eq!(events.len(), 2);
        // Each card had one row and one item, so no draw at all.
        assert_eq!(state.rng_cursor, cursor);
    }

    #[test]
    fn r440_a_scopes_filters_decide_which_hidden_cards_change_and_the_rest_are_cued_none_so_the_cues_number_the_pile()
     {
        let mut state = game("tune");
        let units = in_hand(&mut state, &instance_fx::free_body.id, PlayerId::P1, 2);
        let spells = in_hand(&mut state, &instance_fx::constant.id, PlayerId::P1, 2);
        let events = tuned(
            &run(
                &mut state,
                upgrade(json_as(
                    json!({ "scope": { "zones": ["hand"], "types": ["Unit"] } }),
                )),
                RunOptions::default(),
            ),
            GameEventType::Upgraded,
        );
        assert_eq!(events.len(), 4);
        for spell in &spells {
            assert_eq!(
                events
                    .iter()
                    .find(|event| event.instance_id == spell.id)
                    .map(|event| event.change.clone()),
                Some(TuningChange::None)
            );
            assert_eq!(live(&state, &spell.id).cost_mod, 0);
        }
        for unit in &units {
            assert_ne!(
                events
                    .iter()
                    .find(|event| event.instance_id == unit.id)
                    .map(|event| event.change.clone()),
                Some(TuningChange::None)
            );
        }
    }

    #[test]
    fn r242_the_events_go_out_group_by_group_public_cards_then_the_owners_hidden_ones_then_the_decks() {
        let mut state = game("tune");
        let constant_id = instance_fx::constant.id.clone();
        let deck_card = set_library(&mut state, PlayerId::P1, &owned(&[constant_id.as_str()]))
            .into_iter()
            .next();
        let hand_card = in_hand(&mut state, &constant_id, PlayerId::P1, 1)
            .into_iter()
            .next();
        let unit = put(
            &mut state,
            &instance_fx::body.id,
            slot(PlayerId::P1, Row::Units, 5),
            json!({}),
        );
        let events = tuned(
            &run(
                &mut state,
                degrade(json_as(
                    json!({ "scope": { "zones": ["library", "hand", "field"] } }),
                )),
                RunOptions::default(),
            ),
            GameEventType::Degraded,
        );
        assert_eq!(
            events
                .iter()
                .map(|event| Some(event.instance_id.clone()))
                .collect::<Vec<_>>(),
            vec![
                Some(unit.id),
                hand_card.map(|card| card.id),
                deck_card.map(|card| card.id)
            ]
        );
    }

    #[test]
    fn r60_a_random_pick_over_a_hand_and_a_side_of_the_field_takes_public_and_hidden_cards_alike_by_their_counts_alone()
     {
        let (mut field, mut hand) = (0, 0);
        for seed in 1..=40 {
            let mut state = game(&format!("mix-{seed}"));
            let unit = put(
                &mut state,
                &instance_fx::body.id,
                slot(PlayerId::P2, Row::Units, 1),
                json!({}),
            );
            in_hand(&mut state, &instance_fx::stoic.id, PlayerId::P2, 1);
            let events = run(
                &mut state,
                degrade(json_as(
                    json!({ "scope": { "side": "enemy", "zones": ["hand", "field"] }, "random": 1 }),
                )),
                RunOptions::default(),
            );
            let event = tuned(&events, GameEventType::Degraded).into_iter().next();
            if event.map(|event| event.instance_id) == Some(unit.id.clone()) {
                field += 1;
            } else {
                hand += 1;
            }
        }
        // The Immutable hand card is picked (and cued) as often as the unit, though it can never change.
        assert!(field > 5);
        assert!(hand > 5);
    }
}

mod b3_4_rule_7_what_the_views_show_r177_r311_r386 {
    use super::*;

    #[test]
    fn r386_a_change_to_a_hand_card_reads_in_full_to_its_owner_and_as_a_bare_cue_to_the_other_player() {
        let mut state = game("tune");
        let held = card(&mut state, &instance_fx::constant.id);
        run(
            &mut state,
            degrade(json_as(json!({ "instanceId": held.id }))),
            RunOptions::default(),
        );
        let events = run(
            &mut state,
            degrade(json_as(json!({ "instanceId": held.id }))),
            RunOptions::default(),
        );
        state.applied = vec![AppliedAction {
            nonce: "t".to_string(),
            events,
        }];
        let own = first_event_of(&view_for(&state, PlayerId::P1).events, GameEventType::Degraded);
        let theirs = first_event_of(&view_for(&state, PlayerId::P2).events, GameEventType::Degraded);
        assert_eq!(
            to_json(own),
            json!({
                "type": "degraded",
                "instanceId": held.id,
                "defId": instance_fx::constant.id,
                "change": { "kind": "cost", "delta": 1 },
            })
        );
        assert_eq!(
            to_json(theirs),
            json!({
                "type": "degraded",
                "instanceId": HIDDEN_ID,
                "defId": HIDDEN_ID,
                "change": { "kind": "number", "key": HIDDEN_ID, "delta": 0 },
            })
        );
        let hand = own_hand(&view_for(&state, PlayerId::P1));
        assert_eq!(hand.first().map(|view| view["cost"].clone()), Some(json!(3)));
        assert!(
            !serde_json::to_string(&view_for(&state, PlayerId::P2))
                .expect("serialises")
                .contains(&held.id)
        );
    }

    #[test]
    fn r311_a_change_made_inside_a_deck_stays_unread_by_both_players_for_good_and_the_owners_list_shows_the_card_as_it_went_in()
     {
        let mut state = game("tune");
        let numbered_id = instance_fx::numbered.id.clone();
        let top = set_library(&mut state, PlayerId::P1, &owned(&[numbered_id.as_str()]))
            .into_iter()
            .next()
            .expect("no card");
        let list_before = to_json(&view_for(&state, PlayerId::P1).you.own_library);
        let events = run(
            &mut state,
            upgrade(json_as(json!({ "instanceId": top.id, "times": 2 }))),
            RunOptions::default(),
        );
        state.applied = vec![AppliedAction {
            nonce: "t".to_string(),
            events,
        }];
        assert_eq!(
            to_json(&view_for(&state, PlayerId::P1).you.own_library),
            list_before
        );
        // Drawn, the card reads with its changes — and the events made inside the deck still do not.
        with_sink(&mut state, |sink| {
            jackioh_engine::draw::draw(sink, PlayerId::P1, 1);
        });
        let view = view_for(&state, PlayerId::P1);
        let hand = own_hand(&view);
        assert_eq!(
            hand.iter()
                .find(|entry| entry["instanceId"] == top.id.as_str())
                .map(|entry| entry["tuning"].clone()),
            Some(to_json(&live(&state, &top.id).tuning))
        );
        for event in view
            .events
            .iter()
            .filter(|event| event.event_type() == GameEventType::Upgraded)
        {
            assert!(matches_object(
                &to_json(event),
                &json!({ "instanceId": HIDDEN_ID, "defId": HIDDEN_ID })
            ));
        }
    }

    #[test]
    fn r386_a_changed_cards_view_carries_its_tuning_and_its_declared_numbers_where_its_viewer_may_read_it() {
        let mut state = game("tune");
        let unit = put(
            &mut state,
            &instance_fx::body.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        live_mut(&mut state, &unit.id).tuning =
            tuning(json!({ "attack": 2, "addKeywords": [{ "kind": "Rush" }] }));
        let held = card(&mut state, &instance_fx::numbered.id);
        live_mut(&mut state, &held.id).tuning = tuning(json!({ "numbers": { "damage": 2 } }));
        let mine = view_for(&state, PlayerId::P1);
        let theirs = view_for(&state, PlayerId::P2);
        let (mine_json, theirs_json) = (to_json(&mine), to_json(&theirs));
        assert_eq!(
            mine_json["you"]["units"][0]["tuning"],
            json!({ "attack": 2, "addKeywords": [{ "kind": "Rush" }] })
        );
        assert_eq!(
            theirs_json["opponent"]["units"][0]["tuning"],
            json!({ "attack": 2, "addKeywords": [{ "kind": "Rush" }] })
        );
        assert_eq!(theirs_json["opponent"]["units"][0]["attack"], json!(5));
        let hand = own_hand(&mine);
        assert_eq!(
            hand.iter()
                .find(|entry| entry["instanceId"] == held.id.as_str())
                .map(|entry| entry["params"].clone()),
            Some(json!({ "damage": 4, "threshold": 3, "big": 8, "huge": 20 }))
        );
        assert_eq!(theirs_json["opponent"]["hand"], json!({ "count": 1 }));
    }
}

mod b3_4_rule_4_the_changes_are_the_cards_r57_r78_r102_r386_r443 {
    use super::*;

    #[test]
    fn r386_tuning_is_kept_through_leaving_the_field_and_in_every_zone_r78_a_spent_brittle_count_aside() {
        let mut state = game("tune");
        let unit = put(
            &mut state,
            &instance_fx::body.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        {
            let card = live_mut(&mut state, &unit.id);
            card.tuning =
                tuning(json!({ "attack": -1, "removeKeywords": ["Rush"], "numbers": { "damage": 1 } }));
            card.buffs = AttackHealth { attack: 3, health: 0 };
        }
        let mut moving = live(&state, &unit.id);
        move_to_zone(&mut state, &mut moving, OffFieldZone::Hand, Default::default());
        assert_eq!(
            to_json(&live(&state, &unit.id).tuning),
            json!({ "attack": -1, "removeKeywords": ["Rush"], "numbers": { "damage": 1 } })
        );
        assert_eq!(
            live(&state, &unit.id).buffs,
            AttackHealth { attack: 0, health: 0 }
        );
        let mut moving = live(&state, &unit.id);
        move_to_zone(
            &mut state,
            &mut moving,
            OffFieldZone::Graveyard,
            Default::default(),
        );
        let mut moving = live(&state, &unit.id);
        move_to_zone(&mut state, &mut moving, OffFieldZone::Exile, Default::default());
        assert_eq!(
            to_json(&live(&state, &unit.id).tuning),
            json!({ "attack": -1, "removeKeywords": ["Rush"], "numbers": { "damage": 1 } })
        );
    }

    #[test]
    fn r57_a_copy_on_the_field_keeps_the_sources_tuning_and_never_its_brittle_count() {
        let mut state = game("tune");
        let unit = put(
            &mut state,
            &instance_fx::brittle_unit.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        live_mut(&mut state, &unit.id).tuning = tuning(json!({ "health": 2 }));
        run(
            &mut state,
            summon_copy(json_as(
                json!({ "of": { "of": "instance", "instanceId": unit.id } }),
            )),
            RunOptions::default(),
        );
        let copy = state.players.p1.units[1]
            .as_ref()
            .and_then(|pile| pile.first())
            .cloned();
        assert_eq!(to_json(copy.and_then(|card| card.tuning)), json!({ "health": 2 }));
        // It entered the field, so its own printed Brittle starts; the source's count is not copied.
        live_mut(&mut state, &unit.id).brittle = Some(BrittleCounter {
            count: 1,
            since: 1,
            printed: None,
        });
        run(
            &mut state,
            summon_copy(json_as(
                json!({ "of": { "of": "instance", "instanceId": unit.id } }),
            )),
            RunOptions::default(),
        );
        let second = state.players.p1.units[2]
            .as_ref()
            .and_then(|pile| pile.first())
            .cloned();
        assert_eq!(
            second.and_then(|card| card.brittle),
            Some(BrittleCounter {
                count: 2,
                since: state.turn,
                printed: Some(true),
            })
        );
    }

    #[test]
    fn r57_a_copy_shuffled_into_a_deck_carries_the_sources_tuning() {
        let mut state = game("tune");
        let numbered_id = instance_fx::numbered.id.clone();
        let mut spell = new_instance(
            &mut state,
            &numbered_id,
            PlayerId::P1,
            Zone::Resolving { player: PlayerId::P1 },
        );
        spell.tuning = tuning(json!({ "numbers": { "damage": 1 } }));
        state.players.p1.resolving.push(spell.clone());
        let copies = |state: &GameState| -> Vec<CardInstance> {
            state
                .players
                .p1
                .library
                .iter()
                .filter(|card| card.def_id == numbered_id)
                .cloned()
                .collect()
        };
        run(
            &mut state,
            shuffle_copies_of_self(json_as(json!({ "count": 2 }))),
            RunOptions {
                self_: Some(spell.clone()),
                ..RunOptions::default()
            },
        );
        assert_eq!(
            to_json(
                copies(&state)
                    .iter()
                    .map(|card| card.tuning.clone())
                    .collect::<Vec<_>>()
            ),
            json!([{ "numbers": { "damage": 1 } }, { "numbers": { "damage": 1 } }])
        );
        run(
            &mut state,
            shuffle_into(json_as(
                json!({ "defId": numbered_id, "count": 1, "copyOf": spell.id }),
            )),
            RunOptions::default(),
        );
        assert_eq!(
            copies(&state).iter().filter(|card| card.tuning.is_some()).count(),
            3
        );
        // A card the text names rather than copies is a fresh card with none.
        run(
            &mut state,
            shuffle_into(json_as(json!({ "defId": numbered_id, "count": 1 }))),
            RunOptions::default(),
        );
        assert_eq!(
            copies(&state).iter().filter(|card| card.tuning.is_none()).count(),
            1
        );
    }

    #[test]
    fn r386_a_transform_makes_a_card_without_the_old_ones_tuning() {
        let mut state = game("tune");
        let unit = put(
            &mut state,
            &instance_fx::body.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        live_mut(&mut state, &unit.id).tuning = tuning(json!({ "attack": 3 }));
        run(
            &mut state,
            transform(json_as(
                json!({ "instanceId": unit.id, "defId": combat_fx::plain.id }),
            )),
            RunOptions::default(),
        );
        let replacement = state.players.p1.units[0]
            .as_ref()
            .and_then(|pile| pile.first())
            .cloned();
        assert_eq!(
            replacement.as_ref().map(|card| card.def_id.clone()),
            Some(combat_fx::plain.id.clone())
        );
        assert_eq!(replacement.and_then(|card| card.tuning), None);
    }

    #[test]
    fn r102_a_fuse_sums_its_ingredients_tuning_onto_the_card_it_keeps() {
        let mut state = game("tune");
        let kept = put(
            &mut state,
            &instance_fx::body.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        live_mut(&mut state, &kept.id).tuning = tuning(json!({ "attack": 1, "x": { "Armor": 1 } }));
        let other = put(
            &mut state,
            &combat_fx::plain.id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        );
        live_mut(&mut state, &other.id).tuning = tuning(
            json!({ "attack": 2, "health": -1, "x": { "Armor": 1 }, "addKeywords": [{ "kind": "Rush" }] }),
        );
        let (kept_now, other_now) = (live(&state, &kept.id), live(&state, &other.id));
        let fused = with_sink(&mut state, |sink| {
            jackioh_engine::subsystems::fuse::fuse(
                sink,
                json_as(json!({ "ingredients": [other_now], "target": kept_now })),
            )
        });
        assert_eq!(fused.map(|card| card.id), Some(kept.id.clone()));
        assert_eq!(
            to_json(&live(&state, &kept.id).tuning),
            json!({ "attack": 3, "health": -1, "x": { "Armor": 2 }, "addKeywords": [{ "kind": "Rush" }] })
        );
    }
}

mod classic_plus_41_kys_constant_a_number_set_outright_r386 {
    use super::*;

    #[test]
    fn r386_numbers_on_lists_the_cost_never_an_x_attack_health_numbered_keywords_and_declared_numbers_none_on_an_immutable_card()
     {
        let mut state = game("tune");
        let unit = put(
            &mut state,
            &instance_fx::numbered_body.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        live_mut(&mut state, &unit.id).damage = 1;
        assert_eq!(
            numbers(&state, &unit.id)
                .iter()
                .map(|number| json!([number["id"], number["value"]]))
                .collect::<Vec<_>>(),
            vec![
                json!(["cost", 1]),
                json!(["attack", 2]),
                json!(["health", 1]),
                json!(["keyword:Armor", 2]),
                json!(["keyword:Lucky", 1]),
                json!(["keyword:Spell Damage", 1]),
            ]
        );
        let bolt = card(&mut state, &instance_fx::x_bolt.id);
        assert!(numbers(&state, &bolt.id).is_empty());
        let numbered = card(&mut state, &instance_fx::numbered.id);
        assert_eq!(
            numbers(&state, &numbered.id)
                .iter()
                .map(|number| number["id"].clone())
                .collect::<Vec<_>>(),
            vec![
                json!("cost"),
                json!("param:damage"),
                json!("param:threshold"),
                json!("param:big"),
                json!("param:huge"),
            ]
        );
        let stoic = card(&mut state, &instance_fx::stoic.id);
        assert!(numbers(&state, &stoic.id).is_empty());
    }

    #[test]
    fn r386_each_number_is_set_to_the_value_and_stays_set_as_tuning_a_later_step_counts_from_it() {
        let mut state = game("tune");
        let unit = put(
            &mut state,
            &instance_fx::numbered_body.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        live_mut(&mut state, &unit.id).damage = 1;
        for which in ["cost", "attack", "health", "keyword:Armor", "keyword:Lucky"] {
            run(
                &mut state,
                set_number(json_as(
                    json!({ "instanceId": unit.id, "which": which, "value": 3 }),
                )),
                RunOptions::default(),
            );
        }
        assert_eq!(
            numbers(&state, &unit.id)
                .iter()
                .map(|number| number["value"].clone())
                .collect::<Vec<_>>(),
            vec![json!(3), json!(3), json!(3), json!(3), json!(3), json!(1)]
        );
        assert!(matches_object(
            &to_json(unit_view(&state, &live(&state, &unit.id))),
            &json!({ "attack": 3, "health": 3, "maxHealth": 4 }),
        ));
        {
            // TS `unit.tuning = { ...unit.tuning, x: { Armor: 1 } }`.
            let card = live_mut(&mut state, &unit.id);
            let mut spread = card.tuning.clone().unwrap_or_default();
            spread.x = Some(IndexMap::from([("Armor".to_string(), 1)]));
            card.tuning = Some(spread);
        }
        assert_eq!(
            numbers(&state, &unit.id)
                .iter()
                .find(|number| number["id"] == "keyword:Armor")
                .map(|number| number["value"].clone()),
            Some(json!(4))
        );

        let spell = card(&mut state, &instance_fx::numbered.id);
        let events = run(
            &mut state,
            set_number(json_as(
                json!({ "instanceId": spell.id, "which": "param:huge", "value": 3 }),
            )),
            RunOptions::default(),
        );
        assert_eq!(param(&state, &spell.id, "huge"), 3);
        assert_eq!(
            to_json(&events),
            json!([
                {
                    "type": "numberChanged",
                    "instanceId": spell.id,
                    "defId": instance_fx::numbered.id,
                    "key": "huge",
                    "value": 3,
                    "hiddenFrom": ["p2"],
                },
            ])
        );
    }

    #[test]
    fn r386_random_picks_among_the_numbers_that_are_not_the_value_already_and_does_nothing_with_none_left() {
        let mut state = game("tune");
        let spell = card(&mut state, &instance_fx::numbered.id);
        let set_random = || {
            set_number(json_as(
                json!({ "instanceId": spell.id, "which": "random", "value": 3 }),
            ))
        };
        run(&mut state, set_random(), RunOptions::default());
        assert!(
            numbers(&state, &spell.id)
                .iter()
                .filter(|number| number["value"] == 3)
                .count()
                >= 2
        );
        for _ in 0..6 {
            run(&mut state, set_random(), RunOptions::default());
        }
        assert!(
            numbers(&state, &spell.id)
                .iter()
                .all(|number| number["value"] == 3)
        );
        let cursor = state.rng_cursor;
        assert!(run(&mut state, set_random(), RunOptions::default()).is_empty());
        assert_eq!(state.rng_cursor, cursor);
    }

    #[test]
    fn r386_a_number_the_card_does_not_have_or_an_immutable_card_is_left_alone() {
        let mut state = game("tune");
        let spell = card(&mut state, &instance_fx::constant.id);
        assert!(
            run(
                &mut state,
                set_number(json_as(
                    json!({ "instanceId": spell.id, "which": "attack", "value": 3 })
                )),
                RunOptions::default(),
            )
            .is_empty()
        );
        let wall = card(&mut state, &instance_fx::stoic.id);
        assert!(
            run(
                &mut state,
                set_number(json_as(
                    json!({ "instanceId": wall.id, "which": "cost", "value": 3 })
                )),
                RunOptions::default(),
            )
            .is_empty()
        );
    }

    #[test]
    fn r386_the_number_set_on_a_hand_card_is_the_cards_to_keep_the_other_player_sees_a_bare_cue() {
        let mut state = game("tune");
        let spell = card(&mut state, &instance_fx::numbered.id);
        let events = run(
            &mut state,
            set_number(json_as(
                json!({ "instanceId": spell.id, "which": "param:damage", "value": 3 }),
            )),
            RunOptions::default(),
        );
        state.applied = vec![AppliedAction {
            nonce: "t".to_string(),
            events,
        }];
        assert_eq!(
            to_json(first_event_of(
                &view_for(&state, PlayerId::P2).events,
                GameEventType::NumberChanged
            )),
            json!({
                "type": "numberChanged",
                "instanceId": HIDDEN_ID,
                "defId": HIDDEN_ID,
                "key": HIDDEN_ID,
                "value": 0,
            })
        );
        assert!(matches_object(
            &to_json(first_event_of(
                &view_for(&state, PlayerId::P1).events,
                GameEventType::NumberChanged
            )),
            &json!({ "key": "damage", "value": 3 }),
        ));
    }
}

/// The keywords a hand card will carry onto the field, for the X row test.
fn stats_of_keywords(state: &GameState, c: &CardInstance) -> Vec<Value> {
    if !numbers_on(state, c).is_empty() {
        unit_view_keywords(state, c)
    } else {
        vec![]
    }
}

fn unit_view_keywords(state: &GameState, c: &CardInstance) -> Vec<Value> {
    if c.zone.z() == ZoneName::Field {
        return unit_view(state, c).keywords.iter().map(to_json).collect();
    }
    let hand = to_json(&view_for(state, c.owner).you.hand);
    let Some(hand) = hand.as_array() else {
        return vec![];
    };
    hand.iter()
        .find(|view| view["instanceId"] == c.id.as_str())
        .and_then(|view| view["keywords"].as_array().cloned())
        .unwrap_or_default()
}
