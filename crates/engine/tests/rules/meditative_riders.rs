//! The Meditative riders part's engine rules (docs/meditative-set.md M5): ME-STATS's leader
//! (R1100), De-Radiant (R1102), the either-player Activate (R1105), granted Deaths (R1107), multi-pool
//! fusion (R1108) and `manaSpent` (R1109). Each through the part's fixtures; the real cards' tests
//! cover the same cases again.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::effects::summon::{SummonCopyArgs, clone_of};
use jackioh_engine::effects::targets::TargetSpec;
use jackioh_engine::effects::{clear_radiant, grant_ability, steal};
use jackioh_engine::subsystems::activate::{activation_views_for, why_cannot_activate_ability};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::catalog::token_def;
use crate::rules::fixtures::harness::{in_hand, new_game, put, sink_for, slot};
use crate::rules::fixtures::meditative_riders::{
    GRANT_KEY, LOG_LANE, RIDER_SCRIPTS, bearer, book, closed_mic, ear, fuser, granter, log_card,
    mind, notes, open_mic, riders_catalog,
};

/// TS's module-level `let nonce = 0`: unique across the file's tests, which Rust runs on many threads.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn register() {
    register_catalog(riders_catalog(registered_catalog().clone()));
    let mut scripts = registered_scripts().clone();
    for (id, script) in RIDER_SCRIPTS.clone() {
        scripts.insert(id, script);
    }
    register_scripts(scripts);
}

fn game(seed: &str) -> GameState {
    let state = new_game(&format!("riders-{seed}"), None);
    register();
    state
}

fn act_result(state: &GameState, body: ActionInput) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    reduce(state, &body.with_nonce(format!("rider{nonce}")))
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
        let keep: Vec<String> = state.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        state = act(
            &state,
            input(json!({ "type": "mulligan", "keep": keep, "playerId": player })),
        )
        .0;
    }
    put(
        &mut state,
        &log_card.id,
        slot(P2, Row::Backrow, LOG_LANE),
        json!({}),
    );
    state.players.p1.auto_end_turn = Some(false);
    state.players.p2.auto_end_turn = Some(false);
    state
}

fn activate(player: PlayerId, instance_id: &str) -> ActionInput {
    input(json!({ "type": "activate", "instanceId": instance_id, "playerId": player }))
}

/// The live card (TS held the object itself; Rust looks it up by id).
fn card<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

fn card_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

/// The effect applied on a sink over `state`, the rng cursor written back; its events.
fn run(state: &mut GameState, effect: Effect) -> Vec<GameEvent> {
    run_as(state, effect, P1)
}

fn run_as(state: &mut GameState, effect: Effect, controller: PlayerId) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(controller),
                ..HookOptions::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

/// The events of one action, as a player's view sends them.
fn sent(state: &mut GameState, events: Vec<GameEvent>, viewer: PlayerId) -> Value {
    state.applied = vec![AppliedAction {
        nonce: "t".to_string(),
        events,
    }];
    serde_json::to_value(view_for(state, viewer).events).expect("serialises")
}

fn table(cards: Vec<(&str, i32, i32)>) -> WinRateTable {
    WinRateTable {
        patch: "v0.3.3".to_string(),
        source: WinRateSource::Provisional,
        cards: cards
            .into_iter()
            .map(|(id, wins, games)| WinRateRow {
                id: id.to_string(),
                wins,
                games,
            })
            .collect(),
    }
}

/// R1100 (MD-D1): the leader is the best exact rate, then more games, then the catalog id.
#[test]
fn r1100_leader_by_exact_rate_then_games_then_id() {
    register();
    register_win_rates(table(vec![
        (&open_mic.id, 30, 40),
        (&closed_mic.id, 3, 4),
        (&granter.id, 15, 30),
        (&ear.id, 15, 30),
        (&bearer.id, 10, 20),
    ]));
    // 30/40 beats every .5 row on the exact rate; 3/4 never qualifies (fewer than 20 games).
    assert_eq!(win_rate_leader("meditative-050"), Some(open_mic.id.clone()));
    // Without the leader, the .5 rows tie on rate and games: the catalog id decides.
    assert_eq!(win_rate_leader(&open_mic.id), Some(ear.id.clone()));
}

