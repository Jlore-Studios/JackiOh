//! Activate (docs/classic-sets.md B3.2, R384): a card's "Activate:", "Activate N:" and "Activate ♾️:"
//! abilities, the `activate` action and its alias `activatePower`, the refusal that is also the list,
//! the costs, the choices, the pauses, and the view.
//!
//! Every rule of B3.2 is pinned here through the fixtures of `fixtures/activate.ts`, by observable
//! behaviour: what the action does to the board, what `legalActions` offers, what `viewFor` shows,
//! and what survives a prompt, a JSON round trip and a replay.
//!
//! Port of `packages/engine/test/activate.test.ts`.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::effects::DELAYED_DESTROY_HOOK;
use jackioh_engine::subsystems::activate::{
    ACTIVATION_WORK, abilities_of, activate_ability, activate_actions_for, is_acting_on_field, uses_allowed,
    why_cannot_activate_ability,
};
use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::subsystems::hero_power::POWER_KEY;
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::activate::{
    HIGH_TELL, KEEPER_KEY, LOG_LANE, LOW_TELL, MERCHANT_PRICE, PICK_KEY, SCEPTER_KEY, ZAPPER_DAMAGE,
    activate_catalog, ACTIVATE_SCRIPTS, asker, chooser, endless, ghost, heroic, high_teller, keeper, lockdown,
    log_card, low_teller, merchant, mourner, nose, notes, pinger, punisher, scepter, sentry, spark, trapper,
    turtle, zapper,
};
use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, set_library, setup_catalog, slot};

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

fn register() {
    register_catalog(activate_catalog(registered_catalog().clone()));
    let mut scripts = registered_scripts().clone();
    for (id, script) in ACTIVATE_SCRIPTS.clone() {
        scripts.insert(id, script);
    }
    register_scripts(scripts);
}

fn game(seed: &str) -> GameState {
    let state = new_game(&format!("activate-{seed}"), None);
    register();
    state
}

/// TS's module-level `let nonce = 0`: unique across the file's tests, which Rust runs on many threads.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn act_result(state: &GameState, body: ActionInput) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    reduce(state, &body.with_nonce(format!("act{nonce}")))
}

fn act(state: &GameState, body: ActionInput) -> (GameState, Vec<GameEvent>) {
    let result = act_result(state, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    (result.state, result.events)
}

fn input(body: Value) -> ActionInput {
    json_as(body)
}

/// Past both mulligans, in p1's main phase on turn 1, with the note log in p2's backrow.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&game(seed)).state;
    for player in [P1, P2] {
        let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
        state = act(&state, input(json!({ "type": "mulligan", "keep": keep, "playerId": player }))).0;
    }
    put(&mut state, &log_card.id, slot(P2, Row::Backrow, LOG_LANE), json!({}));
    // R345: the tests end turns themselves, so nothing ends one behind their back (R82).
    state.players.p1.auto_end_turn = Some(false);
    state.players.p2.auto_end_turn = Some(false);
    state
}

fn activate(player: PlayerId, instance_id: &str, extra: Value) -> ActionInput {
    let mut body = json!({ "type": "activate", "instanceId": instance_id, "playerId": player });
    if let (Some(into), Some(from)) = (body.as_object_mut(), extra.as_object()) {
        for (key, value) in from {
            into.insert(key.clone(), value.clone());
        }
    }
    input(body)
}

fn round_trip(state: &GameState) -> GameState {
    serde_json::from_value(json_of(state)).unwrap()
}

/// Answer the one open prompt with its first option, whoever holds it.
fn answer(state: &GameState) -> (GameState, Vec<GameEvent>) {
    let pending = state.pending.as_ref().expect("expected a prompt");
    let option = pending.options.first().expect("the prompt offers nothing");
    act(
        state,
        ActionInput {
            body: ActionBody::Answer {
                choice_id: pending.id.clone(),
                selection: vec![option.selection.clone()],
            },
            player_id: pending.player_id,
        },
    )
}

fn json_of<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).unwrap()
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

/// vitest's `not.toHaveProperty(key)`.
fn lacks(value: &Value, key: &str) -> bool {
    value.as_object().is_none_or(|object| !object.contains_key(key))
}

/// TS `whyCannotActivateAbility(…)`: the refusal's text, or `null` (`None`) when it may be activated.
fn why_not(state: &GameState, player: PlayerId, instance_id: &str, ability: Option<&str>) -> Option<String> {
    why_cannot_activate_ability(state, player, instance_id, ability)
        .err()
        .map(|error| error.message)
}

/// The live card (TS held the object itself; Rust looks it up by id).
fn card<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

fn card_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

fn hero_target() -> Value {
    json!({ "targets": [{ "pick": "hero", "player": "p2" }] })
}

/// The abilities `legal_actions` lists under one card, by name.
fn listed_abilities(state: &GameState, instance: &str) -> Vec<Option<String>> {
    legal_actions(state, P1)
        .into_iter()
        .filter_map(|body| match body {
            ActionBody::Activate {
                instance_id, ability, ..
            } if instance_id == instance => Some(ability),
            _ => None,
        })
        .collect()
}

fn lists_activate_of(state: &GameState, instance: &str) -> bool {
    legal_actions(state, P1)
        .iter()
        .any(|body| matches!(body, ActionBody::Activate { instance_id, .. } if instance_id == instance))
}

fn ability_ids(state: &GameState, instance: &str) -> Vec<String> {
    abilities_of(state, card(state, instance))
        .into_iter()
        .map(|decl| decl.id)
        .collect()
}

/// TS `body.targets?.[0]`, as JSON (null when there is none).
fn first_target(body: &ActionBody) -> Value {
    json_of(body)["targets"][0].clone()
}

fn instance_ids<E: serde::Serialize>(events: &[E]) -> Vec<Value> {
    events.iter().map(|event| json_of(event)["instanceId"].clone()).collect()
}

