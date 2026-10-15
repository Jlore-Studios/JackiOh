//! Duos co-op in the engine (#936; the frozen contract is `.fullsend/SPEC.md`, Surface S1–S4 and
//! behaviours B1–B25). Two plain 1v1 duels, A1 vs B1 and A2 vs B2, held by a `DuosState` that adds a
//! shared life pool per team, passing between teammates and synchronised turn ends. Every test names
//! the behaviour it proves (`b<n>_…`).
//!
//! The engine does not depend on the real cards, so every game here deals vanilla fixture decks (one
//! disjoint block of twenty ids per seat, so a card's id says whose deck it came from) and plays this
//! file's own fixture cards: zero-cost Spells that hit, heal, swap, end the turn, take an extra turn or
//! win, a lethal-hit redirect, a card with an arrival hook and a 5/5. They are registered through the
//! testkit's per-thread override and handed to `duos_create` as `DuosCreateOptions.catalog`.
//!
//! A refusal is checked as the engine's contract has it: an error, and a state that hashes as before.

use std::sync::atomic::{AtomicU32, Ordering};

use serde::Serialize;

use jackioh_engine::config::{
    DUOS_HUMAN_TEAM, DUOS_OWN_HEALTH, DUOS_PASS_CAP, DUOS_PASS_COST, DUOS_POOL_HEALTH, DUOS_POOL_HEALTH_HARD,
    duos_ai_team,
};
use jackioh_engine::duos::{
    DUOS_NONCE_PREFIX, DuosReduceResult, DuosReplayInput, DuosState, duos_begin, duos_create, duos_fold,
    duos_hash, duos_legal_actions, duos_reduce, duos_scenario, duos_seats_to_act, duos_view_for,
};
use jackioh_engine::effects;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::DuosSeat::{A1, A2, B1, B2};
use jackioh_engine::wire::{
    DUOS_SEATS, DuosAction, DuosActionBody, DuosCreateOptions, DuosDecks, DuosEvent, DuosHandicaps,
    DuosOutcome, DuosSeat, DuosTeamRules, PerTeam, Team,
};

use crate::rules::fixtures::catalog::{vanilla_catalog, vanilla_deck};
use crate::rules::fixtures::harness::{play_random_game, put, setup_catalog, slot};

// ---------------------------------------------------------------------------------------------
// Fixture cards
// ---------------------------------------------------------------------------------------------

/// A Spell that does nothing: a move a seat can always make, so the wrapper never ends it as idle.
const NOOP: &str = "fx-duos-noop";
/// BOLT_DAMAGE to the enemy hero.
const BOLT: &str = "fx-duos-bolt";
/// NUKE_DAMAGE to the enemy hero: more than a fresh seat's own HP.
const NUKE: &str = "fx-duos-nuke";
/// NUKE_DAMAGE to your own hero.
const BACKFIRE: &str = "fx-duos-backfire";
/// NUKE_DAMAGE to the enemy hero, then NUKE_DAMAGE to your own, in one effect list.
const BLAST: &str = "fx-duos-blast";
/// The enemy hero loses DRAIN_LOSS health (life loss, not damage).
const DRAIN: &str = "fx-duos-drain";
/// Heal your hero MEND_HEAL.
const MEND: &str = "fx-duos-mend";
/// The two heroes' health values change places.
const SWAP: &str = "fx-duos-swap";
/// Your turn ends once this has resolved.
const TEMPO: &str = "fx-duos-tempo";
/// Take an extra turn after this one.
const AGAIN: &str = "fx-duos-again";
/// You win the game.
const VICTORY: &str = "fx-duos-victory";
/// Your opponent wins the game.
const CONCESSION: &str = "fx-duos-concession";
/// A Field Trap: a lethal hit on its hero goes to the enemy hero instead, and the trap stays.
const GAMBIT: &str = "fx-duos-gambit";
/// A 1-cost Unit: when it enters a hand, TRIPWIRE_DAMAGE to its controller's hero.
const TRIPWIRE: &str = "fx-duos-tripwire";
/// A 1-cost 5/5 Unit.
const BRUTE: &str = "fx-duos-brute";
/// A 1-cost Unit: at the end of its controller's turn, NUKE_DAMAGE to its controller's hero.
const EMBER: &str = "fx-duos-ember";
/// A 1-cost Unit: at the start of its controller's turn, that turn ends (R456, as a Tommy Tempo drawn
/// at its start ends it).
const STALL: &str = "fx-duos-stall";

const BOLT_DAMAGE: i32 = 6;
const NUKE_DAMAGE: i32 = 25;
const DRAIN_LOSS: i32 = 3;
const MEND_HEAL: i32 = 10;
const TRIPWIRE_DAMAGE: i32 = 3;
const BRUTE_ATTACK: i32 = 5;

/// What a fresh seat has left once a NUKE has spent its own HP: the pool less the overflow.
const POOL_AFTER_NUKE: i32 = DUOS_POOL_HEALTH - (NUKE_DAMAGE - DUOS_OWN_HEALTH);

/// The steps a random Duos game may take before the fuzz calls it stuck.
const MAX_FUZZ_STEPS: usize = 20_000;

/// The random-policy games B25's test plays here. B25's 200 seeds are `cargo jackioh fuzz --duos`'s
/// (its default), on the real catalog; 200 in this binary took longer than every other rules test
/// together, so the `rules` group plays a tenth of them on the fixture cards.
const FUZZ_SEEDS: usize = 20;

fn face(text: &str) -> Value {
    json!({ "keywords": [], "text": text })
}

fn spell(id: &str, index: &str, text: &str) -> CardDef {
    json_as(json!({
        "id": id, "index": index, "name": format!("Duos {text}"), "set": "Core", "type": "Spell",
        "tags": [], "rarity": "Common", "token": false, "cost": 0,
        "base": face(text), "radiant": face(text),
    }))
}

fn unit(id: &str, index: &str, attack: i32, health: i32, text: &str) -> CardDef {
    json_as(json!({
        "id": id, "index": index, "name": format!("Duos {text}"), "set": "Core", "type": "Unit",
        "tags": [], "rarity": "Common", "token": false, "cost": 1,
        "base": { "attack": attack, "health": health, "keywords": [], "text": text },
        "radiant": { "attack": attack * 2, "health": health * 2, "keywords": [], "text": text },
    }))
}

fn field_trap(id: &str, index: &str, text: &str) -> CardDef {
    json_as(json!({
        "id": id, "index": index, "name": format!("Duos {text}"), "set": "Core", "type": "Field Trap",
        "tags": [], "rarity": "Common", "token": false, "cost": 0,
        "base": face(text), "radiant": face(text),
    }))
}

/// A stand-in for the catalog's The Coin, as `setup.rs` builds it: the engine deals the id only.
fn coin() -> CardDef {
    json_as(json!({
        "id": COIN_DEF_ID, "index": "T-coin", "name": "Fixture Coin", "set": "Core", "type": "Spell",
        "tags": ["Token"], "rarity": "Token", "token": true, "cost": 0,
        "base": face("gain 1 mana"), "radiant": face("gain 2 mana"),
    }))
}

