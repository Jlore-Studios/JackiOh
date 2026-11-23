//! The fuzz monitor's I7 (`testkit/invariants.rs`): a live game with no prompt open owes no paused
//! sequence, so `work` is empty and `work_cursor` is 0 (§9.3, §10.3, R113). A prompt, the open
//! mulligans (§10.6, R265) or a game over each excuse what is still owed.
//!
//! And its I9 (#1719): each `turnStarted`, `turnEnded`, `turnAutoEnded`, `gameOver` and `manaChanged`
//! an action announces says what the state it left holds. Each `fatigue` is its seat's next fatigue draw
//! at `FATIGUE_DAMAGE` of its count, and each seat's `fatigue_count` moves only with one (§2.4, R3, R315;
//! #1902). Each check is proved by tampering with one payload of a real action's events; the untampered
//! events are clean. And each 1v1 hero's health moves only with a `damage`, `healthLost`, `healed`,
//! `healthSet` or health `swapped` (§10.3, R2201000; #4025), proved on synthetic events and an edited state.
//!
//! And its I17 (#3773): what the state check leaves on a settled state (no unit at 0 health but R69's or
//! R3539003's, no unit in Defense Position that R2179000, R2657000, R3543002 or R1849006 switches out of
//! it, no hero at 0 that can lose, R2201000) and, on every state, no mana or card damage below 0
//! and no `damage` event of 0 or less (§2.3, §4.5, R63).
//!
//! And its I18 (#4111): a plain attack does what the combat forecast its seat was shown said
//! (R3100000, R4111000). Each check is proved by tampering with one payload of a real attack's events;
//! the untampered events are clean, and a step or a board the forecast cannot foretell is not judged.
//!
//! And its I20 (#4478): a field card's stored damage moves only with its `damage` and `healed` events
//! (§4.4, §10.3, R78), proved on synthetic events, edited states, real effects and a planted bump.
//!
//! And its I22 (#4540): every `target` pick is one its source may aim at, no Dormant card (R1839000),
//! no Evasive card in play from a Spell or Field Spell (R4353002), no card Immune to Spells from a Spell
//! (R450). Proved on hand-made random draws (R452's cast, R1200's Mayor), on the record a real random
//! cast and a Mayor's play leave between `before` and `after`, on a listed play or activation retargeted
//! and on an open prompt given one more option; a random pick an effect makes is no targeting.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::SwapWhat;

use crate::rules::fixtures::aura_deaths::{aura_board, register_sweeper, stack_on};
use crate::rules::fixtures::catalog::{spell_def, vanilla_deck};
use crate::rules::fixtures::combat::{big_body, cleaver, plain};
use crate::rules::fixtures::combat_reactions::{POOL, register_e6_fixtures};
use crate::rules::fixtures::harness::{act_ok, both, new_game, put, setup_catalog, slot};
use crate::rules::fixtures::stinkyscape_verbs::{WARP, register_stinkyscape_verbs};

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

// I11: Unit tokens never persist off the field (§3, §3.2, R11)

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

#[test]
fn i6_a_hook_marker_naming_a_readable_card_s_former_definition_hides_nothing() {
    let mut state = playing("i6-hook-marker");
    // A definition no card p2 reads carries: its one card lies in p1's library.
    let secret = state.players[P1]
        .library
        .iter()
        .map(|card| card.def_id.clone())
        .find(|def| {
            !state.players[P2]
                .hand
                .iter()
                .chain(&state.players[P2].library)
                .any(|card| &card.def_id == def)
        })
        .expect("a p1 library card p2 holds no copy of");
    // A face-up Unit both seats read, whose hook began while it was `secret` (a Fuse or a Transform has
    // since given it another definition): its marker is the only event in the window naming it (R2783000).
    let public = put(&mut state, "fx-30", slot(P1, Row::Units, 1), json!({}));
    assert_ne!(public.def_id, secret);
    state.applied.push(AppliedAction {
        nonce: "i6-hook-marker".to_string(),
        events: vec![GameEvent::HookResolving {
            source_id: public.id.clone(),
            def_id: secret.clone(),
            controller: P1,
            hook: HookKind::Cry,
            hidden_from: None,
        }],
    });
    let view = view_for(&state, P2);
    assert!(
        serde_json::to_string(&view.events)
            .expect("events serialize")
            .contains(&secret),
        "the marker reaches p2 naming {secret}"
    );
    let legal = legal_actions(&state, P2);
    assert_eq!(
        invariant("I6", hidden_information_violations(&state, P2, &view, &legal)),
        Vec::<String>::new()
    );
}

// I12: a switch and an attack share exertion unless the Unit has Deft (§4.1, R49, R636)

fn exertion_state(seed: &str, attacker_def: &str) -> (GameState, String, String) {
    let mut state = playing(seed);
    let attacker = put(&mut state, attacker_def, slot(P1, Row::Units, 1), json!({}));
    let defender = put(&mut state, "fx-21", slot(P2, Row::Units, 1), json!({}));
    (state, attacker.id, defender.id)
}

/// Give `def_id` an `afterAttack` hook that does not refresh exertion. I2/I12 must treat its kill
/// credit as speculative until the settled state proves a refresh happened.
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