/// R1100 (MD-D1): tokens, missing ids and thin rows never lead — even a thin 100% row, even alone.
#[test]
fn r1100_tokens_missing_and_thin_rows_never_lead() {
    register();
    let token = token_def("rider", [Tag::Token]);
    let mut defs = registered_catalog().clone();
    defs.insert(token.id.clone(), token.clone());
    register_catalog(defs);
    register_win_rates(table(vec![
        (&token.id, 40, 40),
        ("no-such-card", 50, 50),
        (&closed_mic.id, 19, 19),
        (&book.id, 5, 10),
    ]));
    assert_eq!(win_rate_leader("meditative-050"), None);
}

/// R1100 (MD-D1): an empty table names no leader (a new patch, before any game is counted).
#[test]
fn r1100_empty_table_no_leader() {
    register();
    register_win_rates(table(vec![]));
    assert_eq!(win_rate_leader("meditative-050"), None);
}

/// R1102 (MD-D6): De-Radiant clears the flag — the face swaps back while damage, buffs and tuning
/// stay — and reports `deradianted`.
#[test]
fn r1102_deradiant_swaps_face_keeps_damage_and_buffs() {
    let mut state = playing("deradiant");
    let unit = put(&mut state, &open_mic.id, slot(P1, Row::Units, 1), json!({ "radiant": true }));
    card_mut(&mut state, &unit.id).damage = 1;
    card_mut(&mut state, &unit.id).buffs = AttackHealth { attack: 2, health: 0 };
    assert!(card(&state, &unit.id).radiant);
    // Radiant doubles 2 to 4, and the +2 buff rides on top.
    assert_eq!(unit_view(&state, card(&state, &unit.id)).attack, 6);

    let events = run(
        &mut state,
        clear_radiant(json_as(json!({ "instanceId": unit.id }))),
    );
    let after = card(&state, &unit.id);
    assert!(!after.radiant);
    assert_eq!(after.damage, 1);
    assert_eq!(after.buffs, AttackHealth { attack: 2, health: 0 });
    assert_eq!(unit_view(&state, after).attack, 4);
    assert_eq!(events.len(), 1);
    match &events[0] {
        GameEvent::Deradianted { instance_id, def_id, zone } => {
            assert_eq!(instance_id, &unit.id);
            assert_eq!(def_id, &open_mic.id);
            assert_eq!(zone.z(), ZoneName::Field);
        }
        _ => panic!("not deradianted"),
    }
    // A card that is not Radiant is untouched: no flag, no event.
    let plain = put(&mut state, &closed_mic.id, slot(P1, Row::Units, 2), json!({}));
    let events = run(
        &mut state,
        clear_radiant(json_as(json!({ "instanceId": plain.id }))),
    );
    assert!(!card(&state, &plain.id).radiant);
    assert!(events.is_empty());
}

/// R1102 (MD-D6): a cleared hand card reads redacted to the opponent, as a `radiantSet` does.
#[test]
fn r1102_deradianted_hidden_in_a_hand() {
    let mut state = playing("deradiant-hand");
    let held = in_hand(&mut state, &open_mic.id, P1, 1).remove(0);
    card_mut(&mut state, &held.id).radiant = true;
    let events = run(
        &mut state,
        clear_radiant(json_as(json!({ "instanceId": held.id }))),
    );
    assert!(!card(&state, &held.id).radiant);
    assert_eq!(events.len(), 1);
    let p2 = sent(&mut state, events, P2);
    assert_eq!(p2[0]["instanceId"], json!("hidden"));
    assert_eq!(p2[0]["type"], json!("deradianted"));
}

/// R1105 (MD-D9): the opponent activates the open ability in their own main phase, paying the tuned
/// price from their own mana, and the run pays them.
#[test]
fn r1105_opponent_activates_paying_own_mana() {
    let mut state = playing("either");
    let mic = put(&mut state, &open_mic.id, slot(P1, Row::Units, 1), json!({}));
    let p1_mana = state.players.p1.mana.current;
    state = act(&state, input(json!({ "type": "endTurn", "playerId": "p1" }))).0;
    assert_eq!(state.active, P2);
    // Start of turn refreshes current mana to max, so the price money is set after the pass.
    state.players.p2.mana.max = 5;
    state.players.p2.mana.current = 5;
    // The opponent's seat lists the ability; the controller's ability list is unchanged.
    let listed: Vec<Option<String>> = legal_actions(&state, P2)
        .into_iter()
        .filter_map(|body| match body {
            ActionBody::Activate { instance_id, ability, .. } if instance_id == mic.id => {
                Some(ability)
            }
            _ => None,
        })
        .collect();
    assert_eq!(listed, vec![Some("share".to_string())]);
    let (after, _) = act(&state, activate(P2, &mic.id));
    // Paid 2 from p2's own mana, then the run gained p2 1.
    assert_eq!(after.players.p2.mana.current, 4);
    assert_eq!(after.players.p1.mana.current, p1_mana);
}