fn ids_of(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// The TS `fuse(sinkFor(state), { ingredients, target })`.
fn fuse_onto(state: &mut GameState, ingredients: Vec<CardInstance>, target: &CardInstance) -> Option<CardInstance> {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    fuse(
        &mut sink,
        FuseArgs {
            ingredients,
            target: Some(card(sink.state, &target.id).clone()),
            ..FuseArgs::default()
        },
    )
}

/// TS put the card back on the field as the object the hand still held; Rust takes it out of the
/// hand first, so it is placed once.
fn take_from_hand(state: &mut GameState, player: PlayerId, id: &str) -> CardInstance {
    let hand = &mut state.players[player].hand;
    let at = hand.iter().position(|held| held.id == id).expect("in the hand");
    hand.remove(at)
}

// ---------------------------------------------------------------------------

mod r384_b3_2_activate_using_an_ability {
    use super::*;

    #[test]
    fn r384_the_activate_action_runs_the_ability_with_its_declared_target_announces_it_and_is_not_a_play() {
        let mut state = playing("ping");
        let card = put(&mut state, &pinger.id, slot(P1, Row::Backrow, 1), json!({}));

        let (after, events) = act(
            &state,
            activate(P1, &card.id, json!({ "ability": "ping", "targets": [{ "pick": "hero", "player": "p2" }] })),
        );

        assert_eq!(after.players.p2.hero.health, HERO_HEALTH - 1);
        assert_eq!(
            json_of(events_of_type(&events, GameEventType::Activated)),
            json!([{ "type": "activated", "player": "p1", "instanceId": card.id, "defId": pinger.id, "ability": "ping" }])
        );
        // B3.2 rule 6: nothing that counts plays sees it.
        assert!(events_of_type(&events, GameEventType::CardPlayed).is_empty());
        assert_eq!(after.counters.played, state.counters.played);
        assert_eq!(after.players.p1.turn_log, state.players.p1.turn_log);
        // Its use is counted on the instance, for this turn.
        assert_eq!(
            after.players.p1.backrow[0].as_ref().and_then(|held| held.memory.get("activations")).cloned(),
            Some(json!({ "turn": state.turn, "count": 1 }))
        );
    }

    #[test]
    fn r384_an_ability_named_by_nothing_is_the_cards_only_one_a_card_with_several_needs_it_named() {
        let mut state = playing("naming");
        let card = put(&mut state, &pinger.id, slot(P1, Row::Backrow, 1), json!({}));
        let (mut after, _) = act(&state, activate(P1, &card.id, hero_target()));
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH - 1);

        let many = put(&mut after, &chooser.id, slot(P1, Row::Backrow, 2), json!({}));
        card_mut(&mut after, &many.id).memory.insert(PICK_KEY.to_string(), json!("alpha"));
        assert_eq!(
            why_not(&after, P1, &many.id, None).as_deref(),
            Some("that card has several abilities: name the one to activate")
        );
        assert_eq!(why_not(&after, P1, &many.id, Some("gamma")), None);
    }
}

mod r384_b3_2_rules_1_3_7_9_how_many_uses {
    use super::*;

    #[test]
    fn r384_activate_is_once_per_turn_and_activate_n_is_n_times_each_counted_per_card_per_turn() {
        let mut state = playing("uses");
        let once = put(&mut state, &pinger.id, slot(P1, Row::Backrow, 1), json!({}));
        let twice = put(&mut state, &sentry.id, slot(P1, Row::Units, 1), json!({}));

        state = act(&state, activate(P1, &once.id, hero_target())).0;
        assert_eq!(
            act_result(&state, activate(P1, &once.id, hero_target())).error.as_deref(),
            Some("that ability has already been used this turn")
        );

        state = act(&state, activate(P1, &twice.id, json!({}))).0;
        state = act(&state, activate(P1, &twice.id, json!({}))).0;
        assert_eq!(
            act_result(&state, activate(P1, &twice.id, json!({}))).error.as_deref(),
            Some("that ability has been used 2 times this turn")
        );
        // Two "Gain 1 mana" uses on top of the refreshed crystal.
        assert_eq!(state.players.p1.mana.current, 3);

        // The Radiant face's "Activate 2" is the face's own count. (TS `put(…, { radiant: true })` set
        // the flag before placing; these fixtures place the same either way.)
        let radiant = put(&mut state, &pinger.id, slot(P1, Row::Backrow, 2), json!({}));
        card_mut(&mut state, &radiant.id).radiant = true;
        let face = ACTIVATE_SCRIPTS.clone()
            .get(&pinger.id)
            .map(|scripts| scripts.radiant.clone())
            .unwrap_or_default();
        let decls = activation_decls(&face);
        assert_eq!(uses_allowed(card(&state, &radiant.id), &decls[0]), 2);
    }

    #[test]
    fn r384_uses_are_counted_per_card_a_card_with_several_abilities_counts_every_use_of_any_of_them() {
        let mut state = playing("per-card");
        let card = put(&mut state, &chooser.id, slot(P1, Row::Backrow, 1), json!({}));
        card_mut(&mut state, &card.id).memory.insert(PICK_KEY.to_string(), json!("alpha"));
        let after = act(&state, activate(P1, &card.id, json!({ "ability": "gamma" }))).0;
        assert_eq!(
            after.players.p1.backrow[0].as_ref().and_then(|held| held.memory.get("activations")).cloned(),
            Some(json!({ "turn": state.turn, "count": 1 }))
        );
        assert_eq!(
            why_not(&after, P1, &card.id, Some("alpha")).as_deref(),
            Some("that ability has already been used this turn")
        );
    }

    #[test]
    fn r384_uses_reset_on_the_controllers_next_turn_and_on_the_opponents_turn_nothing_can_be_activated() {
        let mut state = playing("reset");
        let card = put(&mut state, &pinger.id, slot(P1, Row::Backrow, 1), json!({}));
        state = act(&state, activate(P1, &card.id, hero_target())).0;

        state = act(&state, input(json!({ "type": "endTurn", "playerId": "p1" }))).0;
        assert_eq!(state.active, P2);
        assert_eq!(why_not(&state, P1, &card.id, None).as_deref(), Some("it is not your turn"));
        assert_eq!(
            act_result(&state, activate(P1, &card.id, hero_target())).error.as_deref(),
            Some("it is not your turn")
        );

        state = act(&state, input(json!({ "type": "endTurn", "playerId": "p2" }))).0;
        assert_eq!(state.active, P1);
        assert_eq!(why_not(&state, P1, &card.id, None), None);
        assert_eq!(
            act(&state, activate(P1, &card.id, hero_target())).0.players.p2.hero.health,
            HERO_HEALTH - 2
        );
    }

    #[test]
    fn r384_activate_unlimited_is_any_number_of_uses_bounded_by_activate_unlimited_cap() {
        let mut state = playing("unlimited");
        let card = put(&mut state, &endless.id, slot(P1, Row::Backrow, 1), json!({}));
        let action = || ActionBody::Activate {
            instance_id: card.id.clone(),
            ability: None,
            targets: None,
            modes: None,
            tributes: None,
        };
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        for _ in 0..ACTIVATE_UNLIMITED_CAP {
            assert_eq!(activate_ability(&mut sink, P1, &action()).err().map(|error| error.message), None);
        }
        assert_eq!(notes(sink.state).len() as i32, ACTIVATE_UNLIMITED_CAP);
        assert_eq!(
            activate_ability(&mut sink, P1, &action()).err().map(|error| error.message),
            Some(format!("that ability has been used {ACTIVATE_UNLIMITED_CAP} times this turn"))
        );
    }

