//! The fuzz monitor's I7 (`testkit/invariants.rs`): a live game with no prompt open owes no paused
//! sequence, so `work` is empty and `work_cursor` is 0 (§9.3, §10.3, R113). A prompt, the open
//! mulligans (§10.6, R265) or a game over each excuse what is still owed.
//!
//! And its I9 (#1719): each `turnStarted`, `turnEnded`, `turnAutoEnded`, `gameOver` and `manaChanged`
//! an action announces says what the state it left holds. Each `fatigue` is its seat's next fatigue draw
//! at `FATIGUE_DAMAGE` of its count, and each seat's `fatigue_count` moves only with one (§2.4, R3, R315;
//! #1902). Each check is proved by tampering with one payload of a real action's events; the untampered
//! events are clean.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::{spell_def, vanilla_deck};
use crate::rules::fixtures::harness::{both, new_game, put, setup_catalog, slot};

/// Past both mulligans, in the main phase of p1's first turn: nothing owed, no prompt open.
fn playing(seed: &str) -> GameState {
    past_mulligans(&new_game(seed, None))
}

/// `game` begun and both mulligans kept whole.
fn past_mulligans(game: &GameState) -> GameState {
    let mut state = begin_game(game).state;
    for player in [P1, P2] {
        let keep: Vec<String> = state.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        let action = Action::new(ActionBody::Mulligan { keep }, player, format!("i7-{player}"));
        let result = reduce(&state, &action);
        assert!(result.error.is_none(), "{:?}", result.error);
        state = result.state;
    }
    state
}

/// A paused sequence nothing will resume: no fixture script has the hook.
fn orphan() -> WorkItem {
    WorkItem {
        id: "w-orphan".to_string(),
        seq: 0,
        owner: P1,
        resume: Resume {
            def_id: "fx-1".to_string(),
            hook: "resume".to_string(),
            step: "orphaned".to_string(),
            radiant: false,
            instance_id: None,
            data: IndexMap::new(),
        },
    }
}

fn orphaned(found: &[String]) -> Vec<&String> {
    found
        .iter()
        .filter(|finding| finding.starts_with("I7 orphaned work"))
        .collect()
}

fn invariant(id: &str, found: Vec<String>) -> Vec<String> {
    found
        .into_iter()
        .filter(|finding| finding.starts_with(id))
        .collect()
}

#[test]
fn i7_a_settled_game_with_no_prompt_open_owes_nothing_and_the_monitor_is_clean() {
    let state = playing("i7-clean");
    assert!(state.pending.is_none() && state.mulligan.is_none() && state.result.is_none());
    assert!(state.work.is_empty());
    assert_eq!(state.work_cursor, 0);
    let mut monitor = create_invariant_monitor(&state);
    assert_eq!(monitor.after(&[], &state), Vec::<String>::new());
}

#[test]
fn i7_a_work_item_left_with_no_prompt_open_is_orphaned_work() {
    let mut state = playing("i7-item");
    state.work.push(orphan());
    let mut monitor = create_invariant_monitor(&state);
    let found = monitor.after(&[], &state);
    let i7 = orphaned(&found);
    assert_eq!(i7.len(), 1, "{found:?}");
    assert!(i7[0].contains("w-orphan"), "{}", i7[0]);
    assert!(i7[0].contains("R113"), "{}", i7[0]);
}

#[test]
fn i7_a_work_cursor_left_off_zero_with_nothing_owed_is_orphaned_work() {
    let mut state = playing("i7-cursor");
    state.work_cursor = 1;
    let mut monitor = create_invariant_monitor(&state);
    let found = monitor.after(&[], &state);
    assert_eq!(orphaned(&found).len(), 1, "{found:?}");
}