/// Give `def_id` the `afterAttack` refresh whose settled exertion I12 is allowed to observe.
fn register_refresh_after_attack(def_id: &str) {
    let mut scripts = registered_scripts().clone();
    scripts.insert(
        def_id.to_string(),
        both(Script {
            after_attack: Some(hook(|_ctx| {
                vec![effects::may_attack_again(json_as(
                    json!({ "target": { "of": "self" } }),
                ))]
            })),
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
        animated: None,
        died_as: None,
        via: None,
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
fn i12_a_declaration_after_a_noop_kill_hook_does_not_erase_a_spent_switch() {
    let (mut state, attacker_id, defender_id) = exertion_state("i12-noop-refresh", "fx-1");
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
    let exertion = &mut find_instance_mut(&mut state, &attacker_id)
        .expect("the attacker")
        .exertion;
    exertion.attacked = true;
    exertion.switched = true;

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
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("I12 attack after switch"), "{}", found[0]);
}

#[test]
fn i12_a_confirmed_refresh_used_by_an_r1202_substitute_is_clean() {
    let (mut state, declarer_id, defender_id) = exertion_state("i12-r1202-refresh", "fx-1");
    let substitute = put(&mut state, "fx-2", slot(P1, Row::Units, 2), json!({}));
    register_refresh_after_attack("fx-1");
    let mut monitor = create_invariant_monitor(&state);
    let switch = ActionBody::SwitchPosition {
        instance_id: declarer_id.clone(),
    };
    let _ = monitor.before(&state, P1, &switch);
    let _ = monitor.after(
        &[GameEvent::PositionSwitched {
            instance_id: declarer_id.clone(),
            position: Position::Def,
        }],
        &state,
    );

    // The forced kill's `mayAttackAgain` reset both halves before the later declared attack spent
    // only the attack half. R1202 emits the physical substitute as `attacker_id` and the Unit whose
    // exertion was spent as `instead_of`.
    let exertion = &mut find_instance_mut(&mut state, &declarer_id)
        .expect("the declarer")
        .exertion;
    exertion.attacked = true;
    exertion.switched = false;
    let _ = monitor.before(&state, P1, &ActionBody::EndTurn);
    let found = invariant(
        "I12",
        monitor.after(
            &[
                forced_attack(&declarer_id, &defender_id),
                destroyed_by(&defender_id, &declarer_id),
                GameEvent::AttackDeclared {
                    attacker_id: substitute.id,
                    target_id: "hero-p2".to_string(),
                    forced: false,
                    instead_of: Some(declarer_id),
                },
            ],
            &state,
        ),
    );
    assert_eq!(found, Vec::<String>::new());
}

// I9: what an action announces is what it left (#1719)

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

// I9: each fatigue draw is the seat's next, at its damage (#1902)

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

fn hero_health_findings(start: &GameState, events: &[GameEvent], state: &GameState) -> Vec<String> {
    create_invariant_monitor(start)
        .after(events, state)
        .into_iter()
        .filter(|finding| finding.starts_with("I9 silent hero health"))
        .collect()
}

fn hero_damage(seat: PlayerId, amount: i32) -> GameEvent {
    GameEvent::Damage {
        source_id: None,
        target_id: format!("hero-{seat}"),
        amount,
        combat: false,
        absorbed: 0,
        via: None,
    }
}

fn hero_healed(target_id: &str, amount: i32) -> GameEvent {
    GameEvent::Healed {
        target_id: target_id.to_string(),
        amount,
        via: None,
    }
}

/// A started game with p1 at 30 and p2 at 20.
fn health_start(seed: &str) -> GameState {
    let mut start = playing(seed);
    start.players[P1].hero.health = 30;
    start.players[P2].hero.health = 20;
    start
}

#[test]
fn i9_a_hero_s_damage_heal_set_and_health_swap_with_their_events_are_clean() {
    let start = health_start("i9-health-clean");
    let none = Vec::<String>::new();

    let mut hit = start.clone();
    hit.players[P1].hero.health = 25;
    assert_eq!(hero_health_findings(&start, &[hero_damage(P1, 5)], &hit), none);

    let mut healed = hit.clone();
    healed.players[P1].hero.health = 28;
    assert_eq!(
        hero_health_findings(&hit, &[hero_healed("hero-p1", 3)], &healed),
        none
    );

    let mut set = start.clone();
    set.players[P2].hero.health = 7;
    let set_event = GameEvent::HealthSet {
        player: P2,
        health: 7,
        source_id: None,
    };
    assert_eq!(hero_health_findings(&start, &[set_event], &set), none);

    let mut lost = start.clone();
    lost.players[P2].hero.health = 17;
    let lost_event = GameEvent::HealthLost {
        player: P2,
        amount: 3,
    };
    assert_eq!(hero_health_findings(&start, &[lost_event], &lost), none);

    let mut swapped = start.clone();
    swapped.players[P1].hero.health = 20;
    swapped.players[P2].hero.health = 30;
    let swap_event = GameEvent::Swapped {
        what: SwapWhat::Health,
    };
    assert_eq!(hero_health_findings(&start, &[swap_event], &swapped), none);
}

#[test]
fn i9_a_hero_s_health_that_moves_without_its_event_or_an_event_without_its_move_is_flagged() {
    let start = health_start("i9-health-silent");
    let only_one = |found: Vec<String>, prefix: &str| {
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with(prefix), "{}", found[0]);
    };

    let mut drop = start.clone();
    drop.players[P1].hero.health -= 1;
    only_one(
        hero_health_findings(&start, &[], &drop),
        "I9 silent hero health: p1",
    );

    let mut rise = start.clone();
    rise.players[P2].hero.health += 1;
    only_one(
        hero_health_findings(&start, &[], &rise),
        "I9 silent hero health: p2",
    );

    only_one(
        hero_health_findings(&start, &[hero_damage(P1, 5)], &start),
        "I9 silent hero health: p1",
    );

    let mut wrong = start.clone();
    wrong.players[P1].hero.health -= 4;
    only_one(
        hero_health_findings(&start, &[hero_damage(P1, 5)], &wrong),
        "I9 silent hero health: p1",
    );

    // A unit's heal names a unit, not a hero: it moves no tally.
    assert_eq!(
        hero_health_findings(&start, &[hero_healed("fx-1", 4)], &start),
        Vec::<String>::new()
    );

    // One silent move is one finding: the tally then follows the state.
    let mut monitor = create_invariant_monitor(&start);
    let count = |found: Vec<String>| {
        found
            .iter()
            .filter(|finding| finding.starts_with("I9 silent hero health"))
            .count()
    };
    assert_eq!(count(monitor.after(&[], &drop)), 1);
    assert_eq!(count(monitor.after(&[], &drop)), 0);
}

#[test]
fn i9_a_floored_hero_s_hit_past_its_floor_reads_the_floor() {
    let mut start = health_start("i9-health-floor");
    start.players[P1].handicap = Some(TUTORIAL_HUMAN);
    let floor = TUTORIAL_HERO_FLOOR;
    let mut at_floor = start.clone();
    at_floor.players[P1].hero.health = floor;
    assert_eq!(
        hero_health_findings(&start, &[hero_damage(P1, 40)], &at_floor),
        Vec::<String>::new()
    );
    let mut below = start.clone();
    below.players[P1].hero.health = floor - 1;
    assert_eq!(
        hero_health_findings(&start, &[hero_damage(P1, 40)], &below).len(),
        1
    );
}

#[test]
fn i9_a_glitch_reset_starts_the_hero_health_tally_again() {
    let mut late = playing("i9-health-reset");
    late.players[P1].hero.health = 12;
    let mut monitor = create_invariant_monitor(&late);
    // R676: the reset deals a new game, whose heroes start whole.
    let mut reset = playing("i9-health-reset");
    reset.players[P1].hero.health = 30;
    reset.resets = Some(late.resets.unwrap_or(0) + 1);
    let health = |found: Vec<String>| -> Vec<String> {
        found
            .into_iter()
            .filter(|finding| finding.starts_with("I9 silent hero health"))
            .collect()
    };
    assert_eq!(health(monitor.after(&[], &reset)), Vec::<String>::new());
    let mut next = reset.clone();
    next.players[P1].hero.health = 25;
    assert_eq!(
        health(monitor.after(&[hero_damage(P1, 5)], &next)),
        Vec::<String>::new()
    );
    // The control: with `resets` unmoved, the same jump to 30 is a silent move.
    let mut jumped = late.clone();
    jumped.players[P1].hero.health = 30;
    let found = health(create_invariant_monitor(&late).after(&[], &jumped));
    assert_eq!(found.len(), 1, "{found:?}");
}

// I16: a card's own record agrees with the pile that holds it (§3.2, R78, R766)
fn i16(state: &GameState) -> Vec<String> {
    invariant("I16", create_invariant_monitor(state).after(&[], state))
}

/// The one finding, naming `id` and citing each of `cites`.
fn only(found: &[String], id: &str, cites: &[&str]) {
    assert_eq!(found.len(), 1, "{found:?}");
    for want in std::iter::once(id).chain(cites.iter().copied()) {
        assert!(found[0].contains(want), "{}", found[0]);
    }
}

#[test]
fn i16_a_game_with_cards_in_every_pile_and_on_both_boards_is_clean() {
    let mut state = playing("i16-clean");
    put(&mut state, "fx-1", slot(P1, Row::Units, 1), json!({}));
    put(&mut state, "fx-21", slot(P2, Row::Units, 2), json!({}));
    let dead = new_instance(&mut state, "fx-2", P1, Zone::Graveyard { player: P1 });
    state.players[P1].graveyard.push(dead);
    let gone = new_instance(&mut state, "fx-22", P2, Zone::Exile { player: P2 });
    state.players[P2].exile.push(gone);
    assert_eq!(i16(&state), Vec::<String>::new());
}

#[test]
fn i16_a_hand_card_whose_zone_says_graveyard_is_one_zone_drift() {
    let mut state = playing("i16-zone");
    state.players[P1].hand[0].zone = Zone::Graveyard { player: P1 };
    let id = state.players[P1].hand[0].id.clone();
    only(&i16(&state), &id, &["zone drift", "p1's hand", "§3.2"]);
}

#[test]
fn i16_a_field_card_controlled_from_the_other_seat_is_one_controller_drift() {
    let mut state = playing("i16-controller");
    let unit = put(&mut state, "fx-1", slot(P1, Row::Units, 1), json!({}));
    find_instance_mut(&mut state, &unit.id)
        .expect("the unit")
        .controller = P2;
    only(&i16(&state), &unit.id, &["controller drift", "§3.2"]);
}

#[test]
fn i16_a_graveyard_card_with_two_damage_is_one_field_state_leftover() {
    let mut state = playing("i16-damage");
    let mut dead = new_instance(&mut state, "fx-2", P1, Zone::Graveyard { player: P1 });
    dead.damage = 2;
    let id = dead.id.clone();
    state.players[P1].graveyard.push(dead);
    only(&i16(&state), &id, &["R78", "damage", "p1's graveyard"]);
}

#[test]
fn i16_a_graveyard_card_with_a_cost_mod_of_minus_one_is_one_kept_price() {
    let mut state = playing("i16-price");
    let mut dead = new_instance(&mut state, "fx-2", P1, Zone::Graveyard { player: P1 });
    dead.cost_mod = -1;
    let id = dead.id.clone();
    state.players[P1].graveyard.push(dead);
    only(&i16(&state), &id, &["R766", "costMod"]);
}

#[test]
fn i16_a_hand_card_holding_buffs_and_a_cost_mod_is_clean_but_not_one_holding_position() {
    let mut state = playing("i16-hand");
    // A Buff lands on a hand card and rides it onto the field (B5 E38); a price change stays there (R386).
    state.players[P1].hand[0].buffs = AttackHealth { attack: 2, health: 1 };
    state.players[P1].hand[0].cost_mod = -1;
    assert_eq!(i16(&state), Vec::<String>::new());
    state.players[P1].hand[1].position = Some(Position::Atk);
    let id = state.players[P1].hand[1].id.clone();
    only(&i16(&state), &id, &["R78", "position"]);
}

#[test]
fn i16_a_hand_or_library_copy_may_carry_stats_and_armor_overrides_but_a_graveyard_card_may_not() {
    let mut state = playing("i16-overrides");
    // A copy carries its source's overrides into a hand or a library (R57, R1242, R2198001).
    let overrides = |card: &mut CardInstance| {
        card.stats_override = Some(AttackHealth { attack: 7, health: 7 });
        card.armor_override = Some(3);
    };
    overrides(&mut state.players[P1].hand[0]);
    overrides(&mut state.players[P2].library[0]);
    assert_eq!(i16(&state), Vec::<String>::new());
    let mut dead = new_instance(&mut state, "fx-2", P1, Zone::Graveyard { player: P1 });
    dead.stats_override = Some(AttackHealth { attack: 7, health: 7 });
    let id = dead.id.clone();
    state.players[P1].graveyard.push(dead);
    only(&i16(&state), &id, &["R78", "statsOverride"]);
}

/// R3566004: a card that evolved in its graveyard keeps its Evolve's buff, keywords and stat line there,
/// for its Flashback play; damage it could only have brought from the field is still a leftover (R78).
#[test]
fn r3566004_i16_a_card_evolved_in_its_graveyard_keeps_what_its_evolve_gave() {
    let mut state = playing("i16-evolved");
    let mut cheese = new_instance(&mut state, "fx-2", P1, Zone::Graveyard { player: P1 });
    cheese.evolved = Some(true);
    cheese.buffs = AttackHealth { attack: 5, health: 5 };
    cheese.granted_keywords = vec![json_as(json!({ "kind": "Flashback" }))];
    cheese.stats_override = Some(AttackHealth { attack: 0, health: 0 });
    let id = cheese.id.clone();
    state.players[P1].graveyard.push(cheese);
    assert_eq!(i16(&state), Vec::<String>::new());
    let held = state.players[P1].graveyard.last_mut().expect("the card");
    held.damage = 2;
    only(&i16(&state), &id, &["R78", "damage"]);
}

/// R3920000: a card that evolved in a graveyard or an exile pile and goes on to a hand or a library leaves
/// its Evolve behind, so what arrives is the printed card and I16 finds nothing (handicapped fuzz seed 2788:
/// S+ #6.2 Stinky Cheese, evolved in the graveyard, back in the hand by TG #55 Reminisce).
#[test]
fn r3920000_a_card_evolved_in_a_pile_reaches_a_hand_or_a_library_as_its_printed_card() {
    for from in [ZoneName::Graveyard, ZoneName::Exile] {
        for to in [OffFieldZone::Hand, OffFieldZone::Library] {
            let mut state = playing("r3920000");
            let pile = if from == ZoneName::Graveyard {
                Zone::Graveyard { player: P1 }
            } else {
                Zone::Exile { player: P1 }
            };
            let mut cheese = new_instance(&mut state, "fx-2", P1, pile);
            cheese.evolved = Some(true);
            cheese.buffs = AttackHealth { attack: 5, health: 5 };
            cheese.granted_keywords = vec![json_as(json!({ "kind": "Flashback" }))];
            cheese.stats_override = Some(AttackHealth { attack: 0, health: 0 });
            if from == ZoneName::Graveyard {
                state.players[P1].graveyard.push(cheese.clone());
            } else {
                state.players[P1].exile.push(cheese.clone());
            }
            move_to_zone(&mut state, &mut cheese, to, MoveToZoneOptions::default());
            let arrived = find_instance(&state, &cheese.id).expect("the card");
            let at = format!("{from:?} to {to:?}");
            assert_eq!(arrived.zone.z(), to.zone_name(), "{at}");
            assert_eq!(arrived.evolved, None, "{at}");
            assert_eq!(arrived.buffs, AttackHealth { attack: 0, health: 0 }, "{at}");
            assert!(arrived.granted_keywords.is_empty(), "{at}");
            assert_eq!(arrived.stats_override, None, "{at}");
            assert_eq!(i16(&state), Vec::<String>::new(), "{at}");
        }
    }
}

/// One field of R78's reset, given to a card.
type Give = fn(&mut CardInstance);

/// R78's reset clears these too (R3546000, R3554003, R3554007, R3536000), so a hand card holds none of them.
#[test]
fn i16_a_hand_card_holding_giga_a_half_a_damage_chain_or_evolved_is_one_leftover_each() {
    let cases: [(&str, Give); 5] = [
        ("giga", |card| card.giga = Some(true)),
        ("half", |card| card.half = Some(Half::Left)),
        ("sharedChain", |card| card.shared_chain = Some(1)),
        ("splitChain", |card| card.split_chain = Some(1)),
        ("evolved", |card| card.evolved = Some(true)),
    ];
    for (name, give) in cases {
        let mut state = playing("i16-missed");
        give(&mut state.players[P1].hand[0]);
        let id = state.players[P1].hand[0].id.clone();
        let found = i16(&state);
        only(&found, &id, &["R78", "p1's hand"]);
        assert!(
            found[0].contains(&format!("still holds {name} on turn")),
            "{}",
            found[0]
        );
    }
}

/// R3548001: a card drawn while Tipsy is up is stamped before it reaches the hand and may carry the stamp
/// into a library, where nothing reads it; R78's reset takes it as the card leaves the field or reaches a
/// graveyard or an exile pile.
#[test]
fn i16_a_hand_or_library_card_may_hold_a_tipsy_stamp_but_a_graveyard_or_exile_card_may_not() {
    let stamp = TipsyStamp {
        player: P1,
        level: 2,
        turn: 1,
    };
    let mut state = playing("i16-tipsy");
    state.players[P1].hand[0].tipsy = Some(stamp);
    state.players[P2].library[0].tipsy = Some(stamp);
    assert_eq!(i16(&state), Vec::<String>::new());
    for zone in [Zone::Graveyard { player: P1 }, Zone::Exile { player: P1 }] {
        let mut state = playing("i16-tipsy-gone");
        let mut gone = new_instance(&mut state, "fx-2", P1, zone.clone());
        gone.tipsy = Some(stamp);
        let id = gone.id.clone();
        match zone {
            Zone::Graveyard { .. } => state.players[P1].graveyard.push(gone),
            _ => state.players[P1].exile.push(gone),
        }
        let found = i16(&state);
        only(&found, &id, &["R78"]);
        assert!(found[0].contains("still holds tipsy on turn"), "{}", found[0]);
    }
}

/// What R78's reset changes and a card off the field may keep in every pile: `memory` (a hand's R1841003
/// `tributeLowered`, a graveyard's R429 `@returnPrice`) and a spent Brittle count, which the reset alone drops.
#[test]
fn i16_a_card_off_the_field_may_keep_its_memory_and_a_spent_brittle_count() {
    let mut state = playing("i16-kept");
    let mut dead = new_instance(&mut state, "fx-2", P1, Zone::Graveyard { player: P1 });
    dead.memory.insert("@returnPrice".to_string(), json!(2));
    dead.brittle = Some(BrittleCounter {
        count: 0,
        since: 1,
        printed: None,
    });
    state.players[P1].graveyard.push(dead);
    let hand = &mut state.players[P2].hand[0];
    hand.memory.insert("tributeLowered".to_string(), json!(1));
    hand.brittle = Some(BrittleCounter {
        count: 0,
        since: 1,
        printed: None,
    });
    assert_eq!(i16(&state), Vec::<String>::new());
}

#[test]
fn i16_a_library_card_owned_by_the_other_seat_is_one_owner_drift() {
    let mut state = playing("i16-owner");
    state.players[P1].library[0].owner = P2;
    let id = state.players[P1].library[0].id.clone();
    only(&i16(&state), &id, &["owner drift", "R747"]);
}

// I17: the state check ran and the numbers are in range (§2.3, §4.4, §4.5, R63, R69, R2201000)
fn i17(state: &GameState, events: &[GameEvent]) -> Vec<String> {
    invariant("I17", create_invariant_monitor(state).after(events, state))
}

/// p1's first main phase with a 2/2 `fx-1` in p1's unit zone 1 holding `taken` damage, and its id.
fn hurt(seed: &str, taken: i32) -> (GameState, String) {
    let mut state = playing(seed);
    let unit = put(&mut state, "fx-1", slot(P1, Row::Units, 1), json!({}));
    find_instance_mut(&mut state, &unit.id).expect("the unit").damage = taken;
    (state, unit.id)
}

/// p1's mulligan prompt left open in `pending` (§10.6), as I7's test opens one.
fn open_prompt(state: &mut GameState, seed: &str) {
    let begun = begin_game(&new_game(seed, None)).state;
    state.pending = Some(
        begun.mulligan.as_ref().expect("the mulligans are open")[P1]
            .prompt
            .clone(),
    );
}

fn damage_event(target_id: &str, amount: i32) -> GameEvent {
    json_as(json!({
        "type": "damage",
        "sourceId": null,
        "targetId": target_id,
        "amount": amount,
        "combat": false,
    }))
}

#[test]
fn i17_a_settled_board_with_a_hurt_unit_no_mana_left_and_a_hit_of_1_is_clean() {
    let (mut state, _) = hurt("i17-clean", 1);
    state.players[P1].mana.current = 0;
    assert_eq!(i17(&state, &[damage_event("hero-p2", 1)]), Vec::<String>::new());
    // A real action's state and events.
    let (_, result) = ended("i17-clean");
    assert_eq!(i17(&result.state, &result.events), Vec::<String>::new());
}

#[test]
fn i17_a_unit_at_0_health_or_less_on_a_settled_board_is_one_dead_unit_left_standing() {
    for taken in [2, 3] {
        let (state, id) = hurt("i17-dead", taken);
        only(&i17(&state, &[]), &id, &["dead unit left standing", "§4.5"]);
    }
}

#[test]
fn i17_an_indestructible_unit_at_0_health_stands_while_its_max_health_is_above_0() {
    let (mut state, id) = hurt("i17-indestructible", 2);
    find_instance_mut(&mut state, &id)
        .expect("the unit")
        .granted_keywords = vec![json_as::<Keyword>(json!({ "kind": "Indestructible" }))];
    assert_eq!(i17(&state, &[]), Vec::<String>::new());
    // R69: at max health 0 even an Indestructible unit dies.
    find_instance_mut(&mut state, &id).expect("the unit").buffs = AttackHealth {
        attack: 0,
        health: -2,
    };
    only(&i17(&state, &[]), &id, &["dead unit left standing", "R69"]);
}

/// R3539003: no health judges a Dormant card, so a 0/0 hull stands on a settled board.
#[test]
fn i17_a_dormant_unit_at_0_health_stands() {
    let (mut state, id) = hurt("i17-dormant", 2);
    find_instance_mut(&mut state, &id).expect("the unit").slumber = Some(Slumber { turns: None });
    assert_eq!(i17(&state, &[]), Vec::<String>::new());
    find_instance_mut(&mut state, &id).expect("the unit").slumber = None;
    only(&i17(&state, &[]), &id, &["dead unit left standing"]);
}

#[test]
fn i17_a_dead_unit_waits_for_an_open_prompt_the_mulligans_and_a_game_over() {
    // A prompt in `pending` (§10.6): the state check runs once it is answered.
    let (mut prompted, _) = hurt("i17-wait", 2);
    open_prompt(&mut prompted, "i17-wait");
    assert_eq!(i17(&prompted, &[]), Vec::<String>::new());

    // The mulligans are open (§10.6, R265).
    let mut mulligan = begin_game(&new_game("i17-wait", None)).state;
    assert!(mulligan.mulligan.is_some());
    let unit = put(&mut mulligan, "fx-1", slot(P1, Row::Units, 1), json!({}));
    find_instance_mut(&mut mulligan, &unit.id)
        .expect("the unit")
        .damage = 2;
    assert_eq!(i17(&mulligan, &[]), Vec::<String>::new());

    // A game over (§2.5): nothing is checked after it.
    let (mut over, _) = hurt("i17-wait", 2);
    over.result = Some(GameResult {
        winner: Winner::P1,
        reason: GameOverReason::Concede,
    });
    assert_eq!(i17(&over, &[]), Vec::<String>::new());
}

#[test]
fn i17_a_hero_at_0_that_can_lose_is_one_live_hero_and_a_floored_one_is_not() {
    let mut state = playing("i17-hero");
    state.players[P2].hero.health = 0;
    only(&i17(&state, &[]), "p2's hero", &["live hero at 0", "R2201000"]);

    // R2201000: a seat held at a floor cannot lose.
    state.players[P2].handicap = Some(TUTORIAL_HUMAN);
    assert!(!state::can_lose(&state, P2));
    for health in [TUTORIAL_HERO_FLOOR, 0] {
        state.players[P2].hero.health = health;
        assert_eq!(i17(&state, &[]), Vec::<String>::new());
    }

    // A prompt in `pending`: the hero check waits for it, as the dead-unit check does.
    let mut prompted = playing("i17-hero");
    prompted.players[P1].hero.health = 0;
    open_prompt(&mut prompted, "i17-hero");
    assert_eq!(i17(&prompted, &[]), Vec::<String>::new());
}

#[test]
fn i17_mana_below_0_and_a_card_with_negative_damage_are_flagged_on_every_state() {
    let mut state = playing("i17-mana");
    let mana = state.players[P1].mana;
    state.players[P1].mana.current = -1;
    only(&i17(&state, &[]), "p1", &["negative mana", "§2.3"]);
    state.players[P1].mana = mana;
    state.players[P1].mana.max = -1;
    only(&i17(&state, &[]), "p1", &["negative mana", "§2.3"]);
    state.players[P1].mana = mana;
    // Behind an open prompt too: no step may leave mana below 0.
    state.players[P2].mana.current = -1;
    open_prompt(&mut state, "i17-mana");
    only(&i17(&state, &[]), "p2", &["negative mana"]);

    // A 2/2 holding damage -1 is at 3 health: only the damage is wrong.
    let (state, id) = hurt("i17-damage", -1);
    only(&i17(&state, &[]), &id, &["negative damage", "§4.4"]);
}

#[test]
fn i17_a_damage_event_of_0_or_less_is_one_empty_damage() {
    let state = playing("i17-event");
    for amount in [0, -1] {
        only(
            &i17(&state, &[damage_event("hero-p2", amount)]),
            "hero-p2",
            &["empty damage", "R63"],
        );
    }
}

/// `hurt`'s unit, unhurt, standing in Defense Position, granted `keyword` when one is named.
fn defending(seed: &str, keyword: Option<&str>) -> (GameState, String) {
    let (mut state, id) = hurt(seed, 0);
    let unit = find_instance_mut(&mut state, &id).expect("the unit");
    unit.position = Some(Position::Def);
    if let Some(kind) = keyword {
        unit.granted_keywords = vec![json_as::<Keyword>(json!({ "kind": kind }))];
    }
    (state, id)
}

#[test]
fn i17_an_indestructible_can_t_defend_or_evasive_unit_in_defense_on_a_settled_board_is_one_forbidden_defense()
{
    for (kind, rule) in [
        ("Indestructible", "R2179000"),
        ("Can't Defend", "R2657000"),
        ("Evasive", "R3543002"),
    ] {
        let (state, id) = defending("i17-defense", Some(kind));
        only(&i17(&state, &[]), &id, &["forbidden Defense", rule]);
    }
}

#[test]
fn i17_a_unit_in_defense_while_an_aura_bars_it_is_one_forbidden_defense() {
    let (mut state, id) = defending("i17-defense-warp", None);
    // After `playing`: registering re-runs `setup_catalog`, and adds the Warp fixture to it.
    register_stinkyscape_verbs();
    put(&mut state, WARP, slot(P1, Row::Backrow, 1), json!({}));
    only(&i17(&state, &[]), &id, &["forbidden Defense", "R1849006"]);
}

#[test]
fn i17_a_unit_in_defense_that_may_stand_there_or_waits_for_an_open_prompt_is_clean() {
    let (state, _) = defending("i17-defense-clean", None);
    assert_eq!(i17(&state, &[]), Vec::<String>::new());
    let (mut prompted, _) = defending("i17-defense-prompt", Some("Indestructible"));
    open_prompt(&mut prompted, "i17-defense-prompt");
    assert_eq!(i17(&prompted, &[]), Vec::<String>::new());
}

// I18: a plain attack does what its seat's forecast said (R3100000, R4111000; #4111)

/// p1's main phase on turn 4, a 3/3 in p1's lane 1 facing a 5/10 in p2's: the forecast is dealt 3,
/// taken 5, no kill and the attacker falls. R345: p1's turn stays open after the attack, or R82 would
/// end it inside the step.
fn i18_board(seed: &str) -> (GameState, String, String) {
    let mut state = new_game(seed, None);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state.players[P1].auto_end_turn = Some(false);
    let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
    let defender = put(&mut state, &big_body.id, slot(P2, Row::Units, 1), json!({}));
    (state, attacker.id, defender.id)
}

/// A monitor that saw p1 choose `attacker` on `target` on `state`, and the reducer's attack.
fn i18_attack(state: &GameState, attacker: &str, target: &str) -> (InvariantMonitor, ReduceResult) {
    let mut monitor = create_invariant_monitor(state);
    let action = ActionBody::Attack {
        attacker_id: attacker.to_string(),
        target_id: target.to_string(),
    };
    monitor.before(state, P1, &action);
    let result = act_ok(
        state,
        json!({ "type": "attack", "playerId": "p1", "attackerId": attacker, "targetId": target }),
    );
    (monitor, result)
}

/// `events` with each `damage` into `target` one more than it was.
fn drifted(events: &[GameEvent], target: &str) -> Vec<GameEvent> {
    tampered(events, |event| match event {
        GameEvent::Damage {
            source_id,
            target_id,
            amount,
            combat,
            absorbed,
            via,
        } if target_id == target => Some(GameEvent::Damage {
            source_id: source_id.clone(),
            target_id: target_id.clone(),
            amount: amount + 1,
            combat: *combat,
            absorbed: *absorbed,
            via: *via,
        }),
        _ => None,
    })
}

fn i18(monitor: &mut InvariantMonitor, events: &[GameEvent], state: &GameState) -> Vec<String> {
    invariant("I18", monitor.after(events, state))
}

#[test]
fn r4111000_i18_an_attack_that_did_what_its_forecast_said_is_clean_and_counted() {
    let (state, attacker, defender) = i18_board("r4111000-i18-clean");
    let (mut monitor, result) = i18_attack(&state, &attacker, &defender);
    assert_eq!(
        i18(&mut monitor, &result.events, &result.state),
        Vec::<String>::new()
    );
    assert_eq!(monitor.forecast_judged(), 1);
}

#[test]
fn r4111000_i18_a_hit_one_off_the_forecast_names_both_cards_the_turn_and_both_sets_of_numbers() {
    let (state, attacker, defender) = i18_board("r4111000-i18-drift");
    let (mut monitor, result) = i18_attack(&state, &attacker, &defender);
    let found = i18(&mut monitor, &drifted(&result.events, &defender), &result.state);
    assert_eq!(found.len(), 1, "{found:?}");
    let finding = &found[0];
    assert!(finding.starts_with("I18 forecast drift: "), "{finding}");
    for part in [
        attacker.as_str(),
        "cb-plain",
        "Plain Body (combat fixture)",
        defender.as_str(),
        "cb-big-body",
        "Big Body (combat fixture)",
        "on turn 4",
        "was forecast dealt 3, taken 5, no kill, attacker falls, no shield pop but did dealt 4, taken 5, \
         no kill, attacker falls, no shield pop",
        "(R3100000)",
    ] {
        assert!(finding.contains(part), "{part:?} in {finding}");
    }
}

#[test]
fn r4111000_i18_a_strike_back_or_a_shield_the_forecast_did_not_foretell_is_drift() {
    let (state, attacker, defender) = i18_board("r4111000-i18-back");
    let (mut monitor, result) = i18_attack(&state, &attacker, &defender);
    let found = i18(
        &mut monitor,
        &tampered(&result.events, |event| match event {
            GameEvent::Damage {
                source_id,
                target_id,
                combat,
                absorbed,
                via,
                ..
            } if *target_id == attacker => Some(GameEvent::Damage {
                source_id: source_id.clone(),
                target_id: target_id.clone(),
                amount: 4,
                combat: *combat,
                absorbed: *absorbed,
                via: *via,
            }),
            _ => None,
        }),
        &result.state,
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("but did dealt 3, taken 4,"), "{}", found[0]);

    let (mut monitor, result) = i18_attack(&state, &attacker, &defender);
    let mut popped = result.events.clone();
    popped.push(GameEvent::DivineShieldLost {
        instance_id: defender.clone(),
    });
    let found = i18(&mut monitor, &popped, &result.state);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].ends_with("pops the shield (R3100000)"), "{}", found[0]);
}

#[test]
fn r4111000_i18_a_step_that_ran_a_hook_met_a_trap_or_a_redirect_or_ended_the_turn_is_not_judged() {
    let (state, attacker, defender) = i18_board("r4111000-i18-skipped");
    let declared = |forced: bool, instead_of: Option<String>| GameEvent::AttackDeclared {
        attacker_id: attacker.clone(),
        target_id: defender.clone(),
        forced,
        instead_of,
    };
    let appended: Vec<(&str, GameEvent)> = vec![
        (
            "a hook ran (R2783000)",
            GameEvent::HookResolving {
                source_id: defender.clone(),
                def_id: big_body.id.clone(),
                controller: P2,
                hook: HookKind::Whenever,
                hidden_from: None,
            },
        ),
        (
            "a trap fired (R3477000)",
            GameEvent::TrapFired {
                instance_id: "x-trap".to_string(),
                def_id: "fx-trap".to_string(),
                controller: P2,
                row: Row::Backrow,
                lane: 1,
                again: None,
            },
        ),
        (
            "the attack was called off",
            GameEvent::AttackCancelled {
                attacker_id: attacker.clone(),
                target_id: defender.clone(),
                by_instance_id: "x-trap".to_string(),
            },
        ),
        (
            "a hit was redirected (§4.4 step 4a)",
            GameEvent::Redirected {
                what: RedirectWhat::Damage,
                from_id: defender.clone(),
                to_id: "hero-p2".to_string(),
                by_instance_id: None,
            },
        ),
        (
            "the turn ended (R82)",
            GameEvent::TurnAutoEnded { player: P1, turn: 4 },
        ),
        ("a second declaration", declared(false, None)),
    ];
    for (why, event) in appended {
        let (mut monitor, result) = i18_attack(&state, &attacker, &defender);
        let mut events = drifted(&result.events, &defender);
        events.push(event);
        assert_eq!(
            i18(&mut monitor, &events, &result.state),
            Vec::<String>::new(),
            "{why}"
        );
        assert_eq!(monitor.forecast_judged(), 0, "{why}");
    }

    let declarations: Vec<(&str, GameEvent)> = vec![
        ("a forced attack (R53)", declared(true, None)),
        (
            "a substitute's attack (R1202)",
            declared(false, Some("x-windfast".to_string())),
        ),
    ];
    for (why, replacement) in declarations {
        let (mut monitor, result) = i18_attack(&state, &attacker, &defender);
        let events = tampered(&drifted(&result.events, &defender), |event| {
            matches!(event, GameEvent::AttackDeclared { .. }).then(|| replacement.clone())
        });
        assert_eq!(
            i18(&mut monitor, &events, &result.state),
            Vec::<String>::new(),
            "{why}"
        );
        assert_eq!(monitor.forecast_judged(), 0, "{why}");
    }

    let (mut monitor, result) = i18_attack(&state, &attacker, &defender);
    let mut prompted = result.state.clone();
    open_prompt(&mut prompted, "r4111000-i18-prompt");
    let events = drifted(&result.events, &defender);
    assert_eq!(
        i18(&mut monitor, &events, &prompted),
        Vec::<String>::new(),
        "an answer is still to come (R3477000)"
    );
    assert_eq!(monitor.forecast_judged(), 0);
}

/// `script` registered for `def_id`, over the scripts already registered.
fn register_i18_script(def_id: &str, script: Script) {
    let mut scripts = registered_scripts().clone();
    scripts.insert(def_id.to_string(), both(script));
    register_scripts(scripts);
}

