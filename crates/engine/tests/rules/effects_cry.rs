//! Trigger a Cry (docs/classic-sets.md B5 E13, R467; Classic #54 Rewind): a Unit's Cry run again, on
//! the field or out of a graveyard, for the triggering card's controller, who makes its choices (R70).
//! A Cry's declared choices (R81) are asked as prompts, because a triggered Cry has no play to carry
//! them, and a Cry that asks parks its own tail ahead of what the triggering list still owes (R113).

use jackioh_engine::effects::{has_triggerable_cry, trigger_cry};
use jackioh_engine::testkit::*;
use jackioh_engine::{
    PlayerId::{P1, P2},
    Row::Units,
};

use super::fixtures::combat::plain;
use super::fixtures::harness::{Bench, events_of_type, put, slot};
use super::fixtures::prompt_harness::{
    answer_keys, board, cast_now, event_types, expect_replays, hand_card, open_as, replayable,
    resolving_card, round_trip,
};
use super::fixtures::prompts::{
    aimer, asker, crier, field_crier, grave_rewind, moder, quickdraw_of, rewind, spark, tribute_crier,
};
use crate::rules::fixtures::harness::{act_logged, json_of, live};

/// Rewind resolving with its declared target (R81), as the play pipeline hands it over.
fn rewind_on(state: &mut GameState, target: &CardInstance, player: PlayerId, radiant: bool) -> Vec<String> {
    let mut sink = Bench::new(state);
    let card = resolving_card(state, &rewind().id, player, radiant);
    let targets = vec![Selection::Instance {
        instance_id: target.id.clone(),
    }];
    run_hook_resumable(
        &mut sink.sink(state),
        &card,
        "cry",
        json_as(json!({ "controller": player, "targets": targets })),
    );
    settle(&mut sink.sink(state), Default::default());
    let cursor = sink.rng.cursor();
    let events = std::mem::take(&mut sink.events);
    state.rng_cursor = cursor;
    event_types(&events)
}

fn grave_unit(state: &mut GameState, player: PlayerId, def_id: &str) -> CardInstance {
    let card = new_instance(state, def_id, player, Zone::Graveyard { player });
    state.players[player].graveyard.push(card.clone());
    card
}

fn has_type(types: &[String], wanted: &str) -> bool {
    types.iter().any(|t| t == wanted)
}

mod e13_trigger_a_cry {
    use super::*;