#[test]
fn i7_work_owed_behind_an_open_prompt_is_not_orphaned() {
    let begun = begin_game(&new_game("i7-prompt", None)).state;
    // The mulligans are open (§10.6, R265): owed work waits for them.
    let mut mulligan = begun.clone();
    mulligan.work.push(orphan());
    mulligan.work_cursor = 1;
    let mut monitor = create_invariant_monitor(&mulligan);
    assert_eq!(orphaned(&monitor.after(&[], &mulligan)), Vec::<&String>::new());

    // A prompt in `pending` (§10.6): the answer resumes the work.
    let mut prompted = playing("i7-prompt");
    let prompt = begun.mulligan.as_ref().expect("the mulligans are open")[P1]
        .prompt
        .clone();
    prompted.pending = Some(prompt);
    prompted.work.push(orphan());
    prompted.work_cursor = 1;
    let mut monitor = create_invariant_monitor(&prompted);
    assert_eq!(orphaned(&monitor.after(&[], &prompted)), Vec::<&String>::new());
}

#[test]
fn i7_work_a_game_over_left_owed_is_not_orphaned() {
    let mut state = playing("i7-over");
    state.result = Some(GameResult {
        winner: Winner::P1,
        reason: GameOverReason::Concede,
    });
    state.work.push(orphan());
    let mut monitor = create_invariant_monitor(&state);
    assert_eq!(orphaned(&monitor.after(&[], &state)), Vec::<&String>::new());
}

// ---------------------------------------------------------------------------------------------
// I11: Unit tokens never persist off the field (§3, §3.2, R11)
// ---------------------------------------------------------------------------------------------

#[test]
fn i11_unit_tokens_in_graveyard_exile_or_held_are_flagged_and_a_spell_token_in_graveyard_is_allowed() {
    let mut graveyard = playing("i11-graveyard");
    let token = new_instance(
        &mut graveyard,
        "fx-token-rush",
        P1,
        Zone::Graveyard { player: P1 },
    );
    graveyard.players[P1].graveyard.push(token);
    let found = invariant("I11", create_invariant_monitor(&graveyard).after(&[], &graveyard));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("p1's graveyard"), "{}", found[0]);

    let mut exile = playing("i11-exile");
    let token = new_instance(&mut exile, "fx-token-rush", P2, Zone::Exile { player: P2 });
    exile.players[P2].exile.push(token);
    let found = invariant("I11", create_invariant_monitor(&exile).after(&[], &exile));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("p2's exile"), "{}", found[0]);

    let mut held = playing("i11-held");
    let token = new_instance(&mut held, "fx-token-rush", P1, Zone::Gone { player: P1 });
    held.held = Some(vec![HeldCard {
        holder_id: "i11-holder".to_string(),
        holder_mark: 0,
        to: P1,
        card: token,
        exile_on_release: None,
    }]);
    let found = invariant("I11", create_invariant_monitor(&held).after(&[], &held));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("in held"), "{}", found[0]);

    let mut spell = playing("i11-spell-token");
    let mut defs = catalog_override().expect("the fixture catalog").clone();
    let spell_token = spell_def(999, json!({ "token": true, "rarity": "Token" }));
    defs.insert(spell_token.id.clone(), spell_token);
    register_catalog(defs);
    let token = new_instance(&mut spell, "fx-999", P1, Zone::Graveyard { player: P1 });
    spell.players[P1].graveyard.push(token);
    assert_eq!(
        invariant("I11", create_invariant_monitor(&spell).after(&[], &spell)),
        Vec::<String>::new()
    );
}

// ---------------------------------------------------------------------------------------------
// I12: a switch and an attack share exertion unless the Unit has Deft (§4.1, R49, R636)
// ---------------------------------------------------------------------------------------------

fn exertion_state(seed: &str, attacker_def: &str) -> (GameState, String, String) {
    let mut state = playing(seed);
    let attacker = put(&mut state, attacker_def, slot(P1, Row::Units, 1), json!({}));
    let defender = put(&mut state, "fx-21", slot(P2, Row::Units, 1), json!({}));
    (state, attacker.id, defender.id)
}

/// Give `def_id` an `afterAttack` hook that does not refresh exertion. I2/I12 must treat its kill
/// credit as speculative until the settled state or a later declaration proves a refresh happened.
fn register_no_refresh_after_attack(def_id: &str) {
    let mut scripts = registered_scripts().clone();
    scripts.insert(
        def_id.to_string(),
        both(Script {
            after_attack: Some(hook(|_ctx| vec![])),
            ..Script::default()
        }),
    );
    register_scripts(scripts);
}