#[test]
fn r4111000_i18_an_attack_the_forecast_cannot_see_from_the_board_is_not_judged() {
    let unjudged = |state: &GameState, attacker: &str, defender: &str, why: &str| {
        let (mut monitor, result) = i18_attack(state, attacker, defender);
        let events = drifted(&result.events, defender);
        assert_eq!(
            i18(&mut monitor, &events, &result.state),
            Vec::<String>::new(),
            "{why}"
        );
        assert_eq!(monitor.forecast_judged(), 0, "{why}");
    };

    let (mut state, attacker, defender) = i18_board("r4111000-i18-split");
    find_instance_mut(&mut state, &attacker)
        .expect("on the field")
        .split_chain = Some(1);
    unjudged(&state, &attacker, &defender, "a split damage chain (R3554007)");

    let (mut state, attacker, defender) = i18_board("r4111000-i18-shared");
    find_instance_mut(&mut state, &defender)
        .expect("on the field")
        .shared_chain = Some(1);
    unjudged(&state, &attacker, &defender, "a shared damage chain (R3554007)");

    let (state, attacker, defender) = i18_board("r4111000-i18-when-attacks");
    register_i18_script(
        &plain.id,
        Script {
            when_attacks: Some(hook(|_ctx| vec![])),
            ..Script::default()
        },
    );
    unjudged(&state, &attacker, &defender, "When this attacks (R2285000)");

    let (state, attacker, defender) = i18_board("r4111000-i18-reflects");
    register_i18_script(
        &big_body.id,
        Script {
            static_flags: Some(StaticFlags {
                reflects_to_attacker: Some(true),
                ..StaticFlags::default()
            }),
            ..Script::default()
        },
    );
    unjudged(
        &state,
        &attacker,
        &defender,
        "a reflection onto the attacker (R1849003)",
    );
}

/// R4221000 (fuzz seed 9561): a "would die" replacement acting for either side answers the death check
/// after the combat (§4.5 step 1) with no `trapFired` or `hookResolving`, and the forecast projects none:
/// here the attacker, foretold to fall, flickers home instead.
#[test]
fn r4221000_i18_an_attack_a_would_die_replacement_can_answer_is_not_judged() {
    let (state, attacker, defender) = i18_board("r4221000-i18-would-die");
    register_i18_script(
        &plain.id,
        Script {
            replacements: vec![ReplacementDef {
                id: "step".into(),
                on: ReplacementMoment::WouldDie,
                where_: None,
                when: None,
                instead: json_as(json!({ "flicker": "yours" })),
                then: None,
                by: None,
            }],
            ..Script::default()
        },
    );
    let (mut monitor, result) = i18_attack(&state, &attacker, &defender);
    assert!(
        result.events.iter().any(|event| matches!(
            event,
            GameEvent::Flickered { instance_id, .. } if *instance_id == attacker
        )),
        "the attacker flickered instead of dying"
    );
    let played = PlayedAttack::read(&result.events, &result.state, &attacker, &defender, &[]);
    assert!(played.survives, "{played}");
    assert_eq!(
        i18(&mut monitor, &result.events, &result.state),
        Vec::<String>::new()
    );
    assert_eq!(monitor.forecast_judged(), 0);
}

/// I18 on `attacker`'s attack on `defender`, its `damage` into the defender drifted one more than the
/// forecast: how many attacks it judged, and what it found.
fn judged_drift(state: &GameState, attacker: &str, defender: &str) -> (usize, Vec<String>) {
    let (mut monitor, result) = i18_attack(state, attacker, defender);
    let events = drifted(&result.events, defender);
    let found = i18(&mut monitor, &events, &result.state);
    (monitor.forecast_judged(), found)
}

/// R4221002 (fuzz seeds 4897 and 6161): a combatant whose leaving brings a card in at the state check
/// after the combat (one dormant beneath it in its pile, R1829000, or one it holds, R1850003), which
/// may exile a combatant by its aura (Stinkyscape #115 Scornboi): the forecast foretells nothing past
/// the combat, so I18 does not judge the attack.
#[test]
fn r4221002_i18_an_attack_whose_combatant_has_a_card_beneath_or_held_is_not_judged() {
    let (mut state, attacker, _) = i18_board("r4221002-i18-beneath");
    let mut top = new_instance(&mut state, &plain.id, P2, Zone::Hand { player: P2 });
    assert!(place_on_field(
        &mut state,
        &mut top,
        slot(P2, Row::Units, 1),
        json_as(json!({ "stack": true })),
    ));
    assert_eq!(
        beneath_at(&state, slot(P2, Row::Units, 1)).len(),
        1,
        "the 5/10 lies beneath the new top"
    );
    assert_eq!(
        judged_drift(&state, &attacker, &top.id),
        (0, Vec::new()),
        "a card dormant beneath the target (R1829000)"
    );

    let (mut state, attacker, defender) = i18_board("r4221002-i18-held");
    let card = new_instance(&mut state, &plain.id, P2, Zone::Gone { player: P2 });
    state.held = Some(vec![HeldCard {
        holder_id: defender.clone(),
        holder_mark: 0,
        to: P2,
        card,
        exile_on_release: None,
    }]);
    assert_eq!(
        judged_drift(&state, &attacker, &defender),
        (0, Vec::new()),
        "a card the target holds (R1850003)"
    );
}

/// R4221004 (fuzz seed 9305): a target whose side reflects what it takes onto the attacker's hero
/// (Reflective Plating's base face, R1849003) while the attacker's side has a damage pool (R3550004):
/// the reflected hit spends the pool before the strike back lands, so I18 does not judge the attack;
/// with no pool on the attacker's side, the same attack is judged.
#[test]
fn r4221004_i18_an_attack_on_a_reflector_into_the_attacker_s_pool_is_not_judged() {
    let reflector = || {
        register_i18_script(
            &big_body.id,
            Script {
                static_flags: Some(StaticFlags {
                    reflects_damage: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            },
        );
    };

    let (mut state, attacker, defender) = i18_board("r4221004-i18-pool");
    register_e6_fixtures();
    put(&mut state, POOL, slot(P1, Row::Backrow, 1), json!({}));
    reflector();
    assert_eq!(
        judged_drift(&state, &attacker, &defender),
        (0, Vec::new()),
        "the reflection spends the attacker's side's pool first"
    );

    let (state, attacker, defender) = i18_board("r4221004-i18-no-pool");
    reflector();
    let (judged, found) = judged_drift(&state, &attacker, &defender);
    assert_eq!(judged, 1, "with no pool to spend, the attack is judged");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("I18 forecast drift"), "{}", found[0]);
}

/// R4555001 (fuzz seed 4338): an attacker that reflects what it takes onto the enemy hero (Reflective
/// Plating, either face, R1849003) deals its strike back again to the target's hero, after any Trample
/// excess, which the forecast does not project, so I18 does not judge its attack on a Unit.
#[test]
fn r4555001_i18_an_attack_on_a_unit_by_a_reflector_is_not_judged() {
    let (state, attacker, defender) = i18_board("r4555001-i18-reflecting-attacker");
    register_i18_script(
        &plain.id,
        Script {
            static_flags: Some(StaticFlags {
                reflects_damage: Some(true),
                ..StaticFlags::default()
            }),
            ..Script::default()
        },
    );
    let (_, result) = i18_attack(&state, &attacker, &defender);
    assert!(
        result.events.iter().any(|event| matches!(
            event,
            GameEvent::Damage { source_id: Some(source), target_id, combat: false, .. }
                if *source == attacker && target_id == "hero-p2"
        )),
        "the attacker reflects its strike back onto p2's hero: {:?}",
        result.events
    );
    assert_eq!(
        judged_drift(&state, &attacker, &defender),
        (0, Vec::new()),
        "the reflection onto the target's hero is not projected"
    );
}

/// R4318000, R4379000: a Unit the state check after the combat takes off with a card dormant beneath it
/// (R1829000) brings that card in, which may act on a combatant by its aura, and the forecast is not
/// shown it (R13), so I18 does not judge the attack; a sweep's other hit here, since a Cleave
/// neighbour's pile is R4221002's skip already. With the pile's top left standing, nothing comes in
/// and the attack is judged.
#[test]
fn r4318000_r4379000_i18_an_attack_whose_settled_deaths_bring_a_card_in_is_not_judged() {
    for (seed, top, under, judged) in [
        ("r4379000-i18-pile-falls", &plain.id, &big_body.id, 0),
        ("r4379000-i18-pile-stands", &big_body.id, &plain.id, 1),
    ] {
        let (mut state, attacker, defender) = i18_board(seed);
        register_sweeper(&plain.id);
        put(&mut state, under, slot(P2, Row::Units, 2), json!({}));
        stack_on(&mut state, top, slot(P2, Row::Units, 2));
        let (count, found) = judged_drift(&state, &attacker, &defender);
        assert_eq!(count, judged, "{seed}: {found:?}");
        assert_eq!(found.len(), judged, "{seed}: {found:?}");
        assert!(
            found.iter().all(|finding| finding.contains("I18 forecast drift")),
            "{seed}: {found:?}"
        );
    }
}

/// R4379000 (fuzz seed 6180): an attack whose other hit (a Cleave neighbour's, §4.4 step 10; a sweep's,
/// R3554008, on a Unit or the hero) kills the Unit whose aura holds its target up is judged again: the
/// forecast settles the state check after the combat, so it foretells the target's fall a pass later,
/// and a drift is still found. With the aura's Unit out of reach, the same attack is judged too.
#[test]
fn r4379000_i18_judges_an_attack_whose_other_hits_take_away_a_combatant_s_aura() {
    for (seed, sweeps, aimed_at_hero) in [
        ("r4379000-i18-cleave", false, false),
        ("r4379000-i18-sweep", true, false),
        ("r4379000-i18-sweep-hero", true, true),
    ] {
        let attacker_def = if sweeps { &plain.id } else { &cleaver.id };
        let (state, attacker, defender, _) = aura_board(seed, attacker_def, 2);
        if sweeps {
            register_sweeper(&plain.id);
        }
        let target = if aimed_at_hero {
            "hero-p2".to_string()
        } else {
            defender.clone()
        };
        let (mut monitor, result) = i18_attack(&state, &attacker, &target);
        assert!(
            result.events.iter().any(|event| matches!(
                event,
                GameEvent::Destroyed { instance_id, .. } if *instance_id == defender
            )),
            "{seed}: the aura's Unit died, and the Big Body with it"
        );
        assert_eq!(
            i18(&mut monitor, &result.events, &result.state),
            Vec::<String>::new(),
            "{seed}"
        );
        assert_eq!(monitor.forecast_judged(), 1, "{seed}");
        if !aimed_at_hero {
            let (judged, found) = judged_drift(&state, &attacker, &defender);
            assert_eq!((judged, found.len()), (1, 1), "{seed}: {found:?}");
            assert!(found[0].contains("I18 forecast drift"), "{seed}: {}", found[0]);
        }
    }

    let (state, attacker, defender, _) = aura_board("r4379000-i18-out-of-reach", &cleaver.id, 3);
    let (judged, found) = judged_drift(&state, &attacker, &defender);
    assert_eq!(
        judged, 1,
        "with the aura's Unit out of reach, the attack is judged"
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("I18 forecast drift"), "{}", found[0]);
}

/// R4379000, §4.5 step 4: a Reborn Charger the Cleave kills is back with its aura before the next pass,
/// so the Big Body stands, as the forecast says; I18 judges the attack and finds it clean.
#[test]
fn r4379000_i18_judges_an_attack_past_a_reborn_return() {
    let (mut state, attacker, defender, aura) = aura_board("r4379000-i18-reborn", &cleaver.id, 2);
    find_instance_mut(&mut state, &aura)
        .expect("on the field")
        .granted_keywords
        .push(Keyword::Reborn);
    let (mut monitor, result) = i18_attack(&state, &attacker, &defender);
    assert_eq!(
        i18(&mut monitor, &result.events, &result.state),
        Vec::<String>::new()
    );
    assert_eq!(monitor.forecast_judged(), 1);
}

/// `i18_board` with a 3/8 Cleave Unit with Trample in p1's lane 2, facing a 3/3 hurt to 1 health in
/// p2's lane 2, between `i18_board`'s 5/10 in lane 1 and another in lane 3. The forecast's splash is 3
/// on each 5/10 and its overflow the blow's 2 past the 3/1 onto p2's hero (R4290000). The ids of the
/// cleaver, the 3/1 and the 5/10 in lane 3.
fn i18_cleave_board(seed: &str) -> (GameState, String, String, String) {
    let (mut state, _, _) = i18_board(seed);
    let striker = put(&mut state, &cleaver.id, slot(P1, Row::Units, 2), json!({}));
    find_instance_mut(&mut state, &striker.id)
        .expect("on the field")
        .granted_keywords
        .push(Keyword::Trample);
    let target = put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));
    find_instance_mut(&mut state, &target.id)
        .expect("on the field")
        .damage = 2;
    let beside = put(&mut state, &big_body.id, slot(P2, Row::Units, 3), json!({}));
    (state, striker.id, target.id, beside.id)
}

#[test]
fn r4290000_i18_holds_a_cleave_s_splash_and_a_trample_s_overflow() {
    let (state, striker, target, beside) = i18_cleave_board("r4290000-i18");
    let (mut monitor, result) = i18_attack(&state, &striker, &target);
    assert_eq!(
        i18(&mut monitor, &result.events, &result.state),
        Vec::<String>::new()
    );
    assert_eq!(monitor.forecast_judged(), 1);

    for (drifted_onto, what) in [(beside.as_str(), "the splash"), ("hero-p2", "the overflow")] {
        let (mut monitor, result) = i18_attack(&state, &striker, &target);
        let found = i18(
            &mut monitor,
            &drifted(&result.events, drifted_onto),
            &result.state,
        );
        assert_eq!(found.len(), 1, "{what}: {found:?}");
        assert!(found[0].contains("I18 forecast drift"), "{what}: {}", found[0]);
    }
}

#[test]
fn r4290000_i18_does_not_judge_a_cleave_beside_a_chained_unit() {
    let (mut state, striker, target, beside) = i18_cleave_board("r4290000-i18-chained");
    find_instance_mut(&mut state, &beside)
        .expect("on the field")
        .shared_chain = Some(1);
    let (mut monitor, result) = i18_attack(&state, &striker, &target);
    assert_eq!(
        i18(&mut monitor, &drifted(&result.events, &beside), &result.state),
        Vec::<String>::new(),
        "a shared damage chain beside the target (R3554007)"
    );
    assert_eq!(monitor.forecast_judged(), 0);
}

#[test]
fn r4111000_played_attack_reads_a_return_and_an_exile_as_gone_and_an_alternative_win_as_no_kill() {
    let (state, attacker, defender) = i18_board("r4111000-played");
    let hit = |target: &str, amount: i32| GameEvent::Damage {
        source_id: Some(attacker.clone()),
        target_id: target.to_string(),
        amount,
        combat: true,
        absorbed: 0,
        via: None,
    };

    // Reborn returns a destroyed unit with its own id (R83): the field still holds it, but it died.
    let returned = PlayedAttack::read(
        &[hit(&defender, 3), destroyed_by(&defender, &attacker)],
        &state,
        &attacker,
        &defender,
        &[],
    );
    assert!(returned.kills && returned.survives, "{returned}");

    // An exile on damage raises no `destroyed` (R1124): the unit is simply off the field.
    let mut exiled = state.clone();
    find_instance_mut(&mut exiled, &defender)
        .expect("on the field")
        .zone = Zone::Exile { player: P2 };
    let read = PlayedAttack::read(&[hit(&defender, 3)], &exiled, &attacker, &defender, &[]);
    assert!(read.kills, "{read}");

    // A game won another way over a hero still standing is no kill of it (R850).
    let mut alt_win = state.clone();
    alt_win.players[P2].hero.health = 5;
    alt_win.result = Some(GameResult {
        winner: Winner::P1,
        reason: GameOverReason::AltWin,
    });
    let read = PlayedAttack::read(&[hit("hero-p2", 3)], &alt_win, &attacker, "hero-p2", &[]);
    assert_eq!(
        read,
        PlayedAttack {
            dealt: 3,
            taken: 0,
            kills: false,
            survives: true,
            pops: false,
            splash: Vec::new(),
            overflow: None,
        }
    );

    let mut hero_death = alt_win.clone();
    hero_death.players[P2].hero.health = 0;
    hero_death.result = Some(GameResult {
        winner: Winner::P1,
        reason: GameOverReason::HeroDeath,
    });
    assert!(PlayedAttack::read(&[hit("hero-p2", 3)], &hero_death, &attacker, "hero-p2", &[]).kills);
}

// I19: nothing reaches a Dormant card (§6.1 Dormant, §3.2, R1839000; #4454). Proved on synthetic events
// for each kind that names a card, on edited records, and on real steps over `fixtures/slumber.rs`'s
// Dormant cards and the engine's own board verbs (a swap, a rotation, a board exile, a slash).
mod i19 {
    use super::*;
    use jackioh_engine::effects::{exile_board, fuse_cards, make_all_radiant, slash_board, swap_board};
    use jackioh_engine::subsystems::board_history::{
        RollBackArgs, RollBackSides, record_board_snapshot, roll_back,
    };
    use jackioh_engine::subsystems::rotation::{RotationArgs, RotationDirection, rotate_rings};

    use crate::rules::fixtures::combat::stacker;
    use crate::rules::fixtures::harness::run_effect;
    use crate::rules::fixtures::slumber::{
        LAUNCH, cheer, copycat, flood, frost, heist, imprisoned, napper, quake, register_slumber_fixtures,
        rouse, starship, void, wipe,
    };

    /// I19 of one step from `before` to `after`, read as the fuzz reads it.
    fn i19(before: &GameState, events: &[GameEvent], after: &GameState) -> Vec<String> {
        let mut monitor = create_invariant_monitor(before);
        monitor.before(before, before.active, &ActionBody::EndTurn);
        invariant("I19", monitor.after(events, after))
    }

    /// I19 of the step `act` takes on `s`.
    fn i19_of(s: &mut Scenario, act: impl FnOnce(&mut Scenario)) -> Vec<String> {
        let before = s.state().clone();
        act(s);
        i19(&before, s.last_events(), s.state())
    }

    /// `def_id` put at `at`, Dormant with two turns left: its id.
    fn put_dormant(state: &mut GameState, def_id: &str, at: zones::ZoneSlot) -> String {
        let card = put(state, def_id, at, json!({}));
        find_instance_mut(state, &card.id).expect("the permanent").slumber = Some(Slumber { turns: Some(2) });
        card.id
    }

    /// p1's first main phase with an `fx-21` Dormant in p2's unit zone 1, and its id.
    fn dormant_board(seed: &str) -> (GameState, String) {
        let mut state = playing(seed);
        let id = put_dormant(&mut state, "fx-21", slot(P2, Row::Units, 1));
        (state, id)
    }

    /// slumber.rs's board: `active`'s turn with the given hands and fields over the Dormant fixtures.
    fn board(active: &str, p1_hand: Value, p1_field: Value, p2_hand: Value, p2_field: Value) -> Scenario {
        register_slumber_fixtures();
        let library = json!([plain.id, plain.id, plain.id, plain.id, plain.id, plain.id]);
        scenario(json!({
            "seed": "i19",
            "active": active,
            "p1": { "hand": p1_hand, "field": p1_field, "library": library },
            "p2": { "hand": p2_hand, "field": p2_field, "library": library },
        }))
    }

    fn awakened(id: &str) -> GameEvent {
        json_as(json!({ "type": "awakened", "player": "p2", "instanceId": id, "defId": "fx-21" }))
    }

    fn marker(def_id: &str) -> GameEvent {
        json_as(json!({
            "type": "hookResolving",
            "sourceId": "i19-source",
            "defId": def_id,
            "controller": "p1",
            "hook": "cry",
        }))
    }

    fn fused(id: &str) -> GameEvent {
        json_as(json!({
            "type": "fused",
            "instanceIds": [id, "i19-piece"],
            "resultInstanceId": id,
            "defId": "fx-21",
        }))
    }

    fn starship_fused(id: &str) -> GameEvent {
        json_as(json!({
            "type": "starshipFused",
            "player": "p2",
            "instanceId": id,
            "pieceId": "i19-piece",
            "defId": "fx-2",
            "pieces": 1,
        }))
    }

