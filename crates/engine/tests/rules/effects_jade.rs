//! ME-JADE (Meditative #39.2 Jade, #39.4 Red Jade, #39.5 Jade Beauty; docs/meditative-set.md M5,
//! Group C's systems): each player's public Jade Counter, which only rises, written by `add_jade`
//! with the public `jadeChanged` event (R961, MD-C2), and its crossings of 5 (summon a Jade Beauty)
//! and 10 (make every Jade Beauty Radiant, or summon a Radiant one) inside the same add (R962,
//! MD-C3). Through fixture scripts (fixtures/jade.rs); the real cards' tests cover the same cases
//! again (`crates/cards/src/scripts/meditative/`).

use jackioh_engine::effects::add_jade;
use jackioh_engine::effects::destroy::{DestroyArgs, destroy};
use jackioh_engine::effects::targets::TargetSpec;
use jackioh_engine::state_check::state_check;
use jackioh_engine::testkit::*;
use jackioh_engine::view_for::view_for;

use crate::rules::fixtures::catalog::vanilla_catalog;
use crate::rules::fixtures::harness::{new_game, put, slot};
use crate::rules::fixtures::jade::{JADE_SCRIPTS, jade_catalog};
use crate::rules::fixtures::scripts::{FIXTURE_SCRIPTS, fixture_catalog};

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    register_catalog(jade_catalog(fixture_catalog(vanilla_catalog(40, 1))));
    let mut scripts = FIXTURE_SCRIPTS.clone();
    scripts.extend(JADE_SCRIPTS.clone());
    register_scripts(scripts);
    state.players[PlayerId::P1].hand = vec![];
    state.players[PlayerId::P2].hand = vec![];
    state
}

/// The card running the script, resolving as a Spell does (§10.5 step 4). Like TS's, it is in no pile.
fn resolving(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    let mut card = new_instance(state, def_id, player, Zone::Resolving { player });
    card.radiant = false;
    card
}

/// TS `run(state, effect, self, targets, controller)`: one effect on a sink over `state`, the rng cursor
/// written back; its events.
fn run(
    state: &mut GameState,
    effect: Effect,
    self_: Option<&CardInstance>,
    targets: Vec<Selection>,
    controller: PlayerId,
) -> Vec<GameEvent> {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            self_,
            HookOptions {
                controller: Some(controller),
                targets: Some(targets),
                ..Default::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn add(state: &mut GameState, amount: i32, player: PlayerId) -> Vec<GameEvent> {
    let spell = resolving(state, "fx-mana-well", player);
    run(
        state,
        add_jade(json_as(json!({ "amount": amount }))),
        Some(&spell),
        vec![],
        player,
    )
}

fn beauties(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    state.players[player]
        .units
        .iter()
        .filter_map(|pile| pile.as_ref().and_then(|pile| pile.first()).cloned())
        .filter(|card| card.def_id == JADE_BEAUTY_DEF_ID)
        .collect()
}

fn jade_changed(events: &[GameEvent]) -> Vec<(PlayerId, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::JadeChanged { player, value } => Some((*player, *value)),
            _ => None,
        })
        .collect()
}

#[test]
fn r961_add_jade_raises_the_counter_reports_jade_changed_and_both_views_show_it() {
    let mut state = game("jade-raise");
    let events = add(&mut state, 3, PlayerId::P1);
    assert_eq!(state.players[PlayerId::P1].jade, Some(3));
    assert_eq!(state.players[PlayerId::P2].jade, None);
    assert_eq!(jade_changed(&events), vec![(PlayerId::P1, 3)]);
    // Public on both seats, beside the hero.
    assert_eq!(view_for(&state, PlayerId::P1).you.jade, Some(3));
    assert_eq!(view_for(&state, PlayerId::P1).opponent.jade, None);
    assert_eq!(view_for(&state, PlayerId::P2).you.jade, None);
    assert_eq!(view_for(&state, PlayerId::P2).opponent.jade, Some(3));
}

#[test]
fn r961_an_add_of_0_or_less_changes_and_reports_nothing() {
    let mut state = game("jade-zero");
    for amount in [0, -2] {
        let events = add(&mut state, amount, PlayerId::P1);
        assert_eq!(state.players[PlayerId::P1].jade, None);
        assert!(jade_changed(&events).is_empty());
    }
}

#[test]
fn r961_a_game_without_jade_serialises_no_jade_in_state_or_view() {
    // D14: a game that never uses the counter serializes, hashes and replays as before.
    let state = game("jade-absent");
    let raw = serde_json::to_value(&state).expect("state serialises");
    assert_eq!(raw["players"]["p1"].get("jade"), None);
    assert_eq!(raw["players"]["p2"].get("jade"), None);
    for viewer in [PlayerId::P1, PlayerId::P2] {
        let view = serde_json::to_value(view_for(&state, viewer)).expect("view serialises");
        assert_eq!(view["you"].get("jade"), None);
        assert_eq!(view["opponent"].get("jade"), None);
    }
}

