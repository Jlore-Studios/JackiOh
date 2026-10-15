//! R923000–R923003: the practice sandbox's tools (issue #923, `sandbox.rs`). A sandbox game alone
//! takes them, from either seat on either turn, logged and folded like any action; `legal_actions`
//! never lists one. A spawned card is placed, not played; the numbers stay in their bounds; destroy
//! runs a card's Death and remove does not.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::{spell_def, unit_def, vanilla_catalog, vanilla_deck};

const CRY_UNIT: &str = "sb-cry-unit";
const DEATH_UNIT: &str = "sb-death-unit";
const TRAP: &str = "sb-trap";
const SPELL: &str = "sb-spell";
const BOSS: &str = "sb-boss";
const TOKEN: &str = "fx-token-rush";

/// The vanilla units and the Rush token, plus a unit whose Cry draws, one whose Death draws, a Trap,
/// a Spell and a unit of a set that does not ship.
fn setup() {
    let mut defs = vanilla_catalog(40, 1);
    for def in [
        unit_def(901, json!({ "id": CRY_UNIT, "name": "Cry Drawer" })),
        unit_def(902, json!({ "id": DEATH_UNIT, "name": "Death Drawer" })),
        spell_def(903, json!({ "id": TRAP, "name": "Plain Trap", "type": "Trap" })),
        spell_def(904, json!({ "id": SPELL, "name": "Plain Spell" })),
        unit_def(905, json!({ "id": BOSS, "name": "Boss Unit", "set": "Boss" })),
    ] {
        defs.insert(def.id.clone(), def);
    }
    register_catalog(defs);
    let draws = || hook(|_ctx| vec![jackioh_engine::effects::draw(json_as(json!({ "count": 1 })))]);
    let mut scripts: IndexMap<String, CardScripts> = IndexMap::new();
    scripts.insert(
        CRY_UNIT.to_string(),
        CardScripts {
            base: Script {
                cry: Some(draws()),
                ..Script::default()
            },
            radiant: Script {
                cry: Some(draws()),
                ..Script::default()
            },
        },
    );
    scripts.insert(
        DEATH_UNIT.to_string(),
        CardScripts {
            base: Script {
                death: Some(draws()),
                ..Script::default()
            },
            radiant: Script {
                death: Some(draws()),
                ..Script::default()
            },
        },
    );
    register_scripts(scripts);
}

/// A game past both mulligans, P1 to act on turn 1: a sandbox when `sandbox`.
fn started(seed: &str, sandbox: bool) -> (GameState, Vec<Action>) {
    setup();
    let state = create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21)),
        sandbox: sandbox.then_some(true),
        ..CreateGameOptions::default()
    });
    let mut state = begin_game(&state).state;
    let mut log = Vec::new();
    for (player, nonce) in [(P1, "m1"), (P2, "m2")] {
        let keep: Vec<String> = state.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        let action = Action::new(ActionBody::Mulligan { keep }, player, nonce);
        let out = reduce(&state, &action);
        assert_eq!(out.error, None, "the mulligan is answered");
        state = out.state;
        log.push(action);
    }
    assert_eq!(state.active, P1);
    assert!(state.pending.is_none() && state.mulligan.is_none());
    // Nothing ends the turn by itself under the tools (R82).
    state.players[P1].auto_end_turn = Some(false);
    state.players[P2].auto_end_turn = Some(false);
    (state, log)
}

fn tool(op: Value, player: PlayerId, nonce: &str) -> Action {
    Action::new(ActionBody::Sandbox { op: json_as(op) }, player, nonce)
}

/// Apply `action`, which must be accepted, and log it.
fn apply(state: &mut GameState, log: &mut Vec<Action>, action: Action) -> Vec<GameEvent> {
    let out = reduce(state, &action);
    assert_eq!(out.error, None, "{action:?} is accepted");
    *state = out.state;
    log.push(action);
    out.events
}

fn refused(state: &GameState, action: &Action) -> String {
    let out = reduce(state, action);
    assert_eq!(&out.state, state, "a refusal leaves the state as it was");
    out.error
        .unwrap_or_else(|| panic!("{action:?} should have been refused"))
}