    #[test]
    fn r467_on_the_field_the_cry_runs_as_the_unit_for_the_triggering_cards_controller_and_is_no_play() {
        let mut state = board("cry-field");
        let unit = put(&mut state, &crier().id, slot(P1, Units, 2), json!({}));
        let played = state.counters.played;
        let log = state.players.p1.turn_log.clone();
        let events = rewind_on(&mut state, &unit, P1, false);
        // The Cry: 2 to the enemy hero and +1/+1 on "this"; then Rewind's own heal.
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 2);
        assert_eq!(live(&state, &unit).buffs, AttackHealth { attack: 1, health: 1 });
        assert_eq!(state.players.p1.hero.health, HERO_HEALTH + 1);
        assert!(!has_type(&events, "cardPlayed"));
        assert!(!has_type(&events, "cardAnnounced"));
        assert!(!has_type(&events, "cardResolved"));
        assert_eq!(state.counters.played, played);
        assert_eq!(state.players.p1.turn_log, log);
    }

    #[test]
    fn r467_out_of_a_graveyard_the_cry_runs_and_this_finds_nothing() {
        let mut state = board("cry-grave");
        let dead = grave_unit(&mut state, P1, &crier().id);
        cast_now(&mut state, &grave_rewind().id, P1, false);
        open_as(&state, PromptKind::Pick, P1);
        answer_keys(&mut state, &[format!("instance:{}", dead.id).as_str()]);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 2);
        assert_eq!(live(&state, &dead).buffs, AttackHealth { attack: 0, health: 0 });
        assert_eq!(live(&state, &dead).zone.z(), ZoneName::Graveyard);
    }

    #[test]
    fn r467_the_triggering_cards_controller_runs_the_cry_of_the_other_players_unit() {
        let mut state = board("cry-enemy");
        let theirs = put(&mut state, &crier().id, slot(P2, Units, 1), json!({}));
        rewind_on(&mut state, &theirs, P1, true);
        // Radiant: twice. "The enemy hero" is the triggering controller's enemy both times.
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 4);
        assert_eq!(state.players.p1.hero.health, HERO_HEALTH + 1);
        assert_eq!(live(&state, &theirs).buffs, AttackHealth { attack: 2, health: 2 });
    }

    #[test]
    fn r467_a_crys_declared_target_is_asked_of_the_triggering_controller_and_the_other_seat_sees_only_that() {
        let mut state = board("cry-target");
        let unit = put(&mut state, &aimer().id, slot(P1, Units, 1), json!({}));
        let enemy = put(&mut state, &plain.id, slot(P2, Units, 3), json!({}));
        rewind_on(&mut state, &unit, P1, false);
        let pending = open_as(&state, PromptKind::Target, P1);
        assert_eq!(pending.resume.hook, TRIGGER_CRY_HOOK);
        let selections: Vec<Selection> = pending
            .options
            .iter()
            .map(|option| option.selection.clone())
            .collect();
        assert_eq!(
            json_of(&selections),
            json!([
                { "pick": "instance", "instanceId": enemy.id },
                { "pick": "hero", "player": "p2" },
            ])
        );
        let labels: Vec<String> = pending
            .options
            .iter()
            .map(|option| option.label.clone())
            .collect();
        assert_eq!(labels, vec![plain.name.clone(), "Enemy hero".to_string()]);
        let keys: Vec<String> = pending.options.iter().map(|option| option.key.clone()).collect();
        assert_eq!(
            keys,
            vec![format!("instance:{}", enemy.id), "hero:p2".to_string()]
        );
        assert_eq!(
            json_of(&view_for(&state, P2).pending),
            json!({ "forYou": false, "pendingFor": "p1" })
        );
        // Rewind's heal waits behind the Cry.
        assert_eq!(state.players.p1.hero.health, HERO_HEALTH);

        let mut copy = round_trip(&state);
        answer_keys(&mut state, &["hero:p2"]);
        answer_keys(&mut copy, &["hero:p2"]);
        assert_eq!(hash_state(&copy), hash_state(&state));
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 3);
        assert_eq!(state.players.p1.hero.health, HERO_HEALTH + 1);
        assert!(state.pending.is_none());
    }

    #[test]
    fn r467_radiant_triggers_twice_each_run_with_its_own_choices() {
        let mut state = board("cry-twice");
        let theirs = put(&mut state, &aimer().id, slot(P2, Units, 1), json!({}));
        let other = put(&mut state, &plain.id, slot(P2, Units, 4), json!({}));
        rewind_on(&mut state, &theirs, P1, true);
        open_as(&state, PromptKind::Target, P1);
        answer_keys(&mut state, &[format!("instance:{}", other.id).as_str()]);
        let second = open_as(&state, PromptKind::Target, P1);
        assert_eq!(second.resume.hook, TRIGGER_CRY_HOOK);
        answer_keys(&mut state, &["hero:p2"]);
        // The first run's 3 damage killed the unit it chose; the second run chose the hero.
        assert_eq!(live(&state, &other).zone.z(), ZoneName::Graveyard);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 3);
        assert_eq!(state.players.p1.hero.health, HERO_HEALTH + 1);
    }

    #[test]
    fn r467_r90_modes_are_asked_first_when_a_target_belongs_to_a_mode_and_a_target_only_for_its_mode() {
        let mut hit = board("cry-mode-hit");
        let unit = put(&mut hit, &moder().id, slot(P1, Units, 1), json!({}));
        rewind_on(&mut hit, &unit, P1, false);
        let mode = open_as(&hit, PromptKind::Mode, P1);
        let keys: Vec<String> = mode.options.iter().map(|option| option.key.clone()).collect();
        assert_eq!(keys, vec!["mode:hit".to_string(), "mode:heal".to_string()]);
        answer_keys(&mut hit, &["mode:hit"]);
        open_as(&hit, PromptKind::Target, P1);
        answer_keys(&mut hit, &["hero:p2"]);
        assert_eq!(hit.players.p2.hero.health, HERO_HEALTH - 4);

        let mut heal = board("cry-mode-heal");
        let healer = put(&mut heal, &moder().id, slot(P1, Units, 1), json!({}));
        rewind_on(&mut heal, &healer, P1, false);
        answer_keys(&mut heal, &["mode:heal"]);
        assert!(heal.pending.is_none());
        assert_eq!(heal.players.p1.hero.health, HERO_HEALTH + 4 + 1);
    }

    #[test]
    fn r467_r113_a_cry_that_asks_parks_its_own_tail_ahead_of_the_triggering_lists() {
        let mut state = board("cry-asks");
        let unit = put(&mut state, &asker().id, slot(P1, Units, 1), json!({}));
        rewind_on(&mut state, &unit, P1, false);
        open_as(&state, PromptKind::Mode, P1);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 1);
        assert_eq!(state.players.p1.hero.health, HERO_HEALTH);
        let answered = answer_keys(&mut state, &["mode:left"]);
        // The answered step (4), then the Cry's own tail (2), and only then Rewind's heal.
        let order: Vec<String> = answered
            .events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { amount, .. } => Some(format!("damage {amount}")),
                GameEvent::Healed { .. } => Some("heal".to_string()),
                _ => None,
            })
            .collect();
        assert_eq!(
            order,
            vec!["damage 4".to_string(), "damage 2".to_string(), "heal".to_string()]
        );
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 7);
    }

    #[test]
    fn r467_r90_r123_a_crys_tribute_is_a_plays_price_nothing_is_asked_or_paid_and_its_slot_stays_empty() {
        let mut state = board("cry-tribute");
        let unit = put(&mut state, &tribute_crier().id, slot(P1, Units, 1), json!({}));
        let mine = put(&mut state, &plain.id, slot(P1, Units, 2), json!({}));
        rewind_on(&mut state, &unit, P1, false);
        let pending = open_as(&state, PromptKind::Target, P1);
        let selections: Vec<Selection> = pending
            .options
            .iter()
            .map(|option| option.selection.clone())
            .collect();
        assert_eq!(json_of(&selections), json!([{ "pick": "hero", "player": "p2" }]));
        answer_keys(&mut state, &["hero:p2"]);
        // The empty slot is what the script reads as "nothing tributed": 5, and the unit is untouched.
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 5);
        let lane_two = state.players.p1.units[1]
            .as_ref()
            .and_then(|pile| pile.first())
            .map(|card| card.id.clone());
        assert_eq!(lane_two, Some(mine.id.clone()));
    }

    #[test]
    fn r467_r174_a_unit_that_leaves_the_field_while_its_crys_choices_are_asked_triggers_nothing() {
        let mut state = board("cry-left");
        let unit = put(&mut state, &aimer().id, slot(P1, Units, 1), json!({}));
        rewind_on(&mut state, &unit, P1, false);
        open_as(&state, PromptKind::Target, P1);
        let mut moving = live(&state, &unit).clone();
        move_to_zone(
            &mut state,
            &mut moving,
            OffFieldZone::Graveyard,
            Default::default(),
        );
        answer_keys(&mut state, &["hero:p2"]);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);
        // The triggering list goes on.
        assert_eq!(state.players.p1.hero.health, HERO_HEALTH + 1);
    }

    #[test]
    fn r467_no_cry_is_triggered_for_a_dormant_card_a_card_with_no_cry_or_a_card_that_is_not_a_unit() {
        let mut state = board("cry-none");
        let buried = put(&mut state, &crier().id, slot(P1, Units, 1), json!({}));
        let mut top = new_instance(&mut state, &plain.id, P1, Zone::Hand { player: P1 });
        assert!(place_on_field(
            &mut state,
            &mut top,
            slot(P1, Units, 1),
            json_as(json!({ "stack": true }))
        ));
        let plain_unit = put(&mut state, &plain.id, slot(P1, Units, 2), json!({}));
        let spell = grave_unit(&mut state, P1, &spark().id);
        let dead = grave_unit(&mut state, P1, &crier().id);
        assert!(!has_triggerable_cry(&state, &live(&state, &buried)));
        assert!(!has_triggerable_cry(&state, &live(&state, &plain_unit)));
        assert!(!has_triggerable_cry(&state, &live(&state, &spell)));
        assert!(has_triggerable_cry(&state, &live(&state, &dead)));
        for card in [&buried, &plain_unit, &spell] {
            let mut sink = Bench::new(&state);
            {
                let mut inner = sink.sink(&mut state);
                let mut ctx = make_context(
                    &mut inner,
                    None,
                    HookOptions {
                        controller: Some(P1),
                        ..Default::default()
                    },
                );
                (trigger_cry(json_as(json!({ "instanceId": card.id }))).apply)(&mut ctx);
            }
            settle(&mut sink.sink(&mut state), Default::default());
            assert_eq!(sink.events, Vec::<GameEvent>::new());
        }
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);
    }

    #[test]
    fn r467_a_rewind_game_replays_from_its_log() {
        let aim = quickdraw_of(&aimer()).id;
        let back = quickdraw_of(&rewind()).id;
        let replay = replayable("cry-replay", &[aim.clone(), back.clone()], &[]);
        let mut log = replay.log;
        let decks = replay.decks;
        let mut state = replay.state;
        let unit = hand_card(&state, P1, &aim);
        state = act_logged(
            &state,
            json!({
                "type": "play", "playerId": "p1", "instanceId": unit.id,
                "targets": [{ "pick": "hero", "player": "p2" }],
            }),
            &mut log,
        )
        .state;
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 3);
        let rewinding = hand_card(&state, P1, &back);
        state = act_logged(
            &state,
            json!({
                "type": "play", "playerId": "p1", "instanceId": rewinding.id,
                "targets": [{ "pick": "instance", "instanceId": unit.id }],
            }),
            &mut log,
        )
        .state;
        let pending = open_as(&state, PromptKind::Target, P1);
        state = act_logged(
            &state,
            json!({
                "type": "answer", "playerId": "p1", "choiceId": pending.id,
                "selection": [{ "pick": "hero", "player": "p2" }],
            }),
            &mut log,
        )
        .state;
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 6);
        let last_events = state
            .applied
            .last()
            .map(|applied| applied.events.clone())
            .unwrap_or_default();
        assert!(events_of_type(&last_events, GameEventType::CardPlayed).is_empty());
        expect_replays("cry-replay", &decks, &log, &state);
        let pile = must(state.players.p1.units[0].clone(), "the aimer's pile");
        assert_eq!(pile.first().map(|card| card.id.clone()), Some(unit.id.clone()));
    }
}

