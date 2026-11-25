//! The Commanders' engine pieces (Infinite Assault #217–#240, R4594001, R4594002, R4594004,
//! R4594005, R4594009, R4594014): the face-down destroy guard, the "this turn" lends, the
//! Reborn-return read, the recursion log's Units played and deaths, and Zero-Day's scoped random
//! Spell targets. Each is one verb or read under `run_effect`, except the guard and the history,
//! which need a real settle, and the Spell-target read, which needs a real listing.

use jackioh_engine::effects::{
    destroy, give_units_attack, give_units_attack_this_turn, give_units_keyword_this_turn,
};
use jackioh_engine::modifiers::expire_modifiers;
use jackioh_engine::preview::is_face_down;
use jackioh_engine::recursion_log::{RecursionLog, deaths_this_game, returned_by_reborn, units_played_by};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;
use jackioh_engine::{make_context, record_play};

use crate::rules::fixtures::damage_combat::{bolt, gambit, grunt, playing, reborn_veteran};
use crate::rules::fixtures::harness::{by, in_hand, live, new_game, put, run_effect, slot};

/// A Unit whose static flags guard its controller's face-down cards (Infinite Assault #218
/// Thousand Faces' shape), registered beside the fixtures by the guard test.
fn guard_def() -> CardDef {
    json_as(json!({
        "id": "hx-guard",
        "index": "4901",
        "name": "Guard (commanders fixture)",
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 3,
        "base": { "attack": 3, "health": 8, "keywords": [], "text": "guard" },
        "radiant": { "attack": 6, "health": 16, "keywords": [], "text": "guard" },
    }))
}

/// A token Unit, for the history's "tokens in".
fn token_def() -> CardDef {
    json_as(json!({
        "id": "hx-token",
        "index": "4902",
        "name": "Token (commanders fixture)",
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": true,
        "cost": 1,
        "base": { "attack": 1, "health": 1, "keywords": [], "text": "token" },
        "radiant": { "attack": 2, "health": 2, "keywords": [], "text": "token" },
    }))
}

fn guard_scripts() -> CardScripts {
    let guard = Script {
        static_flags: Some(StaticFlags {
            face_down_indestructible: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    CardScripts {
        base: guard.clone(),
        radiant: guard,
    }
}

fn register_guard() {
    let mut defs = registered_catalog().clone();
    defs.insert(guard_def().id.clone(), guard_def());
    defs.insert(token_def().id.clone(), token_def());
    register_catalog(defs);
    let mut scripts = registered_scripts().clone();
    scripts.insert(guard_def().id.clone(), guard_scripts());
    scripts.insert(token_def().id.clone(), CardScripts::default());
    register_scripts(scripts);
}

fn with_sink<R>(state: &mut GameState, run: impl FnOnce(&mut EngineSink) -> R) -> R {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    run(&mut sink)
}

fn destroy_instance(state: &mut GameState, id: &str, controller: PlayerId) {
    run_effect(
        state,
        destroy(json_as(
            json!({ "target": { "of": "instance", "instanceId": id } }),
        )),
        controller,
    );
}

fn settle(state: &mut GameState) {
    with_sink(state, |sink| {
        jackioh_engine::settle(sink, Default::default());
    });
}

#[test]
fn r4594001_a_guarded_face_down_card_survives_any_destroy() {
    let mut state = playing("hx-commanders-guard");
    register_guard();
    put(&mut state, &guard_def().id, slot(P1, Row::Units, 1), json!({}));
    let facedown = put(&mut state, &gambit.id, slot(P1, Row::Backrow, 1), json!({}));
    let faceup = put(&mut state, &grunt.id, slot(P1, Row::Units, 2), json!({}));
    assert!(is_face_down(&state, &live(&state, &facedown.id)));
    // Either side's destroy stops at the guard: the mark the destroy leaves, the check clears.
    for controller in [P2, P1] {
        destroy_instance(&mut state, &facedown.id, controller);
        settle(&mut state);
        live(&state, &facedown.id);
    }
    // A face-up card has no guard: it dies.
    destroy_instance(&mut state, &faceup.id, P2);
    settle(&mut state);
    assert!(
        state.players[P1]
            .graveyard
            .iter()
            .any(|card| card.id == faceup.id)
    );
}

#[test]
fn r4594002_units_attack_this_turn_reaches_later_units_and_expires() {
    let mut state = playing("hx-commanders-lend");
    run_effect(
        &mut state,
        give_units_attack_this_turn(json_as(json!({ "amount": 2 }))),
        P1,
    );
    // A Unit arriving after the lend reads it as it lands.
    let unit = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), json!({}));
    assert_eq!(unit_view(&state, &live(&state, &unit.id)).attack, 2 + 2);
    // The cleanup takes it back.
    with_sink(&mut state, |sink| expire_modifiers(sink, P1));
    assert_eq!(unit_view(&state, &live(&state, &unit.id)).attack, 2);
}

#[test]
fn r4594002_a_spells_lend_passes_immune_to_spells_by() {
    let mut state = playing("hx-commanders-immune");
    let immune = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), json!({}));
    find_instance_mut(&mut state, &immune.id)
        .expect("on the field")
        .granted_keywords
        .push(Keyword::ImmuneToSpells);
    // A lend from no Spell reaches a Unit Immune to Spells (R4508000).
    run_effect(&mut state, give_units_attack(json_as(json!({ "amount": 2 }))), P1);
    assert_eq!(unit_view(&state, &live(&state, &immune.id)).attack, 2 + 2);
    // A Spell's this-turn lend passes it by.
    let spell = in_hand(&mut state, &bolt.id, P1, 1).remove(0);
    with_sink(&mut state, |sink| {
        let mut ctx = make_context(sink, Some(&spell), by(P1));
        (give_units_attack_this_turn(json_as(json!({ "amount": 3 }))).apply)(&mut ctx);
    });
    assert_eq!(unit_view(&state, &live(&state, &immune.id)).attack, 2 + 2);
    // And so does a Spell's keyword lend.
    let spell = in_hand(&mut state, &bolt.id, P1, 1).remove(0);
    with_sink(&mut state, |sink| {
        let mut ctx = make_context(sink, Some(&spell), by(P1));
        (give_units_keyword_this_turn(json_as(json!({ "keyword": { "kind": "Charge" } }))).apply)(&mut ctx);
    });
    assert!(
        !unit_view(&state, &live(&state, &immune.id))
            .keywords
            .iter()
            .any(|keyword| keyword.kind() == KeywordKind::Charge)
    );
}