#[test]
fn r923000_a_tool_is_refused_outside_a_sandbox_game_and_never_listed() {
    let (state, _) = started("r923000-off", false);
    let spawn = tool(
        json!({ "kind": "spawn", "player": "p1", "defId": "fx-5", "to": { "zone": "hand" } }),
        P1,
        "s1",
    );
    assert_eq!(
        refused(&state, &spawn),
        "the sandbox's tools work only in a sandbox game"
    );

    let (sandbox, _) = started("r923000-on", true);
    assert_eq!(sandbox.sandbox, Some(true));
    assert_eq!(
        state.sandbox, None,
        "a game that is no sandbox carries no flag, so it hashes as before"
    );
    for player in PLAYER_IDS {
        assert!(
            legal_actions(&sandbox, player)
                .iter()
                .all(|body| !matches!(body, ActionBody::Sandbox { .. })),
            "legalActions lists no tool for {player}, so no policy takes one"
        );
    }
}

#[test]
fn r923000_either_seat_uses_a_tool_on_either_turn_and_the_log_folds_with_the_flag() {
    let (mut state, mut log) = started("r923000-fold", true);
    let seed = state.seed.clone();
    // P2 off its turn, then P1 on its own.
    apply(
        &mut state,
        &mut log,
        tool(
            json!({ "kind": "spawn", "player": "p2", "defId": "fx-7", "to": { "zone": "field", "row": "units", "lane": 2 } }),
            P2,
            "s1",
        ),
    );
    apply(
        &mut state,
        &mut log,
        tool(
            json!({ "kind": "setHero", "player": "p2", "health": 12, "armor": 3 }),
            P1,
            "s2",
        ),
    );
    // A tool's nonce dedupes like any other action's.
    let again = reduce(&state, log.last().expect("a logged tool"));
    assert_eq!(again.error, None);
    assert_eq!(again.state, state);

    let input = ReplayInput {
        seed: seed.clone(),
        decks: (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21)),
        log: log.clone(),
        sandbox: Some(true),
        ..ReplayInput::default()
    };
    let folded = fold(&input);
    assert!(folded.errors.is_empty(), "{:?}", folded.errors);
    // The live state had R82's switch turned off by hand; the fold's did not, which no tool reads.
    let mut live = state.clone();
    live.players[P1].auto_end_turn = None;
    live.players[P2].auto_end_turn = None;
    assert_eq!(hash_state(&folded.state), hash_state(&live));

    // Folded without the flag, the tools are refused: the flag is setup the replay must carry.
    let plain = fold(&ReplayInput {
        sandbox: None,
        ..input
    });
    assert_eq!(plain.errors.len(), 2);
}

#[test]
fn r923000_a_tool_waits_on_the_mulligan() {
    setup();
    let state = create_game(&CreateGameOptions {
        seed: "r923000-wait".to_string(),
        decks: (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21)),
        sandbox: Some(true),
        ..CreateGameOptions::default()
    });
    let mulligan = begin_game(&state).state;
    let draw = tool(json!({ "kind": "draw", "player": "p1", "count": 1 }), P1, "s1");
    assert_eq!(refused(&mulligan, &draw), "the mulligan is open: answer it first");
}

