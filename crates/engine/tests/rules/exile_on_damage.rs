//! MD-D31 (docs/meditative-set.md M5): exile on damage (R1124).
//!
//! Damage a card with `StaticFlags.exiles_on_damage` deals to a Unit marks it (SPEC §4.4 step 7),
//! and the next state check exiles it ahead of deaths (SPEC §4.5 step 1): no Death, no Reborn, no
//! `destroyed`. A hit the pipeline stopped — Divine Shield, Indestructible, the zero rule — dealt
//! no damage, so it marks nothing.
//!
//! Port of `packages/engine/test/exileOnDamage.test.ts`.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{big_body, indestructible, plain, shielded};
use crate::rules::fixtures::combat_judge::{banisher, banisher_cleave, combat_judge_game};
use crate::rules::fixtures::harness::{put, sink_for, slot};

/// A game at turn 4, P1's main phase, with the combat-judge fixtures registered.
fn board(seed: &str) -> GameState {
    let mut state = combat_judge_game(seed, None);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

fn by_id<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

/// The card as it stands in `state` now, owned.
fn live(state: &GameState, id: &str) -> CardInstance {
    by_id(state, id).clone()
}

/// TS `sinkFor(state)`: run `f` on a sink over `state` and hand back what it emitted.
fn run<R>(state: &mut GameState, f: impl FnOnce(&mut EngineSink<'_>) -> R) -> (R, Vec<GameEvent>) {
    let mut sink = sink_for(state);
    let out = f(&mut sink);
    let events = sink.events.clone();
    (out, events)
}

/// `declareAttack(sink, attacker, target)` with both read live off the sink.
fn declare(sink: &mut EngineSink<'_>, attacker_id: &str, target: AttackTarget) -> Option<String> {
    let attacker = live(sink.state, attacker_id);
    declare_attack(sink, &attacker, &target).err().map(|error| error.message)
}

fn on_unit(state: &GameState, id: &str) -> AttackTarget {
    AttackTarget::Unit {
        instance: live(state, id),
    }
}

fn exiled_ids(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Exiled { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

fn destroyed_ids(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Destroyed { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

fn pile_of(state: &GameState, player: PlayerId, pile: &str) -> Vec<String> {
    let zone = match pile {
        "graveyard" => state.players[player].graveyard.clone(),
        "exile" => state.players[player].exile.clone(),
        other => panic!("no pile {other}"),
    };
    zone.into_iter().map(|card| card.id).collect()
}

#[test]
fn r1124_exiled_before_deaths_no_death_reborn_destroyed() {
    // A 2/2 Banisher into a 3/3: 2 damage marks it, and the check exiles it — though the strike
    // back destroyed the Banisher first.
    let mut state = board("r1124-exiled");
    let attacker = put(&mut state, &banisher.id, slot(P1, Row::Units, 1), Default::default());
    let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());
    let (_, events) = run(&mut state, |sink| {
        declare(sink, &attacker.id, on_unit(sink.state, &defender.id))
    });
    assert_eq!(exiled_ids(&events), vec![defender.id.clone()]);
    assert_eq!(destroyed_ids(&events), vec![attacker.id.clone()]);
    assert!(pile_of(&state, P2, "exile").contains(&defender.id));
    assert!(pile_of(&state, P1, "graveyard").contains(&attacker.id));
}

#[test]
fn r1124_shield_indestructible_zero_stop_it() {
    // Divine Shield stops the whole hit: nothing marked, nothing exiled, the shield spent instead.
    let mut state = board("r1124-shield");
    let attacker = put(&mut state, &banisher.id, slot(P1, Row::Units, 1), Default::default());
    let defender = put(&mut state, &shielded.id, slot(P2, Row::Units, 1), Default::default());
    let (_, events) = run(&mut state, |sink| {
        declare(sink, &attacker.id, on_unit(sink.state, &defender.id))
    });
    assert!(exiled_ids(&events).is_empty());
    assert_eq!(by_id(&state, &defender.id).divine_shield_spent, Some(true));

    // Indestructible takes nothing: nothing marked, nothing exiled.
    let mut state = board("r1124-indestructible");
    let attacker = put(&mut state, &banisher.id, slot(P1, Row::Units, 1), Default::default());
    let defender = put(&mut state, &indestructible.id, slot(P2, Row::Units, 1), Default::default());
    let (_, events) = run(&mut state, |sink| {
        declare(sink, &attacker.id, on_unit(sink.state, &defender.id))
    });
    assert!(exiled_ids(&events).is_empty());
    assert!(destroyed_ids(&events).is_empty());

    // The zero rule: a 0-damage hit from the Banisher marks nothing.
    let mut state = board("r1124-zero");
    let attacker = put(&mut state, &banisher.id, slot(P1, Row::Units, 1), Default::default());
    let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());
    let (_, events) = run(&mut state, |sink| {
        let source = live(sink.state, &attacker.id);
        let target = DamageTarget::Unit {
            instance: live(sink.state, &defender.id),
        };
        deal_damage(
            sink,
            DamageArgs {
                source: Some(source),
                target,
                amount: 0,
                flags: None,
            },
        );
    });
    assert!(exiled_ids(&events).is_empty());
    assert!(destroyed_ids(&events).is_empty());
}

#[test]
fn r1124_cleave_exiles_source_still_dies() {
    // A 4/2 Cleave Banisher into a 5/10 with a 3/3 beside it: the hit and the cleave both mark,
    // both are exiled, and the source still dies to the strike back.
    let mut state = board("r1124-cleave");
    let attacker = put(&mut state, &banisher_cleave.id, slot(P1, Row::Units, 1), Default::default());
    let defender = put(&mut state, &big_body.id, slot(P2, Row::Units, 2), Default::default());
    let neighbour = put(&mut state, &plain.id, slot(P2, Row::Units, 3), Default::default());
    let (_, events) = run(&mut state, |sink| {
        declare(sink, &attacker.id, on_unit(sink.state, &defender.id))
    });
    assert_eq!(exiled_ids(&events), vec![defender.id.clone(), neighbour.id.clone()]);
    assert_eq!(destroyed_ids(&events), vec![attacker.id.clone()]);
}