// R1873003 (Stinkyscape #6 Tylannt): a face-up Field Spell on the field has its Cry triggered where it
// stands, as itself; out of a graveyard only a Unit's is.
mod r1873003_a_field_spell_on_the_field {
    use super::*;
    use jackioh_engine::Row::Backrow;

    fn trigger(state: &mut GameState, card: &CardInstance) {
        let mut sink = Bench::new(state);
        {
            let mut inner = sink.sink(state);
            let mut ctx = make_context(
                &mut inner,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..Default::default()
                },
            );
            (trigger_cry(json_as(json!({ "instanceId": card.id }))).apply)(&mut ctx);
        }
        settle(&mut sink.sink(state), Default::default());
        let cursor = sink.rng.cursor();
        state.rng_cursor = cursor;
    }

    #[test]
    fn r1873003_a_face_up_field_spells_cry_is_triggered_where_it_stands() {
        let mut state = board("cry-field-spell");
        let field = put(&mut state, &field_crier().id, slot(P1, Backrow, 1), json!({}));
        assert!(has_triggerable_cry(&state, &live(&state, &field)));
        trigger(&mut state, &field);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 2);
    }

    #[test]
    fn r1873003_one_in_a_graveyard_has_no_cry_to_trigger() {
        let mut state = board("cry-field-spell-grave");
        let dead = grave_unit(&mut state, P1, &field_crier().id);
        assert!(!has_triggerable_cry(&state, &live(&state, &dead)));
        trigger(&mut state, &dead);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);
    }
}

