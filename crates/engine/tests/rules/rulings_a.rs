//! SPEC §11, rows R1 to R42 (R27 and R28 have their own subsystem test files): one test per row,
//! named after the row, asserting what that row says against the engine. A "decide" row asserts the
//! constant in `config.rs` and the behaviour it drives; a row whose behaviour belongs to a card that
//! arrives in M4 asserts the engine machinery the card will use and names the card test.
//!
//! Every fixture def and script here is this file's own, registered on top of the shared fixture
//! catalog so nothing collides with another test file (BUILD §0, CLAUDE.md).
//!
//! Port of `packages/engine/test/rulings-a.test.ts` (SURFACE §4.1, §8). TS held live
//! `CardInstance` objects and wrote through them; here a card handed back by a helper is a copy, so a
//! write goes through `edit` (the card under that id in the state) and a read after a change re-reads
//! it with `instance_in`.

use std::borrow::Borrow;
use std::collections::BTreeSet;

use jackioh_engine::effects;
use jackioh_engine::subsystems::fuse::fuse;
use jackioh_engine::subsystems::rotation::{ROTATION_ROWS, rotate_rings};
use jackioh_engine::subsystems::scorer::{ScorerOptions, ZEPHYRS_INDEX, candidate_defs, rank, top_three};
use jackioh_engine::testkit::*;

use super::fixtures::harness::{events_of_type, in_hand, new_game, put, set_library, sink_for, slot};
use super::fixtures::scripts::infinite_reserves;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;
const UNITS: Row = Row::Units;
const BACKROW: Row = Row::Backrow;

// ---------------------------------------------------------------------------
// Fixture definitions. Every id is prefixed `ra-`; the §8 card each one stands in for is named.
// ---------------------------------------------------------------------------

/// A plain body: the control case for placement, combat and zone rows.
const BODY: &str = "ra-body";
/// A second body, so a Transform has somewhere to go (R35).
const OTHER_BODY: &str = "ra-other-body";
/// Small enough for one Cleave hit to kill (R42).
const SMALL: &str = "ra-small";
/// #1 Big D-fender / #65.1 Spikey Pillow: 0 attack, so it cannot declare (R7).
const ZERO_ATTACK: &str = "ra-zero";
/// A Cry that pings the enemy hero, so R1, R17 and R22 can see whether it fired.
const CRIER: &str = "ra-crier";
/// #3 Right-house defender: Reborn plus a Death trigger (R8).
const REBORN_PINGER: &str = "ra-reborn";
/// #8 Mr. Vanilla (R23, R35).
const IMMUTABLE: &str = "ra-immutable";
/// #32r Prem Panther: Cleave, for R42's "Cleave included".
const CLEAVER: &str = "ra-cleaver";
/// #92 Felinor Fiender's Stack (R13).
const STACKER: &str = "ra-stack";
/// #4 Gary the Gambler: a coin-stat Cry, for R32.
const GAMBLER: &str = "ra-gambler";
/// #92 Felinor Fiender: printed stats plus the sum of your Felinors (R39).
const FIENDER: &str = "ra-fiender";
/// A set-stat layer that asks for less than printed, which R39 forbids.
const SHRINKER: &str = "ra-shrink";
/// A Felinor for Fiender to count (R39).
const FELINOR: &str = "ra-felinor";
/// #89 Corpse Eater: a hand trigger fed by units reaching a graveyard (R38).
const EATER: &str = "ra-eater";
/// #22 Carnivorous Cube: a Death hook driven by what the instance remembers (R41).
const CUBE: &str = "ra-cube";
/// Two cost-2 permanents, for R24's "ties go to the card nearest the top".
const TWO_TOP: &str = "ra-two-top";
const TWO_NEXT: &str = "ra-two-next";
/// A cost-1 card, for R26's odd/even split.
const ONE_COST: &str = "ra-one";
/// R65 outside play: X counts as 0 and an embiggen card as its base price (R24, R26).
const X_UNIT: &str = "ra-x-unit";
const EMBIGGEN_UNIT: &str = "ra-embiggen";
/// A unit token, which ceases to exist off the field (R11, R34, R38).
const UNIT_TOKEN: &str = "ra-unit-token";
/// #41's Sheep Token, what Sheepish turns a played unit into (R17).
const SHEEP: &str = "ra-sheep";
/// §7's Bread Token: printed 0/0, no text, cost 0, always summoned X/X (R37).
const BREAD: &str = "ra-bread";
/// A Spell with a Cry, for R1's cast.
const BOLT: &str = "ra-bolt";
/// #21 Hinder / #90.1 CN-Virus: cast on draw (R40).
const CAST_ON_DRAW_SPELL: &str = "ra-cast-on-draw";
/// #80 Zao Gao: the discard is the player's choice (R16).
const DISCARDER: &str = "ra-discarder";
/// A Spell, which can never replace a permanent on the board (R35).
const PLAIN_SPELL: &str = "ra-plain-spell";
/// A spell token, which reaches a graveyard like any spell (R11, R34).
const SPELL_TOKEN: &str = "ra-spell-token";
/// #76 Field of Dreams' replacements (R31).
const REMINISCE: &str = "ra-reminisce";
/// #41 Sheepish: a Trap that answers the opponent's summon (R17).
const SHEEPISH: &str = "ra-sheepish";
/// A script-less Trap, for the face-down rows (R33, R35).
const TRAP: &str = "ra-trap";
/// A Field Trap: same row as a Trap, and face-up once it has fired (R33, R35).
const FIELD_TRAP: &str = "ra-field-trap";
/// #73 Anti-oneshot Armor: the hero cap R18 says a health loss ignores.
const ANTI_ONESHOT: &str = "ra-anti-oneshot";
/// #79 Twinspell: the Field Spell that grants the next Spell an Echo (R30).
const TWINSPELL: &str = "ra-twinspell";

/// TS's object spread `{ ...def, ...extra }`: `extra`'s keys replace the definition's, shallowly.
fn spread(def: &mut Value, extra: Value) {
    if let (Some(fields), Value::Object(extra)) = (def.as_object_mut(), extra) {
        for (key, value) in extra {
            fields.insert(key, value);
        }
    }
}