/// Eighty vanilla units (`fx-1` … `fx-80`), The Coin and this file's cards.
fn catalog() -> CardDefs {
    let mut defs = vanilla_catalog(80, 1);
    for def in [
        coin(),
        spell(NOOP, "D1", "nothing"),
        spell(BOLT, "D2", "bolt"),
        spell(NUKE, "D3", "nuke"),
        spell(BACKFIRE, "D4", "backfire"),
        spell(BLAST, "D5", "blast"),
        spell(DRAIN, "D6", "drain"),
        spell(MEND, "D7", "mend"),
        spell(SWAP, "D8", "swap"),
        spell(TEMPO, "D9", "tempo"),
        spell(AGAIN, "D10", "again"),
        spell(VICTORY, "D11", "victory"),
        spell(CONCESSION, "D12", "concession"),
        field_trap(GAMBIT, "D13", "gambit"),
        unit(TRIPWIRE, "D14", 1, 1, "tripwire"),
        unit(BRUTE, "D15", BRUTE_ATTACK, 5, "brute"),
        unit(EMBER, "D16", 1, 1, "ember"),
        unit(STALL, "D17", 1, 1, "stall"),
    ] {
        defs.insert(def.id.clone(), def);
    }
    defs
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// A Spell whose resolution is `make`'s effects.
fn on_play(make: fn() -> Vec<Effect>) -> CardScripts {
    both(Script {
        cry: Some(hook(move |_ctx| make())),
        ..Script::default()
    })
}

fn hit(who: &str, amount: i32) -> Effect {
    effects::damage(json_as(json!({ "to": { "of": who }, "amount": amount })))
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut map: IndexMap<String, CardScripts> = IndexMap::new();
    map.insert(BOLT.into(), on_play(|| vec![hit("enemyHero", BOLT_DAMAGE)]));
    map.insert(NUKE.into(), on_play(|| vec![hit("enemyHero", NUKE_DAMAGE)]));
    map.insert(BACKFIRE.into(), on_play(|| vec![hit("selfHero", NUKE_DAMAGE)]));
    map.insert(
        BLAST.into(),
        on_play(|| vec![hit("enemyHero", NUKE_DAMAGE), hit("selfHero", NUKE_DAMAGE)]),
    );
    map.insert(
        DRAIN.into(),
        on_play(|| {
            vec![effects::lose_health(json_as(
                json!({ "player": "enemy", "amount": DRAIN_LOSS }),
            ))]
        }),
    );
    map.insert(
        MEND.into(),
        on_play(|| {
            vec![effects::heal(json_as(
                json!({ "target": { "of": "selfHero" }, "amount": MEND_HEAL }),
            ))]
        }),
    );
    map.insert(SWAP.into(), on_play(|| vec![effects::swap_health()]));
    map.insert(
        TEMPO.into(),
        on_play(|| vec![effects::end_turn(json_as(json!({})))]),
    );
    map.insert(
        AGAIN.into(),
        on_play(|| vec![effects::take_extra_turn(json_as(json!({})))]),
    );
    map.insert(
        VICTORY.into(),
        on_play(|| vec![effects::win_game(json_as(json!({})))]),
    );
    map.insert(
        CONCESSION.into(),
        on_play(|| vec![effects::win_game(json_as(json!({ "player": "enemy" })))]),
    );
    map.insert(
        TRIPWIRE.into(),
        both(Script {
            enters_hand: Some(hook(|_ctx| vec![hit("selfHero", TRIPWIRE_DAMAGE)])),
            ..Script::default()
        }),
    );
    map.insert(
        EMBER.into(),
        both(Script {
            end_of_turn: Some(hook(|_ctx| vec![hit("selfHero", NUKE_DAMAGE)])),
            ..Script::default()
        }),
    );
    map.insert(
        STALL.into(),
        both(Script {
            start_of_turn: Some(hook(|_ctx| vec![effects::end_turn(json_as(json!({})))])),
            ..Script::default()
        }),
    );
    map.insert(
        GAMBIT.into(),
        both(Script {
            replacements: vec![ReplacementDef {
                id: "duos-gambit".to_string(),
                on: ReplacementMoment::LethalHit,
                where_: None,
                when: None,
                instead: json_as(json!({ "redirect": "enemyHero" })),
                then: None,
                by: None,
            }],
            ..Script::default()
        }),
    );
    map
}

/// This thread's catalog and scripts (the testkit's override), as `duos_scenario` reads them.
fn setup() {
    register_catalog(catalog());
    register_scripts(scripts());
}

// ---------------------------------------------------------------------------------------------
// Games
// ---------------------------------------------------------------------------------------------

/// One disjoint block of twenty vanilla ids per seat: A1 1–20, B1 21–40, A2 41–60, B2 61–80.
fn decks() -> DuosDecks {
    DuosDecks {
        a1: vanilla_deck(DECK_SIZE, 1),
        a2: vanilla_deck(DECK_SIZE, 41),
        b1: vanilla_deck(DECK_SIZE, 21),
        b2: vanilla_deck(DECK_SIZE, 61),
    }
}

fn deck_of(seat: DuosSeat) -> Vec<String> {
    let decks = decks();
    match seat {
        A1 => decks.a1,
        A2 => decks.a2,
        B1 => decks.b1,
        B2 => decks.b2,
    }
}

fn options_with(
    seed: &str,
    teams: Option<PerTeam<DuosTeamRules>>,
    handicaps: Option<DuosHandicaps>,
) -> DuosCreateOptions {
    DuosCreateOptions {
        seed: seed.to_string(),
        decks: decks(),
        catalog: Some(catalog()),
        teams,
        handicaps,
        dealt: None,
    }
}

fn options(seed: &str) -> DuosCreateOptions {
    options_with(seed, None, None)
}

/// Round 1 open, both teams on DUOS_HUMAN_TEAM.
fn begun(seed: &str) -> DuosState {
    setup();
    duos_scenario(seed, decks(), None)
}

/// Round 1 open, team A on `a`, team B on DUOS_HUMAN_TEAM.
fn begun_with(seed: &str, a: DuosTeamRules) -> DuosState {
    setup();
    duos_scenario(
        seed,
        decks(),
        Some(PerTeam {
            a,
            b: DUOS_HUMAN_TEAM,
        }),
    )
}

fn free_passes() -> DuosTeamRules {
    DuosTeamRules {
        pass_cost: 0,
        ..DUOS_HUMAN_TEAM
    }
}

static NONCE: AtomicU32 = AtomicU32::new(0);

fn nonce() -> String {
    format!("duos-{}", NONCE.fetch_add(1, Ordering::SeqCst) + 1)
}

fn label(body: &DuosActionBody) -> String {
    serde_json::to_string(body).unwrap_or_default()
}

fn act(state: &DuosState, seat: DuosSeat, body: DuosActionBody) -> DuosReduceResult {
    duos_reduce(
        state,
        &DuosAction {
            seat,
            nonce: nonce(),
            body,
        },
    )
}

/// The action, which must be accepted.
fn ok(state: &DuosState, seat: DuosSeat, body: DuosActionBody) -> DuosState {
    let what = label(&body);
    let result = act(state, seat, body);
    if let Some(error) = &result.error {
        panic!("{seat} {what} was refused: {error}");
    }
    result.state
}

/// The action, which must be refused and change nothing.
fn refused(state: &DuosState, seat: DuosSeat, body: DuosActionBody) {
    let what = label(&body);
    let before = duos_hash(state);
    let result = act(state, seat, body);
    assert!(result.error.is_some(), "{seat} {what} was accepted");
    assert_eq!(
        duos_hash(&result.state),
        before,
        "{seat} {what}: a refusal changes nothing"
    );
}

fn duel(body: ActionBody) -> DuosActionBody {
    DuosActionBody::Duel { body }
}

fn end_turn() -> DuosActionBody {
    duel(ActionBody::EndTurn)
}

fn pass(instance_id: &str) -> DuosActionBody {
    DuosActionBody::Pass {
        instance_id: instance_id.to_string(),
    }
}

fn mulligan_keeping_all(state: &DuosState, seat: DuosSeat) -> DuosActionBody {
    duel(ActionBody::Mulligan {
        keep: hand_ids(state, seat),
    })
}

/// All four mulligans kept, A1, A2, B1, B2.
fn keep_all(state: DuosState) -> DuosState {
    let mut state = state;
    for seat in DUOS_SEATS {
        let body = mulligan_keeping_all(&state, seat);
        state = ok(&state, seat, body);
    }
    state
}

/// Every round seat that has not ended ends its turn, so the round closes.
fn finish_round(state: DuosState) -> DuosState {
    let round = state.round;
    let mut state = state;
    for d in [0usize, 1] {
        if state.round == round && !state.ended[d] {
            let seat = state.round_seats[d];
            state = ok(&state, seat, end_turn());
        }
    }
    assert_eq!(state.round, round + 1, "round {round} closed");
    state
}

// ---------------------------------------------------------------------------------------------
// Reading a seat
// ---------------------------------------------------------------------------------------------

fn side(state: &DuosState, seat: DuosSeat) -> &PlayerState {
    &state.duels[seat.duel()].players[seat.player()]
}

fn side_mut(state: &mut DuosState, seat: DuosSeat) -> &mut PlayerState {
    &mut state.duels[seat.duel()].players[seat.player()]
}

fn hand_ids(state: &DuosState, seat: DuosSeat) -> Vec<String> {
    side(state, seat)
        .hand
        .iter()
        .map(|card| card.id.clone())
        .collect()
}

fn health(state: &DuosState, seat: DuosSeat) -> i32 {
    side(state, seat).hero.health
}

fn pool_of(state: &DuosState, seat: DuosSeat) -> i32 {
    side(state, seat)
        .duos
        .as_ref()
        .map(|life| life.pool)
        .unwrap_or_else(|| panic!("{seat}'s side carries no Duos life"))
}

fn is_spent(state: &DuosState, seat: DuosSeat) -> bool {
    side(state, seat)
        .duos
        .as_ref()
        .map(|life| life.spent)
        .unwrap_or_else(|| panic!("{seat}'s side carries no Duos life"))
}

fn on_field(state: &DuosState, seat: DuosSeat, id: &str) -> bool {
    side(state, seat)
        .units
        .iter()
        .flatten()
        .flatten()
        .any(|card| card.id == id)
}

/// A new card of `def_id` in the seat's hand, at its right end; its instance id.
fn give(state: &mut DuosState, seat: DuosSeat, def_id: &str) -> String {
    let player = seat.player();
    let game = &mut state.duels[seat.duel()];
    let card = new_instance(game, def_id, player, Zone::Hand { player });
    game.players[player].hand.push(card.clone());
    card.id
}

fn sorted(mut seats: Vec<DuosSeat>) -> Vec<DuosSeat> {
    seats.sort();
    seats
}

/// The main-phase moves and passes the seat is offered: everything but mulligans, prompt answers,
/// draw offers and concedes.
fn acting_moves(state: &DuosState, seat: DuosSeat) -> Vec<String> {
    duos_legal_actions(state, seat)
        .iter()
        .filter_map(|offered| {
            if let DuosActionBody::Duel { body } = offered {
                match body.action_type() {
                    ActionType::Play
                    | ActionType::Attack
                    | ActionType::SwitchPosition
                    | ActionType::Activate
                    | ActionType::ActivatePower
                    | ActionType::Emote
                    | ActionType::EndTurn => Some(body.action_type().as_str().to_string()),
                    _ => None,
                }
            } else if let DuosActionBody::Pass { instance_id } = offered {
                Some(format!("pass {instance_id}"))
            } else {
                None
            }
        })
        .collect()
}

fn passes_offered(state: &DuosState, seat: DuosSeat) -> Vec<String> {
    duos_legal_actions(state, seat)
        .into_iter()
        .filter_map(|offered| match offered {
            DuosActionBody::Pass { instance_id } => Some(instance_id),
            _ => None,
        })
        .collect()
}

fn offers_play(state: &DuosState, seat: DuosSeat, id: &str) -> bool {
    duos_legal_actions(state, seat).iter().any(|offered| {
        matches!(offered, DuosActionBody::Duel { body: ActionBody::Play { instance_id, .. } } if instance_id.as_str() == id)
    })
}

/// The play of `id` the wrapper offers the seat.
fn legal_play(state: &DuosState, seat: DuosSeat, id: &str) -> DuosActionBody {
    duos_legal_actions(state, seat)
        .into_iter()
        .find(|offered| {
            matches!(offered, DuosActionBody::Duel { body: ActionBody::Play { instance_id, .. } } if instance_id.as_str() == id)
        })
        .unwrap_or_else(|| panic!("{seat} is offered no play of {id}"))
}

/// A well-formed play of `id`: the one the seat's duel itself would offer, else a bare play. For
/// asking the wrapper to refuse what the 1v1 game alone would allow.
fn raw_play(state: &DuosState, seat: DuosSeat, id: &str) -> DuosActionBody {
    let body = legal_actions(&state.duels[seat.duel()], seat.player())
        .into_iter()
        .find(|offered| matches!(offered, ActionBody::Play { instance_id, .. } if instance_id.as_str() == id))
        .unwrap_or_else(|| json_as(json!({ "type": "play", "instanceId": id })));
    duel(body)
}

fn legal_attack(state: &DuosState, seat: DuosSeat, attacker: &str, target: &str) -> DuosActionBody {
    duos_legal_actions(state, seat)
        .into_iter()
        .find(|offered| {
            matches!(
                offered,
                DuosActionBody::Duel { body: ActionBody::Attack { attacker_id, target_id } }
                    if attacker_id.as_str() == attacker && target_id.as_str() == target
            )
        })
        .unwrap_or_else(|| panic!("{seat} is offered no attack of {attacker} on {target}"))
}

/// Gives the seat a `def_id` and plays it, which must be accepted.
fn cast(state: DuosState, seat: DuosSeat, def_id: &str) -> DuosReduceResult {
    let mut state = state;
    let id = give(&mut state, seat, def_id);
    let body = legal_play(&state, seat, &id);
    let result = act(&state, seat, body);
    if let Some(error) = &result.error {
        panic!("{seat}'s {def_id} was refused: {error}");
    }
    result
}

fn has_duel_event(events: &[DuosEvent], kind: GameEventType) -> bool {
    events
        .iter()
        .any(|event| matches!(event, DuosEvent::Duel { event, .. } if event.event_type() == kind))
}

/// The pass's `(from, to, instanceId, newInstanceId)`, one per `passed` event.
fn passed(events: &[DuosEvent]) -> Vec<(DuosSeat, DuosSeat, String, String)> {
    events
        .iter()
        .filter_map(|event| match event {
            DuosEvent::Passed {
                from,
                to,
                instance_id,
                new_instance_id,
            } => Some((*from, *to, instance_id.clone(), new_instance_id.clone())),
            _ => None,
        })
        .collect()
}

fn expect_result(state: &DuosState, outcome: DuosOutcome, winner: Option<Team>, reason: Option<&str>) {
    let result = state.result.as_ref().expect("the Duos game is over");
    assert_eq!(result.outcome, outcome);
    assert_eq!(result.winner, winner);
    if let Some(reason) = reason {
        assert_eq!(result.reason, reason);
    }
    for seat in DUOS_SEATS {
        let seen = duos_view_for(state, seat)
            .result
            .unwrap_or_else(|| panic!("{seat}'s view shows no result"));
        assert_eq!(seen.outcome, outcome, "{seat}");
        assert_eq!(seen.winner, winner, "{seat}");
        assert_eq!(seen.reason, result.reason, "{seat}");
    }
}

fn json_of(value: impl Serialize) -> Value {
    serde_json::to_value(value).expect("serialises")
}

fn find_key<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(map) => map
            .get(key)
            .or_else(|| map.values().find_map(|inner| find_key(inner, key))),
        Value::Array(items) => items.iter().find_map(|inner| find_key(inner, key)),
        _ => None,
    }
}

fn strings_in(value: &Value) -> IndexSet<String> {
    fn collect(value: &Value, out: &mut IndexSet<String>) {
        match value {
            Value::String(text) => {
                out.insert(text.clone());
            }
            Value::Array(items) => items.iter().for_each(|inner| collect(inner, out)),
            Value::Object(map) => map.values().for_each(|inner| collect(inner, out)),
            _ => {}
        }
    }
    let mut out = IndexSet::new();
    collect(value, &mut out);
    out
}

/// Every seat's pool reads its team's, and a spent seat's health reads it too.
fn expect_pools_agree(state: &DuosState) {
    for seat in DUOS_SEATS {
        let pool = state.teams[seat.team()].pool;
        assert_eq!(pool_of(state, seat), pool, "{seat}'s pool is its team's");
        if is_spent(state, seat) {
            assert_eq!(
                health(state, seat),
                pool,
                "{seat} is spent, so its health reads the pool"
            );
        }
    }
}

/// A live game and its log, as a server would record it: created, begun, then one action a step.
struct Live {
    state: DuosState,
    log: Vec<DuosAction>,
}

impl Live {
    fn begin(seed: &str) -> Live {
        setup();
        let begun = duos_begin(&duos_create(&options(seed)));
        if let Some(error) = &begun.error {
            panic!("{seed} did not begin: {error}");
        }
        Live {
            state: begun.state,
            log: Vec::new(),
        }
    }

    fn step(&mut self, seat: DuosSeat, body: DuosActionBody) {
        let what = label(&body);
        let action = DuosAction {
            seat,
            nonce: nonce(),
            body,
        };
        let result = duos_reduce(&self.state, &action);
        if let Some(error) = &result.error {
            panic!("{seat} {what} was refused at step {}: {error}", self.log.len());
        }
        self.state = result.state;
        self.log.push(action);
    }

    fn keep_all(&mut self) {
        for seat in DUOS_SEATS {
            let body = mulligan_keeping_all(&self.state, seat);
            self.step(seat, body);
        }
    }
}

