//! MD-D4 and MD-D5 (docs/meditative-set.md M5): combat-only attack modifiers and
//! `conditionTargets` (R1120, R1121).
//!
//! R1120: an `attack_mods` hook adds to an attacker's strike in the one combat it started against a
//! Unit — the strike, its Cleave and its Trample excess — never the strike back, never a hero hit,
//! never the listed attack. A hook's Poisonous marks with the hit (SPEC §4.4 step 7). The AI's
//! projection reads the same bonus.
//!
//! R1121: `UnitView.condition_targets` names the enemy Units a modifier would apply to if the
//! viewer's own attacker attacked them now — only on attackers that may act now, never off turn,
//! never with a prompt open, never on the enemy's cards.
//!
//! Port of `packages/engine/test/attackMods.test.ts`.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{big_body, cleaver, plain, trampler};
use crate::rules::fixtures::combat_judge::{combat_judge_game, herald, herald_poison};
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
    declare_attack(sink, &attacker, &target)
        .err()
        .map(|error| error.message)
}

fn on_unit(state: &GameState, id: &str) -> AttackTarget {
    AttackTarget::Unit {
        instance: live(state, id),
    }
}

/// Each `damage` event as `(sourceId, targetId, amount)`.
fn hits(events: &[GameEvent]) -> Vec<(Option<String>, String, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage {
                source_id,
                target_id,
                amount,
                ..
            } => Some((source_id.clone(), target_id.clone(), *amount)),
            _ => None,
        })
        .collect()
}

/// The `conditionTargets` of the unit `instance_id` in this viewer's view, if the view shows it.
fn condition_targets(state: &GameState, viewer: PlayerId, instance_id: &str) -> Option<Vec<String>> {
    let view = view_for(state, viewer);
    let seat = if viewer == P1 { &view.you } else { &view.opponent };
    seat.units
        .iter()
        .flatten()
        .find(|unit| unit.instance_id == instance_id)
        .and_then(|unit| unit.condition_targets.clone())
}

mod attack_mods_r1120 {
    use super::*;