    /// Every event kind that changes a card and names it, each naming `card`, by its wire name.
    fn reaching(card: &CardInstance) -> Vec<(&'static str, GameEvent)> {
        let id = card.id.as_str();
        let zone = serde_json::to_value(&card.zone).expect("a zone serializes");
        vec![
            ("damage", damage_event(id, 1)),
            (
                "healed",
                json_as(json!({ "type": "healed", "targetId": id, "amount": 1 })),
            ),
            (
                "buffed",
                json_as(json!({ "type": "buffed", "instanceId": id, "attack": 1, "health": 1 })),
            ),
            (
                "keywordGranted",
                json_as(
                    json!({ "type": "keywordGranted", "instanceId": id, "keyword": { "kind": "Taunt" } }),
                ),
            ),
            (
                "counterChanged",
                json_as(
                    json!({ "type": "counterChanged", "instanceId": id, "counter": "plague", "value": 1 }),
                ),
            ),
            (
                "costChanged",
                json_as(json!({ "type": "costChanged", "instanceId": id, "cost": 1 })),
            ),
            (
                "radiantSet",
                json_as(json!({ "type": "radiantSet", "instanceId": id, "defId": "fx-21", "zone": zone })),
            ),
            (
                "transformed",
                json_as(json!({
                    "type": "transformed",
                    "instanceId": id,
                    "fromDefId": "fx-21",
                    "toDefId": "fx-22",
                    "newInstanceId": "i19-new",
                })),
            ),
            (
                "bounced",
                json_as(json!({ "type": "bounced", "instanceId": id, "defId": "fx-21", "owner": "p2" })),
            ),
            (
                "stolen",
                json_as(json!({
                    "type": "stolen",
                    "instanceId": id,
                    "defId": "fx-21",
                    "from": "p2",
                    "to": "p1",
                    "zone": "field",
                })),
            ),
            (
                "controlChanged",
                json_as(json!({
                    "type": "controlChanged",
                    "instanceId": id,
                    "controller": "p1",
                    "row": "units",
                    "lane": 1,
                })),
            ),
            (
                "positionSwitched",
                json_as(json!({ "type": "positionSwitched", "instanceId": id, "position": "DEF" })),
            ),
            ("fused", fused(id)),
            (
                "held",
                json_as(json!({
                    "type": "held",
                    "player": "p2",
                    "instanceId": id,
                    "defId": "fx-21",
                    "holderId": "i19-holder",
                })),
            ),
            (
                "destroyed",
                json_as(json!({
                    "type": "destroyed",
                    "instanceId": id,
                    "defId": "fx-21",
                    "owner": "p2",
                    "controller": "p2",
                    "attack": 2,
                    "maxHealth": 2,
                })),
            ),
            (
                "exiled",
                json_as(json!({ "type": "exiled", "instanceId": id, "defId": "fx-21", "owner": "p2" })),
            ),
        ]
    }

    #[test]
    fn i19_a_step_that_reaches_no_dormant_card_is_clean() {
        let (state, _) = dormant_board("i19-quiet");
        assert_eq!(i19(&state, &[], &state), Vec::<String>::new());
        // A real end of turn: p2's turn start counts the Dormant card down (R1839001).
        let ended = reduce(&state, &Action::new(ActionBody::EndTurn, P1, "i19-end"));
        assert!(ended.error.is_none(), "{:?}", ended.error);
        assert_eq!(i19(&state, &ended.events, &ended.state), Vec::<String>::new());
        // Every sweep that passes a Dormant card by (R1839000), beside a Unit it reaches.
        for spell in [&quake, &wipe, &void, &flood, &frost, &cheer, &heist] {
            let mut s = board(
                "p1",
                json!([spell.id, plain.id]),
                json!([]),
                json!([plain.id]),
                json!([
                    { "def": imprisoned.id, "lane": 1, "slumber": { "turns": 2 } },
                    { "def": plain.id, "lane": 2 },
                ]),
            );
            let found = i19_of(&mut s, |s| {
                s.play(spell.id.as_str(), json!({}));
            });
            assert_eq!(found, Vec::<String>::new(), "{}", spell.id);
            assert!(!s.last_events().is_empty(), "{} reached the plain Unit", spell.id);
        }
    }

    #[test]
    fn i19_each_event_that_names_a_dormant_card_is_one_finding() {
        let (state, id) = dormant_board("i19-events");
        let card = find_instance(&state, &id).expect("the Dormant card").clone();
        for (kind, event) in reaching(&card) {
            assert_eq!(event.event_type().as_str(), kind);
            let found = i19(&state, &[event], &state);
            only(&found, &id, &["I19 reached Dormant", kind, "§6.1", "R1839000"]);
        }
        // A mark aimed at it reaches it; a mark taken off, its waiting effect gone, does not (R437).
        let mark = |added: bool| -> GameEvent {
            json_as(
                json!({ "type": "marked", "instanceId": id, "mark": "steal", "color": "purple", "added": added }),
            )
        };
        only(
            &i19(&state, &[mark(true)], &state),
            &id,
            &["I19 reached Dormant", "marked"],
        );
        assert_eq!(i19(&state, &[mark(false)], &state), Vec::<String>::new());
        // A card is one finding however often it is reached.
        let twice = [damage_event(&id, 1), damage_event(&id, 2)];
        only(&i19(&state, &twice, &state), &id, &["I19 reached Dormant"]);
        // The running card is named.
        let hit = [marker("fx-1"), damage_event(&id, 1)];
        only(&i19(&state, &hit, &state), &id, &["in fx-1's run"]);
    }

    #[test]
    fn i19_a_record_change_with_no_event_is_one_finding_naming_the_field() {
        let (state, id) = dormant_board("i19-record");
        /// A record field's JSON name and an edit that changes only it.
        type RecordEdit = (&'static str, fn(&mut CardInstance));
        let edits: Vec<RecordEdit> = vec![
            ("damage", |card| card.damage = 1),
            ("buffs", |card| card.buffs = AttackHealth { attack: 1, health: 1 }),
            ("grantedKeywords", |card| {
                card.granted_keywords = vec![json_as(json!({ "kind": "Taunt" }))]
            }),
            ("counters", |card| card.counters.plague = Some(1)),
            ("frozen", |card| card.frozen = Some(1)),
            ("costMod", |card| card.cost_mod = -1),
            ("position", |card| card.position = Some(Position::Def)),
            ("radiant", |card| card.radiant = true),
            ("controller", |card| card.controller = P1),
            ("markedDestroyed", |card| card.marked_destroyed = Some(true)),
            ("defId", |card| card.def_id = "fx-22".to_string()),
        ];
        for (field, edit) in edits {
            let mut after = state.clone();
            edit(find_instance_mut(&mut after, &id).expect("the Dormant card"));
            only(
                &i19(&state, &[], &after),
                &id,
                &["I19 Dormant record changed", field, "§6.1", "R1839000"],
            );
        }
        // The countdown is its own (R1839001), and the zone a move along its side changes (R14).
        let mut after = state.clone();
        find_instance_mut(&mut after, &id)
            .expect("the Dormant card")
            .slumber = Some(Slumber { turns: Some(1) });
        assert_eq!(i19(&state, &[], &after), Vec::<String>::new());
    }

    /// An exile of one card of `walk`'s, drawn on the match rng: The Maestro's field pick before #4322.
    fn random_exile(walk: fn(&GameState) -> Vec<CardInstance>) -> Effect {
        Effect::new("i19-random-exile", move |ctx| {
            let pool = walk(ctx.state);
            if let Some(card) = ctx.rng.pick(&pool).cloned() {
                zones::exile_to_pile(&mut ctx.sink, card);
            }
        })
    }

    #[test]
    fn i19_a_random_exile_drawn_from_a_walk_that_keeps_dormant_cards_is_flagged() {
        let (before, id) = dormant_board("i19-maestro");
        let mut state = before.clone();
        let keeps = random_exile(|state| zones::units_on_field_of(state, P2).into_iter().cloned().collect());
        let events = run_effect(&mut state, keeps, P1);
        assert!(
            state.players[P2].exile.iter().any(|card| card.id == id),
            "the Dormant card is the only pick"
        );
        only(
            &i19(&before, &events, &state),
            &id,
            &["I19 reached Dormant", "exiled", "§6.1", "R1839000"],
        );
        // Drawn from the permanents an effect reaches (R4322000), the pick finds nothing.
        let mut state = before.clone();
        let reaches = random_exile(|state| {
            plague::reachable_permanents_on_field(state, P2)
                .into_iter()
                .cloned()
                .collect()
        });
        let events = run_effect(&mut state, reaches, P1);
        assert!(events.is_empty(), "{events:?}");
        assert_eq!(i19(&before, &events, &state), Vec::<String>::new());
    }

    /// One effect step of `player`'s turn start: the Dormant countdowns (R1839001).
    fn count_down(state: &mut GameState, player: PlayerId) -> Vec<GameEvent> {
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            slumber::awaken_at_turn_start(&mut sink, player);
        }
        state.rng_cursor = rng.cursor();
        events
    }

    /// Silly Silas's rotation (R14) from the active player's seat.
    fn rotate(state: &mut GameState, direction: RotationDirection, radiant: bool) -> Vec<GameEvent> {
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let perspective = state.active;
        {
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            rotate_rings(
                &mut sink,
                &RotationArgs {
                    direction,
                    perspective,
                    radiant: Some(radiant),
                },
            );
        }
        state.rng_cursor = rng.cursor();
        events
    }

    #[test]
    fn r4454000_a_board_swap_and_a_rotation_carry_a_dormant_card_and_i19_allows_it() {
        let (before, id) = dormant_board("r4454000-swap");
        let mut swapped = before.clone();
        let events = run_effect(&mut swapped, swap_board(), P1);
        let card = find_instance(&swapped, &id).expect("still on the field");
        assert_eq!(
            card.zone,
            Zone::Field {
                player: P1,
                row: Row::Units,
                lane: 1
            }
        );
        assert_eq!(card.controller, P1);
        assert_eq!(card.slumber, Some(Slumber { turns: Some(2) }), "still Dormant");
        assert_eq!(i19(&before, &events, &swapped), Vec::<String>::new());
        // Its countdown then counts its new controller's turns, not its owner's (R1839001).
        let mut theirs = swapped.clone();
        count_down(&mut theirs, P2);
        assert_eq!(
            find_instance(&theirs, &id).expect("on the field").slumber,
            Some(Slumber { turns: Some(2) })
        );
        let mut mine = swapped.clone();
        let counted = count_down(&mut mine, P1);
        assert_eq!(
            find_instance(&mine, &id).expect("on the field").slumber,
            Some(Slumber { turns: Some(1) })
        );
        assert_eq!(i19(&swapped, &counted, &mine), Vec::<String>::new());
        // A hit inside the swap's window is still a reach.
        let mut hit = events.clone();
        hit.push(damage_event(&id, 1));
        only(
            &i19(&before, &hit, &swapped),
            &id,
            &["I19 reached Dormant", "damage"],
        );

        // A rotation carries it along its side or across the center line, still Dormant (R14).
        for direction in [RotationDirection::Left, RotationDirection::Right] {
            let mut rotated = before.clone();
            let events = rotate(&mut rotated, direction, false);
            let card = find_instance(&rotated, &id).expect("still on the field");
            assert_ne!(
                card.zone,
                Zone::Field {
                    player: P2,
                    row: Row::Units,
                    lane: 1
                }
            );
            assert_eq!(card.slumber, Some(Slumber { turns: Some(2) }), "{direction:?}");
            assert_eq!(
                i19(&before, &events, &rotated),
                Vec::<String>::new(),
                "{direction:?}"
            );
        }
        // The Radiant rotation bounces a card leaving its caster's side (R14): to its hand, its status
        // gone with R78's reset.
        let mut ends = playing("r4454000-radiant");
        let first = put_dormant(&mut ends, "fx-1", slot(P1, Row::Units, 1));
        let last = put_dormant(&mut ends, "fx-2", slot(P1, Row::Units, 5));
        for direction in [RotationDirection::Left, RotationDirection::Right] {
            let mut rotated = ends.clone();
            let events = rotate(&mut rotated, direction, true);
            let in_hand: Vec<&CardInstance> = rotated.players[P1]
                .hand
                .iter()
                .filter(|card| card.id == first || card.id == last)
                .collect();
            assert_eq!(in_hand.len(), 1, "{direction:?}");
            assert_eq!(in_hand[0].slumber, None);
            assert_eq!(
                i19(&ends, &events, &rotated),
                Vec::<String>::new(),
                "{direction:?}"
            );
        }
    }

    #[test]
    fn i19_the_countdown_awaken_and_a_dormant_card_s_own_launch_are_lawful() {
        let mut s = board(
            "p2",
            json!([rouse.id, plain.id]),
            json!([
                { "def": napper.id, "lane": 1, "slumber": { "turns": 1 } },
                { "def": imprisoned.id, "lane": 2, "slumber": { "turns": 2 } },
                { "def": starship.id, "lane": 3, "slumber": {} },
            ]),
            json!([plain.id]),
            json!([]),
        );
        let ids: Vec<String> = (1..=3)
            .map(|lane| s.unit(P1, lane).expect("a Dormant card").id)
            .collect();
        // p1's turn start: the Napper's countdown ends and the Imprisoned's counts (R1839001).
        assert_eq!(
            i19_of(&mut s, |s| {
                s.end_turn();
            }),
            Vec::<String>::new()
        );
        assert_eq!(s.card(ids[0].as_str()).slumber, None);
        assert_eq!(s.card(ids[1].as_str()).slumber, Some(Slumber { turns: Some(1) }));
        // The Starship's own Activate awakens it (R1839002).
        let launch = i19_of(&mut s, |s| {
            s.activate(ids[2].as_str(), json!({ "ability": LAUNCH }));
        });
        assert_eq!(launch, Vec::<String>::new());
        assert_eq!(s.card(ids[2].as_str()).slumber, None);
        // The Awaken verb from another card (R1839002).
        assert_eq!(
            i19_of(&mut s, |s| {
                s.play(rouse.id.as_str(), json!({}));
            }),
            Vec::<String>::new()
        );
        assert_eq!(s.card(ids[1].as_str()).slumber, None);
    }

    #[test]
    fn i19_a_hull_fuse_is_lawful_and_any_other_fuse_onto_a_dormant_card_is_not() {
        let (before, id) = dormant_board("i19-hull");
        // The hull keeps its zone and status and becomes the fusion (R1852000): a new definition.
        let mut after = before.clone();
        find_instance_mut(&mut after, &id).expect("the hull").def_id = "fx-22".to_string();
        let hull = [fused(&id), starship_fused(&id)];
        assert_eq!(i19(&before, &hull, &after), Vec::<String>::new());
        // An Equip's or a spell's fuse onto it has no `starshipFused` (R2260000, #4424).
        only(
            &i19(&before, &[fused(&id)], &after),
            &id,
            &["I19 reached Dormant", "fused"],
        );
        // Nor does a `starshipFused` naming another ship lend it one.
        let elsewhere = [fused(&id), starship_fused("i19-other")];
        only(&i19(&before, &elsewhere, &after), &id, &["fused"]);
    }

    #[test]
    fn i19_an_exile_inside_a_board_sweep_s_run_is_lawful_and_outside_it_is_not() {
        let (before, id) = dormant_board("i19-sweep");
        let mut swept = before.clone();
        let exiled = run_effect(&mut swept, exile_board(json_as(json!({ "player": "enemy" }))), P1);
        assert!(swept.players[P2].exile.iter().any(|card| card.id == id));
        let behind = |markers: &[GameEvent]| -> Vec<GameEvent> {
            markers.iter().chain(exiled.iter()).cloned().collect()
        };
        // S+ #79, #80 and #81's sweeps (R3539002).
        for sweep in [
            "stinkyscapeplus-079",
            "stinkyscapeplus-080",
            "stinkyscapeplus-081",
        ] {
            assert_eq!(
                i19(&before, &behind(&[marker(sweep)]), &swept),
                Vec::<String>::new(),
                "{sweep}"
            );
        }
        // Any other card's run, none, or another card's marker opened since the sweep's is a reach.
        only(
            &i19(&before, &behind(&[marker("fx-1")]), &swept),
            &id,
            &["exiled", "fx-1"],
        );
        only(&i19(&before, &exiled, &swept), &id, &["exiled", "no card"]);
        let interrupted = behind(&[marker("stinkyscapeplus-080"), marker("fx-1")]);
        only(&i19(&before, &interrupted, &swept), &id, &["exiled"]);

        // S+ #78's slash exiles lane 3, Dormant cards included (R3554002).
        let mut lane = playing("i19-slash");
        let middle = put_dormant(&mut lane, "fx-21", slot(P2, Row::Units, 3));
        let mut slashed = lane.clone();
        let events = run_effect(&mut slashed, slash_board(json_as(json!({}))), P1);
        assert!(slashed.players[P2].exile.iter().any(|card| card.id == middle));
        let marked: Vec<GameEvent> = std::iter::once(marker("stinkyscapeplus-078"))
            .chain(events.iter().cloned())
            .collect();
        assert_eq!(i19(&lane, &marked, &slashed), Vec::<String>::new());
        // The Radiant face's lane answer continues the slash's run with no marker of its own (R2783000).
        let mut asked = lane.clone();
        open_prompt(&mut asked, "i19-slash-asked");
        asked.pending.as_mut().expect("a prompt").resume.def_id = "stinkyscapeplus-078".to_string();
        assert_eq!(i19(&asked, &events, &slashed), Vec::<String>::new());
        only(&i19(&lane, &events, &slashed), &middle, &["exiled"]);
    }

    #[test]
    fn i19_a_stack_card_laid_onto_a_dormant_card_is_clean() {
        for top in [&stacker, &copycat] {
            let mut s = board(
                "p1",
                json!([top.id, plain.id]),
                json!([{ "def": imprisoned.id, "lane": 1, "slumber": { "turns": 2 } }]),
                json!([plain.id]),
                json!([]),
            );
            let dormant = s.unit(P1, 1).expect("the Dormant card").id;
            let found = i19_of(&mut s, |s| {
                s.play(top.id.as_str(), json!({ "zone": 1 }));
            });
            assert_eq!(found, Vec::<String>::new(), "{}", top.id);
            // It lies beneath, still Dormant and still itself (R1839003).
            assert_eq!(s.unit(P1, 1).expect("the top").def_id, top.id);
            assert_eq!(s.card(dormant.as_str()).def_id, imprisoned.id);
            assert_eq!(s.card(dormant.as_str()).slumber, Some(Slumber { turns: Some(2) }));
        }
    }

    #[test]
    fn i19_a_card_dormant_only_after_the_step_began_or_woken_before_the_hit_is_clean() {
        let (dormant, id) = dormant_board("i19-timing");
        let mut awake = dormant.clone();
        find_instance_mut(&mut awake, &id).expect("the card").slumber = None;
        // Dormant only by the step's end: it is judged from the next step.
        assert_eq!(
            i19(&awake, &[damage_event(&id, 1)], &dormant),
            Vec::<String>::new()
        );
        // Woken, then hit (R1839002).
        let woken_first = [awakened(&id), damage_event(&id, 1)];
        assert_eq!(i19(&dormant, &woken_first, &awake), Vec::<String>::new());
        // Hit, then woken.
        let hit_first = [damage_event(&id, 1), awakened(&id)];
        only(
            &i19(&dormant, &hit_first, &awake),
            &id,
            &["I19 reached Dormant", "damage"],
        );
        // Launched, then hit (R1852003); its own Activate, then hit (R1839002).
        let launched: GameEvent = json_as(json!({
            "type": "starshipLaunched",
            "player": "p2",
            "instanceId": id,
            "defId": "fx-21",
            "name": "i19",
            "pieces": [],
        }));
        let activated: GameEvent = json_as(json!({
            "type": "activated",
            "player": "p2",
            "instanceId": id,
            "defId": "fx-21",
            "ability": "launch",
        }));
        for first in [launched, activated] {
            let hits = [first, damage_event(&id, 1)];
            assert_eq!(i19(&dormant, &hits, &awake), Vec::<String>::new());
        }
        // An `after` with no `before` judges nothing.
        let mut unprimed = create_invariant_monitor(&dormant);
        assert_eq!(
            invariant("I19", unprimed.after(&[damage_event(&id, 1)], &dormant)),
            Vec::<String>::new()
        );
    }

