//! Port of `packages/engine/test/kill-credit.test.ts`.
//!
//! A kill credited to another unit (R42, R412: `killCredit.ts`, `effects/killCredit.ts`'s
//! `withKillCredit`), as Classic+ #19.2 Jungle Loser credits Classic+ #19.5 Bot Loser; and whether a card
//! is being cast on draw (`castOnDrawNow.ts`, R58), which Classic+ #26 Tommy Tempo's ability reads.
//!
//! Pinned: with a credit in force, a lethal hit on a paired victim names the paired unit as R42's killer
//! — in the `destroyed` event and for its kill triggers — while the striker stays the hit's source; the
//! credit is gone once the watched effect is; a kill of an unpaired victim is the striker's own; without
//! a transfer the striker keeps the kill and the follow-up runs for the pair whose victim died; a Death
//! that asks during the watched attack loses nothing, and the paused game survives JSON and replays. A
//! cast-on-draw card's Cry reads `isCastOnDraw` true; the same card played from hand reads it false.

use jackioh_engine::testkit::*;

use crate::rules::fixtures::damage_combat::{answer, death_asker, grunt, playing, recorder, replays_to, round_trip};
use crate::rules::fixtures::harness::{in_hand, put, set_library, slot};
use crate::rules::fixtures::kill_credit::{bot, jungle, register_kill_credit, tempo};

fn game(seed: &str) -> GameState {
    let state = playing(seed);
    register_kill_credit();
    state
}

/// `put(state, defId, ref, { radiant: true })`: placed, then made Radiant in the state (these
/// fixtures print nothing that depends on the face as they enter).
fn put_radiant(state: &mut GameState, def_id: &str, at: ZoneSlot) -> CardInstance {
    let placed = put(state, def_id, at);
    let card = find_instance_mut(state, &placed.id).expect("the card was just placed");
    card.radiant = true;
    card.clone()
}

/// `{ ...body, playerId }`.
fn input(player: PlayerId, body: ActionBody) -> ActionInput {
    ActionInput { body, player_id: player }
}

/// Every `destroyed` event as `(instanceId, killerId)`.
fn destroyed(events: &[GameEvent]) -> Vec<(String, Option<String>)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Destroyed { instance_id, killer_id, .. } => Some((instance_id.clone(), killer_id.clone())),
            _ => None,
        })
        .collect()
}

fn attack_of(state: &GameState, id: &str) -> Option<i32> {
    find_instance(state, id).map(|card| card.buffs.attack)
}

mod r42_r412_with_kill_credit {
    use super::*;

    #[test]
    fn with_a_transfer_the_paired_unit_is_the_killer_the_destroyed_event_names_it_and_its_kill_trigger_fires() {
        let mut state = game("kc-transfer");
        let striker = put_radiant(&mut state, &jungle().id, slot(PlayerId::P1, Row::Units, 2));
        let credited = put(&mut state, &bot().id, slot(PlayerId::P1, Row::Units, 4));
        let victim = put(&mut state, &grunt().id, slot(PlayerId::P2, Row::Units, 4));
        let mut play = recorder(&state);
        let ended = play.play(input(PlayerId::P1, ActionBody::EndTurn));
        assert_eq!(destroyed(&ended.events), vec![(victim.id.clone(), Some(credited.id.clone()))]);
        // The hit was the striker's: its damage event names it as the source.
        assert!(ended.events.iter().any(|event| matches!(
            event,
            GameEvent::Damage { source_id: Some(source), target_id, .. }
                if *source == striker.id && *target_id == victim.id
        )));
        let after = play.state().clone();
        assert_eq!(attack_of(&after, &credited.id), Some(5));
        assert_eq!(attack_of(&after, &striker.id), Some(0));
        assert!(find_instance(&after, &striker.id)
            .and_then(|card| card.memory.get(kill_credit::KILL_CREDIT_KEY))
            .is_none());
        assert!(replays_to(&play.start, &play.log, &after));
    }

    #[test]
    fn a_kill_of_a_victim_with_no_pair_stays_the_strikers() {
        let mut state = game("kc-unpaired");
        let striker = put_radiant(&mut state, &jungle().id, slot(PlayerId::P1, Row::Units, 2));
        let credited = put(&mut state, &bot().id, slot(PlayerId::P1, Row::Units, 4));
        let victim = put(&mut state, &grunt().id, slot(PlayerId::P2, Row::Units, 1));
        let result = recorder(&state).play(input(PlayerId::P1, ActionBody::EndTurn));
        assert_eq!(destroyed(&result.events), vec![(victim.id.clone(), Some(striker.id.clone()))]);
        assert_eq!(attack_of(&result.state, &credited.id), Some(0));
    }