    #[test]
    fn r384_degrade_and_upgrade_move_activate_n_by_the_tuning_key_activate_never_below_1_and_never_unlimited() {
        let mut state = playing("tuned");
        let once = put(&mut state, &pinger.id, slot(P1, Row::Backrow, 1), json!({}));
        let twice = put(&mut state, &sentry.id, slot(P1, Row::Units, 1), json!({}));
        let always = put(&mut state, &endless.id, slot(P1, Row::Backrow, 2), json!({}));
        let ping_decl = abilities_of(&state, &once).into_iter().next().expect("no ability");
        let surge_decl = abilities_of(&state, &twice).into_iter().next().expect("no ability");
        let again_decl = abilities_of(&state, &always).into_iter().next().expect("no ability");

        // An Upgrade turns "Activate" into "Activate 2".
        card_mut(&mut state, &once.id).tuning = Some(json_as(json!({ "x": { "Activate": 1 } })));
        assert_eq!(uses_allowed(card(&state, &once.id), &ping_decl), 2);
        // A Degrade takes "Activate 2" to 1, and no further.
        card_mut(&mut state, &twice.id).tuning = Some(json_as(json!({ "x": { "Activate": -1 } })));
        assert_eq!(uses_allowed(card(&state, &twice.id), &surge_decl), 1);
        card_mut(&mut state, &twice.id).tuning = Some(json_as(json!({ "x": { "Activate": -5 } })));
        assert_eq!(uses_allowed(card(&state, &twice.id), &surge_decl), 1);
        // KY's Constant sets the number outright.
        card_mut(&mut state, &twice.id).tuning = Some(json_as(json!({ "set": { "Activate": 3 } })));
        assert_eq!(uses_allowed(card(&state, &twice.id), &surge_decl), 3);
        // ♾️ has no number to move.
        card_mut(&mut state, &always.id).tuning = Some(json_as(json!({ "x": { "Activate": -3 } })));
        assert_eq!(uses_allowed(card(&state, &always.id), &again_decl), ACTIVATE_UNLIMITED_CAP);

        // The tuned count is the one the refusal reads.
        let mut next = act(&state, activate(P1, &once.id, hero_target())).0;
        next = act(&next, activate(P1, &once.id, hero_target())).0;
        assert_eq!(next.players.p2.hero.health, HERO_HEALTH - 2);
        assert_eq!(
            why_not(&next, P1, &once.id, None).as_deref(),
            Some("that ability has been used 2 times this turn")
        );
    }

    #[test]
    fn r384_r78_leaving_the_field_resets_the_uses_so_a_card_bounced_and_played_again_starts_fresh() {
        let mut state = playing("bounced");
        let card = put(&mut state, &pinger.id, slot(P1, Row::Backrow, 1), json!({}));
        state = act(&state, activate(P1, &card.id, hero_target())).0;
        let used = state.players.p1.backrow[0].clone().expect("on the field");
        assert_eq!(
            why_not(&state, P1, &used.id, None).as_deref(),
            Some("that ability has already been used this turn")
        );

        move_to_zone(&mut state, &used.id, OffFieldZone::Hand, MoveToZoneOptions::default());
        assert_eq!(super::card(&state, &used.id).memory.get("activations"), None);
        let held = take_from_hand(&mut state, P1, &used.id);
        assert!(place_on_field(
            &mut state,
            held,
            slot(P1, Row::Backrow, 1),
            PlaceOnFieldOptions::default()
        ));
        assert_eq!(why_not(&state, P1, &used.id, None), None);
    }
}

mod r384_b3_2_rule_2_who_and_when {
    use super::*;

    #[test]
    fn r384_only_the_controller_in_their_main_phase_with_no_prompt_open_and_the_game_not_over() {
        let mut state = playing("when");
        let card = put(&mut state, &pinger.id, slot(P1, Row::Backrow, 1), json!({}));

        assert_eq!(why_not(&state, P2, &card.id, None).as_deref(), Some("that card is not yours"));
        assert_eq!(why_not(&state, P1, "c99999", None).as_deref(), Some("no card c99999"));

        let mut in_phase = round_trip(&state);
        in_phase.phase = Phase::Start;
        assert_eq!(
            why_not(&in_phase, P1, &card.id, None).as_deref(),
            Some("an ability is activated in the main phase")
        );

        let mut asking = round_trip(&state);
        asking.pending = Some(PendingChoice {
            id: "q-test".into(),
            player_id: P1,
            kind: PromptKind::Target,
            prompt: "a question".into(),
            options: vec![PromptOption {
                key: "none".into(),
                label: "none".into(),
                selection: Selection::None,
                cost: None,
                radiant: None,
            }],
            min: 1,
            max: 1,
            budget: None,
            resume: Resume {
                def_id: String::new(),
                hook: "resume".into(),
                step: "x".into(),
                radiant: false,
                instance_id: None,
                data: IndexMap::new(),
            },
        });
        assert_eq!(why_not(&asking, P1, &card.id, None).as_deref(), Some("answer the open prompt first"));

        let mut over = round_trip(&state);
        over.result = Some(GameResult {
            winner: Winner::P1,
            reason: GameOverReason::Concede,
        });
        assert_eq!(why_not(&over, P1, &card.id, None).as_deref(), Some("the game is over"));
    }

    #[test]
    fn r384_summoning_sickness_and_exertion_do_not_stop_an_activation_activating_is_not_attacking() {
        let mut state = playing("sick");
        let unit = put(&mut state, &sentry.id, slot(P1, Row::Units, 1), json!({}));
        let turn = state.turn;
        {
            let live = card_mut(&mut state, &unit.id);
            live.summoned_turn = Some(turn);
            live.exertion = Exertion {
                attacked: true,
                switched: true,
                attacks: None,
            };
        }
        assert_eq!(why_not(&state, P1, &unit.id, None), None);
        assert_eq!(
            act(&state, activate(P1, &unit.id, json!({}))).0.players.p1.mana.current,
            state.players.p1.mana.current + 1
        );
    }

    #[test]
    fn r384_r13_a_card_acts_only_on_the_field_not_in_a_hand_not_dormant_under_a_pile_not_face_down() {
        let mut state = playing("acting");
        let held = in_hand(&mut state, &pinger.id, P1, 1).remove(0);
        assert_eq!(why_not(&state, P1, &held.id, None).as_deref(), Some("that card is not on the field"));

        let under = put(&mut state, &sentry.id, slot(P1, Row::Units, 2), json!({}));
        assert!(is_acting_on_field(&state, card(&state, &under.id)));
        let top = new_instance(&mut state, &sentry.id, P1, Zone::Hand { player: P1 });
        assert!(place_on_field(
            &mut state,
            top.clone(),
            slot(P1, Row::Units, 2),
            PlaceOnFieldOptions { stack: Some(true) }
        ));
        assert!(!is_acting_on_field(&state, card(&state, &under.id)));
        assert_eq!(
            why_not(&state, P1, &under.id, None).as_deref(),
            Some("that card is under a pile and does not act")
        );
        assert_eq!(why_not(&state, P1, &top.id, None), None);

        let trap = put(&mut state, &trapper.id, slot(P1, Row::Backrow, 3), json!({}));
        assert_eq!(
            why_not(&state, P1, &trap.id, None).as_deref(),
            Some("a face-down card has no ability to use")
        );
        card_mut(&mut state, &trap.id).face_up = Some(true);
        assert_eq!(why_not(&state, P1, &trap.id, None), None);

        // §6.3 Vanilla: a card with no text has no ability.
        let blank = put(&mut state, &pinger.id, slot(P1, Row::Backrow, 4), json!({}));
        card_mut(&mut state, &blank.id).vanilla = true;
        assert_eq!(why_not(&state, P1, &blank.id, None).as_deref(), Some("that card has no Activate ability"));
    }

    #[test]
    fn r384_a_condition_the_text_sets_is_part_of_the_refusal_classic_7_nothing_remembered_no_activation() {
        let mut state = playing("condition");
        let card = put(&mut state, &scepter.id, slot(P1, Row::Backrow, 1), json!({}));
        assert_eq!(
            why_not(&state, P1, &card.id, None).as_deref(),
            Some("that ability can't be activated now")
        );
        assert!(!lists_activate_of(&state, &card.id));
        card_mut(&mut state, &card.id)
            .memory
            .insert(SCEPTER_KEY.to_string(), json!({ "defId": "fx-1", "radiant": false }));
        assert_eq!(why_not(&state, P1, &card.id, None), None);
        assert_eq!(notes(&act(&state, activate(P1, &card.id, json!({}))).0), vec!["cast"]);
    }
}