fn forced_attack(attacker_id: &str, target_id: &str) -> GameEvent {
    GameEvent::AttackDeclared {
        attacker_id: attacker_id.to_string(),
        target_id: target_id.to_string(),
        forced: true,
        instead_of: None,
    }
}

fn destroyed_by(instance_id: &str, killer_id: &str) -> GameEvent {
    GameEvent::Destroyed {
        instance_id: instance_id.to_string(),
        def_id: "fx-21".to_string(),
        owner: P2,
        controller: P2,
        attack: 1,
        max_health: 1,
        killer_id: Some(killer_id.to_string()),
        radiant: None,
    }
}

#[test]
fn i12_a_non_deft_unit_that_switched_cannot_declare_an_attack_but_deft_can() {
    for (seed, attacker_def, violates) in [
        ("i12-plain", "fx-1", true),
        ("i12-deft", "cb-deft-duelist", false),
    ] {
        let (state, attacker_id, defender_id) = exertion_state(seed, attacker_def);
        let mut monitor = create_invariant_monitor(&state);
        let switch = ActionBody::SwitchPosition {
            instance_id: attacker_id.clone(),
        };
        let _ = monitor.before(&state, P1, &switch);
        assert_eq!(
            invariant(
                "I12",
                monitor.after(
                    &[GameEvent::PositionSwitched {
                        instance_id: attacker_id.clone(),
                        position: Position::Def,
                    }],
                    &state,
                ),
            ),
            Vec::<String>::new()
        );

        let attack = ActionBody::Attack {
            attacker_id: attacker_id.clone(),
            target_id: defender_id.clone(),
        };
        let _ = monitor.before(&state, P1, &attack);
        let found = invariant(
            "I12",
            monitor.after(
                &[GameEvent::AttackDeclared {
                    attacker_id,
                    target_id: defender_id,
                    forced: false,
                    instead_of: None,
                }],
                &state,
            ),
        );
        assert_eq!(found.len(), usize::from(violates), "{found:?}");
        if violates {
            assert!(found[0].starts_with("I12 attack after switch"), "{}", found[0]);
            assert!(found[0].contains("fx-1"), "{}", found[0]);
        }
    }
}

#[test]
fn i12_a_windfury_unit_that_attacked_once_cannot_switch() {
    let (state, attacker_id, defender_id) = exertion_state("i12-windfury", "cb-windfurier");
    let mut monitor = create_invariant_monitor(&state);
    let attack = ActionBody::Attack {
        attacker_id: attacker_id.clone(),
        target_id: defender_id.clone(),
    };
    let _ = monitor.before(&state, P1, &attack);
    assert_eq!(
        invariant(
            "I12",
            monitor.after(
                &[GameEvent::AttackDeclared {
                    attacker_id: attacker_id.clone(),
                    target_id: defender_id,
                    forced: false,
                    instead_of: None,
                }],
                &state,
            ),
        ),
        Vec::<String>::new()
    );

    let switch = ActionBody::SwitchPosition {
        instance_id: attacker_id.clone(),
    };
    let _ = monitor.before(&state, P1, &switch);
    let found = invariant(
        "I12",
        monitor.after(
            &[GameEvent::PositionSwitched {
                instance_id: attacker_id,
                position: Position::Def,
            }],
            &state,
        ),
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("I12 switch after attack"), "{}", found[0]);
    assert!(found[0].contains("R636"), "{}", found[0]);
}