/// TS `raUnit`: a Core unit fixture whose index is the next of this file's (index 301 on).
fn ra_unit(next: &mut i32, name: &str, attack: i32, health: i32, keywords: Value, extra: Value) -> CardDef {
    *next += 1;
    let index = *next;
    let mut def = json!({
        "id": format!("ra-{name}"),
        "index": format!("R{index}"),
        "name": format!("{name} (rulings-a)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "attack": attack, "health": health, "keywords": keywords, "text": name },
        "radiant": { "attack": attack * 2, "health": health * 2, "keywords": keywords, "text": format!("{name} radiant") },
    });
    spread(&mut def, extra);
    json_as(def)
}

/// TS `raCard`: a Core fixture of any type without stats.
fn ra_card(next: &mut i32, name: &str, card_type: &str, extra: Value) -> CardDef {
    *next += 1;
    let index = *next;
    let mut def = json!({
        "id": format!("ra-{name}"),
        "index": format!("R{index}"),
        "name": format!("{name} (rulings-a)"),
        "set": "Core",
        "type": card_type,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": format!("{name} radiant") },
    });
    spread(&mut def, extra);
    json_as(def)
}

/// TS `TOKEN_FIELDS`, spread under `extra`.
fn token_fields(extra: Value) -> Value {
    let mut fields = json!({ "token": true, "rarity": "Token", "tags": ["Token"] });
    spread(&mut fields, extra);
    fields
}

/// TS `DEFS`: built in the TS declaration order (so each index is the one TS gave it), listed in
/// `DEFS`' order.
fn defs() -> Vec<CardDef> {
    let mut next = 300;
    let n = &mut next;
    let body = ra_unit(n, "body", 3, 4, json!([]), json!({}));
    let other_body = ra_unit(n, "other-body", 2, 5, json!([]), json!({}));
    let small = ra_unit(n, "small", 1, 2, json!([]), json!({}));
    let zero_attack = ra_unit(n, "zero", 0, 8, json!([]), json!({}));
    let crier = ra_unit(n, "crier", 2, 6, json!([]), json!({}));
    let reborn_pinger = ra_unit(n, "reborn", 2, 2, json!([{ "kind": "Reborn" }]), json!({}));
    let immutable = ra_unit(n, "immutable", 3, 3, json!([{ "kind": "Immutable" }]), json!({}));
    let cleaver = ra_unit(n, "cleaver", 4, 6, json!([{ "kind": "Cleave" }]), json!({}));
    let stacker = ra_unit(n, "stack", 2, 2, json!([{ "kind": "Stack" }]), json!({}));
    let gambler = ra_unit(n, "gambler", 1, 1, json!([]), json!({}));
    let fiender = ra_unit(n, "fiender", 4, 4, json!([]), json!({}));
    let shrinker = ra_unit(n, "shrink", 3, 3, json!([]), json!({}));
    let felinor = ra_unit(n, "felinor", 1, 1, json!([]), json!({ "tags": ["Felinor"] }));
    let eater = ra_unit(n, "eater", 2, 2, json!([]), json!({}));
    let cube = ra_unit(n, "cube", 2, 2, json!([]), json!({}));
    let two_top = ra_unit(n, "two-top", 2, 2, json!([]), json!({ "cost": 2 }));
    let two_next = ra_unit(n, "two-next", 2, 2, json!([]), json!({ "cost": 2 }));
    let one_cost = ra_unit(n, "one", 1, 1, json!([]), json!({ "cost": 1 }));
    let x_unit = ra_unit(n, "x-unit", 1, 1, json!([]), json!({ "cost": "X" }));
    let embiggen_unit = ra_unit(n, "embiggen", 1, 1, json!([]), json!({ "cost": { "base": 2, "embiggen": 5 } }));
    let unit_token = ra_unit(n, "unit-token", 3, 3, json!([]), token_fields(json!({ "cost": 1 })));
    let sheep = ra_unit(n, "sheep", 1, 1, json!([]), token_fields(json!({ "cost": 1 })));
    let bread = ra_unit(
        n,
        "bread",
        0,
        0,
        json!([]),
        token_fields(json!({
            "cost": 0,
            "base": { "attack": 0, "health": 0, "keywords": [], "text": "" },
            "radiant": { "attack": 0, "health": 0, "keywords": [], "text": "" },
        })),
    );
    let bolt = ra_card(n, "bolt", "Spell", json!({}));
    let cast_on_draw_spell = ra_card(n, "cast-on-draw", "Spell", json!({}));
    let discarder = ra_card(n, "discarder", "Spell", json!({}));
    let plain_spell = ra_card(n, "plain-spell", "Spell", json!({}));
    let spell_token = ra_card(n, "spell-token", "Spell", token_fields(json!({})));
    let reminisce = ra_card(n, "reminisce", "Spell", json!({}));
    let zephyrs = ra_card(n, "zephyrs", "Spell", json!({ "index": ZEPHYRS_INDEX, "rarity": "Mythic" }));
    let sheepish = ra_card(n, "sheepish", "Trap", json!({}));
    let trap = ra_card(n, "trap", "Trap", json!({}));
    let field_trap = ra_card(n, "field-trap", "Field Trap", json!({}));
    let anti_oneshot = ra_card(n, "anti-oneshot", "Field Spell", json!({}));
    let twinspell = ra_card(n, "twinspell", "Field Spell", json!({}));

    vec![
        body,
        other_body,
        small,
        zero_attack,
        crier,
        reborn_pinger,
        immutable,
        cleaver,
        stacker,
        gambler,
        fiender,
        shrinker,
        felinor,
        eater,
        cube,
        two_top,
        two_next,
        one_cost,
        x_unit,
        embiggen_unit,
        unit_token,
        sheep,
        bread,
        bolt,
        cast_on_draw_spell,
        discarder,
        plain_spell,
        spell_token,
        twinspell,
        reminisce,
        zephyrs,
        sheepish,
        trap,
        field_trap,
        anti_oneshot,
    ]
}

/// One of this file's definitions by id (TS named each def by its `const`).
fn def(id: &str) -> CardDef {
    must(defs().into_iter().find(|def| def.id == id), id)
}

// ---------------------------------------------------------------------------
// Fixture effects and scripts.
// ---------------------------------------------------------------------------

/// #4: flip `count` coins, +1 attack per heads and +1 max health per tails (R32).
fn coin_stats(count: i32) -> Effect {
    Effect::new("ra:coinStats", move |ctx| {
        let mut attack = 0;
        let mut health = 0;
        for _ in 0..count {
            if ctx.rng.coin() {
                attack += 1;
            } else {
                health += 1;
            }
        }
        apply_effects(
            &[effects::buff(json_as(json!({ "target": { "of": "self" }, "attack": attack, "health": health })))],
            ctx,
        );
    })
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn ping_enemy_hero(amount: i32) -> Hook {
    hook(move |_ctx| vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))])
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts: IndexMap<String, CardScripts> = IndexMap::new();
    scripts.insert(
        CRIER.into(),
        both(Script {
            cry: Some(ping_enemy_hero(3)),
            ..Script::default()
        }),
    );
    scripts.insert(
        BOLT.into(),
        both(Script {
            cry: Some(ping_enemy_hero(2)),
            ..Script::default()
        }),
    );
    scripts.insert(
        REBORN_PINGER.into(),
        both(Script {
            death: Some(ping_enemy_hero(1)),
            ..Script::default()
        }),
    );
    scripts.insert(
        GAMBLER.into(),
        both(Script {
            cry: Some(hook(|_ctx| vec![coin_stats(5)])),
            ..Script::default()
        }),
    );
    scripts.insert(
        ANTI_ONESHOT.into(),
        both(Script {
            static_flags: Some(StaticFlags {
                anti_oneshot: Some(true),
                ..StaticFlags::default()
            }),
            ..Script::default()
        }),
    );
    scripts.insert(
        CAST_ON_DRAW_SPELL.into(),
        both(Script {
            static_flags: Some(StaticFlags {
                cast_on_draw: Some(true),
                ..StaticFlags::default()
            }),
            cry: Some(ping_enemy_hero(1)),
            ..Script::default()
        }),
    );
    scripts.insert(
        DISCARDER.into(),
        both(Script {
            cry: Some(hook(|_ctx| vec![effects::choose_from_hand(json_as(json!({ "step": "discard" })))])),
            resume: IndexMap::from([("discard", hook(|_ctx| vec![effects::discard(json_as(json!({})))]))]),
            ..Script::default()
        }),
    );
    // #41 Sheepish: when the opponent plays a unit and it resolves, transform it into a Sheep Token
    // (R17, R427: after its Cry).
    scripts.insert(
        SHEEPISH.into(),
        both(Script {
            triggers: vec![
                TriggerDef::new("sheepish", &[GameEventType::CardResolved], |_ctx, event| {
                    let GameEvent::CardResolved {
                        instance_id,
                        permanent: true,
                        ..
                    } = event
                    else {
                        return vec![];
                    };
                    vec![effects::transform(json_as(json!({ "instanceId": instance_id, "defId": SHEEP })))]
                })
                .with_when(|ctx, event| match event {
                    GameEvent::CardResolved { player, def_id, .. } => {
                        *player != ctx.controller && def_of(Some(&*ctx.state), def_id).type_ == CardType::Unit
                    }
                    _ => false,
                }),
            ],
            ..Script::default()
        }),
    );
    // #89 Corpse Eater: while in hand, a unit reaching any graveyard feeds it (R38).
    scripts.insert(
        EATER.into(),
        both(Script {
            hand_triggers: vec![TriggerDef::new("eat", &[GameEventType::EnteredGraveyard], |ctx, event| {
                let GameEvent::EnteredGraveyard { def_id, .. } = event else {
                    return vec![];
                };
                let def = def_of(Some(&*ctx.state), def_id);
                if def.type_ != CardType::Unit {
                    return vec![];
                }
                vec![effects::buff(json_as(json!({
                    "target": { "of": "self" },
                    "attack": def.base.attack.unwrap_or(0),
                    "health": def.base.health.unwrap_or(0),
                })))]
            })],
            ..Script::default()
        }),
    );
    // #22 Carnivorous Cube: the Death hook copies what the instance remembered, or does nothing (R41).
    scripts.insert(
        CUBE.into(),
        both(Script {
            death: Some(hook(|ctx| {
                let eaten = ctx
                    .self_
                    .as_ref()
                    .and_then(|card| card.memory.get("eaten"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                match eaten {
                    Some(eaten) => vec![effects::summon(json_as(json!({ "defId": eaten })))],
                    None => vec![],
                }
            })),
            ..Script::default()
        }),
    );
    // #92 Felinor Fiender: printed plus the combined stats of your Felinors, dormant ones too (R39).
    scripts.insert(
        FIENDER.into(),
        both(Script {
            set_stat: Some(read_hook(|args| {
                let state = args.state;
                let me = args.self_;
                let mut units = owned(active_units_of(state, me.controller));
                units.extend(owned(dormant_units_of(state, me.controller)));
                let mut sum = SetStat {
                    attack: Some(0),
                    max_health: Some(0),
                };
                for unit in units
                    .iter()
                    .filter(|unit| unit.id != me.id && def_of(Some(state), &unit.def_id).tags.contains(&Tag::Felinor))
                {
                    let stats = stats_with_buffs(state, unit);
                    sum = SetStat {
                        attack: Some(sum.attack.unwrap_or(0) + stats.attack),
                        max_health: Some(sum.max_health.unwrap_or(0) + stats.max_health),
                    };
                }
                sum
            })),
            ..Script::default()
        }),
    );
    scripts.insert(
        SHRINKER.into(),
        both(Script {
            set_stat: Some(read_hook(|_args| SetStat {
                attack: Some(-5),
                max_health: Some(-5),
            })),
            ..Script::default()
        }),
    );
    scripts
}

// ---------------------------------------------------------------------------
// Harness.
// ---------------------------------------------------------------------------

/// A fresh game whose catalog and script registry also carry this file's fixtures.
fn game(seed: &str) -> GameState {
    let state = new_game(&format!("rulings-a-{seed}"), None);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    registry.extend(scripts());
    register_scripts(registry);
    state
}

/// Past both mulligans, in p1's main phase on turn 1.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&game(seed)).state;
    for player in [P1, P2] {
        let keep = ids(&state.players[player].hand);
        state = reduce(
            &state,
            &action(json!({ "type": "mulligan", "keep": keep, "playerId": player, "nonce": format!("{seed}-mull-{player}") })),
        )
        .state;
    }
    state
}

fn must<T>(value: Option<T>, what: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("missing {what}"),
    }
}

fn at<T: Clone>(list: &[T], index: usize) -> T {
    must(list.get(index).cloned(), &format!("item {index}"))
}

fn instance_in(state: &GameState, id: &str) -> CardInstance {
    must(find_instance(state, id).cloned(), &format!("instance {id}"))
}

/// TS wrote through the live instance (`unit.damage = 2`): here, the card under that id in the state.
fn edit(state: &mut GameState, card: &CardInstance, change: impl FnOnce(&mut CardInstance)) {
    change(must(find_instance_mut(state, &card.id), &format!("instance {}", card.id)));
}

/// TS `RunOptions = HookOptions & { self?: CardInstance | null }`.
#[derive(Default)]
struct RunOptions {
    self_: Option<CardInstance>,
    hook: HookOptions,
}

fn controlled(controller: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(controller),
        ..HookOptions::default()
    }
}

/// `{ controller }`.
fn by(controller: PlayerId) -> RunOptions {
    RunOptions {
        hook: controlled(controller),
        ..RunOptions::default()
    }
}

/// `{ controller, targets }`.
fn targeting(controller: PlayerId, targets: Vec<Selection>) -> RunOptions {
    RunOptions {
        hook: HookOptions {
            controller: Some(controller),
            targets: Some(targets),
            ..HookOptions::default()
        },
        ..RunOptions::default()
    }
}

/// `{ self }`.
fn as_self(card: &CardInstance) -> RunOptions {
    RunOptions {
        self_: Some(card.clone()),
        ..RunOptions::default()
    }
}

/// Apply one effect the way `resolve.rs` does, and hand back the events it emitted.
fn run(state: &mut GameState, effect: Effect, options: RunOptions) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(&mut sink, options.self_.as_ref(), options.hook);
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn sel(card: &CardInstance) -> Selection {
    Selection::Instance {
        instance_id: card.id.clone(),
    }
}

fn pick(card: &CardInstance) -> Vec<Selection> {
    vec![sel(card)]
}

fn action(body: Value) -> Action {
    json_as(body)
}

fn play(state: &GameState, instance_id: &str, nonce: &str) -> ReduceResult {
    reduce(
        state,
        &action(json!({ "type": "play", "instanceId": instance_id, "playerId": state.active, "nonce": nonce })),
    )
}