mod r384_b3_2_rule_4_costs {
    use super::*;

    #[test]
    fn r384_a_mana_price_is_paid_as_it_is_activated_and_an_ability_the_player_cannot_pay_for_is_refused() {
        let mut state = playing("mana");
        let card = put(&mut state, &merchant.id, slot(P1, Row::Backrow, 1), json!({}));
        set_library(&mut state, P1, &["fx-5", "fx-6"]);
        state.players.p1.mana.current = MERCHANT_PRICE - 1;
        assert_eq!(
            why_not(&state, P1, &card.id, None),
            Some(format!("that ability costs {MERCHANT_PRICE}, more than your mana"))
        );

        state.players.p1.mana.current = MERCHANT_PRICE + 1;
        let hand = state.players.p1.hand.len();
        let (after, events) = act(&state, activate(P1, &card.id, json!({})));
        assert_eq!(after.players.p1.mana.current, 1);
        assert_eq!(after.players.p1.hand.len(), hand + 1);
        assert!(matches_object(
            &json_of(events_of_type(&events, GameEventType::ManaChanged).first()),
            &json!({ "player": "p1", "current": 1 })
        ));
    }

    #[test]
    fn r384_a_random_discard_is_paid_from_the_hand_and_an_empty_hand_cannot_pay_it() {
        let mut state = playing("discard");
        let card = put(&mut state, &nose.id, slot(P1, Row::Units, 1), json!({}));
        state.players.p1.hand = Vec::new();
        assert_eq!(
            why_not(&state, P1, &card.id, None).as_deref(),
            Some("that ability needs a card in your hand to discard")
        );

        let pair = in_hand(&mut state, "fx-3", P1, 2);
        let (after, events) = act(&state, activate(P1, &card.id, json!({})));
        assert_eq!(after.players.p1.hand.len(), 1);
        let discarded = instance_ids(&events_of_type(&events, GameEventType::Discarded));
        assert_eq!(discarded.len(), 1);
        assert!([json!(pair[0].id), json!(pair[1].id)].contains(&discarded[0]));
        assert_eq!(notes(&after), vec!["sniff"]);
    }

    #[test]
    fn r384_tribute_this_bypasses_indestructible_and_the_abilitys_effect_runs_after_it_with_the_card_in_its_graveyard() {
        let mut state = playing("tribute-self");
        let card = put(&mut state, &lockdown.id, slot(P1, Row::Backrow, 1), json!({}));
        let (after, events) = act(&state, activate(P1, &card.id, json!({})));
        assert!(after.players.p1.backrow[0].is_none());
        assert!(ids_of(&after.players.p1.graveyard).contains(&card.id));
        assert_eq!(
            instance_ids(&events_of_type(&events, GameEventType::Destroyed)),
            vec![json!(card.id)]
        );
        assert_eq!(notes(&after), vec!["left:graveyard"]);
    }

    #[test]
    fn r384_a_tribute_cost_takes_the_controllers_units_the_card_itself_allowed_and_one_unit_is_one_tribute() {
        let mut state = playing("tribute");
        let eater = put(&mut state, &turtle.id, slot(P1, Row::Units, 1), json!({}));
        let enemy = put(&mut state, &sentry.id, slot(P2, Row::Units, 1), json!({}));
        let to_p2 = |tributes: Value| {
            json!({ "targets": [{ "pick": "hero", "player": "p2" }], "tributes": tributes })
        };

        // With no other unit, the card pays with itself: 5 damage, its Attack as it stood.
        let alone = act(&state, activate(P1, &eater.id, to_p2(json!([eater.id])))).0;
        assert_eq!(alone.players.p2.hero.health, HERO_HEALTH - 5);
        assert!(alone.players.p1.units[0].is_none());

        // The refusals: none, two, an enemy unit, the same unit twice.
        let ally = put(&mut state, &sentry.id, slot(P1, Row::Units, 2), json!({}));
        assert_eq!(
            act_result(&state, activate(P1, &eater.id, hero_target())).error.as_deref(),
            Some("that ability tributes a Unit")
        );
        assert_eq!(
            act_result(&state, activate(P1, &eater.id, to_p2(json!([ally.id, eater.id])))).error.as_deref(),
            Some("that ability tributes a Unit")
        );
        assert_eq!(
            act_result(&state, activate(P1, &eater.id, to_p2(json!([enemy.id])))).error,
            Some(format!("{} cannot be tributed for that ability", enemy.id))
        );
        assert_eq!(
            act_result(&state, activate(P1, &eater.id, to_p2(json!([ally.id, ally.id])))).error.as_deref(),
            Some("that ability cannot tribute the same Unit twice")
        );
    }

    #[test]
    fn r384_r78_r89_the_tributed_units_attack_is_read_as_it_stood_before_it_died() {
        let mut state = playing("last-known");
        let eater = put(&mut state, &turtle.id, slot(P1, Row::Units, 1), json!({}));
        let fed = put(&mut state, &sentry.id, slot(P1, Row::Units, 2), json!({}));
        card_mut(&mut state, &fed.id).buffs = AttackHealth { attack: 3, health: 0 };
        let (after, _) = act(
            &state,
            activate(
                P1,
                &eater.id,
                json!({ "targets": [{ "pick": "hero", "player": "p2" }], "tributes": [fed.id] }),
            ),
        );
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH - 5);
        // The eater stays, and ♾️ lets it go again while it has something to eat.
        assert_eq!(
            after.players.p1.units[0].as_ref().and_then(|pile| pile.first()).map(|top| top.id.clone()),
            Some(eater.id.clone())
        );
    }

    #[test]
    fn r384_r174_a_target_the_tribute_took_off_the_field_is_gone_for_the_effect_and_nothing_else_is_hit() {
        let mut state = playing("target-tributed");
        let eater = put(&mut state, &turtle.id, slot(P1, Row::Units, 1), json!({}));
        let fed = put(&mut state, &sentry.id, slot(P1, Row::Units, 2), json!({}));
        let (after, events) = act(
            &state,
            activate(
                P1,
                &eater.id,
                json!({ "targets": [{ "pick": "instance", "instanceId": fed.id }], "tributes": [fed.id] }),
            ),
        );
        assert!(ids_of(&after.players.p1.graveyard).contains(&fed.id));
        assert!(events_of_type(&events, GameEventType::Damage).is_empty());
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH);
    }
}

mod r384_r81_r90_b3_2_rules_5_8_choices_and_the_list {
    use super::*;