/// R1105 (MD-D9): a controller-only ability still refuses the opponent with "that card is not
/// yours", word for word.
#[test]
fn r1105_controller_only_ability_still_not_yours() {
    let mut state = playing("closed");
    let mic = put(&mut state, &closed_mic.id, slot(P1, Row::Units, 1), json!({}));
    state = act(&state, input(json!({ "type": "endTurn", "playerId": "p1" }))).0;
    let listed: Vec<ActionBody> = legal_actions(&state, P2)
        .into_iter()
        .filter(|body| {
            matches!(body, ActionBody::Activate { instance_id, .. } if instance_id == &mic.id)
        })
        .collect();
    assert!(listed.is_empty());
    let refused = why_cannot_activate_ability(&state, P2, &mic.id, Some("share"))
        .err()
        .map(|error| error.message);
    assert_eq!(refused.as_deref(), Some("that card is not yours"));
}

/// R1105 (MD-D9): the controller's view lists the ability, and the opponent's view lists only the
/// open one.
#[test]
fn r1105_both_views_list_it() {
    let mut state = playing("views");
    let mic = put(&mut state, &open_mic.id, slot(P1, Row::Units, 1), json!({}));
    let shy = put(&mut state, &closed_mic.id, slot(P1, Row::Units, 2), json!({}));
    let own: Vec<String> = activation_views_for(&state, P1, card(&state, &mic.id))
        .unwrap_or_default()
        .into_iter()
        .map(|view| view.ability)
        .collect();
    assert_eq!(own, vec!["share".to_string()]);
    let seen: Vec<String> = activation_views_for(&state, P2, card(&state, &mic.id))
        .unwrap_or_default()
        .into_iter()
        .map(|view| view.ability)
        .collect();
    assert_eq!(seen, vec!["share".to_string()]);
    assert!(activation_views_for(&state, P2, card(&state, &shy.id)).is_none());
}

/// R1107 (MD-D13): the granted Death runs after the card's own, for the controller at the death —
/// granted while p1 holds it, stolen, it still fires for p2.
#[test]
fn r1107_granted_death_runs_after_own_for_controller_at_death() {
    let mut state = playing("grant");
    let target = put(&mut state, &bearer.id, slot(P1, Row::Units, 1), json!({}));
    run(
        &mut state,
        grant_ability(json_as(json!({
            "instanceId": target.id,
            "grant": GRANT_KEY,
        }))),
    );
    assert_eq!(card(&state, &target.id).grants.as_ref().map(Vec::len), Some(1));
    // The grant shows on the Unit, as its line reads.
    assert_eq!(
        grant_texts(&state, card(&state, &target.id)),
        Some(vec!["Death: notes `granted`.".to_string()])
    );
    // Stolen: p2 holds it at its death, and both Deaths still fire, own first.
    run_as(
        &mut state,
        steal(json_as(json!({ "instanceId": target.id }))),
        P2,
    );
    assert_eq!(card(&state, &target.id).controller, P2);
    card_mut(&mut state, &target.id).marked_destroyed = Some(true);
    let mut sink = sink_for(&mut state);
    state_check(&mut sink);
    assert_eq!(notes(sink.state), vec!["own".to_string(), "granted".to_string()]);
}

