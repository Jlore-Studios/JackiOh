//! The Singularity verbs (Infinite Assault #193–#216, R4593000–R4593011): the per-set play
//! count, the turns-started read, and `transformIntoCopy`.

use jackioh_engine::effects::transform_into_copy;
use jackioh_engine::play_counts::{played_this_game_from_set, record_play};
use jackioh_engine::query::turns_started_of;
use jackioh_engine::testkit::PlayerId::P1;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::generation::playing;
use crate::rules::fixtures::harness::{live, put, sink_for, slot};

/// A vanilla Unit of the held Infinite Assault set.
fn sg_held_def() -> CardDef {
    json_as(json!({
        "id": "sg-held",
        "index": "4901",
        "name": "Held (singularity fixture)",
        "set": "Infinite Assault",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 2,
        "base": { "attack": 2, "health": 2, "keywords": [], "text": "held" },
        "radiant": { "attack": 4, "health": 4, "keywords": [], "text": "held" },
    }))
}

/// A vanilla Unit of a shipped set.
fn sg_core_def() -> CardDef {
    json_as(json!({
        "id": "sg-core",
        "index": "4902",
        "name": "Core (singularity fixture)",
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 2,
        "base": { "attack": 2, "health": 2, "keywords": [], "text": "core" },
        "radiant": { "attack": 4, "health": 4, "keywords": [], "text": "core" },
    }))
}

/// A vanilla Immutable Unit of a shipped set.
fn sg_immutable_def() -> CardDef {
    json_as(json!({
        "id": "sg-immutable",
        "index": "4903",
        "name": "Immutable (singularity fixture)",
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 2,
        "base": { "attack": 2, "health": 2, "keywords": [{ "kind": "Immutable" }], "text": "immutable" },
        "radiant": { "attack": 4, "health": 4, "keywords": [{ "kind": "Immutable" }], "text": "immutable" },
    }))
}

fn sg_scripts() -> CardScripts {
    CardScripts {
        base: Script::default(),
        radiant: Script::default(),
    }
}

fn register_sg() {
    let mut defs = registered_catalog().clone();
    for def in [sg_held_def(), sg_core_def(), sg_immutable_def()] {
        defs.insert(def.id.clone(), def);
    }
    register_catalog(defs);
    let mut scripts = registered_scripts().clone();
    for id in ["sg-held", "sg-core", "sg-immutable"] {
        scripts.insert(id.to_string(), sg_scripts());
    }
    register_scripts(scripts);
}

fn run_with_self(sink: &mut EngineSink<'_>, effect: Effect, self_: Option<&CardInstance>) {
    let mut ctx = make_context(
        sink,
        self_,
        HookOptions {
            controller: Some(P1),
            ..Default::default()
        },
    );
    (effect.apply)(&mut ctx);
}

#[test]
fn r4593000_a_single_player_sets_plays_are_counted_by_set() {
    let mut run = playing("sg-count");
    register_sg();
    let card = put(
        &mut run.state,
        &sg_held_def().id,
        slot(P1, Row::Units, 1),
        json!({}),
    );
    record_play(&mut run.state, P1, &card);
    assert_eq!(
        played_this_game_from_set(&run.state, P1, SetName::InfiniteAssault),
        1
    );
    record_play(&mut run.state, P1, &card);
    assert_eq!(
        played_this_game_from_set(&run.state, P1, SetName::InfiniteAssault),
        2
    );
}

#[test]
fn r4593000_a_shipped_cards_play_writes_no_set_count() {
    let mut run = playing("sg-shipped");
    register_sg();
    let card = put(
        &mut run.state,
        &sg_core_def().id,
        slot(P1, Row::Units, 1),
        json!({}),
    );
    record_play(&mut run.state, P1, &card);
    let log = run.state.players[P1]
        .game_log
        .clone()
        .expect("a play writes a game log");
    assert!(log.played_by_set.is_none());
    let json = serde_json::to_value(&log).expect("a game log serializes");
    assert!(json.get("playedBySet").is_none());
    assert_eq!(
        played_this_game_from_set(&run.state, P1, SetName::InfiniteAssault),
        0
    );
}

#[test]
fn r4593001_turns_started_of_counts_the_current_turn() {
    let mut run = playing("sg-turns");
    run.state.players[P1].turns_started = 3;
    assert_eq!(turns_started_of(&run.state, P1), 3);
}

#[test]
fn r4593007_transform_into_copy_makes_the_sources_copy_in_place() {
    let mut run = playing("sg-copy");
    register_sg();
    let source = put(
        &mut run.state,
        &sg_held_def().id,
        slot(P1, Row::Units, 1),
        json!({}),
    );
    must(find_instance_mut(&mut run.state, &source.id), "the source").buffs =
        AttackHealth { attack: 2, health: 1 };
    must(find_instance_mut(&mut run.state, &source.id), "the source").radiant = true;
    let source = live(&run.state, &source);
    let target = put(
        &mut run.state,
        &sg_core_def().id,
        slot(P1, Row::Units, 2),
        json!({}),
    );
    let events = {
        let mut sink = sink_for(&mut run.state);
        run_with_self(
            &mut sink,
            transform_into_copy(json_as(json!({ "instanceId": target.id }))),
            Some(&source),
        );
        sink.events.clone()
    };
    let lane = must(run.state.players.p1.units[1].clone(), "lane 2");
    assert_eq!(lane.len(), 1);
    let now = &lane[0];
    assert_eq!(now.def_id, sg_held_def().id);
    assert!(now.radiant, "the copy keeps the face");
    assert_eq!(
        now.buffs,
        AttackHealth { attack: 2, health: 1 },
        "the copy as it stands"
    );
    assert_eq!(now.owner, P1);
    assert_eq!(now.controller, P1);
    assert!(is_sick(&run.state, now), "a new body is summoning sick");
    assert!(find_instance(&run.state, &target.id).is_none());
    assert!(
        run.state.players.p1.graveyard.is_empty(),
        "nothing reaches a graveyard"
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Transformed { .. }))
            .count(),
        1
    );
}

#[test]
fn r4593007_transform_into_copy_passes_an_immutable_or_dormant_card_by() {
    let mut run = playing("sg-copy-pass");
    register_sg();
    let source = put(
        &mut run.state,
        &sg_held_def().id,
        slot(P1, Row::Units, 1),
        json!({}),
    );
    let source = live(&run.state, &source);
    let stone = put(
        &mut run.state,
        &sg_immutable_def().id,
        slot(P1, Row::Units, 2),
        json!({}),
    );
    let sleepy = put(
        &mut run.state,
        &sg_core_def().id,
        slot(P1, Row::Units, 3),
        json!({}),
    );
    must(find_instance_mut(&mut run.state, &sleepy.id), "the sleeper").slumber =
        Some(Slumber { turns: Some(2) });
    let events = {
        let mut sink = sink_for(&mut run.state);
        run_with_self(
            &mut sink,
            transform_into_copy(json_as(json!({ "instanceId": stone.id }))),
            Some(&source),
        );
        run_with_self(
            &mut sink,
            transform_into_copy(json_as(json!({ "instanceId": sleepy.id }))),
            Some(&source),
        );
        sink.events.clone()
    };
    assert_eq!(live(&run.state, &stone).def_id, sg_immutable_def().id);
    assert_eq!(live(&run.state, &sleepy).def_id, sg_core_def().id);
    assert!(
        events
            .iter()
            .all(|event| !matches!(event, GameEvent::Transformed { .. })),
        "neither is rewritten"
    );
}