    #[test]
    fn i19_runs_on_i14_s_branches() {
        let (state, id) = dormant_board("i19-i14");
        let hit = damage_event(&id, 1);
        let branch = |state: &GameState, action: &Action| -> ReduceResult {
            let mut result = reduce(state, action);
            result.events.push(hit.clone());
            result
        };
        let found: Vec<String> = listed_refused(&state, P1, &[ActionBody::EndTurn], branch)
            .into_iter()
            .filter(|finding| finding.contains("I19"))
            .collect();
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("I14 branch:"), "{}", found[0]);
        assert!(found[0].contains("I19 reached Dormant"), "{}", found[0]);
        assert!(found[0].contains(&id), "{}", found[0]);
        // The plain reducer's branch reaches nothing.
        let clean: Vec<String> = listed_refused(&state, P1, &[ActionBody::EndTurn], reduce)
            .into_iter()
            .filter(|finding| finding.contains("I19"))
            .collect();
        assert_eq!(clean, Vec::<String>::new());
    }

    /// `effect` applied as p1's and settled, as a card's effect is (§10.3): the events it emitted.
    fn settled(state: &mut GameState, effect: Effect) -> Vec<GameEvent> {
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
        {
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            {
                let mut ctx = make_context(&mut sink, None, by_p1());
                (effect.apply)(&mut ctx);
            }
            settle(&mut sink, SettleOptions::default());
        }
        state.rng_cursor = rng.cursor();
        events
    }

    fn by_p1() -> HookOptions {
        HookOptions {
            controller: Some(P1),
            ..Default::default()
        }
    }

    fn radiant_set(card: &CardInstance) -> GameEvent {
        json_as(json!({
            "type": "radiantSet",
            "instanceId": card.id,
            "defId": card.def_id,
            "zone": serde_json::to_value(&card.zone).expect("a zone serializes"),
        }))
    }

    #[test]
    fn r4454001_tears_of_the_kwzki_tree_s_world_makes_a_dormant_card_radiant_and_i19_allows_it() {
        let (before, id) = dormant_board("r4454001");
        let mut after = before.clone();
        let events = settled(&mut after, make_all_radiant());
        let card = find_instance(&after, &id).expect("still on the field");
        assert!(card.radiant, "Radiant with every other card (R3569000)");
        assert_eq!(card.slumber, Some(Slumber { turns: Some(2) }), "still Dormant");
        assert!(
            events.iter().any(
                |event| matches!(event, GameEvent::RadiantSet { instance_id, .. } if *instance_id == id)
            ),
            "{events:?}"
        );
        assert_eq!(i19(&before, &events, &after), Vec::<String>::new());
        // Outside that world the same `radiantSet` is a reach, and the face a record change.
        let mut no_world = after.clone();
        no_world.radiant_world = None;
        only(
            &i19(&before, &events, &no_world),
            &id,
            &["I19 reached Dormant", "radiantSet"],
        );
        only(
            &i19(&before, &[], &no_world),
            &id,
            &["I19 Dormant record changed", "radiant"],
        );
    }

    #[test]
    fn r4454002_a_board_restore_takes_a_dormant_card_back_with_the_board_and_i19_allows_it() {
        let mut snapshotted = playing("r4454002");
        let held = put_dormant(&mut snapshotted, "fx-21", slot(P2, Row::Units, 1));
        // The only snapshot is this one: the turn's own, of an empty board, would be found first.
        snapshotted.board_history = None;
        record_board_snapshot(&mut snapshotted);
        // Since the snapshot: a piece fused into the held card (R1852000), and a second Dormant card.
        let mut before = snapshotted.clone();
        find_instance_mut(&mut before, &held)
            .expect("the held card")
            .def_id = "fx-22".to_string();
        let later = put_dormant(&mut before, "fx-23", slot(P2, Row::Units, 2));
        let mut after = before.clone();
        let rollback = roll_back(RollBackArgs {
            turns_ago: 0,
            sides: RollBackSides::Both,
        });
        let events = run_effect(&mut after, rollback, P1);
        let back = find_instance(&after, &held).expect("back on the field");
        assert_eq!(back.def_id, "fx-21", "as the snapshot holds it");
        assert_eq!(back.slumber, Some(Slumber { turns: Some(2) }), "still Dormant");
        let returned = find_instance(&after, &later).expect("in a hand");
        assert_eq!(
            returned.zone,
            Zone::Hand { player: P2 },
            "not in the snapshot: to its hand (R419)"
        );
        assert_eq!(returned.slumber, None, "its status gone with R78's reset");
        assert_eq!(i19(&before, &events, &after), Vec::<String>::new());
        // Without its `rolledBack` the same step reaches both.
        let unmarked: Vec<GameEvent> = events
            .iter()
            .filter(|event| !matches!(event, GameEvent::RolledBack { .. }))
            .cloned()
            .collect();
        let found = i19(&before, &unmarked, &after);
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(
            found
                .iter()
                .any(|finding| finding.contains(&later) && finding.contains("bounced"))
        );
        assert!(
            found
                .iter()
                .any(|finding| finding.contains(&held) && finding.contains("defId"))
        );
        // Bogos Binted's Radiant face rebuilds its side Radiant (R3550007).
        let restored: GameEvent = json_as(json!({
            "type": "rolledBack",
            "player": "p2",
            "turnsAgo": 1,
            "sides": ["p2"],
        }));
        let card = find_instance(&before, &held).expect("the held card").clone();
        let mut radiant = before.clone();
        find_instance_mut(&mut radiant, &held)
            .expect("the held card")
            .radiant = true;
        let rebuilt = [restored, radiant_set(&card)];
        assert_eq!(i19(&before, &rebuilt, &radiant), Vec::<String>::new());
        only(
            &i19(&before, &[radiant_set(&card)], &radiant),
            &held,
            &["radiantSet"],
        );
    }

    /// R4524001: after a restore, a card that stood Dormant on a restored side may be reached for the rest
    /// of the step, since the snapshot may hold it awake (R4454002): the fuzz's handicapped seed 1573 shot
    /// one awake with the same activation's next damage. One reached before the restore, or on the side
    /// no restore rebuilt, is still a finding.
    #[test]
    fn r4524001_after_a_restore_its_sides_dormant_cards_may_be_reached_for_the_rest_of_the_step() {
        let (before, id) = dormant_board("r4524001");
        let restored: GameEvent = json_as(json!({
            "type": "rolledBack", "player": "p2", "turnsAgo": 3, "sides": ["p2"],
        }));
        let shot: GameEvent = json_as(json!({
            "type": "damage", "sourceId": "c1", "targetId": id, "amount": 3, "combat": false,
        }));
        assert_eq!(
            i19(&before, &[restored.clone(), shot.clone()], &before),
            Vec::<String>::new()
        );
        only(
            &i19(&before, &[shot.clone(), restored], &before),
            &id,
            &["I19 reached Dormant", "damage"],
        );
        let elsewhere: GameEvent = json_as(json!({
            "type": "rolledBack", "player": "p1", "turnsAgo": 3, "sides": ["p1"],
        }));
        only(&i19(&before, &[elsewhere, shot], &before), &id, &["damage"]);
    }

    /// R4524002: a Unit that runs a copied board sweep's text (Stinkyscape #20 Tnack Trav, R1850000)
    /// sweeps as the card it copies, Dormant cards included (R3539002), though the sweep exiles the
    /// copier with its copy (R78), so I19 reads each field card's copy as the step begins. A copy of no
    /// sweep, or none, is no sweep. (Read alone: the copied card is no fixture, and the other invariants
    /// would read its text.)
    #[test]
    fn r4524002_a_unit_running_a_copied_board_sweep_is_read_as_a_sweep_as_the_step_begins() {
        let (mut state, _) = dormant_board("r4524002");
        let copier = put(&mut state, "fx-22", slot(P2, Row::Units, 2), json!({})).id;
        let copying = |state: &mut GameState, copy: Option<&str>| -> bool {
            let card = find_instance_mut(state, &copier).expect("the copier");
            card.memory.clear();
            if let Some(def_id) = copy {
                card.memory.insert(
                    jackioh_engine::subsystems::unit_text::UNIT_TEXT_KEY.to_string(),
                    json!({ "defId": def_id, "radiant": false }),
                );
            }
            let card = find_instance(state, &copier).expect("the copier").clone();
            runs_a_copied_sweep(state, &card)
        };
        assert!(!copying(&mut state, None), "no copy");
        assert!(
            copying(&mut state, Some("stinkyscapeplus-081")),
            "S+ #81 Doomsayer's sweep"
        );
        assert!(!copying(&mut state, Some("fx-21")), "a copy of no sweep");
    }

    #[test]
    fn r4454003_a_fuse_passes_a_dormant_ingredient_or_target_by() {
        let (before, dormant) = dormant_board("r4454003");
        let mut state = before.clone();
        let awake = put(&mut state, "fx-22", slot(P2, Row::Units, 2), json!({})).id;
        let played = put(&mut state, "fx-1", slot(P1, Row::Units, 1), json!({})).id;
        // Onto the Dormant card alone: nothing happens, as onto an Immutable one (R23, R61).
        let mut onto = state.clone();
        let only_dormant = fuse_cards(json_as(json!({
            "instanceIds": [played],
            "targetInstanceIds": [dormant],
            "pick": "random",
        })));
        let events = run_effect(&mut onto, only_dormant, P2);
        assert!(events.is_empty(), "{events:?}");
        assert_eq!(onto, state, "nothing changed");
        // Beside an awake match, the random pick draws only the awake one.
        for seed in 0..8 {
            let mut picked = state.clone();
            picked.seed = format!("r4454003-{seed}");
            let either = fuse_cards(json_as(json!({
                "instanceIds": [played],
                "targetInstanceIds": [dormant, awake],
                "pick": "random",
            })));
            let events = run_effect(&mut picked, either, P2);
            let dormant_card = find_instance(&picked, &dormant).expect("still on the field");
            assert_eq!(dormant_card.def_id, "fx-21", "seed {seed}");
            assert_ne!(
                find_instance(&picked, &awake).expect("fused").def_id,
                "fx-22",
                "seed {seed}"
            );
            assert_eq!(i19(&state, &events, &picked), Vec::<String>::new(), "seed {seed}");
        }
        // As an ingredient it drops out, leaving too few to fuse.
        let mut into = state.clone();
        let from_dormant = fuse_cards(json_as(json!({
            "instanceIds": [dormant],
            "targetInstanceIds": [awake],
        })));
        assert!(run_effect(&mut into, from_dormant, P2).is_empty());
        assert_eq!(into, state, "nothing changed");
    }

    #[test]
    fn r4454004_a_dormant_card_s_brittle_count_waits_until_it_awakens() {
        let brittle = BrittleCounter {
            count: 1,
            since: -10,
            printed: None,
        };
        let mut state = playing("r4454004");
        let id = put(&mut state, "fx-21", slot(P2, Row::Units, 1), json!({})).id;
        find_instance_mut(&mut state, &id).expect("the card").brittle = Some(brittle);
        let mut dormant = state.clone();
        find_instance_mut(&mut dormant, &id).expect("the card").slumber = Some(Slumber { turns: Some(3) });
        // p2's turn start ticks no count of a Dormant card's, and crumbles nothing.
        let ended = reduce(
            &dormant,
            &Action::new(ActionBody::EndTurn, P1, "r4454004-dormant"),
        );
        assert!(ended.error.is_none(), "{:?}", ended.error);
        let card = find_instance(&ended.state, &id).expect("still on the field");
        assert_eq!(
            card.brittle.map(|brittle| brittle.count),
            Some(1),
            "its count waits"
        );
        assert_eq!(
            card.slumber,
            Some(Slumber { turns: Some(2) }),
            "its countdown counts"
        );
        assert_eq!(i19(&dormant, &ended.events, &ended.state), Vec::<String>::new());
        // Awake, the same count ticks to 0 and crumbles it (R385).
        let ended = reduce(&state, &Action::new(ActionBody::EndTurn, P1, "r4454004-awake"));
        assert!(ended.error.is_none(), "{:?}", ended.error);
        assert!(
            ended
                .events
                .iter()
                .any(|event| matches!(event, GameEvent::Crumbled { instance_id, .. } if *instance_id == id)),
            "{:?}",
            ended.events
        );
        assert!(ended.state.players[P2].graveyard.iter().any(|card| card.id == id));

        // A Burn waits on a Dormant card through the turn's end, and is dealt and removed once awake.
        let mut burning = playing("r4454004-burn");
        let id = put(&mut burning, "fx-21", slot(P2, Row::Units, 1), json!({})).id;
        find_instance_mut(&mut burning, &id)
            .expect("the card")
            .counters
            .burn = Some(1);
        let mut dormant = burning.clone();
        find_instance_mut(&mut dormant, &id).expect("the card").slumber = Some(Slumber { turns: Some(3) });
        let ended = reduce(
            &dormant,
            &Action::new(ActionBody::EndTurn, P1, "r4454004-burn-dormant"),
        );
        assert!(ended.error.is_none(), "{:?}", ended.error);
        let card = find_instance(&ended.state, &id).expect("still on the field");
        assert_eq!((card.counters.burn, card.damage), (Some(1), 0), "its Burn waits");
        assert_eq!(i19(&dormant, &ended.events, &ended.state), Vec::<String>::new());
        let ended = reduce(
            &burning,
            &Action::new(ActionBody::EndTurn, P1, "r4454004-burn-awake"),
        );
        assert!(ended.error.is_none(), "{:?}", ended.error);
        let card = find_instance(&ended.state, &id).expect("still on the field");
        assert_eq!(
            (card.counters.burn, card.damage),
            (None, 1),
            "awake, it burns (R4154000)"
        );
    }

    #[test]
    fn r4454005_the_switch_out_of_defense_passes_a_dormant_card_by_until_it_awakens() {
        let mut state = playing("r4454005");
        let id = put(&mut state, "fx-21", slot(P2, Row::Units, 1), json!({})).id;
        let card = find_instance_mut(&mut state, &id).expect("the card");
        card.position = Some(Position::Def);
        card.granted_keywords = vec![json_as(json!({ "kind": "Indestructible" }))];
        let mut dormant = state.clone();
        find_instance_mut(&mut dormant, &id).expect("the card").slumber = Some(Slumber { turns: Some(2) });
        let check = || Effect::new("r4454005-check", |_ctx| {});
        // The state check leaves the Dormant card in Defense Position, and I17 holds it lawful there.
        let mut checked = dormant.clone();
        let events = settled(&mut checked, check());
        assert_eq!(
            find_instance(&checked, &id).expect("on the field").position,
            Some(Position::Def)
        );
        assert_eq!(i19(&dormant, &events, &checked), Vec::<String>::new());
        assert_eq!(i17(&checked, &events), Vec::<String>::new());
        // Awake, an Indestructible Unit in Defense Position is switched to Attack Position (R2179000).
        let mut checked = state.clone();
        let events = settled(&mut checked, check());
        assert_eq!(
            find_instance(&checked, &id).expect("on the field").position,
            Some(Position::Atk)
        );
        assert!(events.iter().any(
            |event| matches!(event, GameEvent::PositionSwitched { instance_id, .. } if *instance_id == id)
        ));
    }
}

mod i20 {
    use super::*;
    use jackioh_engine::effects;
    use jackioh_engine::zones::{MoveToZoneOptions, OffFieldZone, move_to_zone};

    use crate::rules::fixtures::harness::run_effect;

    fn i20(start: &GameState, events: &[GameEvent], state: &GameState) -> Vec<String> {
        invariant("I20", create_invariant_monitor(start).after(events, state))
    }

    /// `state` with `id`'s stored damage set to `damage`.
    fn with_damage(state: &GameState, id: &str, damage: i32) -> GameState {
        let mut out = state.clone();
        find_instance_mut(&mut out, id).expect("the unit").damage = damage;
        out
    }

    fn healed(id: &str, amount: i32) -> GameEvent {
        json_as(json!({ "type": "healed", "targetId": id, "amount": amount }))
    }

    fn summoned(id: &str, via: Value) -> GameEvent {
        json_as(json!({
            "type": "summoned", "player": "p1", "instanceId": id, "defId": "fx-1",
            "row": "units", "lane": 1, "via": via,
        }))
    }

    #[test]
    fn i20_a_unit_s_damage_and_heal_events_with_their_moves_are_clean() {
        let (start, id) = hurt("i20-clean", 2);
        let mut monitor = create_invariant_monitor(&start);
        let hit = with_damage(&start, &id, 3);
        assert!(invariant("I20", monitor.after(&[damage_event(&id, 1)], &hit)).is_empty());
        let mended = with_damage(&start, &id, 1);
        assert!(invariant("I20", monitor.after(&[healed(&id, 2)], &mended)).is_empty());
    }

    #[test]
    fn i20_stored_damage_that_moves_with_no_event_is_flagged_once() {
        let (start, id) = hurt("i20-silent", 0);
        let bumped = with_damage(&start, &id, 1);
        let mut monitor = create_invariant_monitor(&start);
        only(
            &invariant("I20", monitor.after(&[], &bumped)),
            &id,
            &["I20 silent card damage", "§4.4", "R78"],
        );
        // Reported once: the tally then follows the state.
        assert!(invariant("I20", monitor.after(&[], &bumped)).is_empty());
        // An event with no move is as silent.
        only(
            &i20(&start, &[damage_event(&id, 1)], &start),
            &id,
            &["I20 silent card damage"],
        );
    }

    #[test]
    fn i20_an_overheal_that_emits_no_healed_and_changes_nothing_is_clean() {
        let (mut state, id) = hurt("i20-overheal", 1);
        let start = state.clone();
        assert!(i20(&start, &[], &start).is_empty());
        let events = run_effect(
            &mut state,
            effects::heal(json_as(json!({
                "target": { "of": "instance", "instanceId": id },
                "amount": 20,
            }))),
            P1,
        );
        assert_eq!(find_instance(&state, &id).expect("the unit").damage, 0);
        assert!(i20(&start, &events, &state).is_empty());
        // Healed again at 0 damage: no `healed`, no move.
        let again = state.clone();
        let events = run_effect(
            &mut state,
            effects::heal(json_as(json!({
                "target": { "of": "instance", "instanceId": id },
                "amount": 20,
            }))),
            P1,
        );
        assert!(events.is_empty(), "{events:?}");
        assert!(i20(&again, &events, &state).is_empty());
    }

    #[test]
    fn i20_a_fresh_summon_anchors_at_zero() {
        let start = playing("i20-summon");
        let mut state = start.clone();
        let id = put(&mut state, "fx-1", slot(P1, Row::Units, 1), json!({})).id;
        let arrival = summoned(&id, Value::Null);
        assert!(i20(&start, std::slice::from_ref(&arrival), &state).is_empty());
        let hurt_on_arrival = with_damage(&state, &id, 2);
        only(
            &i20(&start, std::slice::from_ref(&arrival), &hurt_on_arrival),
            &id,
            &["I20 silent card damage"],
        );
        let hit = with_damage(&state, &id, 1);
        assert!(i20(&start, &[arrival, damage_event(&id, 1)], &hit).is_empty());
    }