/// §10.7's random policy over the four seats: a uniform seat among those that may act, then a
/// uniform action, ending the turn when it is all that is left or with AI_END_TURN_PROBABILITY.
/// Concedes are left out, so a game ends by a pool, a duel's own end or the turn cap.
fn play_randomly(live: &mut Live, seed: &str, max_steps: usize) {
    let mut policy = Rng::new(&format!("duos-policy-{seed}"), 0);
    for step in 0..max_steps {
        if live.state.result.is_some() {
            return;
        }
        let seats = duos_seats_to_act(&live.state);
        assert!(
            !seats.is_empty(),
            "{seed}: no seat may act at step {step} (round {})",
            live.state.round
        );
        let seat = seats[policy.int(seats.len() as i32) as usize];
        let mut ends: Vec<DuosActionBody> = Vec::new();
        let mut others: Vec<DuosActionBody> = Vec::new();
        for offered in duos_legal_actions(&live.state, seat) {
            if matches!(offered, DuosActionBody::Concede) {
                continue;
            }
            if matches!(
                &offered,
                DuosActionBody::Duel {
                    body: ActionBody::EndTurn
                }
            ) {
                ends.push(offered);
            } else {
                others.push(offered);
            }
        }
        assert!(
            !ends.is_empty() || !others.is_empty(),
            "{seed}: {seat} is listed to act at step {step} with nothing but concede"
        );
        let take_end = !ends.is_empty() && (others.is_empty() || policy.chance(AI_END_TURN_PROBABILITY));
        let body = if take_end {
            ends.swap_remove(0)
        } else {
            let at = policy.int(others.len() as i32) as usize;
            others.swap_remove(at)
        };
        live.step(seat, body);
    }
}

// ---------------------------------------------------------------------------------------------
// B1: two ordinary duels
// ---------------------------------------------------------------------------------------------

mod b1_two_ordinary_duels {
    use super::*;

    #[test]
    fn b1_create_and_begin_deal_two_ordinary_duels_with_team_a_on_p1_and_team_b_on_p2() {
        setup();
        let created = duos_create(&options("b1"));
        assert_eq!(created.duels.len(), 2);
        for (d, game) in created.duels.iter().enumerate() {
            assert_eq!(game.seed, format!("b1:duel{d}"));
        }
        assert_eq!(created.round, 0);

        let begun = duos_begin(&created);
        assert!(begun.error.is_none());
        let state = begun.state;
        assert_eq!(state.round, 0);
        for (d, game) in state.duels.iter().enumerate() {
            assert_eq!(game.phase, Phase::Mulligan, "duel {d}");
            assert_eq!(game.players.p1.hand.len() as i32, OPENING_DRAW[0], "duel {d}");
            assert_eq!(game.players.p2.hand.len() as i32, OPENING_DRAW[1], "duel {d}");
            assert_eq!(game.players.p1.library.len() as i32, DECK_SIZE - OPENING_DRAW[0]);
            assert_eq!(game.players.p2.library.len() as i32, DECK_SIZE - OPENING_DRAW[1]);
            assert!(
                !game.players.p2.hand.iter().any(|card| card.def_id == COIN_DEF_ID),
                "duel {d}: The Coin waits for the mulligan"
            );
        }
        // Each seat sits in its own duel with its own deck.
        for seat in DUOS_SEATS {
            let deck = deck_of(seat);
            let own = side(&state, seat);
            let dealt: Vec<String> = own
                .hand
                .iter()
                .chain(own.library.iter())
                .map(|card| card.def_id.clone())
                .collect();
            assert_eq!(dealt.len() as i32, DECK_SIZE, "{seat}");
            assert!(
                dealt.iter().all(|id| deck.contains(id)),
                "{seat} plays its own deck"
            );
        }

        let kept = keep_all(state);
        assert_eq!(kept.round, 1);
        for (d, game) in kept.duels.iter().enumerate() {
            assert_eq!(game.turn, 1, "duel {d}");
            assert_eq!(game.active, P1, "duel {d}: team A goes first");
            let coins = |player: PlayerId| {
                game.players[player]
                    .hand
                    .iter()
                    .filter(|card| card.def_id == COIN_DEF_ID)
                    .count()
            };
            assert_eq!(coins(P1), 0, "duel {d}");
            assert_eq!(coins(P2), 1, "duel {d}");
            assert_eq!(
                game.players.p2.hand.last().map(|card| card.def_id.as_str()),
                Some(COIN_DEF_ID),
                "duel {d}: The Coin is team B's last card"
            );
        }
    }

    #[test]
    fn b1_the_four_seats_map_onto_the_two_duels() {
        assert_eq!(DUOS_SEATS, [A1, A2, B1, B2]);
        for (seat, team, duel_index, player, teammate, across) in [
            (A1, Team::A, 0, P1, A2, B1),
            (A2, Team::A, 1, P1, A1, B2),
            (B1, Team::B, 0, P2, B2, A1),
            (B2, Team::B, 1, P2, B1, A2),
        ] {
            assert_eq!(seat.team(), team, "{seat}");
            assert_eq!(seat.duel(), duel_index, "{seat}");
            assert_eq!(seat.player(), player, "{seat}");
            assert_eq!(seat.teammate(), teammate, "{seat}");
            assert_eq!(seat.across(), across, "{seat}");
            assert_eq!(DuosSeat::of(duel_index, player), seat);
        }
    }

    #[test]
    fn b1_a_seats_handicap_reaches_its_own_side_of_its_own_duel_only() {
        setup();
        let handicap = Handicap {
            extra_opening_cards: 1,
            ..HUMAN_HANDICAP
        };
        let created = duos_create(&options_with(
            "b1-handicap",
            None,
            Some(DuosHandicaps {
                a1: None,
                a2: None,
                b1: Some(handicap),
                b2: None,
            }),
        ));
        let state = duos_begin(&created).state;
        assert_eq!(side(&state, B1).handicap, Some(handicap));
        assert_eq!(side(&state, B1).hand.len() as i32, OPENING_DRAW[1] + 1);
        for seat in [A1, A2, B2] {
            assert_eq!(side(&state, seat).handicap, None, "{seat}");
        }
        assert_eq!(side(&state, B2).hand.len() as i32, OPENING_DRAW[1]);
    }
}

// ---------------------------------------------------------------------------------------------
// B2: the four mulligans
// ---------------------------------------------------------------------------------------------

mod b2_mulligans {
    use super::*;

    #[test]
    fn b2_all_four_mulligans_are_open_at_once_and_round_1_waits_for_the_last() {
        setup();
        let mut state = duos_begin(&duos_create(&options("b2"))).state;
        assert_eq!(sorted(duos_seats_to_act(&state)), DUOS_SEATS);
        for seat in DUOS_SEATS {
            let hand = hand_ids(&state, seat);
            let mulligans: Vec<Vec<String>> = duos_legal_actions(&state, seat)
                .into_iter()
                .filter_map(|offered| match offered {
                    DuosActionBody::Duel {
                        body: ActionBody::Mulligan { keep },
                    } => Some(keep),
                    _ => None,
                })
                .collect();
            assert!(!mulligans.is_empty(), "{seat} is offered its own mulligan");
            for keep in &mulligans {
                assert!(
                    keep.iter().all(|id| hand.contains(id)),
                    "{seat}'s mulligan keeps its own cards"
                );
            }
        }

        let order = [B2, A1, B1, A2];
        for (i, seat) in order.into_iter().enumerate() {
            assert_eq!(state.round, 0, "before {seat}'s mulligan");
            let body = mulligan_keeping_all(&state, seat);
            let result = act(&state, seat, body);
            assert!(result.error.is_none(), "{seat}'s mulligan");
            state = result.state;
            let opened = result
                .events
                .iter()
                .any(|event| matches!(event, DuosEvent::RoundStarted { round: 1 }));
            if i + 1 < order.len() {
                assert_eq!(
                    state.round,
                    0,
                    "after {seat}'s mulligan, {} still owed",
                    order.len() - i - 1
                );
                assert!(!opened);
                assert!(!duos_seats_to_act(&state).contains(&seat), "{seat} has answered");
            } else {
                assert_eq!(state.round, 1);
                assert!(opened, "the last mulligan opens round 1");
            }
        }
        assert_eq!(state.round_seats, [A1, A2]);
        assert_eq!(sorted(duos_seats_to_act(&state)), [A1, A2]);
    }

    #[test]
    fn b2_no_seat_takes_a_main_phase_action_until_all_four_mulligans_have_resolved() {
        setup();
        let mut state = duos_begin(&duos_create(&options("b2-wait"))).state;
        // Duel 0's two mulligans are answered, so that 1v1 game alone would be in A1's turn 1.
        for seat in [A1, B1] {
            let body = mulligan_keeping_all(&state, seat);
            state = ok(&state, seat, body);
        }
        assert_eq!(state.round, 0);
        for seat in [A1, B1] {
            assert!(
                acting_moves(&state, seat).is_empty(),
                "{seat}: {:?}",
                acting_moves(&state, seat)
            );
            refused(&state, seat, end_turn());
        }
        let card = hand_ids(&state, A1)[0].clone();
        refused(&state, A1, raw_play(&state, A1, &card));
        refused(&state, A1, pass(&card));
        assert_eq!(sorted(duos_seats_to_act(&state)), [A2, B2]);
    }

    #[test]
    fn b2_a_mulligan_keeping_a_card_not_in_hand_or_answered_twice_is_refused() {
        setup();
        let state = duos_begin(&duos_create(&options("b2-bad"))).state;
        refused(
            &state,
            A1,
            duel(ActionBody::Mulligan {
                keep: vec!["c9999".to_string()],
            }),
        );
        let body = mulligan_keeping_all(&state, A1);
        let state = ok(&state, A1, body);
        let again = mulligan_keeping_all(&state, A1);
        refused(&state, A1, again);
    }
}

// ---------------------------------------------------------------------------------------------
// B3: a 1v1 game is untouched
// ---------------------------------------------------------------------------------------------

mod b3_one_v_one_untouched {
    use super::*;

    #[test]
    fn b3_a_1v1_state_and_its_views_never_carry_a_duos_key() {
        setup_catalog();
        let created = create_game(&CreateGameOptions {
            seed: "b3".to_string(),
            decks: (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21)),
            ..CreateGameOptions::default()
        });
        assert!(find_key(&json_of(&created), "duos").is_none());

        let started = begin_game(&created).state;
        assert!(find_key(&json_of(&started), "duos").is_none());
        for player in PLAYER_IDS {
            assert!(find_key(&json_of(view_for(&started, player)), "duos").is_none());
        }

        let game = play_random_game("b3", None);
        assert!(game.state.result.is_some());
        assert!(find_key(&json_of(&game.state), "duos").is_none());
        for player in PLAYER_IDS {
            assert!(find_key(&json_of(view_for(&game.state, player)), "duos").is_none());
        }
    }
}

// ---------------------------------------------------------------------------------------------
// B4–B9: rounds
// ---------------------------------------------------------------------------------------------

mod b4_round_seats {
    use super::*;

    #[test]
    fn b4_both_seats_of_the_active_team_act_at_once() {
        let mut state = begun("b4");
        assert_eq!(state.round, 1);
        assert_eq!(state.round_seats, [A1, A2]);
        assert_eq!(sorted(duos_seats_to_act(&state)), [A1, A2]);
        // Each keeps a move once its mana is spent, so neither is ended as idle.
        for seat in [A1, A2] {
            give(&mut state, seat, NOOP);
        }
        let a2_card = hand_ids(&state, A2)[0].clone();
        let body = legal_play(&state, A2, &a2_card);
        let state = ok(&state, A2, body);
        let a1_card = hand_ids(&state, A1)[0].clone();
        let body = legal_play(&state, A1, &a1_card);
        let state = ok(&state, A1, body);

        assert!(on_field(&state, A1, &a1_card));
        assert!(on_field(&state, A2, &a2_card));
        assert_eq!(state.round, 1);
        assert_eq!(state.ended, [false, false]);
        assert_eq!(sorted(duos_seats_to_act(&state)), [A1, A2]);
    }

    #[test]
    fn b4_the_team_out_of_its_round_cannot_act() {
        let state = begun("b4-out");
        for seat in [B1, B2] {
            assert!(
                acting_moves(&state, seat).is_empty(),
                "{seat}: {:?}",
                acting_moves(&state, seat)
            );
            refused(&state, seat, end_turn());
            for card in hand_ids(&state, seat) {
                refused(&state, seat, raw_play(&state, seat, &card));
            }
        }
        assert!(
            !duos_seats_to_act(&state)
                .iter()
                .any(|seat| seat.team() == Team::B)
        );
    }
}

mod b5_turn_end_sync {
    use super::*;