/// Enough damage to take a unit to 0 health, then the state check that collects it (§4.5).
fn kill_and_check(state: &mut GameState, unit: &CardInstance) -> Vec<GameEvent> {
    let live = instance_in(state, &unit.id);
    let max_health = unit_view(state, &live).max_health;
    edit(state, unit, |card| card.damage = max_health);
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    state_check(&mut EngineSink::new(state, &mut events, &mut rng));
    state.rng_cursor = rng.cursor();
    events
}

fn hero(player: PlayerId) -> AttackTarget {
    AttackTarget::Hero { player }
}

fn unit_target(card: &CardInstance) -> AttackTarget {
    AttackTarget::Unit { instance: card.clone() }
}

/// `dealDamage`'s `{ source, target, amount }`.
fn hit(source: Option<CardInstance>, target: DamageTarget, amount: i32) -> DamageArgs {
    DamageArgs {
        source,
        target,
        amount,
        flags: None,
    }
}

/// TS `why…` refusals are `string | null`; here `Result<(), EngineError>` (SURFACE §4.4.9).
fn refusal<T>(result: Result<T, EngineError>) -> Option<String> {
    result.err().map(|error| error.message)
}

fn says(text: &str) -> Option<String> {
    Some(text.to_string())
}

fn ids<C: Borrow<CardInstance>>(cards: impl IntoIterator<Item = C>) -> Vec<String> {
    cards.into_iter().map(|card| card.borrow().id.clone()).collect()
}

fn def_ids<C: Borrow<CardInstance>>(cards: impl IntoIterator<Item = C>) -> Vec<String> {
    cards.into_iter().map(|card| card.borrow().def_id.clone()).collect()
}

fn owned<C: Borrow<CardInstance>>(cards: impl IntoIterator<Item = C>) -> Vec<CardInstance> {
    cards.into_iter().map(|card| card.borrow().clone()).collect()
}

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

/// `eventsOfType(events, type)` (fixtures/harness.ts), each event as its wire JSON so a field reads
/// by its TS name.
fn of_type(events: &[GameEvent], event_type: GameEventType) -> Vec<Value> {
    events_of_type(events, event_type).iter().map(to_json).collect()
}

/// `events.map((e) => e[key])`.
fn pluck(events: &[Value], key: &str) -> Vec<Value> {
    events.iter().map(|event| event[key].clone()).collect()
}

/// TS `events.findIndex((e, i) => …)`, -1 when none matches.
fn find_index(events: &[GameEvent], mut matches: impl FnMut(isize, &Value) -> bool) -> isize {
    for (index, event) in events.iter().enumerate() {
        let index = index as isize;
        if matches(index, &to_json(event)) {
            return index;
        }
    }
    -1
}

/// vitest's `toMatchObject`: every key the expected object names matches, recursively; arrays match
/// element for element and in length; anything else is equal.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (_, Value::Object(wanted)) => wanted.iter().all(|(key, value)| {
            actual
                .as_object()
                .and_then(|fields| fields.get(key))
                .is_some_and(|found| matches_object(found, value))
        }),
        (Value::Array(found), Value::Array(wanted)) => {
            found.len() == wanted.len() && found.iter().zip(wanted).all(|(a, b)| matches_object(a, b))
        }
        _ => actual == expected,
    }
}

fn assert_matches(actual: Value, expected: Value) {
    assert!(matches_object(&actual, &expected), "{actual} does not match {expected}");
}

// ---------------------------------------------------------------------------
// R1 to R42.
// ---------------------------------------------------------------------------

mod spec_11_rulings_r1_r42_m3_gate {
    use super::*;

    #[test]
    fn r1_fires_cry_only_on_a_play_from_hand_or_a_cast_never_on_a_summon_recruit_or_transform() {
        // Decide row: the constant, then the behaviour it drives.
        const { assert!(CRY_ON_PLAY_ONLY) };

        // Played from hand: the Cry resolves.
        let mut played = playing("r1");
        let card = at(&in_hand(&mut played, CRIER, P1, 1), 0);
        let result = play(&played, &card.id, "r1-play");
        assert_eq!(result.error, None);
        assert_eq!(result.state.players.p2.hero.health, HERO_HEALTH - 3);

        // Summon, Recruit and Transform never fire it.
        let mut state = game("r1b");
        run(&mut state, effects::summon(json_as(json!({ "defId": CRIER }))), by(P1));
        assert_eq!(must(card_at(&state, slot(P1, UNITS, 1)), "summoned crier").def_id, CRIER);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);