#[test]
fn i12_effect_switches_and_forced_attacks_spend_nothing() {
    let (state, attacker_id, defender_id) = exertion_state("i12-effects", "fx-1");
    let mut monitor = create_invariant_monitor(&state);
    let switch = ActionBody::SwitchPosition {
        instance_id: attacker_id.clone(),
    };
    let _ = monitor.before(&state, P1, &switch);
    assert_eq!(
        invariant(
            "I12",
            monitor.after(
                &[
                    GameEvent::PositionSwitched {
                        instance_id: attacker_id.clone(),
                        position: Position::Def,
                    },
                    // R20: a later effect in the reduction switches it back without another spend.
                    GameEvent::PositionSwitched {
                        instance_id: attacker_id.clone(),
                        position: Position::Atk,
                    },
                ],
                &state,
            ),
        ),
        Vec::<String>::new()
    );

    let _ = monitor.before(&state, P1, &ActionBody::EndTurn);
    assert_eq!(
        invariant(
            "I12",
            monitor.after(
                &[GameEvent::AttackDeclared {
                    attacker_id,
                    target_id: defender_id,
                    forced: true,
                    instead_of: None,
                }],
                &state,
            ),
        ),
        Vec::<String>::new()
    );
}

#[test]
fn i12_a_forced_kill_that_refreshes_nothing_does_not_forget_a_spent_switch() {
    let (mut state, attacker_id, defender_id) = exertion_state("i12-forced-no-refresh", "fx-1");
    register_no_refresh_after_attack("fx-1");
    let mut monitor = create_invariant_monitor(&state);
    let switch = ActionBody::SwitchPosition {
        instance_id: attacker_id.clone(),
    };
    let _ = monitor.before(&state, P1, &switch);
    let _ = monitor.after(
        &[GameEvent::PositionSwitched {
            instance_id: attacker_id.clone(),
            position: Position::Def,
        }],
        &state,
    );
    find_instance_mut(&mut state, &attacker_id)
        .expect("the attacker")
        .exertion
        .switched = true;

    let _ = monitor.before(&state, P1, &ActionBody::EndTurn);
    assert_eq!(
        invariant(
            "I12",
            monitor.after(
                &[
                    forced_attack(&attacker_id, &defender_id),
                    destroyed_by(&defender_id, &attacker_id),
                    // R53: another forced attack does not consume the speculative refresh.
                    forced_attack(&attacker_id, "hero-p2"),
                ],
                &state,
            ),
        ),
        Vec::<String>::new()
    );

    let attack = ActionBody::Attack {
        attacker_id: attacker_id.clone(),
        target_id: "hero-p2".to_string(),
    };
    let _ = monitor.before(&state, P1, &attack);
    let found = invariant(
        "I12",
        monitor.after(
            &[GameEvent::AttackDeclared {
                attacker_id,
                target_id: "hero-p2".to_string(),
                forced: false,
                instead_of: None,
            }],
            &state,
        ),
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("I12 attack after switch"), "{}", found[0]);
}

#[test]
fn i12_a_declaration_in_the_forced_kill_batch_consumes_a_genuine_refresh() {
    let (state, attacker_id, defender_id) = exertion_state("i12-forced-refresh", "fx-1");
    register_no_refresh_after_attack("fx-1");
    let mut monitor = create_invariant_monitor(&state);
    let switch = ActionBody::SwitchPosition {
        instance_id: attacker_id.clone(),
    };
    let _ = monitor.before(&state, P1, &switch);
    let _ = monitor.after(
        &[GameEvent::PositionSwitched {
            instance_id: attacker_id.clone(),
            position: Position::Def,
        }],
        &state,
    );

    let _ = monitor.before(&state, P1, &ActionBody::EndTurn);
    let found = invariant(
        "I12",
        monitor.after(
            &[
                forced_attack(&attacker_id, &defender_id),
                destroyed_by(&defender_id, &attacker_id),
                GameEvent::AttackDeclared {
                    attacker_id,
                    target_id: "hero-p2".to_string(),
                    forced: false,
                    instead_of: None,
                },
            ],
            &state,
        ),
    );
    assert_eq!(found, Vec::<String>::new());
}

// ---------------------------------------------------------------------------------------------
// I9: what an action announces is what it left (#1719)
// ---------------------------------------------------------------------------------------------

