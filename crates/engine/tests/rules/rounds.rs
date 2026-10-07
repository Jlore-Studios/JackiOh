// `castRoundsUntilDeath` (effects/rounds.ts; SPEC §8.7 C+ #32.3 Blade Storm's base face, R59, R652):
// round after round casts the named Spell — each round a real Spell cast (R70) with its own state
// check — until a round in which a Unit died, `rounds` rounds, or no Unit is left. The real card's
// test (packages/cards/test/classic-plus/032-3-blade-storm.test.ts) covers the card again.
//
// Fixtures are this file's own: defs are `pcr-`, indexed from 5960.
//
// Port of `packages/engine/test/rounds.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::effects::{cast_rounds_until_death, damage_all};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{in_hand, new_game, put, slot};

/// TS's module `let nextIndex = 5960`, written out: each def takes the index it had.
fn def(name: &str, type_: &str, index: u32, extra: Value) -> CardDef {
    let face = json!({ "keywords": [], "text": name });
    let mut def = json!({
        "id": format!("pcr-{name}"),
        "index": index.to_string(),
        "name": format!("PCR {name}"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face,
        "radiant": face,
    });
    if let (Some(into), Some(from)) = (def.as_object_mut(), extra.as_object()) {
        for (key, value) in from {
            into.insert(key.clone(), value.clone());
        }
    }
    json_as(def)
}

fn unit(name: &str, index: u32, attack: i32, health: i32) -> CardDef {
    let face = json!({ "attack": attack, "health": health, "keywords": [], "text": name });
    def(name, "Unit", index, json!({ "base": face, "radiant": face }))
}

/// "Deal 1 damage to all Units", the Whirlwind each round casts.
fn ping_all() -> CardDef {
    def("ping-all", "Spell", 5961, json!({}))
}
/// Cast it round after round, up to three.
fn cast_storm() -> CardDef {
    def("cast-storm", "Spell", 5962, json!({}))
}
const SHORT_ROUNDS: i32 = 3;
/// Cast it round after round, up to thirty.
fn long_storm() -> CardDef {
    def("long-storm", "Spell", 5963, json!({}))
}
const LONG_ROUNDS: i32 = 30;
fn body1() -> CardDef {
    unit("body-1", 5964, 1, 1)
}
fn body5() -> CardDef {
    unit("body-5", 5965, 5, 5)
}
/// Survives every round of a short storm, so its hits count the rounds.
fn tank() -> CardDef {
    unit("tank", 5966, 1, SHORT_ROUNDS + 1)
}
fn plain() -> CardDef {
    unit("plain", 5967, 1, 1)
}

fn defs() -> Vec<CardDef> {
    vec![ping_all(), cast_storm(), long_storm(), body1(), body5(), tank(), plain()]
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn cry(f: impl Fn(&mut EffectContext<'_>) -> Vec<Effect> + Send + Sync + 'static) -> Script {
    Script {
        cry: Some(hook(f)),
        ..Script::default()
    }
}

fn scripts() -> Vec<(String, CardScripts)> {
    vec![
        (
            ping_all().id,
            both(cry(|_ctx| vec![damage_all(json_as(json!({ "amount": 1, "side": "any" })))])),
        ),
        (
            cast_storm().id,
            both(cry(|_ctx| {
                vec![cast_rounds_until_death(json_as(
                    json!({ "def": "pcr-ping-all", "rounds": SHORT_ROUNDS }),
                ))]
            })),
        ),
        (
            long_storm().id,
            both(cry(|_ctx| {
                vec![cast_rounds_until_death(json_as(
                    json!({ "def": "pcr-ping-all", "rounds": LONG_ROUNDS }),
                ))]
            })),
        ),
    ]
}

/// TS's module `let nonce`.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn act(state: &GameState, body: Value) -> ReduceResult {
    let n = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let input: ActionInput = json_as(body);
    let result = reduce(state, &input.with_nonce(format!("pcr{n}")));
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

/// p1's main phase, hands empty, this file's cards registered.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    for player in [PlayerId::P1, PlayerId::P2] {
        let keep: Vec<String> = state.players[player].hand.iter().map(|c| c.id.clone()).collect();
        state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": player })).state;
    }
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    for (id, script) in scripts() {
        registry.insert(id, script);
    }
    register_scripts(registry);
    for player in [PlayerId::P1, PlayerId::P2] {
        state.players[player].hand = Vec::new();
    }
    state
}

/// What `casted` hands back: the board before the play, and the play's result.
struct Casted {
    state: GameState,
    events: Vec<GameEvent>,
}

/// p1 casts a storm onto the board `setup` builds (a spare hand card keeps the turn going).
fn casted(seed: &str, setup: impl FnOnce(&mut GameState), def_id: Option<&str>) -> Casted {
    let mut state = playing(seed);
    setup(&mut state);
    let storm = def_id.map(str::to_string).unwrap_or_else(|| cast_storm().id);
    let card = in_hand(&mut state, &storm, PlayerId::P1, 1)
        .into_iter()
        .next()
        .expect("the storm");
    in_hand(&mut state, &plain().id, PlayerId::P1, 1);
    let result = act(
        &state,
        json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }),
    );
    Casted {
        state: result.state,
        events: result.events,
    }
}