#[test]
fn r962_crossing_5_summons_a_jade_beauty_with_no_cry() {
    let mut state = game("jade-summon");
    let events = add(&mut state, 5, PlayerId::P1);
    let summoned = beauties(&state, PlayerId::P1);
    assert_eq!(summoned.len(), 1);
    assert!(!summoned[0].radiant);
    // Summoned, never played or cast: no Cry fires for it.
    assert!(
        events.iter().any(|event| matches!(
            event,
            GameEvent::Summoned { def_id, .. } if def_id == JADE_BEAUTY_DEF_ID
        )),
        "a Jade Beauty is summoned"
    );
    assert!(
        events.iter().all(|event| !matches!(
            event,
            GameEvent::CardPlayed { def_id, .. } if def_id == JADE_BEAUTY_DEF_ID
        )),
        "it is never played"
    );
}

#[test]
fn r962_a_full_row_summons_none_and_the_threshold_is_spent() {
    let mut state = game("jade-full");
    for lane in 1..=5 {
        put(
            &mut state,
            "fx-1",
            slot(PlayerId::P1, Row::Units, lane),
            json!({}),
        );
    }
    let events = add(&mut state, 5, PlayerId::P1);
    assert_eq!(state.players[PlayerId::P1].jade, Some(5));
    assert_eq!(jade_changed(&events), vec![(PlayerId::P1, 5)]);
    assert!(beauties(&state, PlayerId::P1).is_empty());
    // The counter only rises, so the spent threshold never acts again.
    add(&mut state, 1, PlayerId::P1);
    assert_eq!(state.players[PlayerId::P1].jade, Some(6));
    assert!(beauties(&state, PlayerId::P1).is_empty());
}

#[test]
fn r962_crossing_10_makes_every_jade_beauty_radiant() {
    let mut state = game("jade-ascend");
    let first = put(
        &mut state,
        JADE_BEAUTY_DEF_ID,
        slot(PlayerId::P1, Row::Units, 1),
        json!({}),
    );
    let second = put(
        &mut state,
        JADE_BEAUTY_DEF_ID,
        slot(PlayerId::P1, Row::Units, 2),
        json!({}),
    );
    // Cross 5 first (summoning a third), then 10: every Beauty they control becomes Radiant.
    let events = add(&mut state, 10, PlayerId::P1);
    let ascended = beauties(&state, PlayerId::P1);
    assert_eq!(ascended.len(), 3);
    assert!(ascended.iter().all(|card| card.radiant));
    let radiant_set: Vec<String> = events
        .iter()
        .filter_map(|event| match event {
            GameEvent::RadiantSet { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect();
    assert!(radiant_set.contains(&first.id));
    assert!(radiant_set.contains(&second.id));
}

#[test]
fn r962_crossing_10_with_none_summons_a_radiant_one() {
    let mut state = game("jade-ascend-empty");
    add(&mut state, 5, PlayerId::P1);
    assert_eq!(beauties(&state, PlayerId::P1).len(), 1);
    // The Beauty leaves the field (destroyed here: `destroy` marks, §4.5's state check collects
    // the mark and the token vanishes, R11), so 10 finds none to ascend.
    let beauty = beauties(&state, PlayerId::P1)[0].clone();
    let spell = resolving(&mut state, "fx-mana-well", PlayerId::P1);
    {
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let mut ctx = make_context(
                &mut sink,
                Some(&spell),
                HookOptions {
                    controller: Some(PlayerId::P1),
                    targets: Some(vec![]),
                    ..Default::default()
                },
            );
            (destroy(DestroyArgs {
                target: TargetSpec::Instance {
                    instance_id: beauty.id,
                },
            })
            .apply)(&mut ctx);
            state_check(&mut sink);
        }
        state.rng_cursor = rng.cursor();
    }
    assert!(beauties(&state, PlayerId::P1).is_empty());
    add(&mut state, 5, PlayerId::P1);
    assert_eq!(state.players[PlayerId::P1].jade, Some(10));
    let summoned = beauties(&state, PlayerId::P1);
    assert_eq!(summoned.len(), 1);
    assert!(summoned[0].radiant);
}

#[test]
fn r962_0_to_10_in_one_add_summons_then_makes_it_radiant() {
    let mut state = game("jade-both");
    let events = add(&mut state, 10, PlayerId::P1);
    // One Beauty: 5 summons it first, then 10 makes it Radiant.
    let summoned = beauties(&state, PlayerId::P1);
    assert_eq!(summoned.len(), 1);
    assert!(summoned[0].radiant);
    let order: Vec<&str> = events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Summoned { def_id, .. } if def_id == JADE_BEAUTY_DEF_ID => Some("summoned"),
            GameEvent::RadiantSet { .. } => Some("radiantSet"),
            _ => None,
        })
        .collect();
    assert_eq!(order, vec!["summoned", "radiantSet"]);
}

#[test]
fn r962_each_threshold_acts_once_a_game() {
    let mut state = game("jade-once");
    add(&mut state, 3, PlayerId::P1);
    assert!(beauties(&state, PlayerId::P1).is_empty());
    add(&mut state, 2, PlayerId::P1);
    assert_eq!(beauties(&state, PlayerId::P1).len(), 1);
    // Past 5, no add summons again; past 10, no add ascends again.
    add(&mut state, 1, PlayerId::P1);
    assert_eq!(beauties(&state, PlayerId::P1).len(), 1);
    add(&mut state, 4, PlayerId::P1);
    let ascended = beauties(&state, PlayerId::P1);
    assert_eq!(ascended.len(), 1);
    assert!(ascended[0].radiant);
    add(&mut state, 7, PlayerId::P1);
    assert_eq!(beauties(&state, PlayerId::P1).len(), 1);
}