    #[test]
    fn b5_an_end_turn_waits_for_the_other_round_seat_and_applies_nothing_yet() {
        let state = begun("b5");
        let duel0 = hash_state(&state.duels[0]);
        let state = ok(&state, A1, end_turn());
        assert_eq!(state.ended, [true, false]);
        assert_eq!(state.round, 1);
        assert_eq!(state.duels[0].active, P1);
        assert_eq!(state.duels[0].turn, 1);
        assert_eq!(
            hash_state(&state.duels[0]),
            duel0,
            "nothing is applied to duel 0 yet"
        );
        assert_eq!(sorted(duos_seats_to_act(&state)), [A2]);
        refused(&state, A1, end_turn());
    }

    #[test]
    fn b5_the_second_end_turn_ends_duel_0_then_duel_1_and_opens_the_next_round() {
        let state = begun("b5-close");
        let state = ok(&state, A1, end_turn());
        let result = act(&state, A2, end_turn());
        assert!(result.error.is_none());
        let state = result.state;

        assert_eq!(state.round, 2);
        for game in &state.duels {
            assert_eq!(game.active, P2);
            assert_eq!(game.turn, 2);
        }
        assert_eq!(state.round_seats, [B1, B2]);
        assert_eq!(state.round_turns, [2, 2]);
        assert_eq!(state.ended, [false, false]);
        assert_eq!(sorted(duos_seats_to_act(&state)), [B1, B2]);

        let turn_ended = |duel_index: u8| {
            result.events.iter().position(|event| {
                matches!(event, DuosEvent::Duel { duel: d, event } if *d == duel_index && event.event_type() == GameEventType::TurnEnded)
            })
        };
        let first = turn_ended(0).expect("duel 0's turn ended");
        let second = turn_ended(1).expect("duel 1's turn ended");
        assert!(first < second, "duel 0's turn ends before duel 1's");
        let opened = result
            .events
            .iter()
            .position(|event| matches!(event, DuosEvent::RoundStarted { round: 2 }))
            .expect("round 2 started");
        assert!(
            opened > second,
            "the round opens once both duels have ended their turn"
        );
    }

    /// One action under the seat's own nonce.
    fn under(state: &DuosState, seat: DuosSeat, nonce: &str, body: DuosActionBody) -> DuosReduceResult {
        duos_reduce(
            state,
            &DuosAction {
                seat,
                nonce: nonce.to_string(),
                body,
            },
        )
    }

    #[test]
    fn b5_a_seat_action_in_the_wrappers_nonce_namespace_is_refused() {
        let mut state = begun("b5-reserved");
        let card = give(&mut state, A1, NOOP);
        let play = legal_play(&state, A1, &card);
        for nonce in [
            DUOS_NONCE_PREFIX.to_string(),
            format!("{DUOS_NONCE_PREFIX}begin"),
            format!("{DUOS_NONCE_PREFIX}end:r1:d0:mine"),
        ] {
            for body in [play.clone(), end_turn(), pass(&card), DuosActionBody::Concede] {
                let what = label(&body);
                let before = duos_hash(&state);
                let result = under(&state, A1, &nonce, body);
                assert!(result.error.is_some(), "{nonce}: {what} was accepted");
                assert_eq!(duos_hash(&result.state), before, "{nonce}: {what}");
            }
        }
    }

    #[test]
    fn b5_a_move_under_the_nonce_a_rounds_close_once_minted_cannot_stall_the_round() {
        // R270's stall: A1 plays under the nonce the wrapper used to mint for duel 0's End turn when
        // the action `closing` closes round 1 ("<nonce>:end:r<round>:d<duel>"), then closes it.
        let mut state = begun("b5-stall");
        let closing = "b5-stall-close";
        let card = give(&mut state, A1, NOOP);
        let play = legal_play(&state, A1, &card);
        let played = under(&state, A1, &format!("{closing}:end:r1:d0"), play);
        assert!(played.error.is_none(), "{:?}", played.error);
        let state = ok(&played.state, A2, end_turn());

        let closed = under(&state, A1, closing, end_turn());

        assert!(closed.error.is_none(), "{:?}", closed.error);
        let state = closed.state;
        assert_eq!(state.round, 2, "the round closed");
        for game in &state.duels {
            assert_eq!(game.active, P2);
            assert_eq!(game.turn, 2);
        }
        assert_eq!(state.round_seats, [B1, B2]);
        assert_eq!(sorted(duos_seats_to_act(&state)), [B1, B2]);
    }

    #[test]
    fn b5_a_duel_that_has_applied_the_rounds_own_nonce_refuses_its_end_turn_and_the_next_step_closes_the_round()
     {
        let mut state = begun("b5-replayed");
        let closing = "b5-replayed-close";
        // Duel 0 already holds the nonce round 1's close will send its End turn under.
        state.duels[0].applied.push(AppliedAction {
            nonce: format!("{DUOS_NONCE_PREFIX}end:r1:d0:{closing}"),
            events: Vec::new(),
        });
        let state = ok(&state, A2, end_turn());

        let closed = under(&state, A1, closing, end_turn());

        assert!(closed.error.is_none(), "{:?}", closed.error);
        let state = closed.state;
        assert_eq!(state.round, 1, "duel 0 refused the End turn, so the round waits");
        assert_eq!(
            state.end_applied,
            [false, true],
            "a replayed nonce is never taken as the End turn"
        );
        assert_eq!(state.duels[0].turn, 1, "duel 0's turn has not moved");
        assert_eq!(state.duels[1].turn, 2);

        // The next step tries duel 0 again under its own nonce, and the round closes.
        let state = ok(&state, A1, DuosActionBody::Concede);
        assert_eq!(state.round, 2);
        for game in &state.duels {
            assert_eq!(game.active, P2);
            assert_eq!(game.turn, 2);
        }
    }

    #[test]
    fn b5_in_team_bs_round_team_a_can_neither_act_nor_end_it() {
        let state = finish_round(begun("b5-round-2"));
        assert_eq!(state.round_seats, [B1, B2]);
        for seat in [A1, A2] {
            assert!(
                acting_moves(&state, seat).is_empty(),
                "{seat}: {:?}",
                acting_moves(&state, seat)
            );
            refused(&state, seat, end_turn());
            for card in hand_ids(&state, seat) {
                refused(&state, seat, raw_play(&state, seat, &card));
                refused(&state, seat, pass(&card));
            }
        }
    }
}

mod b6_ended_seat {
    use super::*;

    #[test]
    fn b6_an_ended_seat_has_no_main_phase_actions() {
        let state = begun("b6");
        let card = hand_ids(&state, A1)[0].clone();
        let state = ok(&state, A1, end_turn());
        assert!(
            acting_moves(&state, A1).is_empty(),
            "{:?}",
            acting_moves(&state, A1)
        );
        // Duel 0 itself is still A1's main phase: the wrapper is what refuses.
        refused(&state, A1, raw_play(&state, A1, &card));
        refused(&state, A1, pass(&card));
        refused(&state, A1, end_turn());
        assert!(!duos_seats_to_act(&state).contains(&A1));
        assert!(duos_seats_to_act(&state).contains(&A2));
    }

    #[test]
    fn b6_an_ended_seat_still_receives_passes() {
        let mut state = begun("b6-receive");
        give(&mut state, A2, NOOP);
        let state = ok(&state, A1, end_turn());
        let before = side(&state, A1).hand.len();
        let card = hand_ids(&state, A2)[0].clone();
        let state = ok(&state, A2, pass(&card));
        assert_eq!(side(&state, A1).hand.len(), before + 1);
        assert_eq!(state.ended, [true, false]);
        assert_eq!(state.round, 1);
    }
}

mod b7_turn_ended_by_an_effect {
    use super::*;

    #[test]
    fn b7_a_duel_whose_turn_ends_by_itself_counts_its_round_seat_as_ended() {
        let state = begun("b7");
        let state = cast(state, A1, TEMPO).state;
        assert_eq!(state.duels[0].active, P2, "the effect ended duel 0's turn");
        assert_eq!(state.duels[0].turn, 2);
        assert_eq!(state.ended, [true, false]);
        assert_eq!(state.round, 1);
        assert_eq!(sorted(duos_seats_to_act(&state)), [A2]);

        let state = ok(&state, A2, end_turn());
        assert_eq!(state.round, 2);
        assert_eq!(state.duels[0].turn, 2, "duel 0's turn is not ended a second time");
        assert_eq!(state.duels[0].active, P2);
        assert_eq!(state.duels[1].turn, 2);
        assert_eq!(state.duels[1].active, P2);
        assert_eq!(state.round_seats, [B1, B2]);
        assert_eq!(sorted(duos_seats_to_act(&state)), [B1, B2]);
    }

    #[test]
    fn b7_the_duels_new_active_seat_may_not_act_until_the_round_closes() {
        let state = begun("b7-wait");
        let state = cast(state, A1, TEMPO).state;
        assert_eq!(state.duels[0].active, P2);
        // Duel 0 alone would let B1 play now; the round is still team A's.
        assert!(
            acting_moves(&state, B1).is_empty(),
            "{:?}",
            acting_moves(&state, B1)
        );
        refused(&state, B1, end_turn());
        for card in hand_ids(&state, B1) {
            refused(&state, B1, raw_play(&state, B1, &card));
        }
        assert!(!duos_seats_to_act(&state).contains(&B1));
    }

    /// #936: a first turn that ends as it begins, while the other duel's mulligans are still open,
    /// used to open round 1 on the duel's new active player, so round 1 went to B1 and A2 and duel 0
    /// stayed a turn ahead for the rest of the game, every round mixing the teams. On the server a
    /// Tommy Tempo drawn at the start of A1's first turn put A1 in round 2, the AI team's, which then
    /// waited a whole round clock for a person who was not expecting to act.
    #[test]
    fn b7_a_first_turn_ended_before_round_1_opens_is_still_round_1s_and_its_seat_has_ended() {
        setup();
        let mut state = duos_begin(&duos_create(&options("b7-round-0"))).state;
        put(&mut state.duels[0], STALL, slot(P1, Row::Units, 1), json!({}));
        for seat in [A1, B1] {
            let body = mulligan_keeping_all(&state, seat);
            state = ok(&state, seat, body);
        }
        assert_eq!(state.round, 0, "duel 1's mulligans are still open");
        assert_eq!(state.duels[0].turn, 2, "A1's first turn ended as it began");
        assert_eq!(state.duels[0].active, P2);

        for seat in [A2, B2] {
            let body = mulligan_keeping_all(&state, seat);
            state = ok(&state, seat, body);
        }
        assert_eq!(state.round, 1);
        assert_eq!(state.round_seats, [A1, A2], "round 1 is team A's in both duels");
        assert_eq!(state.ended, [true, false], "A1's first turn is over");
        assert_eq!(sorted(duos_seats_to_act(&state)), [A2]);
        assert!(
            acting_moves(&state, B1).is_empty(),
            "{:?}",
            acting_moves(&state, B1)
        );
        refused(&state, B1, end_turn());

        let state = ok(&state, A2, end_turn());
        assert_eq!(state.round, 2);
        assert_eq!(state.round_seats, [B1, B2], "round 2 is team B's in both duels");
        assert_eq!(state.duels[0].turn, 2, "duel 0's turn is not ended a second time");
        assert_eq!(state.duels[1].turn, 2);
        assert_eq!(state.ended, [false, false]);
        assert_eq!(sorted(duos_seats_to_act(&state)), [B1, B2]);
    }
}

mod b8_extra_turns {
    use super::*;

    #[test]
    fn b8_an_extra_turn_keeps_that_duels_next_round_seat_on_the_same_team_and_nothing_deadlocks() {
        let state = begun("b8");
        let state = cast(state, A1, AGAIN).state;
        let state = ok(&state, A1, end_turn());
        let state = ok(&state, A2, end_turn());

        assert_eq!(state.round, 2);
        assert_eq!(state.duels[0].active, P1, "A1 takes its extra turn");
        assert_eq!(state.duels[1].active, P2);
        assert_eq!(state.round_seats, [A1, B2]);
        assert_eq!(sorted(duos_seats_to_act(&state)), [A1, B2]);
        for seat in [A2, B1] {
            assert!(acting_moves(&state, seat).is_empty(), "{seat}");
            refused(&state, seat, end_turn());
        }

        let state = ok(&state, B2, end_turn());
        let state = ok(&state, A1, end_turn());
        assert_eq!(state.round, 3);
        assert_eq!(state.duels[0].active, P2);
        assert_eq!(state.duels[1].active, P1);
        assert_eq!(state.round_seats, [B1, A2]);
        assert_eq!(sorted(duos_seats_to_act(&state)), [A2, B1]);
    }
}

mod b9_idle_round_seats {
    use super::*;