    #[test]
    fn r1120_bonus_on_strike_cleave_trample_vs_units_only() {
        // The strike: 3 + 2 on a Unit.
        let mut state = board("r1120-strike");
        put(
            &mut state,
            &herald.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let defender = put(
            &mut state,
            &big_body.id,
            slot(P2, Row::Units, 1),
            Default::default(),
        );
        let (_, events) = run(&mut state, |sink| {
            declare(sink, &attacker.id, on_unit(sink.state, &defender.id))
        });
        assert_eq!(
            hits(&events)[0],
            (Some(attacker.id.clone()), defender.id.clone(), 5)
        );

        // The Cleave: 3 + 2 on each neighbour too.
        let mut state = board("r1120-cleave");
        put(
            &mut state,
            &herald.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let attacker = put(
            &mut state,
            &cleaver.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 2), Default::default());
        let neighbour = put(&mut state, &plain.id, slot(P2, Row::Units, 3), Default::default());
        let (_, events) = run(&mut state, |sink| {
            declare(sink, &attacker.id, on_unit(sink.state, &defender.id))
        });
        assert_eq!(
            hits(&events)[0],
            (Some(attacker.id.clone()), defender.id.clone(), 5)
        );
        assert_eq!(
            hits(&events)[1],
            (Some(attacker.id.clone()), neighbour.id.clone(), 5)
        );

        // The Trample excess: 6 + 2 over a 3-health body sends 5 on. The body's own `damage` event
        // reads what it took, its 3 health (§4.4 step 5, R63); the bonus shows in the 5 carried on.
        let mut state = board("r1120-trample");
        put(
            &mut state,
            &herald.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let attacker = put(
            &mut state,
            &trampler.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());
        let (_, events) = run(&mut state, |sink| {
            declare(sink, &attacker.id, on_unit(sink.state, &defender.id))
        });
        assert_eq!(
            hits(&events)[0],
            (Some(attacker.id.clone()), defender.id.clone(), 3)
        );
        assert_eq!(
            hits(&events)[1],
            (Some(attacker.id.clone()), "hero-p2".to_string(), 5)
        );

        // A hero takes the listed attack: no bonus.
        let mut state = board("r1120-hero");
        put(
            &mut state,
            &herald.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let (_, events) = run(&mut state, |sink| {
            declare(sink, &attacker.id, AttackTarget::Hero { player: P2 })
        });
        assert_eq!(
            hits(&events)[0],
            (Some(attacker.id.clone()), "hero-p2".to_string(), 3)
        );
    }

    #[test]
    fn r1120_no_bonus_on_strike_back_heroes_or_shown_attack() {
        let mut state = board("r1120-strike-back");
        put(
            &mut state,
            &herald.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let defender = put(
            &mut state,
            &big_body.id,
            slot(P2, Row::Units, 1),
            Default::default(),
        );
        // The listed attack never sees the modifier, before or after.
        assert_eq!(unit_view(&state, by_id(&state, &attacker.id)).attack, 3);
        let (_, events) = run(&mut state, |sink| {
            declare(sink, &attacker.id, on_unit(sink.state, &defender.id))
        });
        assert_eq!(unit_view(&state, by_id(&state, &attacker.id)).attack, 3);
        // The strike back is the defender's own 5.
        assert_eq!(
            hits(&events)[1],
            (Some(defender.id.clone()), attacker.id.clone(), 5)
        );
    }

    #[test]
    fn r1120_hook_poisonous_marks() {
        // A 1-attack hit with the Poisonous rider destroys a 5/10 all the same (step 7).
        let mut state = board("r1120-poisonous");
        put(
            &mut state,
            &herald_poison.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let defender = put(
            &mut state,
            &big_body.id,
            slot(P2, Row::Units, 1),
            Default::default(),
        );
        let (_, events) = run(&mut state, |sink| {
            declare(sink, &attacker.id, on_unit(sink.state, &defender.id))
        });
        assert_eq!(
            hits(&events)[0],
            (Some(attacker.id.clone()), defender.id.clone(), 5)
        );
        assert!(
            events.iter().any(|event| matches!(
                event,
                GameEvent::Destroyed { instance_id, .. } if instance_id == &defender.id
            )),
            "the Poisonous hit destroyed the defender"
        );
    }

    #[test]
    fn r1120_lethal_reads_bonus() {
        // My Pawn projects what an attack would deal the defending hero (R44): a 6/4 Trample with the
        // +2 on a 3/3 carries 8 - 3 = 5 on, where its listed 6 alone would carry 3; on the hero
        // itself it deals the listed 6.
        let mut state = board("r1120-lethal");
        put(
            &mut state,
            &herald.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let attacker = put(
            &mut state,
            &trampler.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());
        let attacker = live(&state, &attacker.id);
        let defender = live(&state, &defender.id);
        assert_eq!(
            jackioh_engine::subsystems::lethal::projected_damage(
                &state,
                &attacker,
                &AttackTarget::Unit { instance: defender }
            ),
            5
        );
        assert_eq!(
            jackioh_engine::subsystems::lethal::projected_damage(
                &state,
                &attacker,
                &AttackTarget::Hero { player: P2 }
            ),
            6
        );
    }
}

mod attack_mods_r1121 {
    use super::*;

    #[test]
    fn r1121_condition_targets_on_own_attackers() {
        let mut state = board("r1121-on");
        put(
            &mut state,
            &herald.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let first = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());
        let second = put(&mut state, &plain.id, slot(P2, Row::Units, 3), Default::default());
        assert_eq!(
            condition_targets(&state, P1, &attacker.id),
            Some(vec![first.id.clone(), second.id.clone()])
        );
    }

    #[test]
    fn r1121_absent_off_turn_in_prompt_on_enemies() {
        let mut state = board("r1121-off");
        put(
            &mut state,
            &herald.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        let foe = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());
        // The enemy's cards carry no yellow in the opponent's view.
        assert_eq!(condition_targets(&state, P2, &attacker.id), None);
        assert_eq!(condition_targets(&state, P2, &foe.id), None);
        // Off turn the viewer's own attackers carry none either.
        state.active = P2;
        assert_eq!(condition_targets(&state, P1, &attacker.id), None);
        // Nor with a prompt open: `can_act` answers false, so the hook is never asked.
        state.active = P1;
        state.pending = Some(json_as(json!({
            "id": "q1",
            "playerId": "p1",
            "kind": "answer",
            "prompt": "test prompt",
            "options": [{ "key": "a", "label": "A", "selection": { "pick": "none" } }],
            "min": 1,
            "max": 1,
            "resume": { "defId": "cj-herald", "hook": "q", "step": "q", "radiant": false, "data": {} },
        })));
        assert_eq!(condition_targets(&state, P1, &attacker.id), None);
        // Nor once the attacker has acted.
        state.pending = None;
        find_instance_mut(&mut state, &attacker.id)
            .unwrap()
            .exertion
            .attacked = true;
        assert_eq!(condition_targets(&state, P1, &attacker.id), None);
    }
}
