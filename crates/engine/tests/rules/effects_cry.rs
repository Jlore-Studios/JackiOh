//! Trigger a Cry (docs/classic-sets.md B5 E13, R467): a Unit's Cry run again, on the field or out of a
//! graveyard, for the player whose card triggered it, who makes its choices (R70) — Classic #54 Rewind.
//! On the field the Unit is the Cry's "this"; out of a graveyard "this" finds nothing. A Cry's declared
//! choices (R81) are asked as prompts, because a triggered Cry has no play to carry them; nothing is
//! played, so nothing counts a play; and a Cry that asks parks its own tail ahead of what the
//! triggering list still owes (R113).
//!
//! Port of `packages/engine/test/effects-cry.test.ts`.

use jackioh_engine::effects::{has_triggerable_cry, trigger_cry};
use jackioh_engine::testkit::*;
use jackioh_engine::{
    PlayerId::{P1, P2},
    Row::Units,
};

use super::fixtures::combat::plain;
use super::fixtures::harness::{events_of_type, put, slot};
use super::fixtures::prompt_harness::{
    act, answer_keys, board, cast_now, event_types, expect_replays, hand_card, must, open_as, replayable,
    resolving_card, round_trip,
};
use super::fixtures::prompts::{
    aimer, asker, crier, grave_rewind, moder, quickdraw_of, rewind, spark, tribute_crier,
};

/// TS `sinkFor(state)` (fixtures/harness.ts): the state, a fresh event list and an rng at the state's
/// cursor, as `reduce` starts one. A sink borrows all three, so they live here and `sink()` lends them
/// out, built from part 1's frozen `EngineSink::new` and `Rng::new`.
struct Bench<'a> {
    state: &'a mut GameState,
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Bench<'_> {
    fn sink(&mut self) -> EngineSink<'_> {
        EngineSink::new(self.state, &mut self.events, &mut self.rng)
    }
}

fn sink_for(state: &mut GameState) -> Bench<'_> {
    let rng = Rng::new(&state.seed, state.rng_cursor);
    Bench {
        state,
        events: Vec::new(),
        rng,
    }
}

/// Rewind resolving with its declared target (R81), as the play pipeline hands it over.
fn rewind_on(state: &mut GameState, target: &CardInstance, player: PlayerId, radiant: bool) -> Vec<String> {
    let mut sink = sink_for(state);
    let card = resolving_card(sink.state, &rewind().id, player, radiant);
    let targets = vec![Selection::Instance {
        instance_id: target.id.clone(),
    }];
    run_hook_resumable(
        &mut sink.sink(),
        &card,
        "cry",
        json_as(json!({ "controller": player, "targets": targets })),
    );
    settle(&mut sink.sink(), Default::default());
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

/// The card as it stands in the state now (TS held the live object).
fn live<'a>(state: &'a GameState, card: &CardInstance) -> &'a CardInstance {
    find_instance(state, &card.id).unwrap_or_else(|| panic!("{} is nowhere", card.id))
}

fn json_of<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
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
        // Nothing was played, cast or announced.
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
        assert!(!has_triggerable_cry(&state, live(&state, &buried)));
        assert!(!has_triggerable_cry(&state, live(&state, &plain_unit)));
        assert!(!has_triggerable_cry(&state, live(&state, &spell)));
        assert!(has_triggerable_cry(&state, live(&state, &dead)));
        for card in [&buried, &plain_unit, &spell] {
            let mut sink = sink_for(&mut state);
            {
                let mut inner = sink.sink();
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
            settle(&mut sink.sink(), Default::default());
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
        state = act(
            &state,
            json!({
                "type": "play", "playerId": "p1", "instanceId": unit.id,
                "targets": [{ "pick": "hero", "player": "p2" }],
            }),
            Some(&mut log),
        );
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 3);
        let rewinding = hand_card(&state, P1, &back);
        state = act(
            &state,
            json!({
                "type": "play", "playerId": "p1", "instanceId": rewinding.id,
                "targets": [{ "pick": "instance", "instanceId": unit.id }],
            }),
            Some(&mut log),
        );
        let pending = open_as(&state, PromptKind::Target, P1);
        state = act(
            &state,
            json!({
                "type": "answer", "playerId": "p1", "choiceId": pending.id,
                "selection": [{ "pick": "hero", "player": "p2" }],
            }),
            Some(&mut log),
        );
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