    #[test]
    fn b9_every_duel_player_has_the_automatic_turn_end_off() {
        setup();
        let created = duos_create(&options("b9"));
        for seat in DUOS_SEATS {
            assert_eq!(
                side(&created, seat).auto_end_turn,
                Some(false),
                "{seat} at creation"
            );
        }
        let begun_state = duos_begin(&created).state;
        for seat in DUOS_SEATS {
            assert_eq!(
                side(&begun_state, seat).auto_end_turn,
                Some(false),
                "{seat} once begun"
            );
        }
        let kept = keep_all(begun_state);
        for seat in DUOS_SEATS {
            assert_eq!(side(&kept, seat).auto_end_turn, Some(false), "{seat} in round 1");
        }
    }

    #[test]
    fn b9_the_wrapper_ends_a_round_seat_left_with_nothing_but_ending_its_turn() {
        let mut state = begun("b9-idle");
        side_mut(&mut state, A1).hand.clear();
        let noop = give(&mut state, A1, NOOP);
        let body = legal_play(&state, A1, &noop);
        let state = ok(&state, A1, body);

        assert_eq!(
            state.ended,
            [true, false],
            "A1 has nothing left but ending its turn"
        );
        assert_eq!(state.round, 1);
        assert_eq!(state.duels[0].active, P1, "duel 0 waits for A2");
        assert_eq!(state.duels[0].turn, 1);
        assert!(!duos_seats_to_act(&state).contains(&A1));
        assert!(duos_seats_to_act(&state).contains(&A2), "A2 still holds moves");
        refused(&state, A1, end_turn());
    }