    #[test]
    fn r384_modes_and_the_targets_they_bind_travel_in_the_action_and_are_checked_like_a_plays() {
        let mut state = playing("modes");
        let card = put(&mut state, &punisher.id, slot(P1, Row::Backrow, 1), json!({}));
        let enemy = put(&mut state, &sentry.id, slot(P2, Row::Units, 1), json!({}));
        let name = "punisher (activate)";

        assert_eq!(
            act_result(&state, activate(P1, &card.id, json!({}))).error,
            Some(format!("{name} needs a mode choice for each of its 1 mode declaration"))
        );
        assert_eq!(
            act_result(&state, activate(P1, &card.id, json!({ "modes": ["boom"] }))).error,
            Some(format!("\"boom\" is not a mode of {name}"))
        );
        assert_eq!(
            act_result(&state, activate(P1, &card.id, json!({ "modes": ["damage"] }))).error,
            Some(format!("{name} needs 1 target for that choice"))
        );
        assert_eq!(
            act_result(
                &state,
                activate(P1, &card.id, json!({ "modes": ["discard"], "targets": [{ "pick": "hero", "player": "p2" }] }))
            )
            .error,
            Some(format!("{name} takes no targets for that choice"))
        );
        assert_eq!(
            act_result(
                &state,
                activate(P1, &card.id, json!({ "modes": ["doom"], "targets": [{ "pick": "hero", "player": "p2" }] }))
            )
            .error,
            Some(format!("that is not a legal target for {name}"))
        );

        let damaged = act(
            &state,
            activate(P1, &card.id, json!({ "modes": ["damage"], "targets": [{ "pick": "hero", "player": "p2" }] })),
        )
        .0;
        assert_eq!(damaged.players.p2.hero.health, HERO_HEALTH - 2);

        let hand = state.players.p2.hand.len();
        assert_eq!(
            act(&state, activate(P1, &card.id, json!({ "modes": ["discard"] }))).0.players.p2.hand.len(),
            hand - 1
        );

        let doomed = act(
            &state,
            activate(
                P1,
                &card.id,
                json!({ "modes": ["doom"], "targets": [{ "pick": "instance", "instanceId": enemy.id }] }),
            ),
        )
        .0;
        assert_eq!(
            doomed
                .delayed
                .iter()
                .map(|entry| (entry.resume.hook.clone(), entry.watch.clone()))
                .collect::<Vec<(String, Option<String>)>>(),
            vec![(DELAYED_DESTROY_HOOK.to_string(), Some(enemy.id.clone()))]
        );
    }

    #[test]
    fn r384_legal_actions_lists_activate_for_exactly_the_usable_abilities_each_tribute_set_crossed_with_each_choice() {
        let mut state = playing("listing");
        let eater = put(&mut state, &turtle.id, slot(P1, Row::Units, 1), json!({}));
        let ally_a = put(&mut state, &sentry.id, slot(P1, Row::Units, 2), json!({}));
        put(&mut state, &sentry.id, slot(P1, Row::Units, 3), json!({}));
        put(&mut state, &sentry.id, slot(P2, Row::Units, 1), json!({}));

        let listed: Vec<ActionBody> = legal_actions(&state, P1)
            .into_iter()
            .filter(|body| matches!(body, ActionBody::Activate { instance_id, .. } if *instance_id == eater.id))
            .collect();
        // Three ways to pay (itself or either ally) times six targets (four units, two heroes). The two
        // sentries' "Activate 2" abilities are listed too, but not under the eater's id.
        assert_eq!(listed.len(), 3 * 6);
        assert_eq!(listed, activate_actions_for(&state, P1, card(&state, &eater.id)));
        assert_eq!(
            json_of(&listed[0]),
            json!({
                "type": "activate",
                "instanceId": eater.id,
                "ability": "eat",
                "tributes": [eater.id],
                "targets": [{ "pick": "instance", "instanceId": eater.id }],
            })
        );
        // Every listed action is one the reducer accepts.
        for body in &listed {
            assert_eq!(
                act_result(
                    &state,
                    ActionInput {
                        body: body.clone(),
                        player_id: P1
                    }
                )
                .error,
                None
            );
        }
        // And each ability of each card acting on the field is listed as its own entry.
        assert!(
            legal_actions(&state, P1)
                .iter()
                .any(|body| json_of(body) == json!({ "type": "activate", "instanceId": ally_a.id, "ability": "surge" }))
        );

        // Used up, it is listed no more; and on the opponent's turn not at all.
        let merchant_card = put(&mut state, &merchant.id, slot(P1, Row::Backrow, 1), json!({}));
        state.players.p1.mana.current = 0;
        assert!(!lists_activate_of(&state, &merchant_card.id));
    }

    #[test]
    fn r450_r682_a_declared_target_that_costs_discards_lists_one_action_carrying_none_paid_random_with_the_costs() {
        let mut state = playing("ghost");
        let card = put(&mut state, &pinger.id, slot(P1, Row::Backrow, 1), json!({}));
        let costly = put(&mut state, &ghost.id, slot(P2, Row::Units, 1), json!({}));
        state.players.p1.hand = Vec::new();
        let spares = in_hand(&mut state, &sentry.id, P1, 3);
        let spare_ids: Vec<Value> = spares.iter().map(|spare| json!(spare.id)).collect();
        let at_ghost = json!({ "pick": "instance", "instanceId": costly.id });

        let listed: Vec<ActionBody> = activate_actions_for(&state, P1, super::card(&state, &card.id))
            .into_iter()
            .filter(|body| first_target(body) == at_ghost)
            .collect();
        // R682: the discards are random at pay time, so one action, carrying none.
        assert_eq!(listed.len(), 1);
        assert!(listed.iter().all(|body| lacks(&json_of(body), "discards")));
        assert!(
            activate_actions_for(&state, P1, super::card(&state, &card.id))
                .iter()
                .filter(|body| first_target(body)["pick"] == json!("hero"))
                .all(|body| lacks(&json_of(body), "discards"))
        );
        // Refused when too few other cards are held; a hero target asks nothing and succeeds.
        let mut poor = playing("ghost-poor");
        let poor_card = put(&mut poor, &pinger.id, slot(P1, Row::Backrow, 1), json!({}));
        let poor_costly = put(&mut poor, &ghost.id, slot(P2, Row::Units, 1), json!({}));
        poor.players.p1.hand = Vec::new();
        in_hand(&mut poor, &sentry.id, P1, 1);
        let poor_ghost = json!({ "pick": "instance", "instanceId": poor_costly.id });
        assert!(
            act_result(&poor, activate(P1, &poor_card.id, json!({ "ability": "ping", "targets": [poor_ghost] })))
                .error
                .is_some()
        );
        assert_eq!(
            act_result(
                &poor,
                activate(P1, &poor_card.id, json!({ "ability": "ping", "targets": [{ "pick": "hero", "player": "p2" }] }))
            )
            .error,
            None
        );

        let (after, events) = act(&state, activate(P1, &card.id, json!({ "ability": "ping", "targets": [at_ghost] })));
        let order: Vec<GameEventType> = events.iter().map(|event| event.event_type()).collect();
        let discarded = instance_ids(&events_of_type(&events, GameEventType::Discarded));
        assert_eq!(discarded.len(), 2);
        for id in &discarded {
            assert!(spare_ids.contains(id));
        }
        let last_discard = order.iter().rposition(|kind| *kind == GameEventType::Discarded).map(|at| at as i64).unwrap_or(-1);
        let first_damage = order.iter().position(|kind| *kind == GameEventType::Damage).map(|at| at as i64).unwrap_or(-1);
        assert!(last_discard < first_damage);
        assert_eq!(after.players.p1.hand.len(), 1);
        assert!(spare_ids.contains(&json!(after.players.p1.hand[0].id)));
        assert_eq!(
            after.players.p2.units[0].as_ref().and_then(|pile| pile.first()).map(|top| top.damage),
            Some(1)
        );

        // With fewer than two cards in hand it is no legal target of the ability.
        state.players.p1.hand = Vec::new();
        in_hand(&mut state, &sentry.id, P1, 1);
        assert!(
            !activate_actions_for(&state, P1, super::card(&state, &card.id))
                .iter()
                .any(|body| first_target(body) == json!({ "pick": "instance", "instanceId": costly.id }))
        );
    }