        set_library(&mut state, P1, &[CRIER]);
        run(&mut state, effects::recruit(json_as(json!({}))), by(P1));
        assert_eq!(must(card_at(&state, slot(P1, UNITS, 2)), "recruited crier").def_id, CRIER);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);

        let victim = put(&mut state, BODY, slot(P1, UNITS, 3), json!({}));
        run(
            &mut state,
            effects::transform(json_as(json!({ "instanceId": victim.id, "defId": CRIER }))),
            by(P1),
        );
        assert_eq!(must(card_at(&state, slot(P1, UNITS, 3)), "transformed crier").def_id, CRIER);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);

        // A cast fires it (R70).
        let cast = at(&in_hand(&mut state, BOLT, P1, 1), 0);
        let mut sink = sink_for(&mut state);
        cast_card(&mut sink, &cast, CastOptions::default());
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 2);
    }

    #[test]
    fn r2_counts_the_cap_in_player_turns_60_turns_30_each_r389_then_the_game_is_a_draw() {
        assert_eq!(TURN_CAP_PLAYER_TURNS, 60);

        let mut state = playing("r2");
        // R389: two 20-card decks that do nothing fatigue out before player-turn 60 (§2.4, R3), so both
        // seats hold #75 Infinite Reserves, whose empty-library draws never fatigue, and the cap is what
        // ends the game.
        let reserves = infinite_reserves().id;
        put(&mut state, &reserves, slot(P1, BACKROW, 1), json!({}));
        put(&mut state, &reserves, slot(P2, BACKROW, 1), json!({}));
        for step in 0..100 {
            if state.result.is_some() {
                break;
            }
            let result = reduce(
                &state,
                &action(json!({ "type": "endTurn", "playerId": state.active, "nonce": format!("r2-{step}") })),
            );
            assert_eq!(result.error, None);
            state = result.state;
        }

        assert_eq!(
            state.result,
            Some(GameResult {
                winner: Winner::Draw,
                reason: GameOverReason::TurnCap,
            })
        );
        assert_eq!(state.turn, TURN_CAP_PLAYER_TURNS);
        assert_eq!(state.players.p1.turns_started, TURN_CAP_PLAYER_TURNS / 2);
        assert_eq!(state.players.p2.turns_started, TURN_CAP_PLAYER_TURNS / 2);
    }

    #[test]
    fn r3_makes_the_nth_draw_from_an_empty_library_deal_n_damage_to_that_hero() {
        assert_eq!(FATIGUE_DAMAGE(4), 4);

        let mut state = game("r3");
        state.players.p1.library = vec![];
        let mut sink = sink_for(&mut state);
        let outcomes = draw::draw(&mut sink, P1, 3);

        assert_eq!(outcomes, vec![DrawOutcome::Fatigue, DrawOutcome::Fatigue, DrawOutcome::Fatigue]);
        assert_eq!(sink.state.players.p1.fatigue_count, 3);
        assert_eq!(sink.state.players.p1.hero.health, HERO_HEALTH - (1 + 2 + 3));
        // No card was drawn, so the damage instance is the whole of what happened (§2.4).
        assert_eq!(of_type(sink.events, GameEventType::Drawn).len(), 0);
        assert_eq!(of_type(sink.events, GameEventType::Damage).len(), 3);
    }

    #[test]
    fn r4_caps_the_hand_at_10_and_burns_an_extra_draw_to_the_graveyard() {
        assert_eq!(HAND_CAP, 10);

        let mut state = game("r4");
        in_hand(&mut state, BODY, P1, HAND_CAP);
        let extra = must(set_library(&mut state, P1, &[OTHER_BODY]).into_iter().next(), "burned card");
        let mut sink = sink_for(&mut state);

        assert_eq!(draw::draw(&mut sink, P1, 1), vec![DrawOutcome::Burned]);
        assert_eq!(sink.state.players.p1.hand.len(), HAND_CAP as usize);
        assert_eq!(ids(&sink.state.players.p1.graveyard), vec![extra.id.clone()]);
        assert_eq!(of_type(sink.events, GameEventType::Burned).len(), 1);
    }

    #[test]
    fn r5_does_not_restrict_attacks_by_lane_any_unit_may_attack_any_enemy_unit_or_the_hero() {
        const { assert!(!LANE_RESTRICTED_ATTACKS) };

        let mut state = game("r5");
        let attacker = put(&mut state, BODY, slot(P1, UNITS, 1), json!({}));
        let far = put(&mut state, BODY, slot(P2, UNITS, 5), json!({}));

        assert_eq!(refusal(why_cannot_attack(&state, &attacker, &unit_target(&far))), None);
        assert_eq!(refusal(why_cannot_attack(&state, &attacker, &hero(P2))), None);
        assert_eq!(attack_targets(&state, &attacker).len(), 2);
    }

    #[test]
    fn r6_refuses_an_attack_from_defense_position_and_switching_to_attack_spends_the_exertion() {
        let mut state = playing("r6");
        let unit = put(&mut state, BODY, slot(P1, UNITS, 1), json!({}));
        edit(&mut state, &unit, |card| card.position = Some(Position::Def));
        let unit = instance_in(&state, &unit.id);

        assert_eq!(
            refusal(why_cannot_attack(&state, &unit, &hero(P2))),
            says("only Attack-Position units may attack")
        );

        let result = reduce(
            &state,
            &action(json!({ "type": "switchPosition", "instanceId": unit.id, "playerId": "p1", "nonce": "r6-switch" })),
        );
        assert_eq!(result.error, None);

        let switched = instance_in(&result.state, &unit.id);
        assert_eq!(switched.position, Some(Position::Atk));
        assert_eq!(to_json(&switched.exertion), json!({ "attacked": false, "switched": true }));
        assert_eq!(
            refusal(why_cannot_attack(&result.state, &switched, &hero(P2))),
            says("that unit has already acted this turn")
        );
    }

    #[test]
    fn r7_refuses_an_attack_declared_by_a_0_attack_unit() {
        let mut state = game("r7");
        let idle = put(&mut state, ZERO_ATTACK, slot(P1, UNITS, 1), json!({}));
        put(&mut state, BODY, slot(P2, UNITS, 1), json!({}));

        assert_eq!(unit_view(&state, &idle).attack, 0);
        assert_eq!(
            refusal(why_cannot_attack(&state, &idle, &hero(P2))),
            says("a unit with 0 attack cannot attack")
        );
        assert!(attack_targets(&state, &idle).is_empty());
    }

    #[test]
    fn r8_fires_death_on_both_deaths_of_a_reborn_unit() {
        let mut state = game("r8");
        let unit = put(&mut state, REBORN_PINGER, slot(P1, UNITS, 1), json!({}));

        kill_and_check(&mut state, &unit);
        // First death: the Death trigger fired and Reborn brought it back at 1 health without Reborn.
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 1);
        let back = must(card_at(&state, slot(P1, UNITS, 1)), "reborn unit").clone();
        assert_eq!(back.id, unit.id);
        assert_eq!(unit_view(&state, &back).health, 1);
        assert!(!unit_has(&state, &back, KeywordKind::Reborn));

        kill_and_check(&mut state, &back);
        // Second death: Death fires again and the unit stays in its owner's graveyard.
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 2);
        assert_eq!(ids(&state.players.p1.graveyard), vec![unit.id.clone()]);
        assert!(card_at(&state, slot(P1, UNITS, 1)).is_none());
    }

    #[test]
    fn r9_draws_the_mulligan_replacements_before_the_returned_cards_are_shuffled_back_in() {
        // p2's mulligan, so p1's turn-1 draw leaves the hand under test alone. The answers are sealed
        // until both are in (R265), so p1 keeps its hand to let p2's resolve.
        let state = begin_game(&game("r9")).state;
        let hand = state.players.p2.hand.clone();
        let returned = at(&hand, 0);
        let keep = ids(hand.iter().skip(1));
        let replacement = at(&state.players.p2.library, 0);

        let sealed = reduce(
            &state,
            &action(json!({ "type": "mulligan", "keep": keep, "playerId": "p2", "nonce": "r9-mull" })),
        );
        assert_eq!(sealed.error, None);
        let result = reduce(
            &sealed.state,
            &action(json!({
                "type": "mulligan",
                "keep": ids(&state.players.p1.hand),
                "playerId": "p1",
                "nonce": "r9-keep",
            })),
        );
        assert_eq!(result.error, None);

        let new_hand = ids(&result.state.players.p2.hand);
        assert_eq!(new_hand.len(), hand.len());
        // The replacement came off the top, before the returned card went back, so it cannot be redrawn.
        assert!(new_hand.contains(&replacement.id));
        assert!(!new_hand.contains(&returned.id));
        assert!(result.state.players.p2.library.iter().any(|card| card.id == returned.id));
    }

    #[test]
    fn r10_gives_the_first_player_their_turn_1_draw() {
        let mut state = begin_game(&game("r10")).state;
        state = reduce(
            &state,
            &action(json!({
                "type": "mulligan",
                "keep": ids(&state.players.p1.hand),
                "playerId": "p1",
                "nonce": "r10-m1",
            })),
        )
        .state;
        let opened = reduce(
            &state,
            &action(json!({
                "type": "mulligan",
                "keep": ids(&state.players.p2.hand),
                "playerId": "p2",
                "nonce": "r10-m2",
            })),
        );

        let started = find_index(&opened.events, |_, e| {
            e["type"] == "turnStarted" && e["player"] == "p1" && e["turn"] == 1
        });
        let drawn = find_index(&opened.events, |i, e| i > started && e["type"] == "drawn" && e["player"] == "p1");
        assert!(started >= 0);
        assert!(drawn > started);

        assert_eq!(opened.state.turn, 1);
        assert_eq!(opened.state.active, P1);
        assert_eq!(opened.state.players.p1.hand.len(), (at(OPENING_DRAW, 0) + 1) as usize);
    }

    #[test]
    fn r11_vanishes_a_unit_token_off_the_field_while_a_spell_token_reaches_the_graveyard() {
        let mut state = game("r11");

        let mut on_field = put(&mut state, UNIT_TOKEN, slot(P1, UNITS, 1), json!({}));
        assert!(is_unit_token(&state, &on_field));
        assert_eq!(
            move_to_zone(&mut state, &mut on_field, OffFieldZone::Graveyard, Default::default()),
            MoveResult::Vanished
        );
        assert!(state.players.p1.graveyard.is_empty());
        assert!(state.players.p1.exile.is_empty());

        // A unit-token card may sit in a hand or a library, and being drawn is not "leaving".
        let from_library = set_library(&mut state, P1, &[UNIT_TOKEN]).into_iter().next();
        {
            let mut sink = sink_for(&mut state);
            assert_eq!(draw::draw_one(&mut sink, P1, None), DrawOutcome::Drawn);
        }
        let drawn = must(from_library, "token from library");
        assert!(state.players.p1.hand.iter().any(|card| card.id == drawn.id));
        // Leaving the hand any other way — here, shuffled back — ends it.
        let mut drawn = instance_in(&state, &drawn.id);
        assert_eq!(
            move_to_zone(&mut state, &mut drawn, OffFieldZone::Library, Default::default()),
            MoveResult::Vanished
        );
        assert!(!state.players.p1.library.iter().any(|card| card.id == drawn.id));

        let mut spell_card = at(&in_hand(&mut state, SPELL_TOKEN, P1, 1), 0);
        assert_eq!(
            move_to_zone(&mut state, &mut spell_card, OffFieldZone::Graveyard, Default::default()),
            MoveResult::Moved
        );
        assert_eq!(ids(&state.players.p1.graveyard), vec![spell_card.id.clone()]);
    }

    #[test]
    fn r12_keeps_ownership_off_the_field_a_stolen_unit_dies_to_its_owner_s_graveyard() {
        let mut state = game("r12");
        let victim = put(&mut state, BODY, slot(P2, UNITS, 2), json!({}));

        run(&mut state, effects::steal(json_as(json!({ "instanceId": victim.id }))), by(P1));
        let stolen = instance_in(&state, &victim.id);
        assert_eq!(stolen.controller, P1);
        assert_eq!(stolen.owner, P2);

        kill_and_check(&mut state, &victim);
        assert_eq!(ids(&state.players.p2.graveyard), vec![victim.id.clone()]);
        assert!(state.players.p1.graveyard.is_empty());
        // Control means nothing off the field, so it goes back to the owner on the way out (R78).
        let dead = instance_in(&state, &victim.id);
        assert_eq!(dead.controller, P2);
        assert_eq!(dead.zone, Zone::Graveyard { player: P2 });
    }

    #[test]
    fn r13_keeps_a_card_under_a_stack_off_the_field_it_neither_acts_nor_can_be_targeted() {
        let mut state = game("r13");
        let buried = put(&mut state, BODY, slot(P1, UNITS, 1), json!({}));
        let mut top = new_instance(&mut state, STACKER, P1, Zone::Hand { player: P1 });
        assert!(place_on_field(
            &mut state,
            &mut top,
            slot(P1, UNITS, 1),
            PlaceOnFieldOptions { stack: Some(true) }
        ));

        assert_eq!(must(card_at(&state, slot(P1, UNITS, 1)), "top of pile").id, top.id);
        let buried = instance_in(&state, &buried.id);
        assert!(!is_active_on_field(&state, &buried));
        assert_eq!(ids(active_units_of(&state, P1)), vec![top.id.clone()]);
        assert_eq!(ids(dormant_units_of(&state, P1)), vec![buried.id.clone()]);

        let enemy = put(&mut state, BODY, slot(P2, UNITS, 1), json!({}));
        assert_eq!(
            refusal(why_cannot_attack(&state, &enemy, &unit_target(&buried))),
            says("that unit is not on the field")
        );
        assert_eq!(
            refusal(why_cannot_attack(&state, &buried, &hero(P2))),
            says("that unit is not on the field")
        );
        // Felinor Fiender's count is the one exception, exercised in the R39 test.
    }

    #[test]
    fn r14_rotates_two_independent_rings_bounces_a_locked_destination_and_carries_damage_and_buffs() {
        assert_eq!(ROTATION_RING, "two-rings");
        assert_eq!(ROTATION_ROWS.to_vec(), vec![UNITS, BACKROW]);

        let ring: Vec<String> = ring_order(UNITS, P1)
            .iter()
            .map(|zone| format!("{}-{}", zone.player, zone.lane))
            .collect();
        assert_eq!(
            ring,
            ["p1-1", "p1-2", "p1-3", "p1-4", "p1-5", "p2-5", "p2-4", "p2-3", "p2-2", "p2-1"]
        );

        let mut state = game("r14");
        let crosser = put(&mut state, BODY, slot(P1, UNITS, 5), json!({}));
        edit(&mut state, &crosser, |card| {
            card.damage = 1;
            card.buffs = AttackHealth { attack: 2, health: 0 };
        });
        let backrow_card = put(&mut state, ANTI_ONESHOT, slot(P1, BACKROW, 1), json!({}));

        let result = {
            let mut sink = sink_for(&mut state);
            rotate_rings(&mut sink, &json_as(json!({ "direction": "right", "perspective": "p1" })))
        };
        assert_eq!(must(card_at(&state, slot(P2, UNITS, 5)), "crossed unit").id, crosser.id);
        let crossed = instance_in(&state, &crosser.id);
        assert_eq!(crossed.controller, P2);
        assert_eq!(crossed.owner, P1);
        assert_eq!(crossed.damage, 1);
        assert_eq!(crossed.buffs, AttackHealth { attack: 2, health: 0 });
        assert_eq!(result.crossed, vec![crosser.id.clone()]);
        // The backrow is its own ring: the Field Spell stays in the backrow, one step on.
        assert_eq!(
            must(card_at(&state, slot(P1, BACKROW, 2)), "rotated backrow card").id,
            backrow_card.id
        );

        // A Locked destination bounces the card to its owner's hand instead.
        let mut locked = game("r14b");
        let blocked = put(&mut locked, BODY, slot(P1, UNITS, 2), json!({}));
        lock_zone(&mut locked, slot(P1, UNITS, 3));
        let locked_result = {
            let mut sink = sink_for(&mut locked);
            rotate_rings(&mut sink, &json_as(json!({ "direction": "right", "perspective": "p1" })))
        };
        assert_eq!(locked_result.bounced, vec![blocked.id.clone()]);
        assert!(ids(&locked.players.p1.hand).contains(&blocked.id));

        // #52 radiant: a card that would cross bounces to its owner's hand at cost 0 (R12, R14).
        let mut radiant = game("r14c");
        let bouncer = put(&mut radiant, BODY, slot(P1, UNITS, 5), json!({}));
        let radiant_result = {
            let mut sink = sink_for(&mut radiant);
            rotate_rings(
                &mut sink,
                &json_as(json!({ "direction": "right", "perspective": "p1", "radiant": true })),
            )
        };
        assert_eq!(radiant_result.bounced, vec![bouncer.id.clone()]);
        assert!(ids(&radiant.players.p1.hand).contains(&bouncer.id));
        assert_eq!(instance_in(&radiant, &bouncer.id).cost_override, Some(0));
    }

    #[test]
    fn r15_steals_into_the_same_lane_when_it_is_free_else_the_first_free_zone_leaving_the_excess() {
        let mut state = game("r15");

        let same_lane = put(&mut state, BODY, slot(P2, UNITS, 3), json!({}));
        run(&mut state, effects::steal(json_as(json!({ "instanceId": same_lane.id }))), by(P1));
        assert_eq!(must(card_at(&state, slot(P1, UNITS, 3)), "same-lane steal").id, same_lane.id);

        put(&mut state, BODY, slot(P1, UNITS, 1), json!({}));
        let displaced = put(&mut state, BODY, slot(P2, UNITS, 1), json!({}));
        run(&mut state, effects::steal(json_as(json!({ "instanceId": displaced.id }))), by(P1));
        assert_eq!(must(card_at(&state, slot(P1, UNITS, 2)), "first free steal").id, displaced.id);

        // With no free zone left the card stays with its opponent.
        put(&mut state, BODY, slot(P1, UNITS, 4), json!({}));
        put(&mut state, BODY, slot(P1, UNITS, 5), json!({}));
        let stays = put(&mut state, BODY, slot(P2, UNITS, 2), json!({}));
        run(&mut state, effects::steal(json_as(json!({ "instanceId": stays.id }))), by(P1));
        assert_eq!(must(card_at(&state, slot(P2, UNITS, 2)), "unstolen unit").id, stays.id);
        assert_eq!(instance_in(&state, &stays.id).controller, P2);
    }

    #[test]
    fn r16_makes_a_discard_the_player_s_choice_unless_the_card_says_random() {
        let mut state = playing("r16");
        let spell = at(&in_hand(&mut state, DISCARDER, P1, 1), 0);
        let hand_before = ids(state.players.p1.hand.iter().filter(|card| card.id != spell.id));

        let played = play(&state, &spell.id, "r16-play");
        assert_eq!(played.error, None);
        let prompt = must(played.state.pending.clone(), "discard prompt");
        assert_eq!(prompt.kind, PromptKind::Hand);
        assert_eq!(prompt.player_id, P1);
        assert_eq!(
            prompt.options.iter().map(|option| option.key.clone()).collect::<Vec<_>>(),
            hand_before.iter().map(|id| format!("instance:{id}")).collect::<Vec<_>>()
        );

        let chosen = at(&prompt.options, 1);
        let answered = reduce(
            &played.state,
            &action(json!({
                "type": "answer",
                "choiceId": prompt.id,
                "selection": [chosen.selection],
                "playerId": "p1",
                "nonce": "r16-answer",
            })),
        );
        assert_eq!(answered.error, None);
        let chosen_id = at(&hand_before, 1);
        assert!(!answered.state.players.p1.hand.iter().any(|card| card.id == chosen_id));
        assert!(answered.state.players.p1.graveyard.iter().any(|card| card.id == chosen_id));

        // "Random" takes no prompt: the match rng picks and the discard resolves at once.
        let mut random = game("r16b");
        let cards = in_hand(&mut random, BODY, P1, 3);
        run(&mut random, effects::discard_random(json_as(json!({ "count": 1 }))), by(P1));
        assert!(random.pending.is_none());
        assert_eq!(random.players.p1.hand.len(), 2);
        assert_eq!(random.players.p1.graveyard.len(), 1);
        assert!(ids(&cards).contains(&at(&random.players.p1.graveyard, 0).id));
    }

    #[test]
    fn r17_r427_fires_a_trap_after_the_played_card_resolves_its_cry_included_and_an_immutable_target_still_consumes_it(
    ) {
        let mut state = playing("r17");
        let armed = put(&mut state, SHEEPISH, slot(P2, BACKROW, 1), json!({}));
        let card = at(&in_hand(&mut state, CRIER, P1, 1), 0);
        let health = state.players.p2.hero.health;

        let result = play(&state, &card.id, "r17-play");
        assert_eq!(result.error, None);
        // The Cry ran first — the crier's 3 reached p2's hero — and then the trap answered the play's
        // resolution: the played unit is a Sheep.
        assert_eq!(result.state.players.p2.hero.health, health - 3);
        assert_eq!(must(card_at(&result.state, slot(P1, UNITS, 1)), "sheep").def_id, SHEEP);
        assert!(find_instance(&result.state, &card.id).is_none());
        let cried = find_index(&result.events, |_, e| e["type"] == "damage" && e["targetId"] == "hero-p2");
        let resolved = find_index(&result.events, |_, e| {
            e["type"] == "cardResolved" && e["instanceId"] == card.id.as_str()
        });
        let fired = find_index(&result.events, |_, e| {
            e["type"] == "trapFired" && e["instanceId"] == armed.id.as_str()
        });
        let transformed = find_index(&result.events, |_, e| e["type"] == "transformed");
        assert!(cried >= 0);
        assert!(resolved > cried);
        assert!(fired > resolved);
        assert!(transformed > fired);
        // The trap is consumed (§5.1).
        assert!(result.state.players.p2.graveyard.iter().any(|c| c.id == armed.id));

        // R23: an Immutable target refuses the Transform, and the trap still fires and is consumed.
        let mut immune = playing("r17b");
        let armed_again = put(&mut immune, SHEEPISH, slot(P2, BACKROW, 1), json!({}));
        let tough = at(&in_hand(&mut immune, IMMUTABLE, P1, 1), 0);
        let second = play(&immune, &tough.id, "r17b-play");
        assert_eq!(second.error, None);
        assert_eq!(
            must(card_at(&second.state, slot(P1, UNITS, 1)), "immutable unit").def_id,
            IMMUTABLE
        );
        assert_eq!(
            pluck(&of_type(&second.events, GameEventType::TrapFired), "instanceId"),
            vec![json!(armed_again.id)]
        );
        assert!(second.state.players.p2.graveyard.iter().any(|c| c.id == armed_again.id));
        // M4: cards/test/41-sheepish.test.ts proves the card half.
    }

    #[test]
    fn r18_makes_a_health_loss_skip_armor_the_hero_cap_and_the_damage_pipeline() {
        let mut state = game("r18");
        state.players.p1.hero.armor = 5;
        put(&mut state, ANTI_ONESHOT, slot(P1, BACKROW, 1), json!({}));
        let mut sink = sink_for(&mut state);

        // Damage pays Armor and then the Anti-oneshot cap (§4.4 steps 2 and 3).
        assert_eq!(
            deal_damage(&mut sink, hit(None, DamageTarget::Hero { player: P1 }, 20)),
            ANTI_ONESHOT_CAP.base
        );

        let before = sink.state.players.p1.hero.health;
        assert_eq!(damage::lose_health(&mut sink, P1, 20), 20);
        assert_eq!(sink.state.players.p1.hero.health, before - 20);
        assert_eq!(
            of_type(sink.events, GameEventType::HealthLost),
            vec![json!({ "type": "healthLost", "player": "p1", "amount": 20 })]
        );
        // It is not a damage instance, so nothing that watches damage ever sees it.
        assert_eq!(of_type(sink.events, GameEventType::Damage).len(), 1);
    }

    #[test]
    fn r19_lets_a_heal_name_any_unit_or_hero_on_either_side() {
        let mut state = game("r19");
        let ally = put(&mut state, BODY, slot(P1, UNITS, 1), json!({}));
        edit(&mut state, &ally, |card| card.damage = 2);
        let foe = put(&mut state, BODY, slot(P2, UNITS, 1), json!({}));
        edit(&mut state, &foe, |card| card.damage = 3);

        let scope = {
            let mut sink = sink_for(&mut state);
            let ctx = make_context(&mut sink, None, controlled(P1));
            effects::targets_in_scope(&ctx, Some(&json_as(json!({ "side": "any", "of": ["unit", "hero"] }))))
        };
        assert!(scope.contains(&sel(&ally)));
        assert!(scope.contains(&sel(&foe)));
        assert!(scope.contains(&Selection::Hero { player: P1 }));
        assert!(scope.contains(&Selection::Hero { player: P2 }));

        let heal = || effects::heal(json_as(json!({ "target": { "of": "chosen" }, "amount": 20 })));
        run(&mut state, heal(), targeting(P1, pick(&foe)));
        assert_eq!(instance_in(&state, &foe.id).damage, 0);
        run(&mut state, heal(), targeting(P1, pick(&ally)));
        assert_eq!(instance_in(&state, &ally.id).damage, 0);
        run(&mut state, heal(), targeting(P1, vec![Selection::Hero { player: P2 }]));
        // A hero has no maximum health, so the heal is not capped (§3, §6.3).
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH + 20);
    }

    #[test]
    fn r20_spends_no_exertion_when_an_effect_switches_a_position() {
        let mut state = game("r20");
        let unit = put(&mut state, BODY, slot(P1, UNITS, 1), json!({}));

        run(
            &mut state,
            effects::switch_position_of(json_as(json!({ "target": { "of": "chosen" }, "to": "DEF" }))),
            targeting(P1, pick(&unit)),
        );
        let switched = instance_in(&state, &unit.id);
        assert_eq!(switched.position, Some(Position::Def));
        assert_eq!(to_json(&switched.exertion), json!({ "attacked": false, "switched": false }));

        run(
            &mut state,
            effects::switch_position_of(json_as(json!({ "target": { "of": "chosen" }, "to": "ATK" }))),
            targeting(P1, pick(&unit)),
        );
        let back = instance_in(&state, &unit.id);
        assert_eq!(back.position, Some(Position::Atk));
        assert_eq!(to_json(&back.exertion), json!({ "attacked": false, "switched": false }));
        assert_eq!(refusal(why_cannot_attack(&state, &back, &hero(P2))), None);
    }

    #[test]
    fn r21_draws_random_keywords_from_the_fourteen_entry_pool_and_never_repeats_one_on_a_unit() {
        assert_eq!(
            RANDOM_KEYWORD_POOL.to_vec(),
            vec![
                "Taunt",
                "Armor 1",
                "Rush",
                "Charge",
                "First Strike",
                "Poisonous",
                "Lifesteal",
                "Reborn",
                "Divine Shield",
                "Trample",
                "Cleave",
                "Pierce",
                "Windfury",
                "Deft",
            ]
        );

        let mut state = game("r21");
        let unit = put(&mut state, BODY, slot(P1, UNITS, 1), json!({}));
        run(
            &mut state,
            effects::grant_random_keywords(json_as(json!({
                "target": { "of": "chosen" },
                "count": RANDOM_KEYWORD_POOL.len(),
            }))),
            targeting(P1, pick(&unit)),
        );

        let kinds: Vec<KeywordKind> = instance_in(&state, &unit.id)
            .granted_keywords
            .iter()
            .map(Keyword::kind)
            .collect();
        assert_eq!(kinds.len(), RANDOM_KEYWORD_POOL.len());
        assert_eq!(kinds.iter().collect::<BTreeSet<_>>().len(), kinds.len());
        assert!(!kinds.contains(&KeywordKind::Indestructible));
        assert!(!kinds.contains(&KeywordKind::Immutable));
        assert!(!kinds.contains(&KeywordKind::Stack));
        assert!(!kinds.contains(&KeywordKind::Lucky));

        // The pool is exhausted, so another draw grants nothing.
        run(
            &mut state,
            effects::grant_random_keywords(json_as(json!({ "target": { "of": "chosen" }, "count": 1 }))),
            targeting(P1, pick(&unit)),
        );
        assert_eq!(
            instance_in(&state, &unit.id).granted_keywords.len(),
            RANDOM_KEYWORD_POOL.len()
        );
    }

    #[test]
    fn r22_swaps_the_base_layer_on_the_field_keeps_damage_and_buffs_and_re_fires_no_cry() {
        let mut state = game("r22");
        let unit = put(&mut state, CRIER, slot(P1, UNITS, 1), json!({}));
        edit(&mut state, &unit, |card| {
            card.damage = 2;
            card.buffs = AttackHealth { attack: 1, health: 0 };
        });

        let events = run(
            &mut state,
            effects::set_radiant(json_as(json!({ "instanceId": unit.id }))),
            by(P1),
        );
        let now = instance_in(&state, &unit.id);
        assert!(now.radiant);
        let view = unit_view(&state, &now);
        assert_eq!(view.attack, 4 + 1);
        assert_eq!(view.max_health, 12);
        assert_eq!(view.health, 12 - 2);
        assert_eq!(now.damage, 2);
        assert_eq!(now.buffs, AttackHealth { attack: 1, health: 0 });
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);
        assert_eq!(
            pluck(&of_type(&events, GameEventType::RadiantSet), "instanceId"),
            vec![json!(unit.id)]
        );

        // Radiant Saintess includes itself: the effect may name the card that is running it.
        let saintess = put(&mut state, BODY, slot(P1, UNITS, 2), json!({}));
        run(
            &mut state,
            effects::set_radiant(json_as(json!({ "target": { "of": "self" } }))),
            as_self(&saintess),
        );
        assert!(instance_in(&state, &saintess.id).radiant);
        // The flag is never unset, so a second call changes nothing (§5.2).
        assert!(run(
            &mut state,
            effects::set_radiant(json_as(json!({ "instanceId": saintess.id }))),
            by(P1)
        )
        .is_empty());
    }

    #[test]
    fn r23_blocks_vanilla_transform_and_fuse_onto_on_an_immutable_card_while_radiant_still_works() {
        let mut state = game("r23");
        let warded = put(&mut state, IMMUTABLE, slot(P1, UNITS, 1), json!({}));
        let ingredient = put(&mut state, BODY, slot(P1, UNITS, 2), json!({}));

        run(&mut state, effects::vanilla(json_as(json!({ "instanceId": warded.id }))), by(P1));
        assert!(!instance_in(&state, &warded.id).vanilla);

        run(
            &mut state,
            effects::transform(json_as(json!({ "instanceId": warded.id, "defId": BODY }))),
            by(P1),
        );
        assert_eq!(must(card_at(&state, slot(P1, UNITS, 1)), "immutable unit").def_id, IMMUTABLE);

        let warded_now = instance_in(&state, &warded.id);
        let ingredient_now = instance_in(&state, &ingredient.id);
        let fused = {
            let mut sink = sink_for(&mut state);
            fuse(
                &mut sink,
                json_as(json!({ "ingredients": [warded_now, ingredient_now], "target": warded_now })),
            )
        };
        assert!(fused.is_none());
        assert_eq!(instance_in(&state, &warded.id).def_id, IMMUTABLE);
        assert_eq!(must(card_at(&state, slot(P1, UNITS, 2)), "ingredient").id, ingredient.id);

        run(&mut state, effects::set_radiant(json_as(json!({ "instanceId": warded.id }))), by(P1));
        assert!(instance_in(&state, &warded.id).radiant);
    }

    #[test]
    fn r24_reads_costs_per_r65_for_highest_and_lowest_and_ties_go_to_the_card_nearest_the_top() {
        // R65 outside play: an X-cost card counts as 0 and an embiggen card as its base price.
        assert_eq!(query_cost(&def(X_UNIT)), 0);
        assert_eq!(query_cost(&def(EMBIGGEN_UNIT)), 2);
        assert_eq!(query_cost(&def(TWO_TOP)), 2);

        let mut tie = game("r24");
        let near_top = must(set_library(&mut tie, P1, &[TWO_TOP, TWO_NEXT]).into_iter().next(), "top card");
        run(&mut tie, effects::recruit(json_as(json!({ "filter": { "cost": 2 } }))), by(P1));
        // The library is scanned top down, so the tie goes to the card nearest the top.
        assert_eq!(must(card_at(&tie, slot(P1, UNITS, 1)), "recruited card").id, near_top.id);

        let mut costs = game("r24b");
        set_library(&mut costs, P1, &[X_UNIT, EMBIGGEN_UNIT]);
        run(&mut costs, effects::recruit(json_as(json!({ "filter": { "cost": 0 } }))), by(P1));
        assert_eq!(must(card_at(&costs, slot(P1, UNITS, 1)), "x-cost card").def_id, X_UNIT);
        run(&mut costs, effects::recruit(json_as(json!({ "filter": { "cost": 2 } }))), by(P1));
        assert_eq!(
            must(card_at(&costs, slot(P1, UNITS, 2)), "embiggen card").def_id,
            EMBIGGEN_UNIT
        );
        // M4: cards/test/30-archivist.test.ts proves the card half.
    }

    #[test]
    fn r25_clamps_the_fib_index_at_fib_11_89() {
        assert_eq!(FIB.len(), 12);
        assert_eq!(at(FIB, 11), 89);
        assert_eq!(fib(11), 89);
        assert_eq!(fib(12), 89);
        assert_eq!(fib(500), 89);
        assert_eq!(fib(4), 3);
        assert_eq!(fib(0), 0);
        assert_eq!(fib(-3), 0);
        // M4: cards/test/31-kys-math-equation.test.ts proves the card half.
    }

    #[test]
    fn r26_reads_genn_s_greed_as_exile_all_odd_cost_cards() {
        assert_eq!(GENN_GREED_EXILES, "odd");

        let mut state = game("r26");
        let odd = at(&in_hand(&mut state, ONE_COST, P1, 1), 0);
        let even = at(&in_hand(&mut state, TWO_TOP, P1, 1), 0);
        let x_card = at(&in_hand(&mut state, X_UNIT, P1, 1), 0);

        let wanted = if GENN_GREED_EXILES == "odd" { 1 } else { 0 };
        for card in state.players.p1.hand.clone() {
            let cost = query_cost(def_of(Some(&state), &card.def_id));
            if cost % 2 != wanted {
                continue;
            }
            run(
                &mut state,
                effects::exile(json_as(json!({ "target": { "of": "chosen" } }))),
                targeting(P1, pick(&card)),
            );
        }

        assert_eq!(ids(&state.players.p1.exile), vec![odd.id.clone()]);
        // The X-cost card reads as 0 outside play, so the odd filter leaves it alone (R65).
        assert_eq!(ids(&state.players.p1.hand), vec![even.id.clone(), x_card.id.clone()]);
        assert_eq!(state.counters.exiled, 1);
        // M4: cards/test/94-genns-greed.test.ts proves the card half.
    }

    #[test]
    fn r29_ranks_every_non_token_core_card_except_97_and_offers_the_top_three() {
        let state = game("r29");
        let pool = candidate_defs();

        assert!(pool.len() > 3);
        assert!(!pool.iter().any(|def| def.index == ZEPHYRS_INDEX));
        assert!(!pool.iter().any(|def| def.token || def.tags.contains(&Tag::Token)));
        assert!(pool.iter().all(|def| def.set == SetName::Core));

        let ranked = rank(&state, P1, &ScorerOptions::default());
        assert_eq!(ranked.len(), pool.len());
        let ranked_ids: Vec<String> = ranked.iter().map(|entry| entry.def.id.clone()).collect();
        // Deterministic: the same state ranks the same way every time (§10.7, §9.3).
        assert_eq!(
            rank(&state, P1, &ScorerOptions::default())
                .iter()
                .map(|entry| entry.def.id.clone())
                .collect::<Vec<_>>(),
            ranked_ids
        );
        assert_eq!(
            top_three(&state, P1, &ScorerOptions::default())
                .iter()
                .map(|entry| entry.def.id.clone())
                .collect::<Vec<_>>(),
            ranked_ids.iter().take(3).cloned().collect::<Vec<_>>()
        );
        // M4: cards/test/97-zephyrs.test.ts proves the card half.
    }

    #[test]
    fn r30_keeps_a_twinspell_echo_until_a_spell_is_played_then_sends_twinspell_to_the_graveyard() {
        let mut state = playing("r30");
        let source = put(&mut state, TWINSPELL, slot(P1, BACKROW, 1), json!({}));
        let echo = {
            let mut sink = sink_for(&mut state);
            add_modifier(
                &mut sink,
                P1,
                ModifierExpiry::Used,
                ModifierKind::EchoNextSpell {
                    amount: 1,
                    source_id: Some(source.id.clone()),
                },
            )
        };

        // Cleanup is not its expiry: a "this turn" sweep leaves it alone (§2.2).
        {
            let mut sink = sink_for(&mut state);
            expire_modifiers(&mut sink, P1);
        }
        let mod_ids = |mods: &[PlayerModifier]| mods.iter().map(|m| m.id.clone()).collect::<Vec<_>>();
        assert_eq!(mod_ids(&state.players.p1.mods), vec![echo.id.clone()]);

        // A unit play is not a Spell, so the Echo waits.
        let unit = at(&in_hand(&mut state, BODY, P1, 1), 0);
        let after_unit = play(&state, &unit.id, "r30-unit");
        assert_eq!(after_unit.error, None);
        assert_eq!(mod_ids(&after_unit.state.players.p1.mods), vec![echo.id.clone()]);
        assert_eq!(
            must(card_at(&after_unit.state, slot(P1, BACKROW, 1)), "twinspell").id,
            source.id
        );

        // The next Spell uses it: the spell resolves twice and Twinspell goes to the graveyard.
        let mut after_unit_state = after_unit.state;
        let spell = at(&in_hand(&mut after_unit_state, BOLT, P1, 1), 0);
        let after_spell = play(&after_unit_state, &spell.id, "r30-spell");
        assert_eq!(after_spell.error, None);
        assert!(after_spell.state.players.p1.mods.is_empty());
        assert_eq!(after_spell.state.players.p2.hero.health, HERO_HEALTH - 4);
        assert!(card_at(&after_spell.state, slot(P1, BACKROW, 1)).is_none());
        assert!(after_spell.state.players.p1.graveyard.iter().any(|card| card.id == source.id));
        // M4: cards/test/79-twinspell.test.ts proves the card half.
    }

    #[test]
    fn r31_sends_a_replaced_hand_to_the_graveyard_where_reminisce_can_still_find_it() {
        let mut state = game("r31");
        let hand = in_hand(&mut state, BODY, P1, 3);

        for card in &hand {
            run(
                &mut state,
                effects::discard(json_as(json!({ "target": { "of": "chosen" } }))),
                targeting(P1, pick(card)),
            );
        }
        assert!(state.players.p1.hand.is_empty());
        assert_eq!(ids(&state.players.p1.graveyard), ids(&hand));
        assert!(state.players.p1.exile.is_empty());

        for _ in 0..hand.len() {
            run(&mut state, effects::add_to_hand(json_as(json!({ "defId": REMINISCE }))), by(P1));
        }
        assert_eq!(def_ids(&state.players.p1.hand), vec![REMINISCE, REMINISCE, REMINISCE]);
        // M4: cards/test/76-field-of-dreams.test.ts proves the card half.
    }

    #[test]
    fn r32_leaves_a_coin_stat_effect_alone_lucky_has_no_defined_best_so_it_changes_nothing() {
        let mut state = game("r32");
        let plain = put(&mut state, GAMBLER, slot(P1, UNITS, 1), json!({}));
        let lucky = put(&mut state, GAMBLER, slot(P1, UNITS, 2), json!({}));
        run(
            &mut state,
            effects::grant_keyword(json_as(json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Lucky", "n": 3 } }))),
            targeting(P1, pick(&lucky)),
        );
        assert!(unit_has(&state, &instance_in(&state, &lucky.id), KeywordKind::Lucky));

        // Both Cries start from the same (seed, cursor), so identical outcomes mean Lucky did nothing.
        let start = state.rng_cursor;
        let plain_now = instance_in(&state, &plain.id);
        let plain_cursor = {
            let mut sink = sink_for(&mut state);
            run_hook(&mut sink, &plain_now, HookName::Cry, HookOptions::default());
            sink.rng.cursor()
        };
        let lucky_now = instance_in(&state, &lucky.id);
        let lucky_cursor = {
            let mut sink = sink_for(&mut state);
            run_hook(&mut sink, &lucky_now, HookName::Cry, HookOptions::default());
            sink.rng.cursor()
        };

        let plain_buffs = instance_in(&state, &plain.id).buffs;
        assert_eq!(instance_in(&state, &lucky.id).buffs, plain_buffs);
        assert_eq!(plain_buffs.attack + plain_buffs.health, 5);
        // No extra rolls were taken for the Lucky unit either.
        assert_eq!(plain_cursor - start, 5);
        assert_eq!(lucky_cursor - start, 5);
        // M4: cards/test/4-gary-the-gambler.test.ts proves the card half.
    }

    #[test]
    fn r33_shows_a_face_down_trap_to_its_current_controller_only_and_a_fired_field_trap_to_both() {
        let mut state = game("r33");
        let hidden = put(&mut state, TRAP, slot(P2, BACKROW, 1), json!({}));

        assert_matches(
            to_json(&at(&view_for(&state, P2).you.backrow, 0)),
            json!({ "faceDown": false, "defId": TRAP }),
        );
        // R351: the marker carries the trap's cost and nothing else.
        assert_eq!(
            to_json(&at(&view_for(&state, P1).opponent.backrow, 0)),
            json!({ "faceDown": true, "cost": 0 })
        );

        run(&mut state, effects::steal(json_as(json!({ "instanceId": hidden.id }))), by(P1));
        let stolen = instance_in(&state, &hidden.id);
        assert_eq!(stolen.controller, P1);
        assert_eq!(stolen.owner, P2);
        assert_matches(
            to_json(&at(&view_for(&state, P1).you.backrow, 0)),
            json!({ "faceDown": false, "defId": TRAP }),
        );
        // Its owner stops seeing it, even though ownership never moved.
        assert_eq!(
            to_json(&at(&view_for(&state, P2).opponent.backrow, 0)),
            json!({ "faceDown": true, "cost": 0 })
        );

        let fired = put(&mut state, FIELD_TRAP, slot(P2, BACKROW, 2), json!({}));
        {
            let mut sink = sink_for(&mut state);
            consume_trap(&mut sink, &fired);
        }
        assert_eq!(instance_in(&state, &fired.id).face_up, Some(true));
        assert_matches(
            to_json(&at(&view_for(&state, P1).opponent.backrow, 1)),
            json!({ "faceDown": false, "defId": FIELD_TRAP }),
        );
        assert_matches(
            to_json(&at(&view_for(&state, P2).you.backrow, 1)),
            json!({ "faceDown": false, "defId": FIELD_TRAP }),
        );
    }

    #[test]
    fn r34_copies_token_cards_too_a_unit_token_card_and_a_spell_token_both_reach_the_library() {
        let mut state = game("r34");
        let before = state.players.p1.library.len();

        run(
            &mut state,
            effects::shuffle_into(json_as(json!({ "defId": UNIT_TOKEN, "count": 3 }))),
            by(P1),
        );
        run(
            &mut state,
            effects::shuffle_into(json_as(json!({ "defId": SPELL_TOKEN, "count": 1, "radiant": true }))),
            by(P1),
        );

        let library = &state.players.p1.library;
        assert_eq!(library.len(), before + 4);
        assert_eq!(library.iter().filter(|card| card.def_id == UNIT_TOKEN).count(), 3);
        assert_eq!(
            library.iter().filter(|card| card.def_id == SPELL_TOKEN && card.radiant).count(),
            1
        );
        // M4: cards/test/33-unstable-clone-machine.test.ts proves the card half.
    }

    #[test]
    fn r35_replaces_a_board_card_in_place_with_its_own_type_and_the_replaced_card_ceases_to_exist() {
        let mut state = game("r35");
        let unit = put(&mut state, BODY, slot(P1, UNITS, 2), json!({}));
        edit(&mut state, &unit, |card| card.position = Some(Position::Def));

        run(
            &mut state,
            effects::transform(json_as(json!({ "instanceId": unit.id, "defId": OTHER_BODY }))),
            by(P1),
        );
        let replacement = must(card_at(&state, slot(P1, UNITS, 2)), "replacement").clone();
        assert_eq!(replacement.def_id, OTHER_BODY);
        assert_ne!(replacement.id, unit.id);
        assert_eq!(replacement.position, Some(Position::Def));
        assert_eq!(replacement.owner, P1);
        // Ceased to exist: no graveyard, no exile pile, no Death.
        assert!(state.players.p1.graveyard.is_empty());
        assert!(!state.players.p1.exile.iter().any(|card| card.id == unit.id));

        // A Spell can never take a permanent's zone, so a same-type replacement is the only one.
        run(
            &mut state,
            effects::transform(json_as(json!({ "instanceId": replacement.id, "defId": PLAIN_SPELL }))),
            by(P1),
        );
        assert_eq!(
            must(card_at(&state, slot(P1, UNITS, 2)), "unchanged unit").def_id,
            OTHER_BODY
        );

        // Field Trap counts as Trap: both live in the backrow, so one replaces the other.
        let armed = put(&mut state, TRAP, slot(P1, BACKROW, 1), json!({}));
        run(
            &mut state,
            effects::transform(json_as(json!({ "instanceId": armed.id, "defId": FIELD_TRAP }))),
            by(P1),
        );
        assert_eq!(
            must(card_at(&state, slot(P1, BACKROW, 1)), "replaced trap").def_id,
            FIELD_TRAP
        );

        // Other zones: a hand card is replaced in place, same count.
        let hand_card = at(&in_hand(&mut state, BODY, P1, 1), 0);
        run(
            &mut state,
            effects::transform(json_as(json!({ "instanceId": hand_card.id, "defId": OTHER_BODY }))),
            by(P1),
        );
        assert_eq!(def_ids(&state.players.p1.hand), vec![OTHER_BODY]);
        // M4: cards/test/83-transmogulate.test.ts proves the card half.
    }

    #[test]
    fn r36_lets_only_the_active_player_offer_a_draw_once_a_turn_and_a_decline_blocks_3_of_their_turns() {
        assert_eq!(DRAW_OFFERS_PER_TURN, 1);
        assert_eq!(DRAW_OFFER_BLOCK_TURNS, 3);

        let mut state = playing("r36");
        assert!(can_offer_draw(&state, P1));
        assert!(!can_offer_draw(&state, P2));

        {
            let mut sink = sink_for(&mut state);
            offer_draw(&mut sink, P1);
            assert!(!can_offer_draw(sink.state, P1));

            answer_draw(&mut sink, P2, false);
        }
        let started = state.players.p1.turns_started;
        assert_eq!(
            state.players.p1.draw_offer.blocked_until,
            Some(started + DRAW_OFFER_BLOCK_TURNS + 1)
        );

        for turn in 1..=DRAW_OFFER_BLOCK_TURNS {
            state.turn += 2;
            state.players.p1.turns_started = started + turn;
            assert!(!can_offer_draw(&state, P1));
        }
        state.turn += 2;
        state.players.p1.turns_started = started + DRAW_OFFER_BLOCK_TURNS + 1;
        assert!(can_offer_draw(&state, P1));

        // Offers belong to the main phase only (§2.5).
        state.phase = Phase::End;
        assert!(!can_offer_draw(&state, P1));
    }

    #[test]
    fn r37_gives_the_unnamed_x_x_token_its_stats_through_statsoverride_at_cost_0() {
        assert_eq!(query_cost(&def(BREAD)), 0);

        let mut state = game("r37");
        run(
            &mut state,
            effects::summon(json_as(json!({ "defId": BREAD, "statsOverride": { "attack": 4, "health": 4 } }))),
            by(P1),
        );

        let token = must(card_at(&state, slot(P1, UNITS, 1)), "bread token").clone();
        assert_eq!(token.stats_override, Some(AttackHealth { attack: 4, health: 4 }));
        let view = unit_view(&state, &token);
        assert_eq!(view.attack, 4);
        assert_eq!(view.max_health, 4);
        // It counts as a Token for every filter and vanishes off the field (R11).
        assert!(is_unit_token(&state, &token));
        assert!(!query(&CatalogQueryArgs::default()).iter().any(|def| def.id == BREAD));
        // M4: cards/test/18-bread-and-butter.test.ts proves the card half.
    }

    #[test]
    fn r38_feeds_a_hand_trigger_from_a_unit_reaching_a_graveyard_and_never_from_a_token() {
        let mut state = game("r38");
        let hungry = at(&in_hand(&mut state, EATER, P1, 1), 0);
        let meal = put(&mut state, BODY, slot(P2, UNITS, 1), json!({}));
        let body = def(BODY);
        let fed = AttackHealth {
            attack: must(body.base.attack, "body attack"),
            health: must(body.base.health, "body health"),
        };

        let max_health = unit_view(&state, &meal).max_health;
        edit(&mut state, &meal, |card| card.damage = max_health);
        {
            let mut sink = sink_for(&mut state);
            state_check(&mut sink);
            settle(&mut sink, SettleOptions::default());
        }
        assert_eq!(instance_in(&state, &hungry.id).buffs, fed);

        // A unit token never reaches a graveyard, so it never feeds the trigger (R11).
        let token = put(&mut state, UNIT_TOKEN, slot(P2, UNITS, 2), json!({}));
        let max_health = unit_view(&state, &token).max_health;
        edit(&mut state, &token, |card| card.damage = max_health);
        let token_events = {
            let mut sink = sink_for(&mut state);
            state_check(&mut sink);
            settle(&mut sink, SettleOptions::default());
            sink.events.clone()
        };
        assert!(of_type(&token_events, GameEventType::EnteredGraveyard).is_empty());
        assert_eq!(instance_in(&state, &hungry.id).buffs, fed);
        // M4: cards/test/89-corpse-eater.test.ts proves the card half.
    }

    #[test]
    fn r39_gives_felinor_fiender_printed_plus_the_sum_of_your_felinors_never_below_printed() {
        assert_eq!(FIENDER_STATS_MODE, "printed-plus-sum");

        let mut state = game("r39");
        let boss = put(&mut state, FIENDER, slot(P1, UNITS, 1), json!({}));
        assert_eq!(unit_view(&state, &boss).attack, 4);

        put(&mut state, FELINOR, slot(P1, UNITS, 2), json!({}));
        // R13's one exception: a Felinor dormant under a Stack still counts for Fiender.
        let dormant = put(&mut state, FELINOR, slot(P1, UNITS, 3), json!({}));
        let mut top = new_instance(&mut state, STACKER, P1, Zone::Hand { player: P1 });
        assert!(place_on_field(
            &mut state,
            &mut top,
            slot(P1, UNITS, 3),
            PlaceOnFieldOptions { stack: Some(true) }
        ));
        assert_eq!(ids(dormant_units_of(&state, P1)), vec![dormant.id.clone()]);

        let view = unit_view(&state, &instance_in(&state, &boss.id));
        assert_eq!(view.attack, 4 + 1 + 1);
        assert_eq!(view.max_health, 4 + 1 + 1);

        // The set-stat layer never takes a unit below its printed stats.
        let shrunk = put(&mut state, SHRINKER, slot(P1, UNITS, 4), json!({}));
        assert_eq!(unit_view(&state, &shrunk).attack, 3);
        assert_eq!(unit_view(&state, &shrunk).max_health, 3);
        // M4: cards/test/92-felinor-fiender.test.ts proves the card half.
    }

    #[test]
    fn r40_counts_a_cast_on_draw_cast_as_a_card_played_this_turn_at_cost_0() {
        let mut state = playing("r40");
        let castable = set_library(&mut state, P1, &[CAST_ON_DRAW_SPELL, BODY]).into_iter().next();
        let spell = must(castable, "cast-on-draw card");
        let played_before = state.players.p1.turn_log.cards_played;
        let counter_before = state.counters.played;
        let mana_before = state.players.p1.mana.current;

        let events = {
            let mut sink = sink_for(&mut state);
            assert_eq!(draw::draw_one(&mut sink, P1, None), DrawOutcome::Cast);
            sink.events.clone()
        };

        assert_eq!(state.players.p1.turn_log.cards_played, played_before + 1);
        assert!(state.players.p1.turn_log.played_ids.contains(&spell.id));
        assert_eq!(state.counters.played, counter_before + 1);
        let played = of_type(&events, GameEventType::CardPlayed)
            .into_iter()
            .find(|event| event["instanceId"] == spell.id.as_str());
        assert_eq!(played.map(|event| event["costPaid"].clone()), Some(json!(0)));
        assert_eq!(state.players.p1.mana.current, mana_before);
        // The script ran and the spell reached the graveyard (R70).
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 1);
        assert!(state.players.p1.graveyard.iter().any(|card| card.id == spell.id));
    }

    #[test]
    fn r41_r428_gives_carnivorous_cube_a_unit_meal_it_never_takes_from_itself_and_a_death_that_can_do_nothing() {
        let mut state = game("r41");
        let hungry = put(&mut state, CUBE, slot(P1, UNITS, 1), json!({}));
        let other = put(&mut state, BODY, slot(P1, UNITS, 2), json!({}));

        // The Tribute choice never offers the Cube itself, and since R428 offers Units only: a backrow
        // card beside it is no meal.
        let beside = put(&mut state, TRAP, slot(P1, BACKROW, 2), json!({}));
        let options = {
            let mut sink = sink_for(&mut state);
            let ctx = make_context(&mut sink, Some(&hungry), controlled(P1));
            effects::targets_in_scope(
                &ctx,
                Some(&json_as(json!({ "side": "ally", "of": ["unit"], "excludeSelf": true }))),
            )
        };
        assert!(options.contains(&sel(&other)));
        assert!(!options.contains(&sel(&hungry)));
        assert!(!options.contains(&sel(&beside)));

        // What it ate lives on its own instance and drives the Death copies, read from the last-known
        // state of the instance as it left the field (R78).
        run(
            &mut state,
            effects::remember(json_as(json!({ "key": "eaten", "value": TRAP }))),
            as_self(&hungry),
        );
        assert_eq!(instance_in(&state, &hungry.id).memory.get("eaten"), Some(&json!(TRAP)));
        let death = kill_and_check(&mut state, &hungry);
        assert_eq!(pluck(&of_type(&death, GameEventType::Summoned), "defId"), vec![json!(TRAP)]);
        // R41: a copy is summoned as the card it copies, into that card's own row — an animated card's
        // copies (a Unit when it was eaten, R383) go to the backrow, not the unit row.
        assert_eq!(must(card_at(&state, slot(P1, BACKROW, 1)), "backrow copy").def_id, TRAP);

        // A copy keeps the eaten card's radiant flag and its `statsOverride`.
        let mut copies = game("r41c");
        run(
            &mut copies,
            effects::summon(json_as(json!({ "defId": TRAP, "radiant": true }))),
            by(P1),
        );
        assert!(must(card_at(&copies, slot(P1, BACKROW, 1)), "radiant copy").radiant);
        run(
            &mut copies,
            effects::summon(json_as(json!({ "defId": BODY, "statsOverride": { "attack": 7, "health": 7 } }))),
            by(P1),
        );
        let stat_copy = must(card_at(&copies, slot(P1, UNITS, 1)), "stat copy").clone();
        assert_eq!(stat_copy.stats_override, Some(AttackHealth { attack: 7, health: 7 }));
        assert_eq!(unit_view(&copies, &stat_copy).attack, 7);

        // Nothing eaten: the Death hook returns no effects at all.
        let mut starved = game("r41b");
        let empty = put(&mut starved, CUBE, slot(P1, UNITS, 1), json!({}));
        let events = kill_and_check(&mut starved, &empty);
        assert!(of_type(&events, GameEventType::Summoned).is_empty());
        assert!(active_units_of(&starved, P1).is_empty());
        // M4: cards/test/22-carnivorous-cube.test.ts proves the card half.
    }

    #[test]
    fn r42_records_the_unit_whose_damage_instance_was_lethal_cleave_hits_included() {
        let mut state = game("r42");
        let attacker = put(&mut state, CLEAVER, slot(P1, UNITS, 1), json!({}));
        let defender = put(&mut state, SMALL, slot(P2, UNITS, 2), json!({}));
        let neighbour = put(&mut state, SMALL, slot(P2, UNITS, 1), json!({}));
        let bystander = put(&mut state, BODY, slot(P2, UNITS, 5), json!({}));

        let mut sink = sink_for(&mut state);
        resolve_combat(&mut sink, &attacker, &unit_target(&defender));

        let struck = instance_in(sink.state, &defender.id);
        assert!(unit_view(sink.state, &struck).health <= 0);
        assert_eq!(struck.last_damaged_by, Some(attacker.id.clone()));
        // The Cleave hit is this unit's damage too, so its kills belong to it as well.
        let cleaved = instance_in(sink.state, &neighbour.id);
        assert!(unit_view(sink.state, &cleaved).health <= 0);
        assert_eq!(cleaved.last_damaged_by, Some(attacker.id.clone()));

        // A death from someone else's damage is not credited to it.
        let bystander_now = instance_in(sink.state, &bystander.id);
        deal_damage(
            &mut sink,
            hit(None, DamageTarget::Unit { instance: bystander_now }, 99),
        );
        assert_eq!(instance_in(sink.state, &bystander.id).last_damaged_by, None);
        // M4: cards/test/32-prem-panther.test.ts proves the card half.
    }
}