    #[test]
    fn b9_when_both_round_seats_are_left_with_nothing_the_round_closes_by_itself() {
        let mut state = begun("b9-both");
        let mut noops = Vec::new();
        for seat in [A1, A2] {
            side_mut(&mut state, seat).hand.clear();
            noops.push(give(&mut state, seat, NOOP));
        }
        let body = legal_play(&state, A1, &noops[0]);
        let state = ok(&state, A1, body);
        assert_eq!(state.ended, [true, false]);
        let body = legal_play(&state, A2, &noops[1]);
        let state = ok(&state, A2, body);
        assert_eq!(state.round, 2);
        assert_eq!(state.round_seats, [B1, B2]);
        for game in &state.duels {
            assert_eq!(game.active, P2);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// B10, B11: draws and concedes
// ---------------------------------------------------------------------------------------------

mod b10_no_draws_no_duel_concede {
    use super::*;

    fn offers_draw_or_duel_concede(state: &DuosState, seat: DuosSeat) -> bool {
        duos_legal_actions(state, seat).iter().any(|offered| {
            matches!(
                offered,
                DuosActionBody::Duel {
                    body: ActionBody::OfferDraw | ActionBody::AnswerDraw { .. } | ActionBody::Concede
                }
            )
        })
    }

    #[test]
    fn b10_draw_offers_answers_and_a_duels_own_concede_are_never_offered() {
        setup();
        let created = duos_begin(&duos_create(&options("b10-list"))).state;
        for seat in DUOS_SEATS {
            assert!(
                !offers_draw_or_duel_concede(&created, seat),
                "{seat} during the mulligans"
            );
        }

        let state = begun("b10");
        // The duel itself would offer both to A1.
        let raw: Vec<ActionType> = legal_actions(&state.duels[0], P1)
            .iter()
            .map(|body| body.action_type())
            .collect();
        assert!(raw.contains(&ActionType::OfferDraw));
        assert!(raw.contains(&ActionType::Concede));
        for seat in DUOS_SEATS {
            assert!(!offers_draw_or_duel_concede(&state, seat), "{seat} in round 1");
        }
        let state = finish_round(state);
        for seat in DUOS_SEATS {
            assert!(!offers_draw_or_duel_concede(&state, seat), "{seat} in round 2");
        }
    }

    #[test]
    fn b10_a_draw_offer_and_a_duels_own_concede_are_refused() {
        let state = begun("b10-refused");
        refused(&state, A1, duel(ActionBody::OfferDraw));
        refused(&state, A1, duel(ActionBody::Concede));
        refused(&state, B1, duel(ActionBody::Concede));
        assert_eq!(state.concede_offer, PerTeam::new(None, None));
        assert!(state.result.is_none());
    }

    #[test]
    fn b10_answering_a_draw_offer_is_refused_even_with_one_standing_in_the_duel() {
        let mut state = begun("b10-answer");
        refused(&state, B1, duel(ActionBody::AnswerDraw { accept: true }));
        // A standing offer written into duel 0 by hand: the wrapper still takes no answer.
        let turn = state.duels[0].turn;
        state.duels[0].players.p1.draw_offer.offered_turn = Some(turn);
        refused(&state, B1, duel(ActionBody::AnswerDraw { accept: true }));
        refused(&state, B1, duel(ActionBody::AnswerDraw { accept: false }));
    }
}

mod b11_team_concede {
    use super::*;

    #[test]
    fn b11_one_concede_opens_the_teams_offer_and_ends_nothing() {
        let state = begun("b11");
        let state = ok(&state, A1, DuosActionBody::Concede);
        assert_eq!(state.concede_offer, PerTeam::new(Some(A1), None));
        assert!(state.result.is_none());
        for seat in [A1, A2] {
            assert_eq!(duos_view_for(&state, seat).concede_offered_by, Some(A1), "{seat}");
        }
        for seat in [B1, B2] {
            assert_eq!(
                duos_view_for(&state, seat).concede_offered_by,
                None,
                "{seat} sees only its own team's offer"
            );
        }
    }

    #[test]
    fn b11_the_teammates_concede_while_the_offer_is_open_ends_the_game_for_the_other_team() {
        let state = begun("b11-end");
        let state = ok(&state, A1, DuosActionBody::Concede);
        let result = act(&state, A2, DuosActionBody::Concede);
        assert!(result.error.is_none());
        assert!(
            result
                .events
                .iter()
                .any(|event| matches!(event, DuosEvent::Ended { .. }))
        );
        expect_result(&result.state, DuosOutcome::Win, Some(Team::B), Some("concede"));
    }

    #[test]
    fn b11_the_same_seat_conceding_again_does_not_end_the_game() {
        let state = begun("b11-again");
        let state = ok(&state, A1, DuosActionBody::Concede);
        let again = act(&state, A1, DuosActionBody::Concede);
        assert!(
            again.state.result.is_none(),
            "only the teammate's concede ends the game"
        );
        assert_eq!(again.state.concede_offer, PerTeam::new(Some(A1), None));
    }

    #[test]
    fn b11_an_offer_lapses_when_its_round_ends() {
        let state = begun("b11-lapse");
        let state = ok(&state, A1, DuosActionBody::Concede);
        let state = finish_round(state);
        assert_eq!(state.concede_offer, PerTeam::new(None, None));
        assert_eq!(duos_view_for(&state, A2).concede_offered_by, None);

        let state = finish_round(state);
        assert_eq!(state.round, 3);
        let state = ok(&state, A2, DuosActionBody::Concede);
        assert!(
            state.result.is_none(),
            "A1's old offer lapsed: A2's concede opens a new one"
        );
        assert_eq!(state.concede_offer, PerTeam::new(Some(A2), None));
    }

    #[test]
    fn b11_each_team_holds_its_own_offer_so_the_other_teams_concede_never_clears_it() {
        let state = begun("b11-two-offers");
        let state = ok(&state, A1, DuosActionBody::Concede);
        let state = ok(&state, B1, DuosActionBody::Concede);
        assert!(state.result.is_none(), "two offers, one per team, end nothing");
        assert_eq!(state.concede_offer, PerTeam::new(Some(A1), Some(B1)));
        for (seat, offer) in [(A1, A1), (A2, A1), (B1, B1), (B2, B1)] {
            assert_eq!(
                duos_view_for(&state, seat).concede_offered_by,
                Some(offer),
                "{seat} sees its own team's offer"
            );
        }

        // Team A's offer still stands: A2's concede joins it and the game ends for team B.
        let result = act(&state, A2, DuosActionBody::Concede);
        assert!(result.error.is_none());
        expect_result(&result.state, DuosOutcome::Win, Some(Team::B), Some("concede"));

        // And the other way: team B's offer stands through team A's.
        let state = ok(&state, B2, DuosActionBody::Concede);
        expect_result(&state, DuosOutcome::Win, Some(Team::A), Some("concede"));
    }

    #[test]
    fn b11_team_b_conceding_gives_team_a_the_win() {
        let state = begun("b11-b");
        let state = ok(&state, B1, DuosActionBody::Concede);
        assert!(state.result.is_none());
        assert_eq!(state.concede_offer, PerTeam::new(None, Some(B1)));
        let state = ok(&state, B2, DuosActionBody::Concede);
        expect_result(&state, DuosOutcome::Win, Some(Team::A), Some("concede"));
    }
}

// ---------------------------------------------------------------------------------------------
// B12–B18: life
// ---------------------------------------------------------------------------------------------

mod b12_starting_life {
    use super::*;

    #[test]
    fn b12_each_seat_starts_with_own_health_and_a_full_pool() {
        let state = begun("b12");
        for seat in DUOS_SEATS {
            assert_eq!(health(&state, seat), DUOS_OWN_HEALTH, "{seat}");
            let life = side(&state, seat)
                .duos
                .as_ref()
                .expect("a Duos side carries its life");
            assert_eq!(life.pool, DUOS_POOL_HEALTH, "{seat}");
            assert_eq!(life.pool_cap, DUOS_POOL_HEALTH, "{seat}");
            assert!(!life.spent, "{seat}");
        }
        assert_eq!(state.teams.a.pool, DUOS_POOL_HEALTH);
        assert_eq!(state.teams.b.pool, DUOS_POOL_HEALTH);
        assert_eq!(state.teams.a.rules, DUOS_HUMAN_TEAM);
        assert_eq!(state.teams.b.rules, DUOS_HUMAN_TEAM);
    }

    #[test]
    fn b12_each_teams_rules_set_its_seats_own_health_and_pool() {
        setup();
        let a = DuosTeamRules {
            own_health: 15,
            pool_health: 25,
            ..DUOS_HUMAN_TEAM
        };
        let b = duos_ai_team(Difficulty::Hard);
        let state = duos_scenario("b12-rules", decks(), Some(PerTeam { a, b }));
        for seat in [A1, A2] {
            assert_eq!(health(&state, seat), 15, "{seat}");
            let life = side(&state, seat).duos.as_ref().expect("Duos life");
            assert_eq!((life.pool, life.pool_cap, life.spent), (25, 25, false), "{seat}");
        }
        for seat in [B1, B2] {
            assert_eq!(health(&state, seat), DUOS_OWN_HEALTH, "{seat}");
            let life = side(&state, seat).duos.as_ref().expect("Duos life");
            assert_eq!(
                (life.pool, life.pool_cap, life.spent),
                (DUOS_POOL_HEALTH_HARD, DUOS_POOL_HEALTH_HARD, false),
                "{seat}"
            );
        }
        assert_eq!(state.teams.a.rules, a);
        assert_eq!(state.teams.b.rules, b);
        assert_eq!(state.teams.b.pool, DUOS_POOL_HEALTH_HARD);
    }

    #[test]
    fn b12_the_rules_constants_and_the_ai_team_tiers() {
        assert_eq!(
            (
                DUOS_OWN_HEALTH,
                DUOS_POOL_HEALTH,
                DUOS_POOL_HEALTH_HARD,
                DUOS_PASS_COST,
                DUOS_PASS_CAP
            ),
            (20, 20, 30, 1, 10)
        );
        assert_eq!(
            DUOS_HUMAN_TEAM,
            DuosTeamRules {
                own_health: DUOS_OWN_HEALTH,
                pool_health: DUOS_POOL_HEALTH,
                pass_cost: DUOS_PASS_COST,
                pass_cap: DUOS_PASS_CAP,
            }
        );
        assert_eq!(duos_ai_team(Difficulty::Easy), DUOS_HUMAN_TEAM);
        assert_eq!(
            duos_ai_team(Difficulty::Medium),
            DuosTeamRules {
                pass_cost: 0,
                ..DUOS_HUMAN_TEAM
            }
        );
        assert_eq!(
            duos_ai_team(Difficulty::Hard),
            DuosTeamRules {
                pass_cost: 0,
                pool_health: DUOS_POOL_HEALTH_HARD,
                ..DUOS_HUMAN_TEAM
            }
        );
    }

    #[test]
    fn b12_a_duos_duels_own_view_shows_both_sides_life() {
        let state = begun("b12-view");
        for (game, viewer) in [(0usize, P1), (0, P2), (1, P1), (1, P2)] {
            let view = json_of(view_for(&state.duels[game], viewer));
            for part in ["you", "opponent"] {
                let life = find_key(&view[part], "duos")
                    .unwrap_or_else(|| panic!("duel {game}, {viewer}'s view: no duos life on {part}"));
                assert_eq!(life["pool"], json!(DUOS_POOL_HEALTH));
                assert_eq!(life["poolCap"], json!(DUOS_POOL_HEALTH));
                assert_eq!(life["spent"], json!(false));
            }
        }
    }
}

mod b13_damage_carries_over {
    use super::*;

    #[test]
    fn b13_damage_goes_to_armor_then_own_hp_then_the_pool() {
        let mut state = begun("b13");
        side_mut(&mut state, B1).hero.health = 3;
        side_mut(&mut state, B1).hero.armor = 2;
        let state = cast(state, A1, BOLT).state; // 6: 2 to armor, 3 to own HP, 1 from the pool
        let carried = BOLT_DAMAGE - 2 - 3;

        // Armor takes its part of the hit "as today" (§4.4 step 2): it is subtracted and stays.
        assert_eq!(side(&state, B1).hero.armor, 2);
        assert!(is_spent(&state, B1));
        assert_eq!(pool_of(&state, B1), DUOS_POOL_HEALTH - carried);
        assert_eq!(
            health(&state, B1),
            DUOS_POOL_HEALTH - carried,
            "a spent hero's health reads the pool"
        );
        assert_eq!(state.teams.b.pool, DUOS_POOL_HEALTH - carried);
        assert!(!is_spent(&state, B2));
        assert_eq!(health(&state, B2), DUOS_OWN_HEALTH);
        assert!(state.result.is_none());

        let enemy = duos_view_for(&state, A1).enemy;
        assert_eq!(enemy.pool, DUOS_POOL_HEALTH - carried);
        assert_eq!((enemy.sides[0].own, enemy.sides[0].spent), (0, true));
        assert_eq!(
            (enemy.sides[1].own, enemy.sides[1].spent),
            (DUOS_OWN_HEALTH, false)
        );
    }

    #[test]
    fn b13_a_hit_own_hp_can_hold_leaves_the_pool_whole() {
        let state = begun("b13-held");
        let state = cast(state, A1, BOLT).state;
        assert_eq!(health(&state, B1), DUOS_OWN_HEALTH - BOLT_DAMAGE);
        assert!(!is_spent(&state, B1));
        assert_eq!(pool_of(&state, B1), DUOS_POOL_HEALTH);
        assert_eq!(state.teams.b.pool, DUOS_POOL_HEALTH);
    }

    #[test]
    fn b13_armor_that_holds_the_whole_hit_leaves_own_hp_and_pool_whole() {
        let mut state = begun("b13-armor");
        side_mut(&mut state, B1).hero.armor = BOLT_DAMAGE;
        let state = cast(state, A1, BOLT).state;
        // Armor takes the hit "as today" (§4.4 step 2): subtracted, and still on the hero.
        assert_eq!(side(&state, B1).hero.armor, BOLT_DAMAGE);
        assert_eq!(health(&state, B1), DUOS_OWN_HEALTH);
        assert!(!is_spent(&state, B1));
        assert_eq!(pool_of(&state, B1), DUOS_POOL_HEALTH);
    }

    #[test]
    fn b13_own_hp_reaching_exactly_zero_is_spent_with_nothing_carried() {
        let mut state = begun("b13-zero");
        side_mut(&mut state, B1).hero.health = BOLT_DAMAGE;
        let state = cast(state, A1, BOLT).state;
        assert!(is_spent(&state, B1), "own HP at 0 is spent");
        assert_eq!(pool_of(&state, B1), DUOS_POOL_HEALTH);
        assert_eq!(health(&state, B1), DUOS_POOL_HEALTH);
        assert!(state.result.is_none());
    }

    #[test]
    fn b13_combat_damage_carries_over_the_same_way() {
        let mut state = begun("b13-combat");
        let brute = put(&mut state.duels[0], BRUTE, slot(P1, Row::Units, 1), json!({}));
        side_mut(&mut state, B1).hero.health = 2;
        let body = legal_attack(&state, A1, &brute.id, "hero-p2");
        let state = ok(&state, A1, body);
        let carried = BRUTE_ATTACK - 2;
        assert!(is_spent(&state, B1));
        assert_eq!(pool_of(&state, B1), DUOS_POOL_HEALTH - carried);
        assert_eq!(health(&state, B1), DUOS_POOL_HEALTH - carried);
        assert_eq!(state.teams.b.pool, DUOS_POOL_HEALTH - carried);
    }

    #[test]
    fn b13_life_loss_carries_over_too() {
        let mut state = begun("b13-loss");
        side_mut(&mut state, B1).hero.health = 1;
        let state = cast(state, A1, DRAIN).state;
        let carried = DRAIN_LOSS - 1;
        assert!(is_spent(&state, B1));
        assert_eq!(pool_of(&state, B1), DUOS_POOL_HEALTH - carried);
        assert_eq!(health(&state, B1), DUOS_POOL_HEALTH - carried);
    }
}

mod b14_spent_heroes {
    use super::*;

    #[test]
    fn b14_a_spent_hero_takes_damage_and_life_loss_off_the_pool() {
        let state = begun("b14");
        let state = cast(state, A1, NUKE).state;
        assert!(is_spent(&state, B1));
        assert_eq!(pool_of(&state, B1), POOL_AFTER_NUKE);

        let state = cast(state, A1, BOLT).state;
        assert_eq!(pool_of(&state, B1), POOL_AFTER_NUKE - BOLT_DAMAGE);
        assert_eq!(health(&state, B1), POOL_AFTER_NUKE - BOLT_DAMAGE);
        assert_eq!(state.teams.b.pool, POOL_AFTER_NUKE - BOLT_DAMAGE);

        let state = cast(state, A1, DRAIN).state;
        let left = POOL_AFTER_NUKE - BOLT_DAMAGE - DRAIN_LOSS;
        assert_eq!(pool_of(&state, B1), left);
        assert_eq!(health(&state, B1), left);
        assert!(is_spent(&state, B1));
        assert!(state.result.is_none());
    }

    #[test]
    fn b14_healing_a_spent_hero_fills_the_pool_never_past_its_cap() {
        let state = begun("b14-cap");
        let state = cast(state, A1, BACKFIRE).state;
        assert!(is_spent(&state, A1));
        assert_eq!(pool_of(&state, A1), POOL_AFTER_NUKE);
        const {
            assert!(
                POOL_AFTER_NUKE + MEND_HEAL > DUOS_POOL_HEALTH,
                "the heal would overshoot the cap"
            )
        };

        let state = cast(state, A1, MEND).state;
        assert_eq!(pool_of(&state, A1), DUOS_POOL_HEALTH, "capped at pool_cap");
        assert_eq!(health(&state, A1), DUOS_POOL_HEALTH);
        assert_eq!(state.teams.a.pool, DUOS_POOL_HEALTH);
        assert_eq!(pool_of(&state, A2), DUOS_POOL_HEALTH);
        assert!(is_spent(&state, A1), "spent for the game");

        let state = cast(state, A1, MEND).state;
        assert_eq!(pool_of(&state, A1), DUOS_POOL_HEALTH, "a full pool takes no more");
    }

    #[test]
    fn b14_healing_a_hero_that_is_not_spent_has_no_cap() {
        let state = begun("b14-free");
        let state = cast(state, A2, MEND).state;
        assert_eq!(health(&state, A2), DUOS_OWN_HEALTH + MEND_HEAL);
        assert!(!is_spent(&state, A2));
        assert_eq!(pool_of(&state, A2), DUOS_POOL_HEALTH);
        assert_eq!(state.teams.a.pool, DUOS_POOL_HEALTH);
    }
}

mod b15_pool_sync {
    use super::*;

    #[test]
    fn b15_both_duels_always_agree_on_each_teams_pool() {
        let state = begun("b15");
        expect_pools_agree(&state);

        let state = cast(state, A1, NUKE).state; // B1 spent
        expect_pools_agree(&state);
        assert_eq!(pool_of(&state, B2), POOL_AFTER_NUKE);
        assert_eq!(
            health(&state, B2),
            DUOS_OWN_HEALTH,
            "B2 is not spent: its health is its own"
        );

        let state = cast(state, A2, NUKE).state; // B2 spent too, from what duel 0 left
        let left = POOL_AFTER_NUKE - (NUKE_DAMAGE - DUOS_OWN_HEALTH);
        assert_eq!(state.teams.b.pool, left);
        assert_eq!(pool_of(&state, B1), left);
        assert_eq!(
            health(&state, B1),
            left,
            "duel 1's hit reaches B1's health in duel 0"
        );
        expect_pools_agree(&state);

        let state = cast(state, A1, BOLT).state;
        assert_eq!(
            health(&state, B2),
            left - BOLT_DAMAGE,
            "duel 0's hit reaches B2's health in duel 1"
        );
        expect_pools_agree(&state);
        assert!(state.result.is_none());
    }
}

mod b16_game_end {
    use super::*;

    #[test]
    fn b16_a_team_whose_pool_empties_loses() {
        let state = begun("b16");
        let state = cast(state, A1, NUKE).state;
        assert!(state.result.is_none());
        let result = cast(state, A1, NUKE);
        assert!(result.state.teams.b.pool <= 0);
        assert!(
            result
                .events
                .iter()
                .any(|event| matches!(event, DuosEvent::Ended { .. }))
        );
        expect_result(
            &result.state,
            DuosOutcome::Win,
            Some(Team::A),
            Some("poolEmptied"),
        );
    }

    #[test]
    fn b16_both_pools_emptied_in_one_step_is_a_draw() {
        let state = begun("b16-draw");
        let state = cast(state, A1, BLAST).state;
        assert!(state.result.is_none());
        assert!(is_spent(&state, A1) && is_spent(&state, B1));
        let state = cast(state, A1, BLAST).state;
        assert!(state.teams.a.pool <= 0 && state.teams.b.pool <= 0);
        expect_result(&state, DuosOutcome::Draw, None, None);
    }

    #[test]
    fn b16_pools_the_two_end_turns_of_a_rounds_close_empty_are_emptied_in_one_step_a_draw() {
        let mut state = begun("b16-close");
        // Duel 0's End turn begins B1's turn, whose draw from an empty library is fatigue, and B1 is
        // spent with one point left in team B's pool.
        state.duels[0].players[P2].library.clear();
        for seat in [B1, B2] {
            let side = side_mut(&mut state, seat);
            if let Some(life) = side.duos.as_mut() {
                life.pool = 1;
                life.spent = seat == B1;
            }
            if seat == B1 {
                side.hero.health = 1;
            }
        }
        state.teams.b.pool = 1;
        // Duel 1's End turn runs A2's end-of-turn Ember: more than A2's own HP and team A's pool.
        for seat in [A1, A2] {
            if let Some(life) = side_mut(&mut state, seat).duos.as_mut() {
                life.pool = 1;
            }
        }
        state.teams.a.pool = 1;
        put(&mut state.duels[1], EMBER, slot(P1, Row::Units, 1), json!({}));
        let state = ok(&state, A1, end_turn());
        assert!(state.result.is_none());

        let result = act(&state, A2, end_turn());

        assert!(result.error.is_none(), "{:?}", result.error);
        let state = result.state;
        assert!(state.teams.b.pool <= 0, "duel 0's End turn emptied team B's pool");
        assert!(state.teams.a.pool <= 0, "duel 1's End turn emptied team A's pool");
        expect_result(&state, DuosOutcome::Draw, None, Some("poolEmptied"));
        assert_eq!(state.round, 1, "no round opens after the game ended");
        assert!(
            !result
                .events
                .iter()
                .any(|event| matches!(event, DuosEvent::RoundStarted { .. }))
        );
    }

    #[test]
    fn b16_a_duels_own_win_ends_the_duos_game_for_the_matching_team() {
        for (def_id, winner) in [(VICTORY, Team::A), (CONCESSION, Team::B)] {
            let state = begun(&format!("b16-{def_id}"));
            let state = cast(state, A1, def_id).state;
            assert!(state.duels[0].result.is_some(), "{def_id} ends duel 0");
            expect_result(&state, DuosOutcome::Win, Some(winner), Some("duelResult"));
        }
    }

    #[test]
    fn b16_a_duel_reaching_the_turn_cap_draws_the_duos_game() {
        let mut state = begun("b16-cap");
        for game in state.duels.iter_mut() {
            game.turn = TURN_CAP_PLAYER_TURNS;
        }
        state.round_turns = vec![TURN_CAP_PLAYER_TURNS; 2];
        let state = ok(&state, A1, end_turn());
        assert!(state.result.is_none(), "nothing ends before the round closes");
        let state = ok(&state, A2, end_turn());
        expect_result(&state, DuosOutcome::Draw, None, Some("turnCap"));
    }
}

mod b17_lethal_judge {
    use super::*;

    #[test]
    fn b17_a_hit_that_leaves_pool_is_not_lethal_so_a_lethal_replacement_holds() {
        let mut state = begun("b17");
        put(&mut state.duels[0], GAMBIT, slot(P2, Row::Backrow, 1), json!({}));
        // In a 1v1 game this hit would be lethal: 6 on 6 health.
        side_mut(&mut state, B1).hero.health = BOLT_DAMAGE;
        let result = cast(state, A1, BOLT);
        assert!(!has_duel_event(&result.events, GameEventType::Redirected));
        let state = result.state;
        assert!(is_spent(&state, B1));
        assert_eq!(pool_of(&state, B1), DUOS_POOL_HEALTH);
        assert_eq!(health(&state, A1), DUOS_OWN_HEALTH, "nothing came back at A1");
        assert!(state.result.is_none());
    }

    #[test]
    fn b17_a_hit_past_own_hp_and_the_pool_is_lethal() {
        let mut state = begun("b17-lethal");
        put(&mut state.duels[0], GAMBIT, slot(P2, Row::Backrow, 1), json!({}));
        let first = cast(state, A1, NUKE); // against 20 own and 20 pool: not lethal
        assert!(!has_duel_event(&first.events, GameEventType::Redirected));
        let state = first.state;
        assert!(is_spent(&state, B1));
        assert_eq!(pool_of(&state, B1), POOL_AFTER_NUKE);

        let second = cast(state, A1, NUKE); // against the pool alone: lethal, so it goes to A1
        assert!(has_duel_event(&second.events, GameEventType::Redirected));
        let state = second.state;
        assert_eq!(pool_of(&state, B1), POOL_AFTER_NUKE, "B1 took nothing");
        assert!(is_spent(&state, A1));
        assert_eq!(pool_of(&state, A1), POOL_AFTER_NUKE);
        assert!(state.result.is_none());
    }
}

mod b18_health_swap {
    use super::*;

    #[test]
    fn b18_a_health_swap_with_neither_hero_spent_swaps_own_hp() {
        let mut state = begun("b18");
        side_mut(&mut state, B1).hero.health = 7;
        let state = cast(state, A1, SWAP).state;
        assert_eq!(health(&state, A1), 7);
        assert_eq!(health(&state, B1), DUOS_OWN_HEALTH);
        assert!(!is_spent(&state, A1) && !is_spent(&state, B1));
        assert_eq!(state.teams.a.pool, DUOS_POOL_HEALTH);
        assert_eq!(state.teams.b.pool, DUOS_POOL_HEALTH);
    }

    #[test]
    fn b18_a_health_swap_with_a_spent_hero_does_nothing() {
        // The enemy hero spent (duel 0).
        let mut state = begun("b18-spent");
        side_mut(&mut state, A1).hero.health = 12;
        let state = cast(state, A1, NUKE).state;
        let state = cast(state, A1, SWAP).state;
        assert_eq!(health(&state, A1), 12);
        assert!(!is_spent(&state, A1));
        assert_eq!(health(&state, B1), POOL_AFTER_NUKE);
        assert!(is_spent(&state, B1));
        assert_eq!(state.teams.a.pool, DUOS_POOL_HEALTH);
        assert_eq!(state.teams.b.pool, POOL_AFTER_NUKE);

        // The caster's own hero spent (duel 1).
        let state = cast(state, A2, BACKFIRE).state;
        assert!(is_spent(&state, A2));
        let state = cast(state, A2, SWAP).state;
        assert_eq!(health(&state, A2), POOL_AFTER_NUKE);
        assert_eq!(health(&state, B2), DUOS_OWN_HEALTH);
        assert!(!is_spent(&state, B2));
        assert_eq!(state.teams.a.pool, POOL_AFTER_NUKE);
    }

    #[test]
    fn b18_a_health_swap_with_both_heroes_spent_does_nothing() {
        let state = begun("b18-both");
        let state = cast(state, A1, BACKFIRE).state;
        let state = cast(state, A1, NUKE).state;
        assert!(is_spent(&state, A1) && is_spent(&state, B1));
        let before = (
            health(&state, A1),
            health(&state, B1),
            state.teams.a.pool,
            state.teams.b.pool,
        );
        let state = cast(state, A1, SWAP).state;
        assert_eq!(
            (
                health(&state, A1),
                health(&state, B1),
                state.teams.a.pool,
                state.teams.b.pool
            ),
            before
        );
    }
}

// ---------------------------------------------------------------------------------------------
// B19–B22: passing
// ---------------------------------------------------------------------------------------------

mod b19_pass_legality {
    use super::*;

    #[test]
    fn b19_a_round_seat_is_offered_a_pass_of_every_card_in_its_hand() {
        let state = begun("b19");
        let mut offered = passes_offered(&state, A1);
        offered.sort();
        let mut hand = hand_ids(&state, A1);
        hand.sort();
        assert!(!hand.is_empty());
        assert_eq!(offered, hand);
    }

    #[test]
    fn b19_no_pass_without_the_mana_for_its_cost() {
        let mut state = begun("b19-mana");
        give(&mut state, A1, NOOP);
        side_mut(&mut state, A1).mana.current = DUOS_PASS_COST - 1;
        assert!(passes_offered(&state, A1).is_empty());
        let card = hand_ids(&state, A1)[0].clone();
        refused(&state, A1, pass(&card));
    }

    #[test]
    fn b19_a_team_that_passes_free_needs_no_mana() {
        let mut state = begun_with("b19-free", free_passes());
        give(&mut state, A1, NOOP);
        side_mut(&mut state, A1).mana.current = 0;
        let card = hand_ids(&state, A1)[0].clone();
        assert!(passes_offered(&state, A1).contains(&card));
        let before = side(&state, A2).hand.len();
        let state = ok(&state, A1, pass(&card));
        assert_eq!(side(&state, A2).hand.len(), before + 1);
        assert_eq!(side(&state, A1).mana.current, 0);
    }

    #[test]
    fn b19_no_pass_past_the_cap() {
        let capped = DuosTeamRules {
            pass_cost: 0,
            pass_cap: 2,
            ..DUOS_HUMAN_TEAM
        };
        let state = begun_with("b19-cap", capped);
        let cards = hand_ids(&state, A1);
        let state = ok(&state, A1, pass(&cards[0]));
        let state = ok(&state, A1, pass(&cards[1]));
        assert!(passes_offered(&state, A1).is_empty());
        refused(&state, A1, pass(&cards[2]));
        // The cap is each seat's own.
        let a2_card = hand_ids(&state, A2)[0].clone();
        assert!(passes_offered(&state, A2).contains(&a2_card));
        ok(&state, A2, pass(&a2_card));
    }

    #[test]
    fn b19_no_pass_into_a_full_hand() {
        let mut state = begun_with("b19-full", free_passes());
        while (side(&state, A2).hand.len() as i32) < HAND_CAP - 1 {
            give(&mut state, A2, NOOP);
        }
        let cards = hand_ids(&state, A1);
        let state = ok(&state, A1, pass(&cards[0])); // the last free slot
        assert_eq!(side(&state, A2).hand.len() as i32, HAND_CAP);
        assert!(passes_offered(&state, A1).is_empty());
        refused(&state, A1, pass(&cards[1]));
    }

    #[test]
    fn b19_no_pass_from_an_ended_seat() {
        let state = begun("b19-ended");
        let card = hand_ids(&state, A1)[0].clone();
        let state = ok(&state, A1, end_turn());
        assert!(passes_offered(&state, A1).is_empty());
        refused(&state, A1, pass(&card));
    }

    #[test]
    fn b19_no_pass_by_the_team_out_of_its_round() {
        let state = begun("b19-out");
        for seat in [B1, B2] {
            assert!(passes_offered(&state, seat).is_empty(), "{seat}");
            for card in hand_ids(&state, seat) {
                refused(&state, seat, pass(&card));
            }
        }
    }

    #[test]
    fn b19_no_pass_of_a_card_the_passer_does_not_hold_in_hand() {
        let mut state = begun("b19-held");
        let library_card = side(&state, A1).library[0].id.clone();
        refused(&state, A1, pass(&library_card));
        refused(&state, A1, pass("c9999"));
        let unit_on_field = put(&mut state.duels[0], BRUTE, slot(P1, Row::Units, 1), json!({}));
        refused(&state, A1, pass(&unit_on_field.id));
    }
}

mod b20_what_a_pass_does {
    use super::*;

    #[test]
    fn b20_a_pass_pays_its_cost_and_puts_the_card_at_the_right_end_of_the_teammates_hand() {
        let mut state = begun("b20");
        give(&mut state, A1, NOOP); // A1 keeps a move once the pass has spent its mana
        let card_id = hand_ids(&state, A1)[0].clone();
        if let Some(card) = side_mut(&mut state, A1)
            .hand
            .iter_mut()
            .find(|card| card.id == card_id)
        {
            card.radiant = true;
            card.cost_mod = 2;
            card.buffs = AttackHealth { attack: 1, health: 3 };
        }
        let original = side(&state, A1)
            .hand
            .iter()
            .find(|card| card.id == card_id)
            .cloned()
            .expect("the card is in A1's hand");
        let mana = side(&state, A1).mana.current;
        let teammate_hand = hand_ids(&state, A2);

        let result = act(&state, A1, pass(&card_id));
        assert!(result.error.is_none());
        let state = result.state;

        assert_eq!(side(&state, A1).mana.current, mana - DUOS_PASS_COST);
        assert!(!hand_ids(&state, A1).contains(&card_id));
        let hand = &side(&state, A2).hand;
        assert_eq!(hand.len(), teammate_hand.len() + 1);
        let kept: Vec<String> = hand[..teammate_hand.len()]
            .iter()
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(
            kept, teammate_hand,
            "the teammate's own cards stay where they were"
        );
        let moved = hand.last().expect("the passed card");
        assert_eq!(moved.def_id, original.def_id);
        assert_eq!(moved.owner, A2.player());
        assert_eq!(moved.controller, A2.player());
        assert_eq!(moved.zone, Zone::Hand { player: A2.player() });
        assert!(moved.radiant, "its face");
        assert_eq!(moved.cost_mod, original.cost_mod, "its cost modifier");
        assert_eq!(moved.buffs, original.buffs, "its buffs");
        assert_eq!(
            hand.iter().filter(|card| card.id == moved.id).count(),
            1,
            "a fresh id in the receiver's duel"
        );
        assert!(!side(&state, A2).library.iter().any(|card| card.id == moved.id));

        assert_eq!(
            passed(&result.events),
            [(A1, A2, card_id.clone(), moved.id.clone())]
        );
        assert_eq!(
            duos_view_for(&state, A1).team.passes_left,
            [DUOS_PASS_CAP - 1, DUOS_PASS_CAP]
        );
    }

    #[test]
    fn b20_a_passed_fused_card_takes_its_match_made_definitions_and_their_ingredients_along() {
        // A Fuse of a crafted card and a vanilla one, minted in duel 0 alone (R77, R102): its
        // definition and the crafted one it is made of live only in duel 0's `transient_defs`.
        let crafted = "craft:#duos";
        let fused = format!("t-1:{crafted}+fx-1");
        let mut state = begun("b20-fused");
        let mut crafted_def = catalog()[NOOP].clone();
        crafted_def.id = crafted.to_string();
        let mut fused_def = catalog()[NOOP].clone();
        fused_def.id = fused.clone();
        state.duels[0]
            .transient_defs
            .insert(crafted.to_string(), crafted_def);
        state.duels[0].transient_defs.insert(fused.clone(), fused_def);
        give(&mut state, A1, NOOP);
        let card = give(&mut state, A1, &fused);
        let state = ok(&state, A1, pass(&card));

        assert_eq!(
            side(&state, A2).hand.last().map(|card| card.def_id.clone()),
            Some(fused.clone())
        );
        let carried = &state.duels[1].transient_defs;
        assert!(carried.contains_key(&fused), "the passed card's definition");
        assert!(
            carried.contains_key(crafted),
            "and the crafted card it is fused from"
        );
        assert!(
            !carried.contains_key("fx-1"),
            "a catalog card's stays in the catalog"
        );
        // The receiver's duel reads it like any card of its own.
        duos_legal_actions(&state, A2);
        duos_view_for(&state, A2);
    }

    #[test]
    fn b20_a_pass_is_no_draw_play_or_discard_and_fires_no_trigger() {
        let mut state = begun("b20-quiet");
        give(&mut state, A1, NOOP);
        let wire = give(&mut state, A1, TRIPWIRE);
        let draws = json_of(side(&state, A2).draws);
        let result = act(&state, A1, pass(&wire));
        assert!(result.error.is_none());
        for kind in [
            GameEventType::Drawn,
            GameEventType::CardPlayed,
            GameEventType::Discarded,
            GameEventType::Burned,
            GameEventType::Damage,
        ] {
            assert!(!has_duel_event(&result.events, kind), "no {kind} event");
        }
        let state = result.state;
        assert_eq!(
            health(&state, A2),
            DUOS_OWN_HEALTH,
            "the card's arrival hook did not fire"
        );
        assert_eq!(json_of(side(&state, A2).draws), draws, "not a draw");
        assert_eq!(
            side(&state, A2).hand.last().map(|card| card.def_id.as_str()),
            Some(TRIPWIRE)
        );
    }
}

mod b21_playing_a_passed_card {
    use super::*;

    fn new_id(events: &[DuosEvent]) -> String {
        let moves = passed(events);
        assert_eq!(moves.len(), 1, "one pass");
        moves[0].3.clone()
    }

    #[test]
    fn b21_a_passed_card_is_playable_by_a_receiver_still_in_its_round() {
        let mut state = begun("b21");
        give(&mut state, A1, NOOP);
        let card = hand_ids(&state, A1)[0].clone();
        let result = act(&state, A1, pass(&card));
        assert!(result.error.is_none());
        let id = new_id(&result.events);
        let state = result.state;
        let body = legal_play(&state, A2, &id);
        let state = ok(&state, A2, body);
        assert!(!hand_ids(&state, A2).contains(&id));
        assert!(on_field(&state, A2, &id));
    }

    #[test]
    fn b21_a_receiver_that_has_ended_cannot_play_a_passed_card() {
        let mut state = begun("b21-ended");
        give(&mut state, A1, NOOP);
        let state = ok(&state, A2, end_turn());
        let card = hand_ids(&state, A1)[0].clone();
        let result = act(&state, A1, pass(&card));
        assert!(result.error.is_none());
        let id = new_id(&result.events);
        let state = result.state;
        assert_eq!(state.round, 1);
        assert_eq!(state.ended, [false, true]);
        assert!(hand_ids(&state, A2).contains(&id));
        assert!(!offers_play(&state, A2, &id));
        refused(&state, A2, raw_play(&state, A2, &id));
    }

    #[test]
    fn b21_a_receiver_outside_the_round_cannot_play_a_passed_card() {
        // A1's extra turn makes round 2 A1's and B2's: A2 is no round seat.
        let state = begun("b21-out");
        let state = cast(state, A1, AGAIN).state;
        let state = ok(&state, A1, end_turn());
        let state = ok(&state, A2, end_turn());
        assert_eq!(state.round_seats, [A1, B2]);
        let card = hand_ids(&state, A1)[0].clone();
        let result = act(&state, A1, pass(&card));
        assert!(result.error.is_none(), "a pass needs no round seat to receive it");
        let id = new_id(&result.events);
        let state = result.state;
        assert!(!offers_play(&state, A2, &id));
        refused(&state, A2, raw_play(&state, A2, &id));
    }
}

mod b22_passes_left {
    use super::*;

    fn capped() -> DuosTeamRules {
        DuosTeamRules {
            pass_cost: 0,
            pass_cap: 2,
            ..DUOS_HUMAN_TEAM
        }
    }

    #[test]
    fn b22_passes_reset_each_round_and_passes_left_reports_what_remains() {
        let state = begun_with("b22", capped());
        assert_eq!(duos_view_for(&state, A1).team.passes_left, [2, 2]);
        assert_eq!(duos_view_for(&state, A1).team.pass_cost, 0);
        assert_eq!(duos_view_for(&state, A1).enemy.pass_cost, DUOS_PASS_COST);

        let card = hand_ids(&state, A1)[0].clone();
        let state = ok(&state, A1, pass(&card));
        assert_eq!(duos_view_for(&state, A1).team.passes_left, [1, 2]);
        assert_eq!(duos_view_for(&state, A2).team.passes_left, [1, 2]);
        let card = hand_ids(&state, A2)[0].clone();
        let state = ok(&state, A2, pass(&card));
        assert_eq!(duos_view_for(&state, A1).team.passes_left, [1, 1]);

        let state = finish_round(state); // round 2: team B's
        let state = finish_round(state); // round 3: team A's again
        assert_eq!(duos_view_for(&state, A1).team.passes_left, [2, 2]);
        let cards = hand_ids(&state, A1);
        let state = ok(&state, A1, pass(&cards[0]));
        let state = ok(&state, A1, pass(&cards[1]));
        assert_eq!(duos_view_for(&state, A1).team.passes_left, [0, 2]);
    }

    #[test]
    fn b22_the_enemy_teams_ended_and_passes_left_are_hidden() {
        let mut state = begun("b22-hidden");
        give(&mut state, A1, NOOP);
        let card = hand_ids(&state, A1)[0].clone();
        let state = ok(&state, A1, pass(&card));
        let state = ok(&state, A2, end_turn());

        let own = duos_view_for(&state, A1).team;
        assert_eq!(own.ended, [false, true]);
        assert_eq!(own.passes_left, [DUOS_PASS_CAP - 1, DUOS_PASS_CAP]);
        for seat in [B1, B2] {
            let enemy = duos_view_for(&state, seat).enemy;
            assert_eq!(enemy.team, Team::A, "{seat}");
            assert_eq!(enemy.ended, [false, false], "{seat}");
            assert_eq!(enemy.passes_left, [0, 0], "{seat}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// B23: views
// ---------------------------------------------------------------------------------------------

mod b23_views {
    use super::*;

    #[test]
    fn b23_own_and_teammate_are_each_duels_own_view_for() {
        let mut state = begun("b23");
        give(&mut state, A1, NOOP);
        let card = hand_ids(&state, A1)[0].clone();
        let body = legal_play(&state, A1, &card);
        let state = ok(&state, A1, body);
        let state = cast(state, A2, BOLT).state;
        for seat in DUOS_SEATS {
            let view = duos_view_for(&state, seat);
            assert_eq!(view.seat, seat);
            assert_eq!(view.round, state.round);
            assert_eq!(
                json_of(&view.own),
                json_of(view_for(&state.duels[seat.duel()], seat.player())),
                "{seat}'s own duel"
            );
            let mate = seat.teammate();
            assert_eq!(
                json_of(&view.teammate),
                json_of(view_for(&state.duels[mate.duel()], mate.player())),
                "{seat}'s teammate's duel, from the teammate's seat"
            );
            assert_eq!(view.team.team, seat.team());
            assert_eq!(view.enemy.team, seat.across().team());
            assert!(view.concede_offered_by.is_none());
            assert!(view.result.is_none());
        }
    }

    #[test]
    fn b23_both_teams_life_is_public() {
        let state = begun("b23-life");
        let state = cast(state, A1, NUKE).state; // B1 spent
        for seat in DUOS_SEATS {
            let view = duos_view_for(&state, seat);
            let (a, b) = if seat.team() == Team::A {
                (&view.team, &view.enemy)
            } else {
                (&view.enemy, &view.team)
            };
            assert_eq!(
                (a.pool, a.pool_cap),
                (DUOS_POOL_HEALTH, DUOS_POOL_HEALTH),
                "{seat}"
            );
            assert_eq!(
                (b.pool, b.pool_cap),
                (POOL_AFTER_NUKE, DUOS_POOL_HEALTH),
                "{seat}"
            );
            let lives = |team: &jackioh_engine::wire::DuosTeamView| -> Vec<(i32, bool)> {
                team.sides.iter().map(|life| (life.own, life.spent)).collect()
            };
            assert_eq!(
                lives(a),
                [(DUOS_OWN_HEALTH, false), (DUOS_OWN_HEALTH, false)],
                "{seat}"
            );
            assert_eq!(lives(b), [(0, true), (DUOS_OWN_HEALTH, false)], "{seat}");
            assert_eq!(
                (a.pass_cost, b.pass_cost),
                (DUOS_PASS_COST, DUOS_PASS_COST),
                "{seat}"
            );
        }
    }

    #[test]
    fn b23_the_enemys_hidden_cards_never_appear_and_the_teammates_do() {
        let mut state = begun("b23-hidden");
        // A face-down card of A2's.
        put(&mut state.duels[1], GAMBIT, slot(P1, Row::Backrow, 1), json!({}));
        for seat in DUOS_SEATS {
            let seen = strings_in(&json_of(duos_view_for(&state, seat)));
            for enemy in DUOS_SEATS.into_iter().filter(|other| other.team() != seat.team()) {
                for id in deck_of(enemy) {
                    assert!(!seen.contains(&id), "{seat} sees {enemy}'s hidden {id}");
                }
            }
            for owner in [seat, seat.teammate()] {
                for card in &side(&state, owner).hand {
                    assert!(
                        seen.contains(&card.def_id),
                        "{seat} sees {owner}'s {}",
                        card.def_id
                    );
                }
            }
            let reads_trap = seen.contains(GAMBIT);
            assert_eq!(
                reads_trap,
                seat.team() == Team::A,
                "{seat} and A2's face-down card"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// B24, B25: replay
// ---------------------------------------------------------------------------------------------

mod b24_fold {
    use super::*;

    #[test]
    fn b24_a_fold_of_the_live_log_reaches_the_live_state_and_hash() {
        let mut live = Live::begin("b24");
        live.keep_all();
        let card = hand_ids(&live.state, A1)[0].clone();
        live.step(A1, pass(&card));
        play_randomly(&mut live, "b24", 100);
        assert!(live.state.round >= 2, "the log crosses a round");

        let hash = duos_hash(&live.state);
        let state_json = json_of(&live.state);
        let replayed = duos_fold(&DuosReplayInput {
            options: options("b24"),
            log: live.log,
        });
        assert_eq!(replayed.refused_at, None);
        assert_eq!(replayed.hash, hash);
        assert_eq!(duos_hash(&replayed.state), hash);
        assert_eq!(json_of(&replayed.state), state_json);
    }

    #[test]
    fn b24_a_refused_action_stops_the_fold_and_reports_its_index() {
        let mut live = Live::begin("b24-refused");
        live.keep_all();
        let kept = duos_hash(&live.state);
        let mut log = live.log;
        assert_eq!(log.len(), 4);
        // Team B is out of its round: refused. A1's end turn after it is never reached.
        log.push(DuosAction {
            seat: B1,
            nonce: nonce(),
            body: end_turn(),
        });
        log.push(DuosAction {
            seat: A1,
            nonce: nonce(),
            body: end_turn(),
        });
        let replayed = duos_fold(&DuosReplayInput {
            options: options("b24-refused"),
            log,
        });
        assert_eq!(replayed.refused_at, Some(4));
        assert_eq!(replayed.hash, kept);
        assert_eq!(
            replayed.state.ended,
            [false, false],
            "nothing after the refusal is applied"
        );
    }

    #[test]
    fn b24_a_refusal_at_the_first_action_leaves_the_begun_game_and_an_empty_log_folds_to_it() {
        let live = Live::begin("b24-first");
        let begun_hash = duos_hash(&live.state);
        let replayed = duos_fold(&DuosReplayInput {
            options: options("b24-first"),
            log: vec![DuosAction {
                seat: A1,
                nonce: nonce(),
                body: duel(ActionBody::OfferDraw),
            }],
        });
        assert_eq!(replayed.refused_at, Some(0));
        assert_eq!(replayed.hash, begun_hash);

        let empty = duos_fold(&DuosReplayInput {
            options: options("b24-first"),
            log: Vec::new(),
        });
        assert_eq!(empty.refused_at, None);
        assert_eq!(empty.hash, begun_hash);
    }
}

mod b25_fuzz {
    use super::*;

    #[test]
    fn b25_random_policy_games_finish_and_fold_to_their_live_hashes() {
        for i in 1..=FUZZ_SEEDS {
            let seed = format!("duos-fuzz-{i}");
            let mut live = Live::begin(&seed);
            play_randomly(&mut live, &seed, MAX_FUZZ_STEPS);
            let result = live
                .state
                .result
                .as_ref()
                .unwrap_or_else(|| panic!("{seed} did not finish in {MAX_FUZZ_STEPS} steps"));
            assert_eq!(
                result.winner.is_some(),
                result.outcome == DuosOutcome::Win,
                "{seed}: a win names its team, a draw none"
            );

            let hash = duos_hash(&live.state);
            let replayed = duos_fold(&DuosReplayInput {
                options: options(&seed),
                log: live.log,
            });
            assert_eq!(replayed.refused_at, None, "{seed}");
            assert_eq!(replayed.hash, hash, "{seed}");
        }
    }
}