// R1869001 (Stinkyscape #71 A'Zoth): `trigger_cry` with `random` asks the controller nothing; each of
// their choices is drawn from the match rng, as R1855001's are.
mod r1869001_a_random_trigger {
    use super::*;

    /// `trigger_cry` with `random` on `card` for p1, its rng cursor written back.
    fn random_trigger(state: &mut GameState, card: &CardInstance) -> Vec<String> {
        let mut sink = Bench::new(state);
        {
            let mut inner = sink.sink(state);
            let mut ctx = make_context(
                &mut inner,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..Default::default()
                },
            );
            (trigger_cry(json_as(json!({ "instanceId": card.id, "random": true }))).apply)(&mut ctx);
        }
        settle(&mut sink.sink(state), Default::default());
        let cursor = sink.rng.cursor();
        let events = std::mem::take(&mut sink.events);
        state.rng_cursor = cursor;
        event_types(&events)
    }

    /// p1's aimer (Cry: 3 to an enemy Unit or hero) in unit zone 1, p2's plain in unit zone 3.
    fn aiming(seed: &str) -> (GameState, CardInstance, CardInstance) {
        let mut state = board(seed);
        let unit = put(&mut state, &aimer().id, slot(P1, Units, 1), json!({}));
        let enemy = put(&mut state, &plain.id, slot(P2, Units, 3), json!({}));
        (state, unit, enemy)
    }

    #[test]
    fn r1869001_a_random_trigger_of_aimers_cry_asks_nothing_and_hits_one_target() {
        let (mut state, unit, enemy) = aiming("cry-random");
        random_trigger(&mut state, &unit);
        assert!(state.pending.is_none(), "nothing is asked");
        let hero_hit = state.players.p2.hero.health == HERO_HEALTH - 3;
        // The plain dies of the 3, which takes its damage off with it (R78).
        let unit_hit = live(&state, &enemy).zone.z() == ZoneName::Graveyard;
        assert!(hero_hit != unit_hit, "exactly one of the two took the 3");
    }

    #[test]
    fn r1869001_the_same_seed_draws_the_same_target() {
        let (state, unit, _) = aiming("cry-random-replay");
        let run = |mut copy: GameState| {
            random_trigger(&mut copy, &unit);
            hash_state(&copy)
        };
        assert_eq!(run(round_trip(&state)), run(round_trip(&state)));
    }

    #[test]
    fn r1869001_a_random_trigger_draws_the_crys_mode_too() {
        let mut state = board("cry-random-mode");
        let unit = put(&mut state, &moder().id, slot(P1, Units, 1), json!({}));
        random_trigger(&mut state, &unit);
        assert!(state.pending.is_none(), "nothing is asked");
        let hit = state.players.p2.hero.health == HERO_HEALTH - 4;
        let healed = state.players.p1.hero.health == HERO_HEALTH + 4;
        assert!(hit != healed, "one mode ran");
    }
}