/// The I9 findings of a monitor started on `start` and fed one action's `events`, which left `state`.
fn wire_findings(start: &GameState, events: &[GameEvent], state: &GameState) -> Vec<String> {
    create_invariant_monitor(start)
        .after(events, state)
        .into_iter()
        .filter(|finding| finding.starts_with("I9"))
        .collect()
}

/// `events`, each one `tamper` returns a replacement for replaced by it.
fn tampered(events: &[GameEvent], tamper: impl Fn(&GameEvent) -> Option<GameEvent>) -> Vec<GameEvent> {
    events
        .iter()
        .map(|event| tamper(event).unwrap_or_else(|| event.clone()))
        .collect()
}

/// p1 ends their first turn: `turnEnded` (p1, 1), then `turnStarted` (p2, 2) and p2's refresh to 1/1.
fn ended(seed: &str) -> (GameState, ReduceResult) {
    let start = playing(seed);
    let result = reduce(&start, &Action::new(ActionBody::EndTurn, P1, "i9-end"));
    assert!(result.error.is_none(), "{:?}", result.error);
    (start, result)
}

#[test]
fn i9_an_end_turn_s_announcements_are_the_state_it_left() {
    let (start, result) = ended("i9-clean");
    assert_eq!(
        wire_findings(&start, &result.events, &result.state),
        Vec::<String>::new()
    );
}

#[test]
fn i9_a_turn_started_that_names_another_turn_or_player_than_the_state_holds_is_flagged() {
    let (start, result) = ended("i9-turn-started");
    let (turn, active) = (result.state.turn, result.state.active);
    for (player, turn) in [(active, turn + 1), (opponent_of(active), turn)] {
        let events = tampered(&result.events, |event| match event {
            GameEvent::TurnStarted { extra, .. } => Some(GameEvent::TurnStarted {
                player,
                turn,
                extra: *extra,
            }),
            _ => None,
        });
        let found = wire_findings(&start, &events, &result.state);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("I9 wrong turn start"), "{}", found[0]);
    }
}

#[test]
fn i9_a_turn_ended_that_names_another_turn_or_player_than_the_one_under_way_is_flagged() {
    let (start, result) = ended("i9-turn-ended");
    for (player, turn) in [(P1, start.turn + 1), (P2, start.turn)] {
        let events = tampered(&result.events, |event| match event {
            GameEvent::TurnEnded { unspent_mana, .. } => Some(GameEvent::TurnEnded {
                player,
                turn,
                unspent_mana: *unspent_mana,
            }),
            _ => None,
        });
        let found = wire_findings(&start, &events, &result.state);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("I9 wrong turn end"), "{}", found[0]);
    }
}

#[test]
fn i9_a_turn_auto_ended_that_names_another_turn_or_player_than_the_one_under_way_is_flagged() {
    let state = playing("i9-auto-ended");
    let clean = [GameEvent::TurnAutoEnded {
        player: P1,
        turn: state.turn,
    }];
    assert_eq!(wire_findings(&state, &clean, &state), Vec::<String>::new());
    for (player, turn) in [(P1, state.turn + 1), (P2, state.turn)] {
        let found = wire_findings(&state, &[GameEvent::TurnAutoEnded { player, turn }], &state);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("I9 wrong turn end"), "{}", found[0]);
    }
}

#[test]
fn i9_a_seat_s_last_mana_changed_that_is_not_its_mana_is_flagged() {
    let (start, result) = ended("i9-mana");
    let mana = result.state.players[P2].mana;
    for (current, max) in [(mana.current + 1, mana.max), (mana.current, mana.max + 1)] {
        let events = tampered(&result.events, |event| match event {
            GameEvent::ManaChanged { player: P2, .. } => Some(GameEvent::ManaChanged {
                player: P2,
                current,
                max,
            }),
            _ => None,
        });
        let found = wire_findings(&start, &events, &result.state);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("I9 wrong mana"), "{}", found[0]);
    }
}