    #[test]
    fn r384_an_ability_a_card_does_not_have_now_is_neither_listed_nor_accepted_the_heroic_power_patchs_rolled_power() {
        let mut state = playing("has");
        let card = put(&mut state, &chooser.id, slot(P1, Row::Backrow, 1), json!({}));
        card_mut(&mut state, &card.id).memory.insert(PICK_KEY.to_string(), json!("beta"));
        assert_eq!(ability_ids(&state, &card.id), vec!["beta", "gamma"]);
        assert_eq!(
            why_not(&state, P1, &card.id, Some("alpha")).as_deref(),
            Some("that card has no ability \"alpha\"")
        );
        assert_eq!(
            listed_abilities(&state, &card.id),
            vec![Some("beta".to_string()), Some("gamma".to_string())]
        );
        assert_eq!(
            notes(&act(&state, activate(P1, &card.id, json!({ "ability": "beta" }))).0),
            vec!["beta"]
        );
    }

    #[test]
    fn r384_r102_a_fused_card_has_every_ingredients_abilities_the_second_of_two_with_one_id_named_id_2() {
        let run: Hook = hook(|_ctx| vec![]);
        let decl = |id: &str| ActivationDecl {
            id: id.into(),
            label: id.into(),
            uses: ActivationUses::Count(1),
            cost: None,
            targets: vec![],
            modes: vec![],
            can_activate: None,
            has: None,
            run: run.clone(),
        };
        let script = Script {
            activations: vec![decl("ping"), decl("ping"), decl("surge")],
            ..Script::default()
        };
        let ids: Vec<String> = activation_decls(&script).into_iter().map(|entry| entry.id).collect();
        assert_eq!(ids, vec!["ping", "ping#2", "surge"]);
        // A tail its prompt parked comes back to the same ability by that id.
        let second = &script.activations[1];
        let resume = Resume {
            def_id: String::new(),
            hook: activation_hook("ping#2"),
            step: String::new(),
            radiant: false,
            instance_id: None,
            data: IndexMap::new(),
        };
        assert!(resume.hook.starts_with(ACTIVATION_HOOK_PREFIX));
        assert_eq!(
            script_step_for(&script, &resume).map(|found| Arc::ptr_eq(&found, &second.run)),
            Some(true)
        );
    }
}

mod r384_r113_r117_b3_2_and_s9_3_an_activation_across_a_prompt {
    use super::*;

    #[test]
    fn r384_a_prompt_inside_the_ability_parks_the_rest_of_its_list_under_the_abilitys_own_hook_and_the_answer_finishes_it()
     {
        let mut state = playing("asker");
        let card = put(&mut state, &asker.id, slot(P1, Row::Backrow, 1), json!({}));
        let paused = act(&state, activate(P1, &card.id, json!({}))).0;

        assert_eq!(notes(&paused), vec!["ask"]);
        assert_eq!(paused.pending.as_ref().map(|pending| pending.player_id), Some(P1));
        assert_eq!(
            paused.work.iter().map(|item| item.resume.hook.clone()).collect::<Vec<String>>(),
            vec![activation_hook("ask")]
        );
        // §9.3: plain data, so the paused game survives a JSON round trip and finishes identically.
        let copy = round_trip(&paused);
        assert_eq!(copy, paused);

        let live = answer(&paused).0;
        let restored = answer(&copy).0;
        assert_eq!(notes(&live), vec!["ask", "answered", "ask:tail"]);
        assert_eq!(hash_state(&restored), hash_state(&live));
        // The use was spent before the pause, so the answer cannot buy another.
        assert_eq!(
            why_not(&live, P1, &card.id, None).as_deref(),
            Some("that ability has already been used this turn")
        );
    }

    #[test]
    fn r384_a_tribute_whose_death_asks_owes_the_abilitys_effect_on_state_work_behind_the_death_passs_own_remainder() {
        let mut state = playing("tribute-pause");
        let eater = put(&mut state, &turtle.id, slot(P1, Row::Units, 1), json!({}));
        let fed = put(&mut state, &mourner.id, slot(P1, Row::Units, 2), json!({}));
        let paused = act(
            &state,
            activate(
                P1,
                &eater.id,
                json!({ "targets": [{ "pick": "hero", "player": "p2" }], "tributes": [fed.id] }),
            ),
        )
        .0;

        assert_eq!(notes(&paused), vec!["death"]);
        assert_eq!(paused.players.p2.hero.health, HERO_HEALTH);
        // R113: the Death pass's remainder (its hook's tail with it) first, then the ability it paid for.
        assert_eq!(
            paused.work.iter().map(|item| item.resume.hook.clone()).collect::<Vec<String>>(),
            vec![DEATHS_WORK.to_string(), ACTIVATION_WORK.to_string()]
        );

        let copy = round_trip(&paused);
        let live = answer(&paused).0;
        assert_eq!(notes(&live), vec!["death", "mourned", "death:tail"]);
        // The effect ran once the cost was whole: the mourner's Attack, 1.
        assert_eq!(live.players.p2.hero.health, HERO_HEALTH - 1);
        assert!(live.work.is_empty());
        assert_eq!(hash_state(&answer(&copy).0), hash_state(&live));
    }