#[test]
fn r923001_a_spawned_card_is_placed_not_played() {
    let (mut state, mut log) = started("r923001-place", true);
    let hand = state.players[P1].hand.len();
    let played = state.counters.played;

    // On the field: summoned, its Cry not run (no draw), sick as any summon, and no play counted.
    let events = apply(
        &mut state,
        &mut log,
        tool(
            json!({ "kind": "spawn", "player": "p1", "defId": CRY_UNIT, "radiant": true, "to": { "zone": "field", "row": "units", "lane": 1 } }),
            P1,
            "s1",
        ),
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, GameEvent::Summoned { def_id, lane: 1, .. } if def_id == CRY_UNIT))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, GameEvent::Drawn { .. })),
        "no Cry: {events:?}"
    );
    assert_eq!(state.players[P1].hand.len(), hand);
    assert_eq!(state.counters.played, played);
    let placed = state.players[P1].units[0]
        .as_ref()
        .and_then(|pile| pile.last())
        .expect("a unit in lane 1")
        .clone();
    assert!(placed.radiant);
    assert_eq!(placed.summoned_turn, Some(state.turn));

    // A Trap into the backrow is face-down, as a summon leaves one.
    apply(
        &mut state,
        &mut log,
        tool(
            json!({ "kind": "spawn", "player": "p2", "defId": TRAP, "to": { "zone": "field", "row": "backrow", "lane": 3 } }),
            P1,
            "s2",
        ),
    );
    let trap = state.players[P2].backrow[2].clone().expect("a trap in lane 3");
    assert_eq!(trap.def_id, TRAP);
    assert_ne!(trap.face_up, Some(true));

    // Into the hand, a token included.
    apply(
        &mut state,
        &mut log,
        tool(
            json!({ "kind": "spawn", "player": "p1", "defId": TOKEN, "to": { "zone": "hand" } }),
            P1,
            "s3",
        ),
    );
    assert_eq!(
        state.players[P1].hand.last().map(|card| card.def_id.as_str()),
        Some(TOKEN)
    );

    // Into the graveyard, and on top of the library: the next card drawn.
    apply(
        &mut state,
        &mut log,
        tool(
            json!({ "kind": "spawn", "player": "p2", "defId": SPELL, "to": { "zone": "graveyard" } }),
            P1,
            "s4",
        ),
    );
    assert_eq!(
        state.players[P2]
            .graveyard
            .last()
            .map(|card| card.def_id.as_str()),
        Some(SPELL)
    );
    apply(
        &mut state,
        &mut log,
        tool(
            json!({ "kind": "spawn", "player": "p1", "defId": "fx-33", "to": { "zone": "libraryTop" } }),
            P1,
            "s5",
        ),
    );
    assert_eq!(
        state.players[P1].library.first().map(|card| card.def_id.as_str()),
        Some("fx-33")
    );
    apply(
        &mut state,
        &mut log,
        tool(json!({ "kind": "draw", "player": "p1", "count": 1 }), P1, "s6"),
    );
    assert_eq!(
        state.players[P1].hand.last().map(|card| card.def_id.as_str()),
        Some("fx-33")
    );
}

#[test]
fn r923001_a_spawn_names_a_shipped_card_and_an_open_zone_its_type_stands_in() {
    let (mut state, mut log) = started("r923001-refuse", true);
    let spawn = |def: &str, to: Value| {
        tool(
            json!({ "kind": "spawn", "player": "p1", "defId": def, "to": to }),
            P1,
            "x",
        )
    };
    assert_eq!(
        refused(&state, &spawn("nope", json!({ "zone": "hand" }))),
        "there is no card \"nope\" to spawn"
    );
    assert_eq!(
        refused(&state, &spawn(BOSS, json!({ "zone": "hand" }))),
        "there is no card \"sb-boss\" to spawn",
        "a set that does not ship is no card a player may hold (R1420)"
    );
    assert_eq!(
        refused(
            &state,
            &spawn(SPELL, json!({ "zone": "field", "row": "backrow", "lane": 1 }))
        ),
        "Plain Spell is a Spell, which is cast and never stands on the field"
    );
    assert_eq!(
        refused(
            &state,
            &spawn(TRAP, json!({ "zone": "field", "row": "units", "lane": 1 }))
        ),
        "Plain Trap stands in the backrow row"
    );
    assert_eq!(
        refused(
            &state,
            &spawn("fx-3", json!({ "zone": "field", "row": "units", "lane": 9 }))
        ),
        "there is no lane 9 in that row"
    );
    apply(
        &mut state,
        &mut log,
        spawn("fx-3", json!({ "zone": "field", "row": "units", "lane": 2 })),
    );
    let mut taken = spawn("fx-4", json!({ "zone": "field", "row": "units", "lane": 2 }));
    taken.nonce = "y".to_string();
    assert_eq!(refused(&state, &taken), "that zone is taken");
}