#[test]
fn i9_a_game_over_that_is_not_the_result_is_flagged() {
    let start = playing("i9-game-over");
    let result = reduce(&start, &Action::new(ActionBody::Concede, P1, "i9-concede"));
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(
        result.state.result,
        Some(GameResult {
            winner: Winner::P2,
            reason: GameOverReason::Concede
        })
    );
    assert_eq!(
        wire_findings(&start, &result.events, &result.state),
        Vec::<String>::new()
    );
    for (winner, reason) in [
        (Winner::P1, GameOverReason::Concede),
        (Winner::P2, GameOverReason::HeroDeath),
    ] {
        let events = tampered(&result.events, |event| match event {
            GameEvent::GameOver { .. } => Some(GameEvent::GameOver { winner, reason }),
            _ => None,
        });
        let found = wire_findings(&start, &events, &result.state);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("I9 wrong game over"), "{}", found[0]);
    }
}

// ---------------------------------------------------------------------------------------------
// I9: each fatigue draw is the seat's next, at its damage (#1902)
// ---------------------------------------------------------------------------------------------

/// p1 ends their first turn into p2's draw from an empty library, p2 having taken `taken` fatigue
/// draws before it: one `fatigue` for p2's draw `taken + 1` (§2.4, R3, R315).
fn fatigued(seed: &str, taken: i32) -> (GameState, ReduceResult) {
    let mut start = playing(seed);
    start.players[P2].library.clear();
    start.players[P2].fatigue_count = taken;
    let result = reduce(&start, &Action::new(ActionBody::EndTurn, P1, "i9-fatigue"));
    assert!(result.error.is_none(), "{:?}", result.error);
    (start, result)
}

fn fatigue(player: PlayerId, count: i32, amount: i32) -> GameEvent {
    GameEvent::Fatigue {
        player,
        count,
        amount,
    }
}

#[test]
fn i9_a_fatigue_draw_s_announcement_is_the_seat_s_next_count_at_its_damage() {
    let (start, result) = fatigued("i9-fatigue", 2);
    let announced: Vec<&GameEvent> = result
        .events
        .iter()
        .filter(|event| matches!(event, GameEvent::Fatigue { .. }))
        .collect();
    assert_eq!(announced, vec![&fatigue(P2, 3, FATIGUE_DAMAGE(3))]);
    assert_eq!(result.state.players[P2].fatigue_count, 3);
    assert_eq!(
        wire_findings(&start, &result.events, &result.state),
        Vec::<String>::new()
    );
}

#[test]
fn i9_a_fatigue_that_repeats_or_skips_a_count_is_flagged() {
    let (start, result) = fatigued("i9-fatigue-count", 2);
    for count in [2, 4] {
        let events = tampered(&result.events, |event| match event {
            GameEvent::Fatigue { player, .. } => Some(fatigue(*player, count, FATIGUE_DAMAGE(count))),
            _ => None,
        });
        let found = wire_findings(&start, &events, &result.state);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("I9 wrong fatigue count"), "{}", found[0]);
    }
}

#[test]
fn i9_a_fatigue_whose_amount_is_not_fatigue_damage_of_its_count_is_flagged() {
    let (start, result) = fatigued("i9-fatigue-damage", 2);
    for amount in [FATIGUE_DAMAGE(3) - 1, FATIGUE_DAMAGE(3) + 1] {
        let events = tampered(&result.events, |event| match event {
            GameEvent::Fatigue { player, count, .. } => Some(fatigue(*player, *count, amount)),
            _ => None,
        });
        let found = wire_findings(&start, &events, &result.state);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("I9 wrong fatigue damage"), "{}", found[0]);
    }
}

#[test]
fn i9_a_fatigue_count_that_moves_without_its_event_or_an_event_without_its_move_is_flagged() {
    let (start, result) = fatigued("i9-fatigue-silent", 2);
    let unannounced: Vec<GameEvent> = result
        .events
        .iter()
        .filter(|event| !matches!(event, GameEvent::Fatigue { .. }))
        .cloned()
        .collect();
    let mut unmoved = result.state.clone();
    unmoved.players[P2].fatigue_count -= 1;
    let mut other = result.state.clone();
    other.players[P1].fatigue_count += 1;
    for (events, state, seat) in [
        (&unannounced, &result.state, P2),
        (&result.events, &unmoved, P2),
        (&result.events, &other, P1),
    ] {
        let found = wire_findings(&start, events, state);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].starts_with(&format!("I9 silent fatigue: {seat}")),
            "{}",
            found[0]
        );
    }
}