    #[test]
    fn r384_a_game_with_activations_replays_from_its_log_to_the_same_state_s9_3() {
        let mut deck = vec![asker.id.clone()];
        deck.extend(vanilla_deck(DECK_SIZE - 1, 1));
        let decks = (deck, vanilla_deck(DECK_SIZE, 21));
        let seed = "activate-replay";
        setup_catalog();
        register();
        let mut state = begin_game(&create_game(&CreateGameOptions {
            seed: seed.into(),
            decks: decks.clone(),
            ..CreateGameOptions::default()
        }))
        .state;
        let mut log: Vec<Action> = Vec::new();
        fn step(state: &mut GameState, log: &mut Vec<Action>, body: ActionInput) {
            let action = body.with_nonce(format!("r{}", log.len()));
            let result = reduce(state, &action);
            if let Some(error) = result.error {
                panic!("{error}");
            }
            log.push(action);
            *state = result.state;
        }
        for player in [P1, P2] {
            let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
            step(
                &mut state,
                &mut log,
                input(json!({ "type": "mulligan", "keep": keep, "playerId": player })),
            );
        }
        let held = state
            .players
            .p1
            .hand
            .iter()
            .find(|card| card.def_id == asker.id)
            .cloned()
            .unwrap_or_else(|| panic!("Quickdraw put the asker in the opening hand"));
        step(
            &mut state,
            &mut log,
            input(json!({ "type": "play", "instanceId": held.id, "playerId": "p1" })),
        );
        step(
            &mut state,
            &mut log,
            input(json!({ "type": "activate", "instanceId": held.id, "playerId": "p1" })),
        );
        assert!(state.pending.is_some());
        let pending = state.pending.clone().expect("expected a prompt");
        let to_act = seat_to_act(&state).expect("a seat to act");
        step(
            &mut state,
            &mut log,
            input(json!({ "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "none" }], "playerId": to_act })),
        );

        let replayed = fold(&FoldArgs {
            seed: seed.into(),
            decks,
            log,
            ..FoldArgs::default()
        });
        assert!(replayed.errors.is_empty());
        assert_eq!(hash_state(&replayed.state), hash_state(&state));
    }
}

mod r384_r752_b3_2_rule_10_activate_power_is_an_alias_of_activate {
    use super::*;

    fn with_power(state: &mut GameState, lane: i32) -> CardInstance {
        let card = put(state, &heroic.id, slot(P1, Row::Backrow, lane), json!({}));
        // R754: "burn" is Steady Shot, "Deal {shot} damage to the enemy hero" for (1).
        card_mut(state, &card.id).memory.insert(POWER_KEY.to_string(), json!("burn"));
        super::card(state, &card.id).clone()
    }

    #[test]
    fn r752_a_heroic_powers_power_is_its_activate_ability_listed_as_an_activate_and_activate_power_names_it() {
        let mut state = playing("alias");
        let card = with_power(&mut state, 1);
        let listed: Vec<Value> = legal_actions(&state, P1)
            .iter()
            .map(json_of)
            .filter(|body| body["instanceId"] == json!(card.id))
            .collect();
        // The rolled power is the card's one ability, listed once, by its stored name (R103).
        assert_eq!(
            Value::Array(listed),
            json!([{ "type": "activate", "instanceId": card.id, "ability": "burn" }])
        );

        let via_activate = act(&state, activate(P1, &card.id, json!({}))).0;
        assert_eq!(via_activate.players.p2.hero.health, HERO_HEALTH - 2);
        assert_eq!(via_activate.players.p1.mana.current, state.players.p1.mana.current - 1);
        assert_eq!(
            via_activate.players.p1.backrow[0].as_ref().and_then(|held| held.memory.get("activations")).cloned(),
            Some(json!({ "turn": state.turn, "count": 1 }))
        );
        assert_eq!(
            act_result(
                &via_activate,
                input(json!({ "type": "activatePower", "instanceId": card.id, "playerId": "p1" }))
            )
            .error
            .as_deref(),
            Some("that ability has already been used this turn")
        );

        let via_alias = act(
            &state,
            input(json!({ "type": "activatePower", "instanceId": card.id, "playerId": "p1" })),
        )
        .0;
        assert_eq!(hash_state(&via_alias), hash_state(&via_activate));

        // TS asserted `.not.toBeNull()` on an `error` that is a string or undefined, which holds either
        // way: what it pins is that the call returns rather than throws (spec-gaps-part-25-1.md).
        let _ = act_result(&state, activate(P1, &card.id, json!({ "modes": ["x"] })));
        assert_eq!(
            act_result(&state, activate(P1, &card.id, json!({ "ability": "ping" }))).error.as_deref(),
            Some("that card has no ability \"ping\"")
        );
    }

    #[test]
    fn r384_activate_power_on_a_card_with_an_activate_ability_uses_that_ability() {
        let mut state = playing("alias-ability");
        let card = put(&mut state, &pinger.id, slot(P1, Row::Backrow, 1), json!({}));
        let (after, events) = act(
            &state,
            input(json!({
                "type": "activatePower",
                "instanceId": card.id,
                "targets": [{ "pick": "hero", "player": "p2" }],
                "playerId": "p1",
            })),
        );
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH - 1);
        assert_eq!(events_of_type(&events, GameEventType::Activated).len(), 1);
    }
}

mod r384_r97_b3_2_and_s10_8_what_each_player_sees {
    use super::*;

    #[test]
    fn r384_the_controllers_own_view_of_a_card_on_the_field_carries_its_abilities_and_the_other_players_never_does() {
        let mut state = playing("view");
        let once = put(&mut state, &pinger.id, slot(P1, Row::Backrow, 1), json!({}));
        put(&mut state, &endless.id, slot(P1, Row::Backrow, 2), json!({}));
        put(&mut state, &sentry.id, slot(P1, Row::Units, 1), json!({}));

        let mine = json_of(view_for(&state, P1));
        assert!(matches_object(
            &mine["you"]["backrow"][0],
            &json!({ "activations": [{ "ability": "ping", "label": "Deal 1 damage", "usesLeft": 1, "usable": true }] })
        ));
        assert!(matches_object(
            &mine["you"]["backrow"][1],
            &json!({ "activations": [{ "ability": "again", "usesLeft": null, "usable": true }] })
        ));
        assert!(matches_object(
            &mine["you"]["units"][0],
            &json!({ "activations": [{ "ability": "surge", "usesLeft": 2, "usable": true }] })
        ));
        // A card without abilities carries no key at all.
        assert!(lacks(&mine["opponent"]["backrow"][(LOG_LANE - 1) as usize], "activations"));

        let theirs = json_of(view_for(&state, P2));
        assert!(lacks(&theirs["opponent"]["backrow"][0], "activations"));
        assert!(lacks(&theirs["opponent"]["units"][0], "activations"));

        let used = act(&state, activate(P1, &once.id, hero_target())).0;
        assert!(matches_object(
            &json_of(view_for(&used, P1))["you"]["backrow"][0],
            &json!({
                "activations": [{ "ability": "ping", "usesLeft": 0, "usable": false, "reason": "that ability has already been used this turn" }],
            })
        ));
    }

    #[test]
    fn r384_r97_the_activated_event_is_public_while_its_card_is_readable_and_the_sentinel_once_it_is_in_a_hand() {
        let mut state = playing("event");
        let card = put(&mut state, &pinger.id, slot(P1, Row::Backrow, 1), json!({}));
        let after = act(&state, activate(P1, &card.id, hero_target())).0;
        let seen = |viewer: PlayerId, at: &GameState| -> Value {
            view_for(at, viewer)
                .events
                .iter()
                .find(|event| event.event_type() == GameEventType::Activated)
                .map(json_of)
                .unwrap_or(Value::Null)
        };

        assert!(matches_object(
            &seen(P2, &after),
            &json!({ "instanceId": card.id, "defId": pinger.id })
        ));
        let mut moved = round_trip(&after);
        let on_field = moved.players.p1.backrow[0].clone().expect("on the field");
        move_to_zone(&mut moved, &on_field.id, OffFieldZone::Hand, MoveToZoneOptions::default());
        assert!(matches_object(
            &seen(P2, &moved),
            &json!({ "instanceId": HIDDEN_ID, "defId": HIDDEN_ID, "ability": "ping" })
        ));
        assert!(matches_object(&seen(P1, &moved), &json!({ "instanceId": card.id })));
    }
}

mod r102_r384_a_fused_cards_abilities_each_in_its_ingredients_place {
    use super::*;

    #[test]
    fn r102_a_card_a_fuse_kept_still_activates_on_what_its_own_text_remembered_and_reads_it_back() {
        let mut state = playing("fused-keeper");
        let card = put(&mut state, &keeper.id, slot(P1, Row::Backrow, 1), json!({}));
        remember_on(
            &mut card_mut(&mut state, &card.id).memory,
            &IndexMap::new(),
            KEEPER_KEY,
            json!("spell"),
        );
        let other = in_hand(&mut state, &endless.id, P1, 1).remove(0);
        assert_eq!(
            fuse_onto(&mut state, vec![other], &card).map(|fused| fused.id),
            Some(card.id.clone())
        );
        // R77: the kept card is the last ingredient, and what its text remembered moved to that place.
        let kept = super::card(&state, &card.id);
        assert_eq!(kept.memory.get(&format!("{KEEPER_KEY}@1")).cloned(), Some(json!("spell")));
        assert_eq!(kept.memory.get(KEEPER_KEY), None);
        assert_eq!(ability_ids(&state, &card.id), vec!["again", "recall"]);
        assert_eq!(why_not(&state, P1, &card.id, Some("recall")), None);
        assert_eq!(
            notes(&act(&state, activate(P1, &card.id, json!({ "ability": "recall" }))).0),
            vec!["recall:spell"]
        );
    }

    #[test]
    fn r102_two_fused_copies_read_their_own_memories_the_one_that_remembered_nothing_cant_be_activated() {
        let mut state = playing("fused-keepers");
        let card = put(&mut state, &keeper.id, slot(P1, Row::Backrow, 1), json!({}));
        remember_on(
            &mut card_mut(&mut state, &card.id).memory,
            &IndexMap::new(),
            KEEPER_KEY,
            json!("mine"),
        );
        let copy = in_hand(&mut state, &keeper.id, P1, 1).remove(0);
        fuse_onto(&mut state, vec![copy], &card);
        assert_eq!(ability_ids(&state, &card.id), vec!["recall", "recall#2"]);
        assert_eq!(
            why_not(&state, P1, &card.id, Some("recall")).as_deref(),
            Some("that ability can't be activated now")
        );
        assert_eq!(why_not(&state, P1, &card.id, Some("recall#2")), None);
        assert_eq!(listed_abilities(&state, &card.id), vec![Some("recall#2".to_string())]);
        assert_eq!(
            notes(&act(&state, activate(P1, &card.id, json!({ "ability": "recall#2" }))).0),
            vec!["recall:mine"]
        );
    }

    #[test]
    fn r102_a_fused_ability_reads_its_own_declared_number_not_one_an_ingredient_ahead_of_it_declares_under_the_same_key() {
        let mut state = playing("fused-zapper");
        let card = put(&mut state, &zapper.id, slot(P1, Row::Backrow, 1), json!({}));
        let first = in_hand(&mut state, &spark.id, P1, 1).remove(0);
        fuse_onto(&mut state, vec![first], &card);
        let after = act(
            &state,
            activate(
                P1,
                &card.id,
                json!({ "ability": "zap", "targets": [{ "pick": "hero", "player": "p2" }] }),
            ),
        )
        .0;
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH - ZAPPER_DAMAGE);
    }

    #[test]
    fn r102_an_abilitys_has_on_a_fused_card_still_reads_the_engines_own_entry_which_no_fuse_moves() {
        let mut state = playing("fused-chooser");
        let card = put(&mut state, &chooser.id, slot(P1, Row::Backrow, 1), json!({}));
        card_mut(&mut state, &card.id).memory.insert(PICK_KEY.to_string(), json!("beta"));
        let other = in_hand(&mut state, &endless.id, P1, 1).remove(0);
        fuse_onto(&mut state, vec![other], &card);
        assert_eq!(ability_ids(&state, &card.id), vec!["again", "beta", "gamma"]);
        assert_eq!(
            notes(&act(&state, activate(P1, &card.id, json!({ "ability": "beta" }))).0),
            vec!["beta"]
        );
    }

    #[test]
    fn r102_r113_a_fused_cards_activation_survives_json_and_replays_to_the_same_hash() {
        let mut state = playing("fused-roundtrip");
        let card = put(&mut state, &keeper.id, slot(P1, Row::Backrow, 1), json!({}));
        remember_on(
            &mut card_mut(&mut state, &card.id).memory,
            &IndexMap::new(),
            KEEPER_KEY,
            json!("spell"),
        );
        let other = in_hand(&mut state, &endless.id, P1, 1).remove(0);
        fuse_onto(&mut state, vec![other], &card);
        let copy = round_trip(&state);
        let live = act(&state, activate(P1, &card.id, json!({ "ability": "recall" }))).0;
        let restored = act(&copy, activate(P1, &card.id, json!({ "ability": "recall" }))).0;
        assert_eq!(notes(&restored), vec!["recall:spell"]);
        assert_eq!(hash_state(&restored), hash_state(&live));
    }

    #[test]
    fn r102_r113_a_fused_ability_that_asks_mid_list_parks_its_tail_under_its_own_hook_and_the_answer_finishes_it_in_its_place()
     {
        let mut state = playing("fused-asker");
        let card = put(&mut state, &asker.id, slot(P1, Row::Backrow, 1), json!({}));
        let other = in_hand(&mut state, &endless.id, P1, 1).remove(0);
        fuse_onto(&mut state, vec![other], &card);
        let paused = act(&state, activate(P1, &card.id, json!({ "ability": "ask" }))).0;

        assert_eq!(notes(&paused), vec!["ask"]);
        assert_eq!(
            paused.work.iter().map(|item| item.resume.hook.clone()).collect::<Vec<String>>(),
            vec![activation_hook("ask")]
        );
        let copy = round_trip(&paused);
        assert_eq!(copy, paused);

        let live = answer(&paused).0;
        assert_eq!(notes(&live), vec!["ask", "answered", "ask:tail"]);
        assert!(live.work.is_empty());
        assert_eq!(hash_state(&answer(&copy).0), hash_state(&live));
    }

    #[test]
    fn r102_r113_a_question_an_ability_asks_two_fusions_down_comes_back_to_its_own_ingredient_not_the_first_that_names_its_step()
     {
        let mut state = playing("fused-tellers");
        let card = put(&mut state, &high_teller.id, slot(P1, Row::Backrow, 1), json!({}));
        let low = in_hand(&mut state, &low_teller.id, P1, 1).remove(0);
        let outer = in_hand(&mut state, &spark.id, P1, 1).remove(0);
        // Low Teller + High Teller, then a Spark fused onto that: High Teller's text runs at place 1.1.
        fuse_onto(&mut state, vec![low], &card);
        fuse_onto(&mut state, vec![outer], &card);
        assert_eq!(ability_ids(&state, &card.id), vec!["tell", "tell#2"]);
        let before = round_trip(&state);

        let paused = act(&state, activate(P1, &card.id, json!({ "ability": "tell#2" }))).0;
        let copy = round_trip(&paused);
        let live = answer(&paused).0;
        assert_eq!(notes(&live), vec![format!("told:{HIGH_TELL}")]);
        assert_eq!(hash_state(&answer(&copy).0), hash_state(&live));
        assert_eq!(
            notes(&answer(&act(&before, activate(P1, &card.id, json!({ "ability": "tell" }))).0).0),
            vec![format!("told:{LOW_TELL}")]
        );
    }
}