#[test]
fn r4594004_returned_by_reborn_reads_a_reborn_return() {
    let mut state = playing("hx-commanders-reborn");
    let veteran = put(&mut state, &reborn_veteran.id, slot(P1, Row::Units, 1), json!({}));
    assert!(
        !returned_by_reborn(&live(&state, &veteran.id)),
        "a plain arrival is no return"
    );
    destroy_instance(&mut state, &veteran.id, P2);
    settle(&mut state);
    let back = live(&state, &veteran.id);
    assert_eq!(unit_view(&state, &back).health, 1, "Reborn at 1 health");
    assert!(returned_by_reborn(&back), "the return reads");
}

#[test]
fn r4594005_deaths_are_logged_by_controller() {
    let mut state = assault(playing("hx-commanders-deaths"));
    register_guard();
    let mine = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), json!({}));
    let token = put(&mut state, &token_def().id, slot(P1, Row::Units, 2), json!({}));
    let theirs = put(&mut state, &grunt.id, slot(P2, Row::Units, 1), json!({}));
    destroy_instance(&mut state, &mine.id, P2);
    destroy_instance(&mut state, &token.id, P2);
    destroy_instance(&mut state, &theirs.id, P1);
    settle(&mut state);
    // One record per death under the card's controller (R172), tokens in.
    let mine_log = deaths_this_game(&state, P1);
    assert_eq!(mine_log.len(), 2);
    assert_eq!(mine_log[0].def_id, grunt.id);
    assert_eq!(mine_log[0].turn, state.turn);
    assert_eq!(mine_log[1].def_id, token_def().id);
    assert_eq!(deaths_this_game(&state, P2).len(), 1);
    // A discard is no death: straight to the graveyard, never collected.
    let hand = in_hand(&mut state, &grunt.id, P1, 1).remove(0);
    let player = &mut state.players[P1];
    let at = player
        .hand
        .iter()
        .position(|card| card.id == hand.id)
        .expect("in hand");
    let discarded = player.hand.remove(at);
    player.graveyard.push(discarded);
    assert_eq!(
        deaths_this_game(&state, P1).len(),
        2,
        "the discard logged nothing"
    );
}