    #[test]
    fn i20_vanilla_s_one_health_write_passes_through_the_transformed_re_anchor() {
        let (mut state, id) = hurt("i20-vanilla", 3);
        find_instance_mut(&mut state, &id).expect("the unit").buffs.health = 2;
        let start = state.clone();
        let events = run_effect(
            &mut state,
            effects::vanilla(json_as(json!({ "instanceId": id }))),
            P1,
        );
        assert_eq!(find_instance(&state, &id).expect("the unit").damage, 1);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, GameEvent::Transformed { .. }))
        );
        assert!(i20(&start, &events, &state).is_empty());
        only(&i20(&start, &[], &state), &id, &["I20 silent card damage"]);
    }

    #[test]
    fn i20_a_card_that_leaves_the_field_drops_its_entry() {
        let (start, id) = hurt("i20-leaves", 2);
        let mut gone = start.clone();
        let mut card = find_instance(&gone, &id).expect("the unit").clone();
        move_to_zone(
            &mut gone,
            &mut card,
            OffFieldZone::Hand,
            MoveToZoneOptions::default(),
        );
        assert!(find_instance(&gone, &id).is_some_and(|c| c.zone.z() != ZoneName::Field));
        let mut monitor = create_invariant_monitor(&start);
        assert!(invariant("I20", monitor.after(&[], &gone)).is_empty());
        // Back on the field at 0 with no event: the old tally of 2 did not survive.
        assert!(
            invariant(
                "I20",
                monitor.after(&[], &{
                    let mut back = start.clone();
                    find_instance_mut(&mut back, &id).expect("the unit").damage = 0;
                    back
                })
            )
            .is_empty()
        );
    }

    #[test]
    fn i20_set_stats_a_reborn_return_a_flicker_a_rollback_and_a_glitch_reset_re_anchor() {
        let (start, id) = hurt("i20-anchors", 2);
        let cleared = with_damage(&start, &id, 0);
        let number: GameEvent = json_as(json!({
            "type": "numberChanged", "instanceId": id, "defId": "fx-1", "key": "health", "value": 5,
        }));
        let rolled: GameEvent = json_as(json!({
            "type": "rolledBack", "player": "p1", "turnsAgo": 1, "sides": ["p1"],
        }));
        let flicker: GameEvent = json_as(json!({
            "type": "flickered", "player": "p1", "instanceId": id, "defId": "fx-1",
            "row": "units", "lane": 1,
        }));
        for (name, events) in [
            ("numberChanged", vec![number]),
            ("rolledBack", vec![rolled]),
            ("flickered", vec![flicker]),
        ] {
            assert!(i20(&start, &events, &cleared).is_empty(), "{name}");
            assert_eq!(i20(&start, &[], &cleared).len(), 1, "{name} without its event");
        }
        // Reborn returns a card with its maximum health less 1 as damage, announced by its `summoned`.
        let (fresh, id) = hurt("i20-reborn", 0);
        let reborn = with_damage(&fresh, &id, 1);
        assert!(i20(&fresh, &[summoned(&id, json!("Reborn"))], &reborn).is_empty());
        assert_eq!(i20(&fresh, &[summoned(&id, Value::Null)], &reborn).len(), 1);
        // A Glitch's reset (R676) starts every tally again.
        let mut reset = cleared.clone();
        reset.resets = Some(start.resets.unwrap_or(0) + 1);
        assert!(i20(&start, &[], &reset).is_empty());
    }

    #[test]
    fn i20_a_planted_stored_damage_bump_is_caught_on_a_real_step() {
        let (start, id) = hurt("i20-plant", 1);
        let result = reduce(&start, &Action::new(ActionBody::EndTurn, P1, "i20-end"));
        assert!(result.error.is_none(), "{:?}", result.error);
        assert!(i20(&start, &result.events, &result.state).is_empty());
        let mut planted = result.state.clone();
        find_instance_mut(&mut planted, &id).expect("the unit").damage += 1;
        only(
            &i20(&start, &result.events, &planted),
            &id,
            &["I20 silent card damage", "§4.4"],
        );
    }
}

// I21 (#4508): no Spell changes a Unit Immune to Spells on the field (§6.1, R4453000), proved on
// synthetic events over a stand-in immune Unit, real Spells and Units, and the engine's board verbs.
mod i21 {
    use super::*;
    use jackioh_engine::effects::{make_all_radiant, swap_board};
    use jackioh_engine::geometry::BlackHole;
    use jackioh_engine::subsystems::board_history::{
        RollBackArgs, RollBackSides, record_board_snapshot, roll_back,
    };

    use jackioh_engine::zones::{MoveToZoneOptions, OffFieldZone, move_to_zone};

    use crate::rules::fixtures::card_on_card::{register_card_on_card, tag_exiler};
    use crate::rules::fixtures::freeze::{blizzard, frost, goad, register_freeze_fixtures, warded};
    use crate::rules::fixtures::harness::run_effect;

    const IMMUNE: &str = "stand-in (1) 4/4 Immune to Spells";
    const PLAIN: &str = "stand-in (1) 4/4";
    const SPELL: &str = "stand-in (0) Spell";
    const FIELD_SPELL: &str = "stand-in (0) Field Spell";

    /// I21 of one step from `before` to `after`, read as the fuzz reads it.
    fn i21(before: &GameState, events: &[GameEvent], after: &GameState) -> Vec<String> {
        let mut monitor = create_invariant_monitor(before);
        monitor.before(before, before.active, &ActionBody::EndTurn);
        invariant("I21", monitor.after(events, after))
    }

    /// I21 of the step `act` takes on `s`.
    fn i21_of(s: &mut Scenario, act: impl FnOnce(&mut Scenario)) -> Vec<String> {
        let before = s.state().clone();
        act(s);
        i21(&before, s.last_events(), s.state())
    }

    /// p1's first main phase with an immune stand-in in p2's unit zone 1, and its id.
    fn immune_board(seed: &str) -> (GameState, String) {
        let mut state = playing(seed);
        let id = put(&mut state, IMMUNE, slot(P2, Row::Units, 1), json!({})).id;
        (state, id)
    }

    /// `state` with `id` moved off the field to `to`.
    fn moved(state: &GameState, id: &str, to: OffFieldZone) -> GameState {
        let mut moved = state.clone();
        let mut card = find_instance(&moved, id).expect("the Unit").clone();
        move_to_zone(&mut moved, &mut card, to, MoveToZoneOptions::default());
        assert!(find_instance(&moved, id).is_some_and(|card| card.zone.z() != ZoneName::Field));
        moved
    }

    /// The marker of `def_id`'s hook beginning to resolve (R2783000).
    fn marker(def_id: &str) -> GameEvent {
        marker_of("i21-source", def_id)
    }

    /// The marker of the card `source_id`'s hook, running `def_id`'s text.
    fn marker_of(source_id: &str, def_id: &str) -> GameEvent {
        json_as(json!({
            "type": "hookResolving",
            "sourceId": source_id,
            "defId": def_id,
            "controller": "p1",
            "hook": "cry",
        }))
    }

    /// `effect` applied as p1's with no card of its own and settled, as a card's effect is (§10.3).
    fn settled(state: &mut GameState, effect: Effect) -> Vec<GameEvent> {
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
        {
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            {
                let options = HookOptions {
                    controller: Some(P1),
                    ..Default::default()
                };
                let mut ctx = make_context(&mut sink, None, options);
                (effect.apply)(&mut ctx);
            }
            settle(&mut sink, SettleOptions::default());
        }
        state.rng_cursor = rng.cursor();
        events
    }

    fn buffed(id: &str) -> GameEvent {
        json_as(json!({ "type": "buffed", "instanceId": id, "attack": 1, "health": 1 }))
    }

    fn trap_fired() -> GameEvent {
        json_as(json!({
            "type": "trapFired",
            "instanceId": "i21-trap",
            "defId": "stand-in (0) Trap",
            "controller": "p2",
            "row": "backrow",
            "lane": 1,
        }))
    }

    fn control_changed(id: &str) -> GameEvent {
        json_as(json!({
            "type": "controlChanged",
            "instanceId": id,
            "controller": "p1",
            "row": "units",
            "lane": 1,
        }))
    }