#[test]
fn i9_a_hard_seat_s_two_fatigue_draws_in_one_turn_are_its_next_two() {
    // R183: a Hard seat's start-of-turn draws are two fatigue steps from an empty library, N and then
    // N + 1, inside p1's one endTurn (handicap.rs's fatigue case, through the reducer and the monitor).
    setup_catalog();
    let mut start = past_mulligans(&create_game(&CreateGameOptions {
        seed: "i9-fatigue-hard".to_string(),
        decks: (
            vanilla_deck(DECK_SIZE, 1),
            vanilla_deck(AI_DIFFICULTY.hard.deck_size, 1),
        ),
        handicaps: Some(PerPlayerOpt {
            p1: None,
            p2: Some(AI_DIFFICULTY.hard),
        }),
        ..CreateGameOptions::default()
    }));
    assert_eq!(start.active, P1);
    start.players[P2].library.clear();
    start.players[P2].fatigue_count = 2;
    let result = reduce(&start, &Action::new(ActionBody::EndTurn, P1, "i9-fatigue-hard"));
    assert!(result.error.is_none(), "{:?}", result.error);
    let announced: Vec<&GameEvent> = result
        .events
        .iter()
        .filter(|event| matches!(event, GameEvent::Fatigue { .. }))
        .collect();
    assert_eq!(
        announced,
        vec![
            &fatigue(P2, 3, FATIGUE_DAMAGE(3)),
            &fatigue(P2, 4, FATIGUE_DAMAGE(4))
        ]
    );
    assert_eq!(result.state.players[P2].fatigue_count, 4);
    assert_eq!(
        wire_findings(&start, &result.events, &result.state),
        Vec::<String>::new()
    );
}

#[test]
fn i9_each_seat_counts_its_own_fatigue_draws_on_from_the_last_within_one_action() {
    // Two seats' fatigue draws in one action, interleaved, are each counted on from that seat's last.
    let start = playing("i9-fatigue-two");
    let mut state = start.clone();
    state.players[P1].fatigue_count = 2;
    state.players[P2].fatigue_count = 1;
    let events = [
        fatigue(P1, 1, FATIGUE_DAMAGE(1)),
        fatigue(P2, 1, FATIGUE_DAMAGE(1)),
        fatigue(P1, 2, FATIGUE_DAMAGE(2)),
    ];
    assert_eq!(wire_findings(&start, &events, &state), Vec::<String>::new());
}

#[test]
fn i9_a_glitch_reset_starts_the_fatigue_count_again() {
    let i9 = |found: Vec<String>| -> Vec<String> {
        found
            .into_iter()
            .filter(|finding| finding.starts_with("I9"))
            .collect()
    };
    let mut late = playing("i9-fatigue-reset");
    late.players[P1].fatigue_count = 3;
    let mut monitor = create_invariant_monitor(&late);
    // R676: the reset deals a new game, whose counts start at 0.
    let mut reset = playing("i9-fatigue-reset");
    reset.resets = Some(late.resets.unwrap_or(0) + 1);
    assert_eq!(reset.players[P1].fatigue_count, 0);
    assert_eq!(i9(monitor.after(&[], &reset)), Vec::<String>::new());
    // The new game's first fatigue draw is draw 1: the shadow started again.
    let mut next = reset.clone();
    next.players[P1].fatigue_count = 1;
    assert_eq!(
        i9(monitor.after(&[fatigue(P1, 1, FATIGUE_DAMAGE(1))], &next)),
        Vec::<String>::new()
    );
    // The control: with `resets` unmoved, the same drop to 0 is a silent move.
    let found = i9(create_invariant_monitor(&late).after(&[], &playing("i9-fatigue-reset")));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("I9 silent fatigue"), "{}", found[0]);
}