#[test]
fn r923002_a_hero_and_mana_are_set_within_their_bounds_and_a_draw_draws() {
    let (mut state, mut log) = started("r923002-numbers", true);
    let events = apply(
        &mut state,
        &mut log,
        tool(
            json!({ "kind": "setHero", "player": "p1", "health": SANDBOX_HERO_HEALTH_MAX, "armor": 0 }),
            P1,
            "s1",
        ),
    );
    assert_eq!(state.players[P1].hero.health, SANDBOX_HERO_HEALTH_MAX);
    assert_eq!(state.players[P1].hero.armor, 0);
    assert!(events.contains(&GameEvent::HealthSet {
        player: P1,
        health: SANDBOX_HERO_HEALTH_MAX,
        source_id: None
    }));
    let events = apply(
        &mut state,
        &mut log,
        tool(
            json!({ "kind": "setMana", "player": "p2", "current": 7, "max": SANDBOX_MANA_MAX }),
            P1,
            "s2",
        ),
    );
    assert_eq!(
        (state.players[P2].mana.current, state.players[P2].mana.max),
        (7, SANDBOX_MANA_MAX)
    );
    assert!(events.contains(&GameEvent::ManaChanged {
        player: P2,
        current: 7,
        max: SANDBOX_MANA_MAX
    }));
    let library = state.players[P2].library.len();
    apply(
        &mut state,
        &mut log,
        tool(json!({ "kind": "draw", "player": "p2", "count": 3 }), P1, "s3"),
    );
    assert_eq!(state.players[P2].library.len(), library - 3);

    for (op, message) in [
        (
            json!({ "kind": "setHero", "player": "p1", "health": 0 }),
            "a hero's health must be from 1 to 99",
        ),
        (
            json!({ "kind": "setHero", "player": "p1", "armor": SANDBOX_ARMOR_MAX + 1 }),
            "a hero's Armor must be from 0 to 99",
        ),
        (
            json!({ "kind": "setHero", "player": "p1" }),
            "set a health or an Armor",
        ),
        (
            json!({ "kind": "setMana", "player": "p1", "current": -1 }),
            "current mana must be from 0 to 10",
        ),
        (
            json!({ "kind": "setMana", "player": "p1", "max": SANDBOX_MANA_MAX + 1 }),
            "maximum mana must be from 0 to 10",
        ),
        (
            json!({ "kind": "draw", "player": "p1", "count": 0 }),
            "a draw must be from 1 to 10",
        ),
        (
            json!({ "kind": "draw", "player": "p1", "count": SANDBOX_DRAW_MAX + 1 }),
            "a draw must be from 1 to 10",
        ),
    ] {
        assert_eq!(refused(&state, &tool(op.clone(), P1, "x")), message, "{op}");
    }
}

#[test]
fn r923003_destroy_runs_a_card_s_death_and_remove_exiles_it_without_one() {
    let (mut state, mut log) = started("r923003", true);
    for (nonce, lane) in [("s1", 1), ("s2", 2)] {
        apply(
            &mut state,
            &mut log,
            tool(
                json!({ "kind": "spawn", "player": "p2", "defId": DEATH_UNIT, "to": { "zone": "field", "row": "units", "lane": lane } }),
                P1,
                nonce,
            ),
        );
    }
    let id_in = |state: &GameState, lane: usize| {
        state.players[P2].units[lane]
            .as_ref()
            .and_then(|pile| pile.last())
            .map(|card| card.id.clone())
            .expect("a unit")
    };
    let (first, second) = (id_in(&state, 0), id_in(&state, 1));
    let hand = state.players[P2].hand.len();

    let events = apply(
        &mut state,
        &mut log,
        tool(json!({ "kind": "destroy", "instanceId": first }), P1, "s3"),
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, GameEvent::Destroyed { .. })),
        "{events:?}"
    );
    assert_eq!(
        state.players[P2].graveyard.last().map(|card| card.id.as_str()),
        Some(first.as_str())
    );
    assert_eq!(state.players[P2].hand.len(), hand + 1, "its Death drew a card");

    let events = apply(
        &mut state,
        &mut log,
        tool(json!({ "kind": "remove", "instanceId": second }), P1, "s4"),
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, GameEvent::Exiled { .. }))
    );
    assert_eq!(
        state.players[P2].exile.last().map(|card| card.id.as_str()),
        Some(second.as_str())
    );
    assert_eq!(state.players[P2].hand.len(), hand + 1, "no Death ran");

    // Destroy takes a card on the field, remove one anywhere but exile or mid-resolution.
    let in_hand = state.players[P1].hand[0].id.clone();
    assert_eq!(
        refused(
            &state,
            &tool(json!({ "kind": "destroy", "instanceId": in_hand }), P1, "x")
        ),
        "destroy a card on the field"
    );
    assert_eq!(
        refused(
            &state,
            &tool(json!({ "kind": "remove", "instanceId": second }), P1, "y")
        ),
        "remove a card in a hand, a library, a graveyard or on the field"
    );
    apply(
        &mut state,
        &mut log,
        tool(json!({ "kind": "remove", "instanceId": in_hand }), P1, "s5"),
    );
    assert!(state.players[P1].exile.iter().any(|card| card.id == in_hand));
}