    #[test]
    fn i21_a_spell_s_change_to_an_immune_field_unit_is_one_finding_per_unit() {
        let (state, id) = immune_board("i21-spell");
        // A buff and a hit in one Spell's run: one finding, naming the Unit, the event and the Spell.
        let hit = [marker(SPELL), buffed(&id), damage_event(&id, 1)];
        only(
            &i21(&state, &hit, &state),
            &id,
            &[
                "I21 Spell reached Immune to Spells",
                "buffed",
                SPELL,
                "§6.1",
                "R4453000",
            ],
        );
        // Each other kind of change, alone.
        let changes: [(&str, GameEvent); 3] = [
            (
                "keywordGranted",
                json_as(
                    json!({ "type": "keywordGranted", "instanceId": id, "keyword": { "kind": "Taunt" } }),
                ),
            ),
            (
                "counterChanged",
                json_as(
                    json!({ "type": "counterChanged", "instanceId": id, "counter": "plague", "value": 1 }),
                ),
            ),
            (
                "transformed",
                json_as(json!({
                    "type": "transformed",
                    "instanceId": id,
                    "fromDefId": IMMUNE,
                    "toDefId": PLAIN,
                    "newInstanceId": "i21-new",
                })),
            ),
        ];
        for (kind, event) in changes {
            only(
                &i21(&state, &[marker(SPELL), event], &state),
                &id,
                &["I21 Spell reached Immune to Spells", kind],
            );
        }
        // Two immune Units reached are two findings; a plain Unit reached is none.
        let mut two = state.clone();
        let other = put(&mut two, IMMUNE, slot(P1, Row::Units, 2), json!({})).id;
        let plain_id = put(&mut two, PLAIN, slot(P2, Row::Units, 3), json!({})).id;
        let found = i21(
            &two,
            &[marker(SPELL), buffed(&id), buffed(&other), buffed(&plain_id)],
            &two,
        );
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found[0].contains(&id) && found[1].contains(&other), "{found:?}");
        // An `after` with no `before` judges nothing.
        let mut unprimed = create_invariant_monitor(&state);
        assert_eq!(
            invariant("I21", unprimed.after(&hit, &state)),
            Vec::<String>::new()
        );
    }

    #[test]
    fn i21_a_unit_s_cry_a_field_spell_s_and_a_trap_s_change_are_clean() {
        let (state, id) = immune_board("i21-clean");
        assert_eq!(
            i21(&state, &[buffed(&id)], &state),
            Vec::<String>::new(),
            "no card running"
        );
        for runner in [PLAIN, FIELD_SPELL] {
            assert_eq!(
                i21(&state, &[marker(runner), buffed(&id)], &state),
                Vec::<String>::new(),
                "{runner}"
            );
        }
        // A trap fired inside a Spell's run runs its own effects.
        let trapped = [marker(SPELL), trap_fired(), buffed(&id)];
        assert_eq!(i21(&state, &trapped, &state), Vec::<String>::new());
        // A turn edge ends the Spell's run.
        let ended: GameEvent =
            json_as(json!({ "type": "turnEnded", "player": "p1", "turn": state.turn, "unspentMana": 0 }));
        assert_eq!(
            i21(&state, &[marker(SPELL), ended, buffed(&id)], &state),
            Vec::<String>::new()
        );
        // A Unit's marker after the Spell's ends its run, and a Spell's after a Unit's begins one.
        let after_unit = [marker(SPELL), marker(PLAIN), buffed(&id)];
        assert_eq!(i21(&state, &after_unit, &state), Vec::<String>::new());
        let after_spell = [marker(PLAIN), marker(SPELL), buffed(&id)];
        only(&i21(&state, &after_spell, &state), &id, &["I21", SPELL]);
    }

    /// freeze.rs's board: `active`'s turn with the given hands and fields over the freeze fixtures.
    fn freeze_board(p1_hand: Value, p1_field: Value, p2_field: Value) -> Scenario {
        register_freeze_fixtures();
        let library = json!([plain.id, plain.id, plain.id, plain.id]);
        scenario(json!({
            "seed": "i21-freeze",
            "active": "p1",
            "p1": { "hand": p1_hand, "field": p1_field, "library": library },
            "p2": { "hand": [plain.id], "field": p2_field, "library": library },
        }))
    }

    #[test]
    fn i21_a_real_spell_passes_an_immune_unit_by_and_a_unit_s_cry_reaches_it() {
        let mut s = freeze_board(
            json!([blizzard.id, frost.id]),
            json!([]),
            json!([{ "def": plain.id, "lane": 4 }, { "def": warded.id, "lane": 5 }]),
        );
        let (exposed, immune) = (
            s.unit(P2, 4).expect("the plain Unit").id,
            s.unit(P2, 5).expect("the warded Unit").id,
        );
        let found = i21_of(&mut s, |s| {
            s.play(blizzard.id.as_str(), json!({}));
        });
        assert_eq!(found, Vec::<String>::new());
        assert!(
            s.card(exposed.as_str()).frozen.is_some(),
            "the Spell froze the plain Unit"
        );
        assert!(
            s.card(immune.as_str()).frozen.is_none(),
            "and passed the immune one by"
        );
        let found = i21_of(&mut s, |s| {
            s.play(frost.id.as_str(), json!({ "zone": 1 }));
        });
        assert_eq!(found, Vec::<String>::new());
        assert!(
            s.card(immune.as_str()).frozen.is_some(),
            "a Unit's Cry reaches it"
        );
    }

    #[test]
    fn i21_a_spell_s_buff_on_an_immune_card_in_a_hand_is_clean() {
        let mut state = playing("i21-hand");
        let held = new_instance(&mut state, IMMUNE, P2, Zone::Hand { player: P2 });
        let id = held.id.clone();
        state.players[P2].hand.push(held);
        assert_eq!(
            i21(&state, &[marker(SPELL), buffed(&id)], &state),
            Vec::<String>::new()
        );
    }

    #[test]
    fn i21_a_unit_whose_immunity_the_step_took_is_not_judged() {
        let mut before = playing("i21-taken");
        let id = put(&mut before, PLAIN, slot(P2, Row::Units, 1), json!({})).id;
        find_instance_mut(&mut before, &id)
            .expect("the Unit")
            .granted_keywords = vec![Keyword::ImmuneToSpells];
        let mut after = before.clone();
        find_instance_mut(&mut after, &id)
            .expect("the Unit")
            .granted_keywords = Vec::new();
        let hit = [marker(SPELL), buffed(&id)];
        assert_eq!(i21(&before, &hit, &after), Vec::<String>::new());
        // With it kept, the buff is a reach.
        only(&i21(&before, &hit, &before), &id, &["I21", "buffed"]);
        // Immune only as the step ends is not judged either: it was not immune as it began.
        assert_eq!(i21(&after, &hit, &before), Vec::<String>::new());
    }

    #[test]
    fn i21_a_unit_gone_from_the_field_is_judged_by_its_own_immunity() {
        let (before, id) = immune_board("i21-gone");
        let after = moved(&before, &id, OffFieldZone::Graveyard);
        let destroyed: GameEvent = json_as(json!({
            "type": "destroyed",
            "instanceId": id,
            "defId": IMMUNE,
            "owner": "p2",
            "controller": "p2",
            "attack": 4,
            "maxHealth": 4,
        }));
        only(
            &i21(&before, &[marker(SPELL), destroyed], &after),
            &id,
            &["I21", "destroyed"],
        );
    }

    #[test]
    fn i21_a_card_played_inside_a_spell_s_run_runs_its_own_play() {
        let (state, id) = immune_board("i21-played");
        let played: GameEvent = json_as(json!({
            "type": "cardPlayed",
            "player": "p1",
            "instanceId": "i21-played",
            "defId": PLAIN,
            "costPaid": 1,
        }));
        let resolved: GameEvent = json_as(json!({
            "type": "cardResolved",
            "player": "p1",
            "instanceId": "i21-played",
            "defId": PLAIN,
            "permanent": true,
            "costPaid": 1,
        }));
        // Inside the nested play, before its own marker: its arrival and unmarked hooks are no Spell's.
        let nested = [marker(SPELL), played.clone(), buffed(&id)];
        assert_eq!(i21(&state, &nested, &state), Vec::<String>::new());
        // Once it resolves, the Spell's run goes on.
        let resumed = [marker(SPELL), played.clone(), resolved.clone(), buffed(&id)];
        only(&i21(&state, &resumed, &state), &id, &["I21", "buffed", SPELL]);
        // A Unit's run that a cast Spell interrupted goes on as the Unit's (a Cast on draw inside a
        // start-of-turn hook): the Spell's own run is judged, the Unit's after it is not.
        let cast: GameEvent = json_as(json!({
            "type": "cardPlayed",
            "player": "p1",
            "instanceId": "i21-cast",
            "defId": SPELL,
            "costPaid": 0,
        }));
        let cast_resolved: GameEvent = json_as(json!({
            "type": "cardResolved",
            "player": "p1",
            "instanceId": "i21-cast",
            "defId": SPELL,
            "permanent": false,
            "costPaid": 0,
        }));
        let inside_unit = [
            marker(PLAIN),
            cast.clone(),
            marker_of("i21-cast", SPELL),
            cast_resolved.clone(),
            damage_event(&id, 1),
        ];
        assert_eq!(i21(&state, &inside_unit, &state), Vec::<String>::new());
        let reached = [
            marker(PLAIN),
            cast,
            marker_of("i21-cast", SPELL),
            buffed(&id),
            cast_resolved,
        ];
        only(&i21(&state, &reached, &state), &id, &["I21", "buffed", SPELL]);
    }

    #[test]
    fn i21_a_spell_set_face_down_runs_as_a_trap_as_it_is_set_and_as_it_reveals() {
        let (before, id) = immune_board("i21-set");
        let mut set = before.clone();
        let card = put(&mut set, SPELL, slot(P1, Row::Backrow, 1), json!({}));
        find_instance_mut(&mut set, &card.id)
            .expect("the set Spell")
            .set_as = Some(SetAs {
            reveal: RevealAt::Cloaked,
            set_turn: set.turn,
            echo: None,
            revealing: Some(true),
        });
        // Set as the step began, and revealing in it (M #99's row, R3806000).
        let reveal = [marker_of(&card.id, SPELL), buffed(&id)];
        assert_eq!(i21(&set, &reveal, &set), Vec::<String>::new());
        // Set only as the step ends: resolved as it was set, under Paranoia.
        assert_eq!(i21(&before, &reveal, &set), Vec::<String>::new());
        // Fired as a trap in the step, and gone to its graveyard by its end.
        let fired: GameEvent = json_as(json!({
            "type": "trapFired",
            "instanceId": card.id,
            "defId": SPELL,
            "controller": "p1",
            "row": "backrow",
            "lane": 1,
        }));
        let revealed = [fired, marker_of(&card.id, SPELL), buffed(&id)];
        assert_eq!(i21(&before, &revealed, &before), Vec::<String>::new());
        // The same Spell face-up is a Spell.
        let mut face_up = set.clone();
        find_instance_mut(&mut face_up, &card.id)
            .expect("the Spell")
            .set_as = None;
        only(&i21(&face_up, &reveal, &face_up), &id, &["I21", "buffed"]);
    }

    #[test]
    fn i21_runs_on_i14_s_branches() {
        let (state, id) = immune_board("i21-i14");
        let hit = [marker(SPELL), buffed(&id)];
        let branch = |state: &GameState, action: &Action| -> ReduceResult {
            let mut result = reduce(state, action);
            result.events.extend(hit.iter().cloned());
            result
        };
        let found: Vec<String> = listed_refused(&state, P1, &[ActionBody::EndTurn], branch)
            .into_iter()
            .filter(|finding| finding.contains("I21"))
            .collect();
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("I14 branch:"), "{}", found[0]);
        assert!(
            found[0].contains("I21 Spell reached Immune to Spells"),
            "{}",
            found[0]
        );
        assert!(found[0].contains(&id), "{}", found[0]);
        // The plain reducer's branch reaches nothing.
        let clean: Vec<String> = listed_refused(&state, P1, &[ActionBody::EndTurn], reduce)
            .into_iter()
            .filter(|finding| finding.contains("I21"))
            .collect();
        assert_eq!(clean, Vec::<String>::new());
    }

    #[test]
    fn r4508001_a_board_swap_a_rotation_and_a_restore_carry_an_immune_unit_and_i21_allows_it() {
        let (before, id) = immune_board("r4508001");
        // A Spell's board swap carries it to p1's side, immune still.
        let mut swapped = before.clone();
        let mut events = vec![marker(SPELL)];
        events.extend(run_effect(&mut swapped, swap_board(), P1));
        let card = find_instance(&swapped, &id).expect("still on the field");
        assert_eq!(card.controller, P1);
        assert!(
            events.iter().any(
                |event| matches!(event, GameEvent::ControlChanged { instance_id, .. } if *instance_id == id)
            ),
            "{events:?}"
        );
        assert_eq!(i21(&before, &events, &swapped), Vec::<String>::new());
        // A rotation's carry and a restore's bounce, inside their windows.
        let rotated: GameEvent = json_as(json!({ "type": "rotated", "direction": "left" }));
        assert_eq!(
            i21(&before, &[marker(SPELL), rotated, control_changed(&id)], &swapped),
            Vec::<String>::new()
        );
        let bounced: GameEvent =
            json_as(json!({ "type": "bounced", "instanceId": id, "defId": IMMUNE, "owner": "p2" }));
        let rolled: GameEvent =
            json_as(json!({ "type": "rolledBack", "player": "p1", "turnsAgo": 0, "sides": ["p2"] }));
        let back = moved(&before, &id, OffFieldZone::Hand);
        assert_eq!(
            i21(&before, &[marker(SPELL), rolled, bounced.clone()], &back),
            Vec::<String>::new()
        );
        // A real restore of the board as a snapshot held it, before the Unit arrived: to its hand.
        let mut snapshotted = playing("r4508001-restore");
        snapshotted.board_history = None;
        record_board_snapshot(&mut snapshotted);
        let mut restored_before = snapshotted.clone();
        let later = put(&mut restored_before, IMMUNE, slot(P2, Row::Units, 1), json!({})).id;
        let mut restored = restored_before.clone();
        let rollback = roll_back(RollBackArgs {
            turns_ago: 0,
            sides: RollBackSides::Both,
        });
        let mut events = vec![marker(SPELL)];
        events.extend(run_effect(&mut restored, rollback, P1));
        assert_eq!(
            find_instance(&restored, &later).expect("in a hand").zone,
            Zone::Hand { player: P2 }
        );
        assert_eq!(i21(&restored_before, &events, &restored), Vec::<String>::new());
        // A bare carry, and one after the Spell's next marker, are reaches.
        only(
            &i21(&before, &[marker(SPELL), control_changed(&id)], &swapped),
            &id,
            &["I21", "controlChanged"],
        );
        let rotated: GameEvent = json_as(json!({ "type": "rotated", "direction": "left" }));
        only(
            &i21(
                &before,
                &[marker(SPELL), rotated, marker(SPELL), control_changed(&id)],
                &swapped,
            ),
            &id,
            &["I21", "controlChanged"],
        );
        // A restore's window lets no buff through.
        let rolled: GameEvent =
            json_as(json!({ "type": "rolledBack", "player": "p1", "turnsAgo": 0, "sides": ["p2"] }));
        only(
            &i21(&before, &[marker(SPELL), rolled, buffed(&id)], &before),
            &id,
            &["I21", "buffed"],
        );
    }

    #[test]
    fn r4508002_a_spell_s_forced_attack_a_melee_and_the_ai_s_turn_are_attacks_and_i21_allows_them() {
        // A real Spell's forced attack: the immune Unit attacks the enemy hero (R53).
        let mut s = freeze_board(
            json!([goad.id]),
            json!([{ "def": warded.id, "lane": 1 }]),
            json!([]),
        );
        let attacker = s.unit(P1, 1).expect("the warded Unit").id;
        let health = s.state().players.p2.hero.health;
        let found = i21_of(&mut s, |s| {
            s.play(goad.id.as_str(), json!({}));
        });
        assert!(
            s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::AttackDeclared { attacker_id, forced: true, .. } if *attacker_id == attacker
            )),
            "{:?}",
            s.last_events()
        );
        assert!(
            s.state().players.p2.hero.health < health,
            "the forced attack happened"
        );
        assert_eq!(found, Vec::<String>::new());
        // As the target of a forced attack, struck and destroyed by the attacking Unit.
        let (state, id) = immune_board("r4508002");
        let declared: GameEvent = json_as(json!({
            "type": "attackDeclared",
            "attackerId": "i21-attacker",
            "targetId": id,
            "forced": true,
        }));
        let destroyed: GameEvent = json_as(json!({
            "type": "destroyed",
            "instanceId": id,
            "defId": IMMUNE,
            "owner": "p2",
            "controller": "p2",
            "attack": 4,
            "maxHealth": 4,
        }));
        let struck = [marker(SPELL), declared.clone(), damage_event(&id, 4), destroyed];
        assert_eq!(i21(&state, &struck, &state), Vec::<String>::new());
        // The AI's turn (M+ #18) plays its own actions, a switch among them.
        let engaged: GameEvent =
            json_as(json!({ "type": "autopilotEngaged", "player": "p1", "turn": state.turn }));
        let switched: GameEvent =
            json_as(json!({ "type": "positionSwitched", "instanceId": id, "position": "DEF" }));
        assert_eq!(
            i21(&state, &[marker(SPELL), engaged, switched], &state),
            Vec::<String>::new()
        );
        // The attack's window ends at the Spell's next marker: a buff there is a reach.
        only(
            &i21(
                &state,
                &[marker(SPELL), declared, marker(SPELL), buffed(&id)],
                &state,
            ),
            &id,
            &["I21", "buffed"],
        );
    }

    #[test]
    fn r4508005_the_state_check_s_switch_out_of_defense_and_an_affliction_s_tick_are_no_spell_s_and_i21_allows_them()
     {
        let (state, id) = immune_board("r4508005");
        // §4.5 step 1: a Spell's run brings in what bars Defense, and the state check switches the Unit out.
        let switched: GameEvent =
            json_as(json!({ "type": "positionSwitched", "instanceId": id, "position": "ATK" }));
        let mut barred = state.clone();
        find_instance_mut(&mut barred, &id)
            .expect("the immune Unit")
            .granted_keywords
            .push(Keyword::CantDefend);
        assert_eq!(
            i21(&state, &[marker(SPELL), switched.clone()], &barred),
            Vec::<String>::new()
        );
        // With nothing barring Defense, the switch is the Spell's.
        only(
            &i21(&state, &[marker(SPELL), switched], &state),
            &id,
            &["I21", "positionSwitched"],
        );
        // A Burn tick and the Burn's removal, and a Toxic tick, sourceless (R4154000, R3540001).
        let tick = |via: &str, absorbed: bool| -> GameEvent {
            if absorbed {
                json_as(json!({
                    "type": "damageAbsorbed",
                    "sourceId": null,
                    "targetId": id,
                    "absorbed": 1,
                    "combat": false,
                    "via": via,
                }))
            } else {
                json_as(json!({
                    "type": "damage",
                    "sourceId": null,
                    "targetId": id,
                    "amount": 1,
                    "combat": false,
                    "via": via,
                }))
            }
        };
        let cleared: GameEvent =
            json_as(json!({ "type": "counterChanged", "instanceId": id, "counter": "burn", "value": 0 }));
        for absorbed in [false, true] {
            let burn = [marker(SPELL), tick("Burn", absorbed), cleared.clone()];
            assert_eq!(i21(&state, &burn, &state), Vec::<String>::new(), "{absorbed}");
            let toxic = [marker(SPELL), tick("Toxic", absorbed)];
            assert_eq!(i21(&state, &toxic, &state), Vec::<String>::new(), "{absorbed}");
        }
        // A Burn cleared with no tick before it, and a hit with a source, are the Spell's.
        only(
            &i21(&state, &[marker(SPELL), cleared], &state),
            &id,
            &["I21", "counterChanged"],
        );
        let sourced: GameEvent = json_as(json!({
            "type": "damage",
            "sourceId": "i21-source",
            "targetId": id,
            "amount": 1,
            "combat": false,
            "via": "Burn",
        }));
        only(
            &i21(&state, &[marker(SPELL), sourced], &state),
            &id,
            &["I21", "damage"],
        );
    }

    #[test]
    fn r4508003_tears_of_the_kwzki_tree_s_world_makes_an_immune_unit_radiant_and_i21_allows_it() {
        let (before, id) = immune_board("r4508003");
        let mut after = before.clone();
        let mut events = vec![marker(SPELL)];
        events.extend(settled(&mut after, make_all_radiant()));
        assert!(find_instance(&after, &id).expect("on the field").radiant);
        assert!(
            events.iter().any(
                |event| matches!(event, GameEvent::RadiantSet { instance_id, .. } if *instance_id == id)
            ),
            "{events:?}"
        );
        assert_eq!(after.radiant_world, Some(true));
        assert_eq!(i21(&before, &events, &after), Vec::<String>::new());
        // Outside that world the same `radiantSet` is a reach.
        let mut no_world = after.clone();
        no_world.radiant_world = None;
        only(&i21(&before, &events, &no_world), &id, &["I21", "radiantSet"]);
    }

    #[test]
    fn r4599001_a_death_naming_a_killer_or_after_a_tick_is_its_hit_s_and_i21_allows_it() {
        let (state, id) = immune_board("r4599001");
        let dead = moved(&state, &id, OffFieldZone::Graveyard);
        let destroyed = |killer: Option<&str>| -> GameEvent {
            json_as(json!({
                "type": "destroyed",
                "instanceId": id,
                "defId": IMMUNE,
                "owner": "p2",
                "controller": "p2",
                "attack": 4,
                "maxHealth": 4,
                "killerId": killer,
            }))
        };
        // A combat death a trap's Spell cast came between: the lethal hit was the attacker's (R42).
        assert_eq!(
            i21(&state, &[marker(SPELL), destroyed(Some("i21-attacker"))], &dead),
            Vec::<String>::new()
        );
        // A Burn tick's death, in the Spell's run or with the tick before it (R4508005).
        let tick: GameEvent = json_as(json!({
            "type": "damage",
            "sourceId": null,
            "targetId": id,
            "amount": 1,
            "combat": false,
            "via": "Burn",
        }));
        let cleared: GameEvent =
            json_as(json!({ "type": "counterChanged", "instanceId": id, "counter": "burn", "value": 0 }));
        let in_run = [marker(SPELL), tick.clone(), cleared.clone(), destroyed(None)];
        assert_eq!(i21(&state, &in_run, &dead), Vec::<String>::new());
        let before_run = [tick, cleared, marker(SPELL), destroyed(None)];
        assert_eq!(i21(&state, &before_run, &dead), Vec::<String>::new());
        // A death with no killer and no tick is the Spell's destroy.
        only(
            &i21(&state, &[marker(SPELL), destroyed(None)], &dead),
            &id,
            &["I21", "destroyed"],
        );
    }

    #[test]
    fn r4599002_a_card_played_face_down_in_the_step_runs_as_a_trap_though_it_leaves_the_field() {
        let (state, id) = immune_board("r4599002");
        let gone = moved(&state, &id, OffFieldZone::Exile);
        let announced = |face_down: bool| -> GameEvent {
            let mut event = json!({
                "type": "cardAnnounced",
                "player": "p1",
                "instanceId": "i21-set",
                "defId": SPELL,
                "cardType": if face_down { "Trap" } else { "Spell" },
                "costPaid": 0,
                "targets": [],
                "row": "backrow",
                "lane": 4,
            });
            if face_down {
                event["faceDown"] = json!(true);
            }
            json_as(event)
        };
        let laid: GameEvent = json_as(json!({
            "type": "cardPlayed",
            "player": "p1",
            "instanceId": "i21-laid",
            "defId": SPELL,
            "costPaid": 0,
            "formerId": "i21-set",
        }));
        let exiled: GameEvent =
            json_as(json!({ "type": "exiled", "instanceId": id, "defId": IMMUNE, "owner": "p2" }));
        let run = |face_down: bool| {
            [
                announced(face_down),
                laid.clone(),
                marker_of("i21-laid", SPELL),
                exiled.clone(),
            ]
        };
        // M #99 Paranoia's set Spell resolves as it is set, a Trap (R3806000), and is gone after the step.
        assert_eq!(i21(&state, &run(true), &gone), Vec::<String>::new());
        // Cast face-up, the same run is a Spell's.
        only(&i21(&state, &run(false), &gone), &id, &["I21", "exiled"]);
    }

    #[test]
    fn r4599003_a_run_begun_before_the_step_ends_at_its_own_card_resolved() {
        let (state, id) = immune_board("r4599003");
        let resolved = |instance: &str| -> GameEvent {
            json_as(json!({
                "type": "cardResolved",
                "player": "p1",
                "instanceId": instance,
                "defId": SPELL,
                "permanent": false,
                "costPaid": 0,
            }))
        };
        // A cast on draw resolves, and another card's hit follows it.
        assert_eq!(
            i21(
                &state,
                &[
                    marker_of("i21-cast", SPELL),
                    resolved("i21-cast"),
                    damage_event(&id, 1)
                ],
                &state
            ),
            Vec::<String>::new()
        );
        // Another card's `cardResolved` leaves the Spell running.
        only(
            &i21(
                &state,
                &[
                    marker_of("i21-cast", SPELL),
                    resolved("i21-other"),
                    damage_event(&id, 1),
                ],
                &state,
            ),
            &id,
            &["I21", "damage"],
        );
        // The step answers the Spell's own prompt (a cast on draw's target, asked before its play): its
        // play nests in the run the answer continues, and its `cardResolved` ends both.
        let mut asked = state.clone();
        open_prompt(&mut asked, "r4599003-asked");
        let resume = &mut asked.pending.as_mut().expect("a prompt").resume;
        resume.def_id = SPELL.to_string();
        resume.instance_id = Some("i21-cast".to_string());
        let played: GameEvent = json_as(json!({
            "type": "cardPlayed",
            "player": "p1",
            "instanceId": "i21-cast",
            "defId": SPELL,
            "costPaid": 0,
        }));
        let answered = |last: &str| {
            vec![
                played.clone(),
                marker_of("i21-cast", SPELL),
                resolved(last),
                damage_event(&id, 1),
            ]
        };
        assert_eq!(i21(&asked, &answered("i21-cast"), &state), Vec::<String>::new());
        // Before its `cardResolved`, the hit is still the Spell's.
        let mut early = answered("i21-cast");
        early.swap(2, 3);
        only(&i21(&asked, &early, &state), &id, &["I21", "damage"]);
    }

    #[test]
    fn r4599004_an_exile_aura_s_and_an_open_black_hole_s_exile_are_no_spell_s_and_i21_allows_them() {
        let (state, id) = immune_board("r4599004");
        register_card_on_card();
        let mut aura = state.clone();
        put(&mut aura, &tag_exiler().id, slot(P1, Row::Units, 2), json!({}));
        let exiled: GameEvent =
            json_as(json!({ "type": "exiled", "instanceId": id, "defId": IMMUNE, "owner": "p2" }));
        let events = [marker(SPELL), exiled];
        // S #115 Scornboi's aura stands as the step begins, or arrives during it (R1850002).
        assert_eq!(
            i21(&aura, &events, &moved(&aura, &id, OffFieldZone::Exile)),
            Vec::<String>::new()
        );
        assert_eq!(
            i21(&state, &events, &moved(&aura, &id, OffFieldZone::Exile)),
            Vec::<String>::new()
        );
        // The aura's card arrives in the step and is gone again by its end.
        let arrived: GameEvent = json_as(json!({
            "type": "summoned",
            "player": "p1",
            "instanceId": "i21-aura",
            "defId": tag_exiler().id,
            "row": "units",
            "lane": 2,
        }));
        assert_eq!(
            i21(
                &state,
                &[marker(SPELL), arrived, events[1].clone()],
                &moved(&state, &id, OffFieldZone::Exile)
            ),
            Vec::<String>::new()
        );
        // S+ #77 Black Hole stands open (R3554000).
        let mut hole = state.clone();
        hole.black_hole = Some(BlackHole {
            lane: 3,
            swallowed: None,
            pulls: None,
            capacity: None,
            at_end: None,
        });
        assert_eq!(
            i21(&hole, &events, &moved(&hole, &id, OffFieldZone::Exile)),
            Vec::<String>::new()
        );
        // With no rule that exiles, the exile is the Spell's.
        only(
            &i21(&state, &events, &moved(&state, &id, OffFieldZone::Exile)),
            &id,
            &["I21", "exiled"],
        );
    }

    #[test]
    fn r4599007_a_burn_s_removal_after_its_tick_and_a_shield_its_tick_took_are_no_spell_s() {
        let (state, id) = immune_board("r4599007");
        let tick: GameEvent = json_as(json!({
            "type": "damage",
            "sourceId": null,
            "targetId": id,
            "amount": 1,
            "combat": false,
            "via": "Burn",
        }));
        let echo: GameEvent = json_as(json!({
            "type": "damage",
            "sourceId": null,
            "targetId": "i21-chained",
            "amount": 1,
            "combat": false,
            "via": "Burn",
        }));
        let cleared: GameEvent =
            json_as(json!({ "type": "counterChanged", "instanceId": id, "counter": "burn", "value": 0 }));
        let popped: GameEvent = json_as(json!({ "type": "divineShieldLost", "instanceId": id }));
        // The tick's echo onto a chained Unit comes between the tick and the Burn's removal (R3554007).
        assert_eq!(
            i21(&state, &[marker(SPELL), tick, echo, cleared.clone()], &state),
            Vec::<String>::new()
        );
        // The tick took its Divine Shield, and the Burn goes right after.
        assert_eq!(
            i21(&state, &[marker(SPELL), popped.clone(), cleared.clone()], &state),
            Vec::<String>::new()
        );
        // An Indestructible Unit's tick leaves no event (§4.4 step 4), and its Burn goes.
        let mut sturdy = state.clone();
        find_instance_mut(&mut sturdy, &id)
            .expect("the immune Unit")
            .granted_keywords
            .push(Keyword::Indestructible);
        assert_eq!(
            i21(&sturdy, &[marker(SPELL), cleared.clone()], &sturdy),
            Vec::<String>::new()
        );
        // A shield lost with no Burn going after it, and a Burn removed with no tick, are the Spell's.
        only(
            &i21(&state, &[marker(SPELL), popped], &state),
            &id,
            &["I21", "divineShieldLost"],
        );
        only(
            &i21(&state, &[marker(SPELL), cleared], &state),
            &id,
            &["I21", "counterChanged"],
        );
    }

    #[test]
    fn r4599008_a_death_whose_max_health_its_damage_met_is_a_layer_s_and_i21_allows_it() {
        let (mut state, id) = immune_board("r4599008");
        find_instance_mut(&mut state, &id)
            .expect("the immune Unit")
            .damage = 3;
        let dead = moved(&state, &id, OffFieldZone::Graveyard);
        let destroyed = |max_health: i32| -> GameEvent {
            json_as(json!({
                "type": "destroyed",
                "instanceId": id,
                "defId": IMMUNE,
                "owner": "p2",
                "controller": "p2",
                "attack": 4,
                "maxHealth": max_health,
                "killerId": null,
            }))
        };
        // An aura it stood by left (its max health down to the 3 damage it held), or one took all its
        // health (Core #46 Suppressive Aura's shape, below 0).
        for max_health in [3, 0, -1] {
            assert_eq!(
                i21(&state, &[marker(SPELL), destroyed(max_health)], &dead),
                Vec::<String>::new(),
                "{max_health}"
            );
        }
        // A heal in the step leaves too little damage to explain the death.
        let healed: GameEvent = json_as(json!({ "type": "healed", "targetId": id, "amount": 1 }));
        let found = i21(&state, &[healed, marker(SPELL), destroyed(3)], &dead);
        assert!(
            found.iter().any(|finding| finding.contains("destroyed")),
            "{found:?}"
        );
        // With health left over its damage, the death is the Spell's destroy.
        only(
            &i21(&state, &[marker(SPELL), destroyed(4)], &dead),
            &id,
            &["I21", "destroyed"],
        );
    }

    #[test]
    fn r4599010_after_a_restore_its_sides_immune_units_may_be_reached_for_the_rest_of_the_step() {
        let (state, id) = immune_board("r4599010");
        let restored = |side: &str| -> GameEvent {
            json_as(json!({ "type": "rolledBack", "player": "p1", "turnsAgo": 1, "sides": [side] }))
        };
        // C+ #35 Rollback restores p2's side, and a later Spell reaches what stands there.
        assert_eq!(
            i21(
                &state,
                &[marker(SPELL), restored("p2"), marker(SPELL), buffed(&id)],
                &state
            ),
            Vec::<String>::new()
        );
        // A restore of the other side leaves it judged.
        only(
            &i21(
                &state,
                &[marker(SPELL), restored("p1"), marker(SPELL), buffed(&id)],
                &state,
            ),
            &id,
            &["I21", "buffed"],
        );
    }

    #[test]
    fn r4599011_a_unit_s_own_evolution_inside_a_spell_s_run_is_no_spell_s() {
        let (state, id) = immune_board("r4599011");
        let evolved: GameEvent =
            json_as(json!({ "type": "evolved", "player": "p2", "instanceId": id, "defId": IMMUNE }));
        // C+ #49 Jay Fungus evolves as a Spell fills its hand with (0) Cost cards.
        assert_eq!(
            i21(&state, &[marker(SPELL), evolved.clone()], &state),
            Vec::<String>::new()
        );
        // The Spell's own change after it is still a reach.
        only(
            &i21(&state, &[marker(SPELL), evolved, buffed(&id)], &state),
            &id,
            &["I21", "buffed"],
        );
    }

    #[test]
    fn r4599012_a_kill_deadline_s_destroy_is_no_spell_s_and_i21_allows_it() {
        let (state, id) = immune_board("r4599012");
        let dead = moved(&state, &id, OffFieldZone::Graveyard);
        let unmarked = |mark: &str| -> GameEvent {
            json_as(json!({
                "type": "marked",
                "instanceId": id,
                "mark": mark,
                "color": "crimson",
                "added": false,
            }))
        };
        let destroyed: GameEvent = json_as(json!({
            "type": "destroyed",
            "instanceId": id,
            "defId": IMMUNE,
            "owner": "p2",
            "controller": "p2",
            "attack": 4,
            "maxHealth": 4,
            "killerId": null,
        }));
        // S+ #94 PIP's deadline comes due inside another card's run (R3550006).
        assert_eq!(
            i21(
                &state,
                &[marker(SPELL), unmarked("deadline"), destroyed.clone()],
                &dead
            ),
            Vec::<String>::new()
        );
        // Another mark's removal, or none, leaves the destroy the Spell's.
        only(
            &i21(
                &state,
                &[marker(SPELL), unmarked("duo"), destroyed.clone()],
                &dead,
            ),
            &id,
            &["I21", "destroyed"],
        );
        only(
            &i21(&state, &[marker(SPELL), destroyed], &dead),
            &id,
            &["I21", "destroyed"],
        );
    }

    #[test]
    fn r4599009_a_unit_gone_from_the_field_is_not_judged_after_it_changed_hands() {
        let (state, id) = immune_board("r4599009");
        let handed: GameEvent = json_as(json!({
            "type": "controlChanged",
            "instanceId": id,
            "controller": "p1",
            "row": "units",
            "lane": 1,
        }));
        let exiled: GameEvent =
            json_as(json!({ "type": "exiled", "instanceId": id, "defId": IMMUNE, "owner": "p2" }));
        let gone = moved(&state, &id, OffFieldZone::Exile);
        // A steal's end hands it back, and a Spell then exiles it: its keywords as the step began no
        // longer say whether it was immune then.
        assert_eq!(
            i21(&state, &[handed.clone(), marker(SPELL), exiled.clone()], &gone),
            Vec::<String>::new()
        );
        // With no change of hands it is judged by them.
        only(
            &i21(&state, &[marker(SPELL), exiled], &gone),
            &id,
            &["I21", "exiled"],
        );
        // Still on the field after the step, it is judged as it stands then.
        only(
            &i21(&state, &[handed, marker(SPELL), buffed(&id)], &state),
            &id,
            &["I21", "buffed"],
        );
    }
}