fn casts(events: &[GameEvent]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if *def_id == ping_all().id))
        .count()
}

fn resolved(events: &[GameEvent]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, GameEvent::CardResolved { def_id, .. } if *def_id == ping_all().id))
        .count()
}

fn deaths(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Destroyed { def_id, .. } => Some(def_id.clone()),
            _ => None,
        })
        .collect()
}

mod r652_cast_rounds_until_death_each_round_casts_and_the_storm_stops_after_a_round_in_which_a_unit_died {
    use super::*;

    #[test]
    fn r652_a_round_in_which_a_unit_died_ends_it_a_1_1_dies_in_round_1_and_only_one_cast_happens() {
        let long = long_storm().id;
        let Casted { events, .. } = casted(
            "pcr-basic",
            |s| {
                put(s, &body1().id, slot(PlayerId::P2, Row::Units, 1), Default::default());
                put(s, &body5().id, slot(PlayerId::P1, Row::Units, 1), Default::default());
            },
            Some(long.as_str()),
        );
        assert_eq!(casts(&events), 1);
        assert_eq!(deaths(&events), vec![body1().id]);
    }

    #[test]
    fn r652_r70_each_cast_is_a_real_spell_cast_every_cast_is_announced_played_and_resolved() {
        let Casted { events, .. } = casted(
            "pcr-real",
            |s| {
                put(s, &tank().id, slot(PlayerId::P2, Row::Units, 1), Default::default());
            },
            None,
        );
        assert_eq!(casts(&events) as i32, SHORT_ROUNDS);
        assert_eq!(resolved(&events) as i32, SHORT_ROUNDS);
        let announced = events
            .iter()
            .filter(|event| matches!(event, GameEvent::CardAnnounced { def_id, .. } if *def_id == ping_all().id))
            .count();
        assert_eq!(announced as i32, SHORT_ROUNDS);
    }

    #[test]
    fn r652_with_no_death_the_storm_runs_exactly_the_round_cap_and_stops_settled() {
        let mut counter: Option<CardInstance> = None;
        let Casted { state, events } = casted(
            "pcr-cap",
            |s| {
                counter = Some(put(s, &tank().id, slot(PlayerId::P1, Row::Units, 1), Default::default()));
            },
            None,
        );
        assert!(counter.is_some(), "setup");
        assert_eq!(casts(&events) as i32, SHORT_ROUNDS);
        assert!(deaths(&events).is_empty());
        assert!(state.pending.is_none());
    }

    #[test]
    fn r652_with_no_unit_left_it_casts_nothing() {
        let Casted { events, .. } = casted("pcr-empty", |_| {}, None);
        assert_eq!(casts(&events), 0);
        assert!(!events.iter().any(|event| event.event_type() == GameEventType::Damage));
        assert!(deaths(&events).is_empty());
    }
}