/// R1107 (MD-D13): the grant survives a JSON round trip, a Vanilla and a copy, and is lost when the
/// card leaves the field.
#[test]
fn r1107_grants_survive_json_vanilla_copy_and_reset_off_field() {
    let mut state = playing("grant-keep");
    let target = put(&mut state, &bearer.id, slot(P1, Row::Units, 1), json!({}));
    run(
        &mut state,
        grant_ability(json_as(json!({
            "instanceId": target.id,
            "grant": GRANT_KEY,
        }))),
    );
    // A JSON round trip keeps it.
    let round: GameState = serde_json::from_value(serde_json::to_value(&state).expect("serialises"))
        .expect("deserialises");
    assert_eq!(
        card(&round, &target.id).grants,
        card(&state, &target.id).grants
    );
    // A Vanilla keeps it (grants behave like granted keywords, §10.4).
    card_mut(&mut state, &target.id).vanilla = true;
    assert_eq!(card(&state, &target.id).grants.as_ref().map(Vec::len), Some(1));
    // A copy keeps it (R57).
    let source = card(&state, &target.id).clone();
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let copy = {
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        let mut ctx = make_context(&mut sink, None, HookOptions::default());
        clone_of(
            &mut ctx,
            &source,
            P1,
            &SummonCopyArgs {
                of: TargetSpec::Chosen { index: None },
                player: None,
                lane: None,
                stack: None,
                vanilla: None,
                granted_keywords: None,
            },
        )
    };
    assert_eq!(copy.grants, source.grants);
    // Leaving the field loses it (R78).
    let mut gone = card(&state, &target.id).clone();
    reset_instance(&mut gone);
    assert_eq!(gone.grants, None);
}

/// R1108 (MD-D14): one pick per pool — the Book first, the AI second — fused into one hand card.
#[test]
fn r1108_one_pick_per_pool() {
    register();
    let mut state = playing("pools");
    assert!(find_def(Some(&state), &book.id).is_some());
    assert!(find_def(Some(&state), &mind.id).is_some());
    let mut hand = in_hand(&mut state, &fuser.id, P1, 1);
    let spell = hand.remove(0);
    let (after, _) = act(
        &state,
        input(json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" })),
    );
    let fused = after.players.p1.hand.iter().find(|held| held.def_id.starts_with("t-"));
    let fused = fused.unwrap_or_else(|| panic!("a fused card reaches the hand"));
    let def = def_of(Some(&after), &fused.def_id);
    let ingredients: Vec<String> = def
        .ingredients
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|ingredient| ingredient.def_id)
        .collect();
    assert_eq!(ingredients, vec![book.id.clone(), mind.id.clone()]);
}

/// R1109 (MD-D26): while a card answers it, a play's price and an activation's price are reported.
#[test]
fn r1109_reports_play_and_activate_prices_when_heard() {
    let mut state = playing("heard");
    put(&mut state, &ear.id, slot(P1, Row::Units, 2), json!({}));
    assert!(mana_spent_heard(&state));
    let mic = put(&mut state, &open_mic.id, slot(P1, Row::Units, 1), json!({}));
    state.players.p1.mana.current = 10;
    let (after, events) = act(&state, activate(P1, &mic.id));
    let spent: Vec<(i32, String)> = events
        .iter()
        .filter_map(|event| match event {
            GameEvent::ManaSpent { amount, for_, .. } => {
                Some((*amount, serde_json::to_value(for_).expect("serialises").as_str().unwrap_or("?").to_string()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(spent, vec![(2, "activate".to_string())]);
    assert_eq!(notes(&after), vec!["heard".to_string()]);
    // A play's price is reported too.
    let played = in_hand(&mut state, &closed_mic.id, P1, 1).remove(0);
    state.players.p1.mana.current = 10;
    let (_, events) = act(
        &state,
        input(json!({ "type": "play", "instanceId": played.id, "zone": { "row": "units", "lane": 3 }, "playerId": "p1" })),
    );
    let spent: Vec<(i32, String)> = events
        .iter()
        .filter_map(|event| match event {
            GameEvent::ManaSpent { amount, for_, .. } => {
                Some((*amount, serde_json::to_value(for_).expect("serialises").as_str().unwrap_or("?").to_string()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(spent, vec![(1, "play".to_string())]);
}

/// R1109 (MD-D26): with no listener, no `manaSpent` is emitted and the game hashes as before.
#[test]
fn r1109_silent_without_listener() {
    let mut state = playing("unheard");
    assert!(!mana_spent_heard(&state));
    let mic = put(&mut state, &open_mic.id, slot(P1, Row::Units, 1), json!({}));
    state.players.p1.mana.current = 10;
    let (_, events) = act(&state, activate(P1, &mic.id));
    assert!(
        !events
            .iter()
            .any(|event| event.event_type() == GameEventType::ManaSpent)
    );
}