mod i22 {
    use super::*;
    use jackioh_engine::effects::{buff_random_unit, cast_random, trigger_cry};
    use jackioh_engine::testkit::seams::{record_target_draw, start_target_draws, take_target_draws};

    use crate::rules::fixtures::activate::{ACTIVATE_SCRIPTS, activate_catalog, pinger};
    use crate::rules::fixtures::harness::{act_result, in_hand, run_effect};
    use crate::rules::fixtures::mb22 as mb;
    use crate::rules::fixtures::play_pipeline_a::{PA, register_play_a, with_play_a};

    /// p1's first main phase over play pipeline A's fixtures, both seats at 8 mana with empty hands.
    fn board(seed: &str) -> GameState {
        let mut state = past_mulligans(&with_play_a(new_game(seed, None)));
        for player in [P1, P2] {
            state.players[player].mana.current = 8;
            state.players[player].mana.max = 8;
            state.players[player].hand = vec![];
        }
        state
    }

    /// An `fx-1` put at `player`'s unit zone `lane`, granted `keyword`.
    fn granted(state: &mut GameState, player: PlayerId, lane: i32, keyword: Keyword) -> CardInstance {
        let card = put(state, "fx-1", slot(player, Row::Units, lane), json!({}));
        let live = find_instance_mut(state, &card.id).expect("the unit");
        live.granted_keywords.push(keyword);
        live.clone()
    }

    /// An `fx-21` put at p2's unit zone `lane`, Dormant with two turns left.
    fn dormant(state: &mut GameState, lane: i32) -> CardInstance {
        let card = put(state, "fx-21", slot(P2, Row::Units, lane), json!({}));
        let live = find_instance_mut(state, &card.id).expect("the unit");
        live.slumber = Some(Slumber { turns: Some(2) });
        live.clone()
    }

    fn at(card: &CardInstance) -> Selection {
        Selection::Instance {
            instance_id: card.id.clone(),
        }
    }

    /// One `def_id` in p1's hand.
    fn one(state: &mut GameState, def_id: &str) -> CardInstance {
        in_hand(state, def_id, P1, 1).remove(0)
    }

    /// I22 of one draw for p1 on `state`.
    fn drawn(
        state: &GameState,
        source: Option<&CardInstance>,
        by: TargetDrawBy,
        picks: &[Selection],
    ) -> Vec<String> {
        target_draw_findings(&TargetDraw {
            state: state.clone(),
            source: source.cloned(),
            seat: P1,
            by,
            picks: picks.to_vec(),
        })
    }

    /// Apply effects as p1's own, then run the resolution loop as a Cry's list is; its events.
    fn run(state: &mut GameState, effects: Vec<Effect>) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..Default::default()
                },
            );
            apply_effects(&effects, &mut ctx);
            settle(&mut ctx.sink, Default::default());
        }
        state.rng_cursor = rng.cursor();
        events
    }

    #[test]
    fn i22_a_spell_draw_of_an_evasive_unit_in_play_is_flagged() {
        let mut state = board("i22-evasive");
        let evasive = granted(&mut state, P2, 1, Keyword::Evasive);
        let bolt = one(&mut state, &PA.bolt.id);
        let found = drawn(&state, Some(&bolt), TargetDrawBy::RandomCast, &[at(&evasive)]);
        only(
            &found,
            &evasive.id,
            &["I22 unlawful random target", &bolt.id, "R4353002", "R452"],
        );
    }

    #[test]
    fn i22_a_field_spell_draw_of_an_evasive_unit_is_flagged_and_of_an_immune_one_is_not() {
        let mut state = board("i22-field-spell");
        let evasive = granted(&mut state, P2, 1, Keyword::Evasive);
        let immune = granted(&mut state, P2, 2, Keyword::ImmuneToSpells);
        let field = put(&mut state, &PA.field.id, slot(P1, Row::Backrow, 1), json!({}));
        let found = drawn(&state, Some(&field), TargetDrawBy::Mayor, &[at(&evasive)]);
        only(&found, &evasive.id, &[&field.id, "R4353002", "R1200"]);
        assert!(drawn(&state, Some(&field), TargetDrawBy::Mayor, &[at(&immune)]).is_empty());
    }

    #[test]
    fn i22_a_spell_draw_of_a_card_immune_to_spells_is_flagged() {
        let mut state = board("i22-immune");
        let immune = granted(&mut state, P2, 1, Keyword::ImmuneToSpells);
        let bolt = one(&mut state, &PA.bolt.id);
        let found = drawn(&state, Some(&bolt), TargetDrawBy::RandomCast, &[at(&immune)]);
        only(
            &found,
            &immune.id,
            &["I22 unlawful random target", &bolt.id, "R450"],
        );
    }

    #[test]
    fn i22_a_draw_of_a_dormant_card_is_flagged_whatever_its_source() {
        let mut state = board("i22-dormant");
        let asleep = dormant(&mut state, 1);
        let bolt = one(&mut state, &PA.bolt.id);
        let zapper = one(&mut state, &PA.zapper.id);
        for source in [Some(&bolt), Some(&zapper), None] {
            let found = drawn(&state, source, TargetDrawBy::Mayor, &[at(&asleep)]);
            only(&found, &asleep.id, &["I22 unlawful random target", "R1839000"]);
        }
    }

    #[test]
    fn i22_a_unit_or_trap_source_may_aim_at_an_evasive_card() {
        let mut state = board("i22-unit-trap");
        let evasive = granted(&mut state, P2, 1, Keyword::Evasive);
        let immune = granted(&mut state, P2, 2, Keyword::ImmuneToSpells);
        let zapper = one(&mut state, &PA.zapper.id);
        let trap = put(
            &mut state,
            &PA.hidden_trap.id,
            slot(P1, Row::Backrow, 1),
            json!({}),
        );
        let field_trap = put(
            &mut state,
            &PA.hidden_field_trap.id,
            slot(P1, Row::Backrow, 2),
            json!({}),
        );
        // An Animated Field Spell standing in a unit zone aims as the Unit it is there (R383).
        let mut animated = put(&mut state, &PA.field.id, slot(P1, Row::Backrow, 3), json!({}));
        animated.zone = Zone::Field {
            player: P1,
            row: Row::Units,
            lane: 4,
        };
        for source in [&zapper, &trap, &field_trap, &animated] {
            let found = drawn(&state, Some(source), TargetDrawBy::Mayor, &[at(&evasive)]);
            assert!(found.is_empty(), "{}: {found:?}", source.def_id);
        }
        assert!(drawn(&state, Some(&zapper), TargetDrawBy::Mayor, &[at(&immune)]).is_empty());
    }

    #[test]
    fn i22_a_lawful_draw_is_clean() {
        let mut state = board("i22-lawful");
        let bare = put(&mut state, "fx-1", slot(P2, Row::Units, 1), json!({}));
        let bolt = one(&mut state, &PA.bolt.id);
        // Out of play, Evasive protects nothing from targeting; face-down, its keywords sleep (R4353002).
        let held = in_hand(&mut state, "fx-1", P2, 1).remove(0);
        find_instance_mut(&mut state, &held.id)
            .expect("the hand card")
            .granted_keywords
            .push(Keyword::Evasive);
        let set = put(
            &mut state,
            &PA.hidden_trap.id,
            slot(P2, Row::Backrow, 1),
            json!({}),
        );
        find_instance_mut(&mut state, &set.id)
            .expect("the set card")
            .granted_keywords
            .push(Keyword::Evasive);
        let picks = [at(&bare), Selection::Hero { player: P2 }, at(&held), at(&set)];
        for pick in picks {
            let found = drawn(
                &state,
                Some(&bolt),
                TargetDrawBy::RandomCast,
                std::slice::from_ref(&pick),
            );
            assert!(found.is_empty(), "{pick:?}: {found:?}");
        }
    }

    #[test]
    fn i22_the_monitor_judges_only_the_draws_made_between_before_and_after() {
        let mut state = board("i22-window");
        let evasive = granted(&mut state, P2, 1, Keyword::Evasive);
        let bolt = one(&mut state, &PA.bolt.id);
        let record = |state: &GameState| {
            record_target_draw(state, Some(&bolt), P1, TargetDrawBy::RandomCast, &[at(&evasive)]);
        };
        // No record is open: the draw is kept nowhere.
        record(&state);
        assert!(take_target_draws().is_empty());

        let mut monitor = create_invariant_monitor(&state);
        monitor.before(&state, P1, &ActionBody::EndTurn);
        record(&state);
        let found = invariant("I22", monitor.after(&[], &state));
        only(&found, &evasive.id, &["I22 unlawful random target", "R4353002"]);
        // `after` took the record and closed it.
        record(&state);
        assert!(invariant("I22", monitor.after(&[], &state)).is_empty());
    }

    #[test]
    fn i22_a_random_pick_made_by_an_effect_is_not_judged() {
        let mut state = board("i22-effect-pick");
        let evasive = granted(&mut state, P2, 1, Keyword::Evasive);
        let mut monitor = create_invariant_monitor(&state);
        monitor.before(&state, P1, &ActionBody::EndTurn);
        // "A random enemy Unit gets +1/+1" may land on an Evasive one (R4353002): no targeting.
        let events = run_effect(
            &mut state,
            buff_random_unit(json_as(json!({ "player": "enemy", "attack": 1, "health": 1 }))),
            P1,
        );
        assert!(
            events.iter().any(
                |event| matches!(event, GameEvent::Buffed { instance_id, .. } if *instance_id == evasive.id)
            ),
            "{events:?}"
        );
        assert!(invariant("I22", monitor.after(&events, &state)).is_empty());
    }

    #[test]
    fn i22_a_random_casts_prompt_draw_is_recorded_and_lawful() {
        for seed in 1..=6 {
            let mut state = board(&format!("i22-cast-{seed}"));
            granted(&mut state, P2, 1, Keyword::Evasive);
            let bare = put(&mut state, "fx-1", slot(P2, Row::Units, 2), json!({}));
            start_target_draws();
            run(
                &mut state,
                vec![cast_random(json_as(
                    json!({ "query": { "defId": PA.chooser.id }, "count": 1 }),
                ))],
            );
            let draws = take_target_draws();
            assert!(!draws.is_empty(), "seed {seed}: the cast's prompt drew");
            for draw in &draws {
                assert_eq!(draw.by, TargetDrawBy::RandomCast, "seed {seed}");
                assert_eq!(
                    draw.source.as_ref().map(|card| card.def_id.as_str()),
                    Some(PA.chooser.id.as_str()),
                    "seed {seed}"
                );
                assert_eq!(draw.picks, vec![at(&bare)], "seed {seed}");
                assert!(target_draw_findings(draw).is_empty(), "seed {seed}");
            }
        }
    }

    /// The random `target` draws `effects`, run as p1's, make on `state`.
    fn draws_of(state: &mut GameState, effects: Vec<Effect>) -> Vec<TargetDraw> {
        start_target_draws();
        run(state, effects);
        take_target_draws()
    }

    /// There is a draw, and each of `draws` is `by`'s, from `source_def`, never names `barred` and is clean.
    fn all_lawful(draws: &[TargetDraw], by: TargetDrawBy, source_def: &str, barred: &CardInstance) {
        assert!(!draws.is_empty(), "{source_def} drew");
        for draw in draws {
            assert_eq!(draw.by, by, "{draw:?}");
            assert_eq!(
                draw.source.as_ref().map(|card| card.def_id.as_str()),
                Some(source_def)
            );
            assert!(!draw.picks.contains(&at(barred)), "{:?}", draw.picks);
            assert!(target_draw_findings(draw).is_empty(), "{:?}", draw.picks);
        }
    }

    /// A random cast's declared targets (`play_steps`' cast choices) and a random Cry's (`cry_trigger`,
    /// R1855001) are recorded: the Spell never draws the Evasive unit, and a Unit's draws are clean.
    #[test]
    fn i22_a_random_casts_and_a_random_crys_declared_draws_are_recorded_and_lawful() {
        for seed in 1..=6 {
            let mut state = board(&format!("i22-declared-{seed}"));
            let evasive = granted(&mut state, P2, 1, Keyword::Evasive);
            put(&mut state, "fx-1", slot(P2, Row::Units, 2), json!({}));
            let cast = cast_random(json_as(json!({ "query": { "defId": PA.bolt.id }, "count": 1 })));
            let draws = draws_of(&mut state, vec![cast]);
            all_lawful(&draws, TargetDrawBy::RandomCast, &PA.bolt.id, &evasive);

            let zapper = put(&mut state, &PA.zapper.id, slot(P1, Row::Units, 1), json!({}));
            let cry = trigger_cry(json_as(json!({ "instanceId": zapper.id, "random": true })));
            let draws = draws_of(&mut state, vec![cry]);
            assert!(!draws.is_empty(), "seed {seed}: the Cry drew");
            for draw in &draws {
                assert_eq!(draw.by, TargetDrawBy::RandomCast, "seed {seed}");
                assert_eq!(draw.source.as_ref().map(|card| &card.id), Some(&zapper.id));
                assert!(target_draw_findings(draw).is_empty(), "seed {seed}");
            }
        }
    }

    /// A Mayor's draws (R1200) are recorded: a play's declared target, a cast's and a triggered Cry's,
    /// each from the same board (a bolt may kill the Mayor).
    #[test]
    fn i22_a_mayors_declared_draws_are_recorded_and_lawful() {
        for seed in 1..=6 {
            let mut start = mb::playing(&format!("i22-mayor-{seed}"));
            register_play_a();
            put(&mut start, &mb::mayor.id, slot(P1, Row::Units, 1), json!({}));
            let zapper = put(&mut start, &PA.zapper.id, slot(P1, Row::Units, 2), json!({}));
            let bolt = one(&mut start, &mb::bolt.id);
            let evasive = granted(&mut start, P2, 1, Keyword::Evasive);
            put(&mut start, &mb::ally.id, slot(P2, Row::Units, 2), json!({}));

            start_target_draws();
            let result = act_result(
                &start,
                json!({ "type": "play", "instanceId": bolt.id, "playerId": "p1" }),
            );
            assert!(result.error.is_none(), "seed {seed}: {:?}", result.error);
            all_lawful(&take_target_draws(), TargetDrawBy::Mayor, &mb::bolt.id, &evasive);

            let cast = cast_random(json_as(json!({ "query": { "defId": mb::bolt.id }, "count": 1 })));
            let draws = draws_of(&mut start.clone(), vec![cast]);
            all_lawful(&draws, TargetDrawBy::Mayor, &mb::bolt.id, &evasive);

            let cry = trigger_cry(json_as(json!({ "instanceId": zapper.id })));
            let draws = draws_of(&mut start.clone(), vec![cry]);
            assert!(!draws.is_empty(), "seed {seed}: the Cry drew");
            assert!(
                draws
                    .iter()
                    .all(|draw| draw.by == TargetDrawBy::Mayor && target_draw_findings(draw).is_empty()),
                "seed {seed}: {draws:?}"
            );
        }
    }

    /// `board` with `pinger` (a Field Spell whose Activate deals 1 to a declared target) in p1's
    /// backrow, a bolt and a zapper in p1's hand, and an Evasive and a bare unit on p2's side.
    fn listing_board() -> (GameState, [CardInstance; 5]) {
        let mut state = board("i22-listed");
        register_catalog(activate_catalog(registered_catalog().clone()));
        let mut scripts = registered_scripts().clone();
        for (id, script) in ACTIVATE_SCRIPTS.clone() {
            scripts.insert(id, script);
        }
        register_scripts(scripts);
        let evasive = granted(&mut state, P2, 1, Keyword::Evasive);
        let bare = put(&mut state, "fx-1", slot(P2, Row::Units, 2), json!({}));
        let bolt = one(&mut state, &PA.bolt.id);
        let zapper = one(&mut state, &PA.zapper.id);
        let ping = put(&mut state, &pinger.id, slot(P1, Row::Backrow, 1), json!({}));
        (state, [evasive, bare, bolt, zapper, ping])
    }

    /// The listed action of `card` whose targets are exactly `[at(bare)]`, retargeted at `evasive`.
    fn retargeted(
        legal: &[ActionBody],
        card: &CardInstance,
        bare: &CardInstance,
        evasive: &CardInstance,
    ) -> ActionBody {
        let mut action = legal
            .iter()
            .find(|body| match body {
                ActionBody::Play {
                    instance_id,
                    targets: Some(targets),
                    ..
                }
                | ActionBody::Activate {
                    instance_id,
                    targets: Some(targets),
                    ..
                } => *instance_id == card.id && *targets == vec![at(bare)],
                _ => false,
            })
            .cloned()
            .unwrap_or_else(|| panic!("{} is listed at {}: {legal:?}", card.def_id, bare.id));
        match &mut action {
            ActionBody::Play { targets, .. } | ActionBody::Activate { targets, .. } => {
                *targets = Some(vec![at(evasive)]);
            }
            _ => unreachable!(),
        }
        action
    }

    #[test]
    fn i22_a_listed_target_declaration_offering_an_unlawful_card_is_flagged() {
        let (state, [evasive, bare, bolt, zapper, ping]) = listing_board();
        let legal = legal_actions(&state, P1);
        assert_eq!(listed_target_findings(&state, P1, &legal), Vec::<String>::new());
        assert!(invariant("I22", create_invariant_monitor(&state).hidden(&state)).is_empty());

        for (card, verb) in [(&bolt, "play"), (&ping, "activation")] {
            let forged = retargeted(&legal, card, &bare, &evasive);
            let found = listed_target_findings(&state, P1, &[forged]);
            only(
                &found,
                &evasive.id,
                &["I22 unlawful listed target", verb, &card.id, "R4353002"],
            );
        }
        // A Unit's declaration may name an Evasive card (R4353002).
        let forged = retargeted(&legal, &zapper, &bare, &evasive);
        assert_eq!(
            listed_target_findings(&state, P1, &[forged]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn i22_an_open_target_prompt_offering_an_unlawful_card_is_flagged() {
        let mut state = board("i22-offered");
        let evasive = granted(&mut state, P2, 1, Keyword::Evasive);
        let bare = put(&mut state, "fx-1", slot(P2, Row::Units, 2), json!({}));
        let chooser = one(&mut state, &PA.chooser.id);
        let mut state = act_ok(
            &state,
            json!({ "type": "play", "instanceId": chooser.id, "playerId": "p1" }),
        )
        .state;
        let pending = state.pending.as_ref().expect("the chooser's prompt");
        assert_eq!(pending.kind, PromptKind::Target);
        let offered: Vec<&Selection> = pending.options.iter().map(|option| &option.selection).collect();
        assert_eq!(offered, vec![&at(&bare)]);
        assert!(invariant("I22", create_invariant_monitor(&state).hidden(&state)).is_empty());

        let pending = state.pending.as_mut().expect("the chooser's prompt");
        let mut forged = pending.options[0].clone();
        forged.key = evasive.id.clone();
        forged.selection = at(&evasive);
        pending.options.push(forged);
        let found = invariant("I22", create_invariant_monitor(&state).hidden(&state));
        only(
            &found,
            &evasive.id,
            &["I22 unlawful target offered", &chooser.id, "R4353002"],
        );
    }
}