    #[test]
    fn without_a_transfer_the_striker_keeps_the_kill_and_the_follow_up_runs_for_the_pair_whose_victim_died() {
        let mut state = game("kc-follow");
        let striker = put(&mut state, &jungle().id, slot(PlayerId::P1, Row::Units, 2));
        let credited = put(&mut state, &bot().id, slot(PlayerId::P1, Row::Units, 4));
        let victim = put(&mut state, &grunt().id, slot(PlayerId::P2, Row::Units, 4));
        let result = recorder(&state).play(input(PlayerId::P1, ActionBody::EndTurn));
        assert_eq!(destroyed(&result.events), vec![(victim.id.clone(), Some(striker.id.clone()))]);
        let live = find_instance(&result.state, &credited.id);
        assert_eq!(live.map(|card| card.buffs.attack), Some(0));
        assert!(live.is_some_and(|card| restrictions::is_berserk(card)));
    }

    #[test]
    fn r113_a_death_that_asks_during_the_watched_attack_the_credit_already_landed_and_the_pause_survives_json() {
        let mut state = game("kc-pause");
        put_radiant(&mut state, &jungle().id, slot(PlayerId::P1, Row::Units, 2));
        let credited = put(&mut state, &bot().id, slot(PlayerId::P1, Row::Units, 4));
        let victim = put(&mut state, &death_asker().id, slot(PlayerId::P2, Row::Units, 4));
        let mut play = recorder(&state);
        let ended = play.play(input(PlayerId::P1, ActionBody::EndTurn));
        assert_eq!(destroyed(&ended.events), vec![(victim.id.clone(), Some(credited.id.clone()))]);
        let paused = play.state().clone();
        assert!(paused.pending.is_some());
        assert_eq!(round_trip(&paused), paused);
        let answered = answer(&round_trip(&paused));
        assert_eq!(attack_of(&answered.state, &credited.id), Some(5));
        assert!(answered.state.pending.is_none());
    }

    #[test]
    fn credited_killer_id_reads_the_record_and_falls_back_to_the_striker() {
        let mut state = game("kc-read");
        let striker = put(&mut state, &jungle().id, slot(PlayerId::P1, Row::Units, 2));
        let victim = put(&mut state, &grunt().id, slot(PlayerId::P2, Row::Units, 1));
        assert_eq!(kill_credit::credited_killer_id(&striker, &victim.id), striker.id);
        find_instance_mut(&mut state, &striker.id)
            .expect("the striker is on the field")
            .memory
            .insert(kill_credit::KILL_CREDIT_KEY.to_string(), json!([{ "victimId": victim.id, "toId": "c999" }]));
        let striker = find_instance(&state, &striker.id).expect("the striker is on the field").clone();
        assert_eq!(kill_credit::credited_killer_id(&striker, &victim.id), "c999");
        assert_eq!(kill_credit::credited_killer_id(&striker, "c1000"), striker.id);
    }
}

mod r58_is_cast_on_draw {
    use super::*;

    #[test]
    fn a_cast_on_draw_cards_cry_reads_true_and_the_draw_is_complete_once_the_cast_resolved() {
        let mut state = game("kc-cast");
        let library = set_library(&mut state, PlayerId::P1, &[tempo().id, grunt().id]);
        let drawn = library.first().expect("no card").clone();
        // `sinkFor(state)`: an rng at the state's cursor; TS did not write it back.
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let outcome = {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            draw::draw_one(&mut sink, PlayerId::P1, None)
        };
        assert_eq!(outcome, draw::DrawOutcome::Cast);
        assert_eq!(
            find_instance(&state, &drawn.id).and_then(|card| card.memory.get("castOnDraw")),
            Some(&json!(true))
        );
        assert!(state.held_draws.is_none());
    }

    #[test]
    fn the_same_card_played_from_hand_reads_false() {
        let mut state = game("kc-hand");
        let hand = in_hand(&mut state, &tempo().id, PlayerId::P1, 1);
        let card = hand.first().expect("no card").clone();
        let body: ActionInput = json_as(json!({
            "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": 1 }, "playerId": "p1"
        }));
        let result = recorder(&state).play(body);
        assert_eq!(
            find_instance(&result.state, &card.id).and_then(|found| found.memory.get("castOnDraw")),
            Some(&json!(false))
        );
    }
}