/// `state` keeping the recursion log, as a game holding an Infinite Assault card does (R4589000):
/// the one history the Commanders read (R4594005, R4594009).
fn assault(mut state: GameState) -> GameState {
    state.players.p1.recursion = Some(RecursionLog::default());
    state
}

/// A Unit into `state`'s hand.
fn hold(state: &mut GameState, player: PlayerId, def_id: &str) -> CardInstance {
    in_hand(state, def_id, player, 1)
        .into_iter()
        .next()
        .expect("no card")
}

/// `body` with `playerId` and a nonce, reduced; a refusal panics.
fn reduce_ok(state: &GameState, player_id: PlayerId, body: Value) -> ReduceResult {
    let mut action = body;
    action["playerId"] = json!(player_id);
    action["nonce"] = json!(crate::rules::fixtures::harness::next_nonce());
    let result = reduce(state, &json_as::<Action>(action));
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

/// A game in p1's first main phase with the play pipeline's fixtures registered.
fn pipeline_game(seed: &str) -> GameState {
    use crate::rules::fixtures::play_pipeline_a::with_play_a;
    let mut ready = begin_game(&with_play_a(new_game(seed, None))).state;
    for player in [P1, P2] {
        let keep: Vec<String> = ready.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        ready = reduce_ok(&ready, player, json!({ "type": "mulligan", "keep": keep })).state;
    }
    for player in [P1, P2] {
        ready.players[player].mana.current = 8;
    }
    assault(ready)
}

#[test]
fn r4594009_units_played_are_logged() {
    use crate::rules::fixtures::play_pipeline_a::PA;
    // A resolved Unit play logs; a countered one never resolves, so nothing logs (R448).
    let mut state = pipeline_game("hx-commanders-plays");
    let crier = hold(&mut state, P1, &PA.crier.id);
    let after = reduce_ok(&state, P1, json!({ "type": "play", "instanceId": crier.id })).state;
    assert_eq!(units_played_by(&after, P1), vec![PA.crier.id.clone()]);
    assert!(units_played_by(&after, P2).is_empty());
    // A countered Unit play never gets here (R448): nothing was resolved, nothing logs.
    let mut trapped = pipeline_game("hx-commanders-countered");
    put(
        &mut trapped,
        &PA.counter_trap.id,
        slot(P2, Row::Backrow, 1),
        Default::default(),
    );
    let crier = hold(&mut trapped, P1, &PA.crier.id);
    let after = reduce_ok(&trapped, P1, json!({ "type": "play", "instanceId": crier.id })).state;
    assert!(units_played_by(&after, P1).is_empty());
}

#[test]
fn r4594009_a_game_without_an_assault_card_logs_nothing() {
    // No Infinite Assault card, no log (R4589000): the state, and so the hash, stays as it was.
    let mut state = playing("hx-commanders-hash");
    let unit = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), json!({}));
    let played = live(&state, &unit.id);
    record_play(&mut state, P1, &played);
    destroy_instance(&mut state, &unit.id, P2);
    settle(&mut state);
    assert!(units_played_by(&state, P1).is_empty());
    assert!(deaths_this_game(&state, P1).is_empty());
    assert!(state.players.p1.recursion.is_none() && state.players.p2.recursion.is_none());
}

#[test]
fn r4594014_a_flagged_enemy_unit_randomizes_spell_targets() {
    jackioh_cards::register_all();
    let s = jackioh_engine::testkit::scenario(json!({
        "seed": "hx-commanders-zero-day",
        "active": "p1",
        "p1": {
            "hand": ["core-035", "core-008"],
            "field": ["core-008"],
            "library": ["core-005", "core-005"],
        },
        "p2": {
            "field": [{ "def": "infiniteassault-239" }],
            "hand": ["core-005"],
            "library": ["core-005", "core-005"],
        },
    }));
    // The opponent's targeted Spell lists once, with its targets drawn, not declared.
    let plays = s.plays_of("core-035");
    assert_eq!(plays.len(), 1);
    assert!(
        matches!(&plays[0], ActionBody::Play { targets: None, .. }),
        "no declared targets"
    );
    // Units keep theirs: the field unit's attacks list each target, the hero and Zero-Day.
    let attacker = s.unit_at(P1, 1);
    assert_eq!(s.attacks_of(&attacker).len(), 2);
}
