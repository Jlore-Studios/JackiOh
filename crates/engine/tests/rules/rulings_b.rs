//! The M3 gate's second half (BUILD.md): one `#[test] fn r<n>_…` per SPEC §11 row from R43 to R84.
//!
//! Every fixture in this file is its own — defs prefixed `rb-`, registered on top of the shared
//! fixture catalog by `game()` — so it cannot collide with another test file's fixtures (BUILD §0).
//! Rows whose behaviour belongs to a card that does not exist until M4 test the engine machinery the
//! card will call and name the card test that proves the rest.
//!
//! Port of `packages/engine/test/rulings-b.test.ts` (SURFACE §4.1, §8). TS held live
//! `CardInstance` objects and wrote through them; here a card handed back by a helper is a copy, so a
//! write goes through `edit` (the card under that id in the state) and a read after a change re-reads
//! it with `instance_in`.

use std::borrow::Borrow;
use std::cell::Cell;
use std::collections::BTreeSet;

use jackioh_engine::effects;
use jackioh_engine::subsystems::activate::{activate_ability, why_cannot_activate_ability};
use jackioh_engine::subsystems::fuse::fuse;
use jackioh_engine::subsystems::hero_power::{ensure_power, power_abilities, power_of};
use jackioh_engine::subsystems::{
    AI_SKIPPED_ACTIONS, PolicyOptions, choose_action, is_lethal, policy_actions, projected_damage,
};
use jackioh_engine::testkit::*;

use super::fixtures::harness::{
    events_of_type, in_hand, new_game, play_random_game, put, set_library, sink_for, slot,
};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;
const UNITS: Row = Row::Units;
const BACKROW: Row = Row::Backrow;

// ---------------------------------------------------------------------------
// Fixture definitions. Ids are `rb-` prefixed and indexes start above every other
// fixture file's, so `defByIndex` and the catalog query stay unambiguous.
// ---------------------------------------------------------------------------

const PLAIN: &str = "rb-plain";
const BIG_BODY: &str = "rb-big-body";
const SMALL: &str = "rb-small";
const WARDED_TAUNTER: &str = "rb-warded-taunt";
const WARDED_PINGER: &str = "rb-warded-pinger";
const REBORN_UNIT: &str = "rb-reborn";
const DUELIST: &str = "rb-duelist";
const IMMUTABLE: &str = "rb-immutable";
const TRAMPLER: &str = "rb-trampler";
const TRAMPLE_LEECH: &str = "rb-trample-leech";
const CLEAVER: &str = "rb-cleaver";
const SHIELDED: &str = "rb-shielded";
const ARMOURED: &str = "rb-armoured";
const POISONER: &str = "rb-poisoner";
const ZERO_ATTACK: &str = "rb-zero-attack";
/// §5.2's last bullet: a card with no radiant form still sets the flag (R74).
const SAME_FACE: &str = "rb-same-face";
const REMEMBERER: &str = "rb-rememberer";
const KPOP: &str = "rb-kpop";
const CLOCK: &str = "rb-clock";
const FUSE_A: &str = "rb-fuse-a";
const FUSE_B: &str = "rb-fuse-b";
/// §7: a unit token, which ceases to exist off the field (R11, R80).
const UNIT_TOKEN: &str = "rb-token";

const NOOP: &str = "rb-noop";
const PRICEY: &str = "rb-pricey";
/// #31 KY's Math Equation's engine half (R429): a Spell whose plays the engine counts.
const EQUATION: &str = "rb-equation";
const GIGA: &str = "rb-giga";
const X_CARD: &str = "rb-x-card";
const EMBIGGEN_CARD: &str = "rb-embiggen";
/// R75: a token index of the "N.1" form the §5.3 table normalises (#90.1).
const DOTTED: &str = "rb-dotted";
const SPELL_TOKEN: &str = "rb-spell-token";
const CASTER: &str = "rb-caster";
const SPLITTER: &str = "rb-splitter";
const ALL_ENEMIES: &str = "rb-all-enemies";
const MODE_SPELL: &str = "rb-mode-spell";
const DISCOVER_SPELL: &str = "rb-discover-spell";
const RECYCLER: &str = "rb-recycler";
const LOG_CARD: &str = "rb-log";
const HEROIC: &str = "rb-heroic";
const WINDOW_TRAP: &str = "rb-window-trap";
const BREAD_TRAP: &str = "rb-bread-trap";
const EMPTY_TRAP: &str = "rb-empty-trap";

/// TS's object spread `{ ...def, ...overrides }`: the overrides' keys replace the definition's.
fn spread(def: &mut Value, overrides: Value) {
    if let (Some(fields), Value::Object(overrides)) = (def.as_object_mut(), overrides) {
        for (key, value) in overrides {
            fields.insert(key, value);
        }
    }
}

/// TS `unitDefOf`: a Core unit fixture whose index is the next of this file's (1201 on).
fn unit_def_of(next: &mut i32, name: &str, attack: i32, health: i32, keywords: Value, overrides: Value) -> CardDef {
    *next += 1;
    let index = *next;
    let mut def = json!({
        "id": format!("rb-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (rulings-b)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "attack": attack, "health": health, "keywords": keywords, "text": name },
        "radiant": { "attack": attack * 2, "health": health * 2, "keywords": keywords, "text": format!("{name} radiant") },
    });
    spread(&mut def, overrides);
    json_as(def)
}

/// TS `cardDefOf`: a Core fixture of any type without stats.
fn card_def_of(next: &mut i32, name: &str, card_type: &str, overrides: Value) -> CardDef {
    *next += 1;
    let index = *next;
    let mut def = json!({
        "id": format!("rb-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (rulings-b)"),
        "set": "Core",
        "type": card_type,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    });
    spread(&mut def, overrides);
    json_as(def)
}

/// TS `DEFS`, in declaration order (so each index is the one TS gave it), which is also `DEFS`' order.
fn defs() -> Vec<CardDef> {
    let mut next = 1200;
    let n = &mut next;
    let none = || json!([]);
    let no = || json!({});
    vec![
        unit_def_of(n, "plain", 3, 3, none(), no()),
        unit_def_of(n, "big-body", 5, 5, none(), no()),
        unit_def_of(n, "small", 1, 2, none(), no()),
        unit_def_of(n, "warded-taunt", 4, 4, json!([{ "kind": "Indestructible" }, { "kind": "Taunt" }]), no()),
        unit_def_of(n, "warded-pinger", 4, 4, json!([{ "kind": "Indestructible" }]), no()),
        unit_def_of(n, "reborn", 2, 2, json!([{ "kind": "Reborn" }]), no()),
        unit_def_of(n, "duelist", 4, 3, json!([{ "kind": "Deft" }]), no()),
        unit_def_of(n, "immutable", 2, 2, json!([{ "kind": "Immutable" }]), no()),
        unit_def_of(n, "trampler", 6, 4, json!([{ "kind": "Trample" }]), no()),
        unit_def_of(n, "trample-leech", 6, 4, json!([{ "kind": "Trample" }, { "kind": "Lifesteal" }]), no()),
        unit_def_of(n, "cleaver", 3, 6, json!([{ "kind": "Cleave" }]), no()),
        unit_def_of(n, "shielded", 2, 2, json!([{ "kind": "Divine Shield" }]), no()),
        unit_def_of(n, "armoured", 1, 6, json!([{ "kind": "Armor", "n": 5 }]), no()),
        unit_def_of(n, "poisoner", 1, 4, json!([{ "kind": "Poisonous" }]), no()),
        unit_def_of(n, "zero-attack", 0, 6, none(), no()),
        unit_def_of(
            n,
            "same-face",
            2,
            2,
            none(),
            json!({ "radiant": { "attack": 2, "health": 2, "keywords": [], "text": "no radiant form" } }),
        ),
        unit_def_of(n, "rememberer", 2, 2, none(), no()),
        unit_def_of(n, "kpop", 2, 2, none(), no()),
        unit_def_of(n, "clock", 1, 1, none(), no()),
        unit_def_of(n, "fuse-a", 2, 3, json!([{ "kind": "Taunt" }]), json!({ "cost": 2, "tags": ["Human"] })),
        unit_def_of(n, "fuse-b", 1, 1, json!([{ "kind": "Rush" }]), json!({ "cost": 3, "tags": ["Felinor"] })),
        unit_def_of(
            n,
            "token",
            3,
            3,
            json!([{ "kind": "Rush" }]),
            json!({ "token": true, "rarity": "Token", "tags": ["Token"], "index": "T-rb" }),
        ),
        card_def_of(n, "noop", "Spell", no()),
        card_def_of(n, "pricey", "Spell", json!({ "cost": 3 })),
        card_def_of(n, "equation", "Spell", json!({ "cost": 1 })),
        card_def_of(n, "giga", "Spell", json!({ "cost": 6 })),
        card_def_of(n, "x-card", "Spell", json!({ "cost": "X" })),
        card_def_of(n, "embiggen", "Spell", json!({ "cost": { "base": 2, "embiggen": 4 } })),
        card_def_of(n, "dotted", "Spell", json!({ "index": "51.1" })),
        card_def_of(n, "spell-token", "Spell", json!({ "token": true, "rarity": "Token", "tags": ["Token"] })),
        card_def_of(n, "caster", "Spell", no()),
        card_def_of(n, "splitter", "Spell", no()),
        card_def_of(n, "all-enemies", "Spell", no()),
        card_def_of(n, "mode-spell", "Spell", no()),
        card_def_of(n, "discover-spell", "Spell", no()),
        card_def_of(n, "recycler", "Field Spell", no()),
        card_def_of(n, "log", "Field Spell", no()),
        card_def_of(n, "heroic", "Field Spell", json!({ "cost": 0 })),
        card_def_of(n, "window-trap", "Field Trap", no()),
        card_def_of(n, "bread-trap", "Field Trap", no()),
        card_def_of(n, "empty-trap", "Trap", no()),
    ]
}

/// One of this file's definitions by id (TS named each def by its `const`).
fn def(id: &str) -> CardDef {
    must(defs().into_iter().find(|def| def.id == id), id)
}

// ---------------------------------------------------------------------------
// Fixture effects: the verbs these rows need that no card ships until M4.
// ---------------------------------------------------------------------------

/// R51 "all enemies": every enemy unit plus the enemy hero, one damage instance each.
fn rb_damage_all_enemies(amount: i32) -> Effect {
    Effect::new("rb:damageAllEnemies", move |ctx| {
        let enemy = if ctx.controller == P1 { P2 } else { P1 };
        for unit in owned(active_units_of(ctx.state, enemy)) {
            let source = ctx.self_.clone();
            deal_damage(ctx, hit(source, DamageTarget::Unit { instance: unit }, amount));
        }
        let source = ctx.self_.clone();
        deal_damage(ctx, hit(source, DamageTarget::Hero { player: enemy }, amount));
    })
}

/// One effect, many hits: R59's "never between the hits of one effect".
fn rb_damage_enemy_units(amount: i32) -> Effect {
    Effect::new("rb:damageEnemyUnits", move |ctx| {
        let enemy = if ctx.controller == P1 { P2 } else { P1 };
        for unit in owned(active_units_of(ctx.state, enemy)) {
            let source = ctx.self_.clone();
            deal_damage(ctx, hit(source, DamageTarget::Unit { instance: unit }, amount));
        }
    })
}

/// #39 Recycling Initiative's engine half (R71): every other card played this turn, copied.
fn rb_copy_others_played_this_turn() -> Effect {
    Effect::new("rb:copyOthersPlayed", |ctx| {
        let controller = ctx.controller;
        let self_id = ctx.self_.as_ref().map(|card| card.id.clone());
        let played_ids = ctx.state.players[controller].turn_log.played_ids.clone();
        for id in played_ids {
            if self_id.as_ref() == Some(&id) {
                continue;
            }
            let Some(played) = find_instance(ctx.state, &id).cloned() else {
                continue;
            };
            let mut copy = new_instance(
                &mut *ctx.sink.state,
                &played.def_id,
                controller,
                Zone::Hand { player: controller },
            );
            let _ = draw::add_to_hand(ctx, &mut copy);
        }
    })
}

/// R62's ordering log, kept on the card in p1's backrow lane 5 so it survives a `reduce` clone.
fn rb_note(name: String) -> Effect {
    Effect::new("rb:note", move |ctx| {
        let Some(Some(log)) = ctx.sink.state.players.p1.backrow.get_mut(4) else {
            return;
        };
        let mut steps: Vec<Value> = log.memory.get("steps").and_then(Value::as_array).cloned().unwrap_or_default();
        steps.push(json!(name));
        log.memory.insert("steps".to_string(), Value::Array(steps));
    })
}

fn rb_steps(state: &GameState) -> Vec<String> {
    state
        .players
        .p1
        .backrow
        .get(4)
        .and_then(Option::as_ref)
        .and_then(|log| log.memory.get("steps"))
        .and_then(Value::as_array)
        .map(|steps| steps.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Fixture scripts.
// ---------------------------------------------------------------------------

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn ping_enemy_hero(amount: i32) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))]
        })),
        ..Script::default()
    }
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts: IndexMap<String, CardScripts> = IndexMap::new();
    scripts.insert(DUELIST.into(), both(Script::default()));
    scripts.insert(
        CASTER.into(),
        both(Script {
            static_flags: Some(StaticFlags {
                cast_on_draw: Some(true),
                ..StaticFlags::default()
            }),
            ..Script::default()
        }),
    );
    scripts.insert(
        SPLITTER.into(),
        both(Script {
            cry: Some(hook(|_ctx| vec![rb_damage_enemy_units(9)])),
            ..Script::default()
        }),
    );
    scripts.insert(
        ALL_ENEMIES.into(),
        both(Script {
            cry: Some(hook(|_ctx| vec![rb_damage_all_enemies(2)])),
            ..Script::default()
        }),
    );
    scripts.insert(PRICEY.into(), both(ping_enemy_hero(1)));
    scripts.insert(
        EQUATION.into(),
        both(Script {
            static_flags: Some(StaticFlags {
                counts_plays: Some(true),
                ..StaticFlags::default()
            }),
            cry: Some(hook(|_ctx| vec![])),
            ..Script::default()
        }),
    );
    scripts.insert(FUSE_A.into(), both(ping_enemy_hero(1)));
    scripts.insert(FUSE_B.into(), both(ping_enemy_hero(2)));
    // R752: each power is one of the card's Activate abilities, paying its X as it is activated.
    scripts.insert(
        HEROIC.into(),
        both(Script {
            activations: power_abilities(false),
            ..Script::default()
        }),
    );
    scripts.insert(
        RECYCLER.into(),
        both(Script {
            end_of_turn: Some(hook(|_ctx| vec![rb_copy_others_played_this_turn()])),
            ..Script::default()
        }),
    );
    scripts.insert(
        CLOCK.into(),
        both(Script {
            start_of_turn: Some(hook(|ctx| {
                vec![rb_note(format!("start:lib{}", ctx.state.players.p1.library.len()))]
            })),
            end_of_turn: Some(hook(|_ctx| vec![rb_note("end".to_string())])),
            delayed: Some(hook(|_ctx| vec![rb_note("delayed".to_string())])),
            ..Script::default()
        }),
    );
    scripts.insert(
        WINDOW_TRAP.into(),
        both(Script {
            triggers: vec![TriggerDef::new("turn-end", &[GameEventType::TurnEnded], |ctx, _event| {
                vec![rb_note(format!("trap:{}", ctx.controller))]
            })],
            ..Script::default()
        }),
    );
    // #18 Bread and Butter's engine half (R52): the token goes to the trap's controller.
    scripts.insert(
        BREAD_TRAP.into(),
        both(Script {
            triggers: vec![TriggerDef::new("turn-end", &[GameEventType::TurnEnded], |_ctx, event| {
                match event {
                    GameEvent::TurnEnded { unspent_mana, .. } if *unspent_mana > 0 => {
                        vec![effects::summon(json_as(json!({ "defId": UNIT_TOKEN })))]
                    }
                    _ => vec![],
                }
            })],
            ..Script::default()
        }),
    );
    // R61: a trap with no legal target still fires, is consumed and does nothing.
    scripts.insert(
        EMPTY_TRAP.into(),
        both(Script {
            triggers: vec![TriggerDef::new("no-target", &[GameEventType::CardPlayed], |_ctx, _event| vec![])],
            ..Script::default()
        }),
    );
    // #50 K-Pop Fanatic's engine half (R76): a delayed steal naming its target as data.
    scripts.insert(
        KPOP.into(),
        both(Script {
            delayed: Some(hook(|ctx| {
                match ctx.data.get("target").and_then(Value::as_str).map(str::to_string) {
                    Some(target) => vec![effects::steal(json_as(json!({ "instanceId": target })))],
                    None => vec![],
                }
            })),
            ..Script::default()
        }),
    );
    // R78: the Death hook reads what the card remembered just before it left the field.
    scripts.insert(
        REMEMBERER.into(),
        both(Script {
            death: Some(hook(|ctx| {
                let noted = ctx
                    .self_
                    .as_ref()
                    .and_then(|card| card.memory.get("note"))
                    .is_some_and(Value::is_string);
                if noted {
                    vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 4 })))]
                } else {
                    vec![]
                }
            })),
            ..Script::default()
        }),
    );
    scripts.insert(
        WARDED_PINGER.into(),
        both(Script {
            death: Some(hook(|_ctx| {
                vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 })))]
            })),
            ..Script::default()
        }),
    );
    // R81: the choices a card declares travel in the `play` action, never as a prompt.
    scripts.insert(
        MODE_SPELL.into(),
        both(Script {
            targets: vec![json_as(json!({ "kind": "target", "min": 1, "max": 1 }))],
            modes: vec![json_as(json!({ "kind": "direction", "options": ["left", "right"] }))],
            cry: Some(hook(|ctx| {
                let amount = if ctx.modes.first().map(String::as_str) == Some("left") { 1 } else { 5 };
                vec![effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))]
            })),
            ..Script::default()
        }),
    );
    // R81: a choice made during resolution opens a PendingChoice instead.
    scripts.insert(
        DISCOVER_SPELL.into(),
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::discover_from_catalog(json_as(json!({ "step": "pick", "query": { "type": "Unit" } })))]
            })),
            resume: IndexMap::from([(
                "pick",
                hook(|ctx| match effects::chosen_options(ctx).first().cloned() {
                    Some(def_id) => vec![effects::add_to_hand(json_as(json!({ "defId": def_id })))],
                    None => vec![],
                }),
            )]),
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
    let state = new_game(seed, None);
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

thread_local! {
    /// TS's module-level `let nonce = 0`: each action this file sends gets a fresh `rb<n>` nonce.
    /// Per test thread, since nonces only need to differ within one game.
    static NONCE: Cell<u32> = const { Cell::new(0) };
}

/// TS `actResult(state, body)`: the action body (TS `ActionInput`, as its JSON) with a fresh nonce.
fn act_result(state: &GameState, mut body: Value) -> ReduceResult {
    let nonce = NONCE.with(|nonce| {
        nonce.set(nonce.get() + 1);
        nonce.get()
    });
    body["nonce"] = json!(format!("rb{nonce}"));
    reduce(state, &json_as(body))
}

fn act(state: &GameState, body: Value) -> GameState {
    let result = act_result(state, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

/// Past the mulligans, in the main phase of turn 1.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&game(seed)).state;
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": ids(&state.players.p1.hand), "playerId": "p1" }),
    );
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": ids(&state.players.p2.hand), "playerId": "p2" }),
    );
    state
}

fn only<T: Clone>(items: &[T]) -> T {
    match items.first() {
        Some(first) => first.clone(),
        None => panic!("expected at least one item"),
    }
}

/// TS `handCard(state, defId, player = "p1")`.
fn hand_card(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    only(&in_hand(state, def_id, player, 1))
}

fn must<T>(value: Option<T>, what: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("missing {what}"),
    }
}

fn instance_in(state: &GameState, id: &str) -> CardInstance {
    must(find_instance(state, id).cloned(), &format!("instance {id}"))
}

/// TS wrote through the live instance (`card.costMod = -2`): here, the card under that id in the state.
fn edit(state: &mut GameState, card: &CardInstance, change: impl FnOnce(&mut CardInstance)) {
    change(must(find_instance_mut(state, &card.id), &format!("instance {}", card.id)));
}

/// TS handed the engine the live instance, so what the engine wrote to it was in the state: the
/// copy it wrote to goes back under its id.
fn put_back(state: &mut GameState, card: &CardInstance) {
    *must(find_instance_mut(state, &card.id), &format!("instance {}", card.id)) = card.clone();
}

fn controlled(controller: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(controller),
        ..HookOptions::default()
    }
}

fn targeted(controller: PlayerId, targets: Vec<Selection>) -> HookOptions {
    HookOptions {
        controller: Some(controller),
        targets: Some(targets),
        ..HookOptions::default()
    }
}

fn pick(card: &CardInstance) -> Vec<Selection> {
    vec![sel(card)]
}

fn sel(card: &CardInstance) -> Selection {
    Selection::Instance {
        instance_id: card.id.clone(),
    }
}

fn hero(player: PlayerId) -> AttackTarget {
    AttackTarget::Hero { player }
}

fn unit_target(card: &CardInstance) -> AttackTarget {
    AttackTarget::Unit { instance: card.clone() }
}

fn unit_hit(card: &CardInstance) -> DamageTarget {
    DamageTarget::Unit { instance: card.clone() }
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

/// `{ kind: "costDiscount", amount, minCurrentCost? }`.
fn cost_discount(amount: i32, min_current_cost: Option<i32>) -> ModifierKind {
    ModifierKind::CostDiscount {
        amount,
        only_type: None,
        min_current_cost,
        once_per_turn: None,
    }
}

/// `{ defId, hook: "delayed", step: "", radiant: false, instanceId?, data }`.
fn delayed_resume(def_id: &str, instance_id: Option<&str>, data: Value) -> Resume {
    json_as(json!({
        "defId": def_id,
        "hook": "delayed",
        "step": "",
        "radiant": false,
        "instanceId": instance_id,
        "data": data,
    }))
}

/// `effectiveCost(state, card)` on the card as it stands in the state now.
fn price_now(state: &GameState, card: &CardInstance) -> i32 {
    effective_cost(state, &instance_in(state, &card.id), Default::default())
}

/// TS `why…` refusals are `string | null` and `CombatResult` is `{ error? }`; here
/// `Result<_, EngineError>` (SURFACE §4.4.9).
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

/// `cardAt(state, zone)?.id`.
fn id_at(state: &GameState, zone: ZoneSlot) -> Option<String> {
    card_at(state, zone).map(|card| card.id.clone())
}

/// `cardAt(state, zone)?.defId`.
fn def_at(state: &GameState, zone: ZoneSlot) -> Option<String> {
    card_at(state, zone).map(|card| card.def_id.clone())
}

fn plays_card(action: &ActionBody, instance: &str) -> bool {
    matches!(action, ActionBody::Play { instance_id, .. } if instance_id == instance)
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

// R79: the server owns these, in apps/server/src/config.ts (BUILD §2, M7) — the match lifecycle,
// and the ranked ladder's R603–R612 numbers. TS's `SERVER_CONSTANTS` list fed one assertion that
// `config.ts` exports none of them; Rust cannot list a module's names at run time, so that assertion
// is in `.fullsend/notes/spec-gaps-part-25-5.md` and the list is not ported.

mod spec_11_rulings_r43_r84_m3_gate {
    use super::*;

    #[test]
    fn r43_stores_heroic_power_s_power_on_the_instance_pays_its_x_as_it_is_used_once_a_turn_and_recruits_a_permanent(
    ) {
        let mut state = game("r43");
        let card = put(&mut state, HEROIC, slot(P1, BACKROW, 1), json!({}));
        let mut sink = sink_for(&mut state);
        sink.state.active = P1;
        sink.state.phase = Phase::Main;
        sink.state.players.p1.mana.current = 3;

        // The power lives on the instance, and one that arrives without a power rolls as it arrives.
        assert!(instance_in(sink.state, &card.id).memory.get("power").is_none());
        let mut live = instance_in(sink.state, &card.id);
        let rolled = ensure_power(&mut sink, &mut live);
        put_back(sink.state, &live);
        assert!(rolled.is_some());
        let rolled_name = rolled.as_ref().map(|power| to_json(&power.name));
        assert_eq!(instance_in(sink.state, &card.id).memory.get("power").cloned(), rolled_name);
        let mut live = instance_in(sink.state, &card.id);
        let again = ensure_power(&mut sink, &mut live);
        put_back(sink.state, &live);
        assert_eq!(again.map(|power| to_json(&power.name)), rolled_name);

        // R752: the card costs (0); the power's X is the price of each use.
        edit(sink.state, &card, |live| {
            live.memory.insert("power".to_string(), json!("recruit"));
        });
        let recruiting = instance_in(sink.state, &card.id);
        assert_eq!(power_of(&recruiting).map(|power| power.x), Some(3));
        assert_eq!(effective_cost(sink.state, &recruiting, Default::default()), 0);

        // Expedition Map's "Recruit a permanent": the Spell on top of the library is skipped.
        set_library(sink.state, P1, &[NOOP, PLAIN]);
        assert_eq!(
            refusal(activate_ability(
                &mut sink,
                P1,
                &json_as(json!({ "type": "activate", "instanceId": card.id }))
            )),
            None
        );
        assert_eq!(sink.state.players.p1.mana.current, 0);
        assert_eq!(def_ids(active_units_of(sink.state, P1)), vec![PLAIN]);
        assert_eq!(def_ids(&sink.state.players.p1.library), vec![NOOP]);

        // And that activation is the turn's use.
        sink.state.players.p1.mana.current = 3;
        assert_eq!(
            refusal(why_cannot_activate_ability(sink.state, P1, &card.id, None)),
            says("that ability has already been used this turn")
        );
    }

    #[test]
    fn r44_projects_lethal_after_armor_the_cap_and_trample_excess_and_the_policy_draws_from_legalactions() {
        let mut state = game("r44");
        let attacker = put(&mut state, PLAIN, slot(P1, UNITS, 1), json!({})); // 3/3
        state.players.p2.hero.health = 3;
        assert_eq!(projected_damage(&state, &attacker, &hero(P2)), 3);
        assert!(is_lethal(&state, &attacker, &hero(P2)));

        state.players.p2.hero.health = 4;
        assert!(!is_lethal(&state, &attacker, &hero(P2)));

        // Trample excess from an attack on a unit counts toward the projection.
        let tramp = put(&mut state, TRAMPLER, slot(P1, UNITS, 2), json!({})); // 6/4 Trample
        let blocker = put(&mut state, SMALL, slot(P2, UNITS, 1), json!({})); // 1/2
        assert_eq!(projected_damage(&state, &tramp, &unit_target(&blocker)), 4);
        assert!(is_lethal(&state, &tramp, &unit_target(&blocker)));

        // The policy: uniform over `legalActions`, ending the turn on the AI_END_TURN_PROBABILITY roll.
        let live = playing("r44-policy");
        let chosen = choose_action(&live, P1, &mut create_rng("r44", 0));
        assert!(chosen.is_some_and(|chosen| legal_actions(&live, P1).contains(&chosen)));
        assert_eq!(AI_END_TURN_PROBABILITY, 0.1);

        // The cancellation itself is My Pawn's, and rides the `attackCancelled` event (§10.3).
        // M4: cards/test/96-my-pawn.test.ts proves the card half.
        assert!(GameEventType::ALL.iter().any(|event_type| event_type.as_str() == "attackCancelled"));
    }

    #[test]
    fn r45_keeps_players_as_a_map_so_the_seats_and_each_opposing_hero_are_read_from_it_not_hard_coded() {
        let mut state = game("r45");
        let mut seats: Vec<String> = to_json(&state.players)
            .as_object()
            .map(|players| players.keys().cloned().collect())
            .unwrap_or_default();
        seats.sort();
        let mut expected: Vec<String> = PLAYER_IDS.iter().map(|player| player.to_string()).collect();
        expected.sort();
        assert_eq!(seats, expected);

        // The opening draw is a table indexed by seat (§2.1), not two constants.
        assert_eq!(
            PLAYER_IDS.iter().map(|&player| opening_draw_for(player)).collect::<Vec<_>>(),
            OPENING_DRAW.to_vec()
        );

        // "Each opposing hero" is derived from the acting player, so p2 acting reaches p1.
        let mut sink = sink_for(&mut state);
        apply_effects(
            &[effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 3 })))],
            &mut make_context(&mut sink, None, controlled(P2)),
        );
        assert_eq!(sink.state.players.p1.hero.health, HERO_HEALTH - 3);
        assert_eq!(sink.state.players.p2.hero.health, HERO_HEALTH);

        // The hero check iterates the map, so every seat is read in one pass (§4.5 step 2).
        for player in PLAYER_IDS {
            sink.state.players[player].hero.health = 0;
        }
        state_check(&mut sink);
        assert_eq!(
            sink.state.result,
            Some(GameResult {
                winner: Winner::Draw,
                reason: GameOverReason::BothHeroesDead,
            })
        );
    }

    #[test]
    fn r46_turns_a_would_destroy_indestructible_unit_to_attack_position_without_taunt_and_leaves_a_field_spell_alone(
    ) {
        let mut state = game("r46");
        let warded = put(&mut state, WARDED_TAUNTER, slot(P1, UNITS, 1), json!({}));
        edit(&mut state, &warded, |card| {
            card.position = Some(Position::Def);
            card.marked_destroyed = Some(true);
        });
        let field = put(&mut state, LOG_CARD, slot(P1, BACKROW, 1), json!({}));
        edit(&mut state, &field, |card| card.marked_destroyed = Some(true));
        // A Field Spell is Indestructible only if its face says so; #98's does (§8).
        let mut catalog = registered_catalog().clone();
        let mut warded_field = def(LOG_CARD);
        warded_field.base =
            json_as(json!({ "keywords": [{ "kind": "Indestructible" }], "text": "warded field spell" }));
        warded_field.radiant =
            json_as(json!({ "keywords": [{ "kind": "Indestructible" }], "text": "warded field spell" }));
        catalog.insert(LOG_CARD.to_string(), warded_field);
        register_catalog(catalog);

        {
            let mut sink = sink_for(&mut state);
            state_check(&mut sink);
        }

        assert_eq!(id_at(&state, slot(P1, UNITS, 1)), Some(warded.id.clone()));
        let warded_now = instance_in(&state, &warded.id);
        assert_eq!(warded_now.marked_destroyed, Some(false));
        assert_eq!(warded_now.position, Some(Position::Atk));
        assert_eq!(warded_now.taunt_suppressed_turn, Some(state.turn));
        assert!(!has_keyword(&unit_view(&state, &warded_now).keywords, KeywordKind::Taunt));

        // The Indestructible Field Spell simply stays and the mark is dropped.
        assert_eq!(id_at(&state, slot(P1, BACKROW, 1)), Some(field.id.clone()));
        assert_eq!(instance_in(&state, &field.id).marked_destroyed, Some(false));
    }

    #[test]
    fn r47_a_lane_targeted_summon_fizzles_on_an_occupied_zone_but_lands_on_a_locked_one_r688_and_holds_a_reborn_unit_s_zone(
    ) {
        let mut state = game("r47");
        let mut sink = sink_for(&mut state);
        let mut ctx = make_context(&mut sink, None, controlled(P1));

        put(ctx.sink.state, PLAIN, slot(P1, UNITS, 2), json!({}));
        lock_zone(ctx.sink.state, slot(P1, UNITS, 3));

        let summon_in = |lane: i32| effects::summon(json_as(json!({ "defId": SMALL, "lane": lane })));
        apply_effects(&[summon_in(2)], &mut ctx); // occupied: fizzles
        assert_eq!(def_ids(active_units_of(ctx.sink.state, P1)), vec![PLAIN]);

        apply_effects(&[summon_in(3)], &mut ctx); // Locked: lands (R688)
        assert_eq!(def_at(ctx.sink.state, slot(P1, UNITS, 3)), Some(SMALL.to_string()));

        apply_effects(&[summon_in(4)], &mut ctx);
        assert_eq!(def_at(ctx.sink.state, slot(P1, UNITS, 4)), Some(SMALL.to_string()));

        // A zone reserved for a dying Reborn unit counts as occupied for everything else (R64).
        reserve_zone(ctx.sink.state, slot(P1, UNITS, 1));
        assert!(!is_open(ctx.sink.state, slot(P1, UNITS, 1)));
        apply_effects(&[summon_in(1)], &mut ctx);
        assert!(card_at(ctx.sink.state, slot(P1, UNITS, 1)).is_none());
        release_zone(ctx.sink.state, slot(P1, UNITS, 1));

        // With the zone free, the Reborn unit comes back to it.
        let rb = put(ctx.sink.state, REBORN_UNIT, slot(P1, UNITS, 5), json!({}));
        edit(ctx.sink.state, &rb, |card| card.damage = 99);
        drop(ctx);
        state_check(&mut sink);
        assert_eq!(id_at(sink.state, slot(P1, UNITS, 5)), Some(rb.id.clone()));
    }

    #[test]
    fn r48_applies_professor_curvature_to_a_card_whose_cost_is_4_once_the_other_modifiers_have_landed() {
        let mut state = game("r48");
        let mut sink = sink_for(&mut state);
        add_modifier(&mut sink, P1, ModifierExpiry::Never, cost_discount(2, None));
        add_modifier(&mut sink, P1, ModifierExpiry::Never, cost_discount(1, Some(4)));

        // 6 − 2 = 4, which is then Curvature's target: 3.
        let six = hand_card(sink.state, GIGA, P1);
        assert_eq!(price_now(sink.state, &six), 3);

        // 3 − 2 = 1 never reaches 4, so Curvature does nothing.
        let three = hand_card(sink.state, PRICEY, P1);
        assert_eq!(price_now(sink.state, &three), 1);

        // The embiggen price is read at play time too: 4 chosen, 2 not.
        let emb = hand_card(sink.state, EMBIGGEN_CARD, P1);
        assert_eq!(price_now(sink.state, &emb), 0); // 2 − 2
        edit(sink.state, &emb, |card| card.embiggened = Some(true));
        assert_eq!(price_now(sink.state, &emb), 2); // 4 − 2 = 2, which is not 4, so no Curvature
    }

    #[test]
    fn r49_gives_deft_duelist_two_exertions_one_attack_and_one_switch_where_a_plain_unit_has_one() {
        let mut state = playing("r49");
        let duel = put(&mut state, DUELIST, slot(P1, UNITS, 1), json!({}));
        let victim = put(&mut state, SMALL, slot(P2, UNITS, 1), json!({}));
        let mut sink = sink_for(&mut state);

        assert_eq!(refusal(declare_attack(&mut sink, &duel, &unit_target(&victim))), None);
        let duel_now = instance_in(sink.state, &duel.id);
        assert!(duel_now.exertion.attacked);
        assert!(has_exertion(sink.state, &duel_now, ExertionKind::Switch));
        assert_eq!(refusal(switch_position(&mut sink, &duel_now, Default::default())), None);
        assert_eq!(instance_in(sink.state, &duel.id).position, Some(Position::Def));

        let ordinary = put(sink.state, PLAIN, slot(P1, UNITS, 2), json!({}));
        assert_eq!(refusal(declare_attack(&mut sink, &ordinary, &hero(P2))), None);
        let ordinary_now = instance_in(sink.state, &ordinary.id);
        assert!(!has_exertion(sink.state, &ordinary_now, ExertionKind::Switch));
        assert_eq!(
            refusal(switch_position(&mut sink, &ordinary_now, Default::default())),
            says("that unit has already acted this turn")
        );

        // R49 reads the keyword through the layers, so a granted Deft works like a printed one.
        let gifted = put(sink.state, PLAIN, slot(P1, UNITS, 3), json!({}));
        edit(sink.state, &gifted, |card| card.granted_keywords.push(json_as(json!({ "kind": "Deft" }))));
        let gifted_now = instance_in(sink.state, &gifted.id);
        assert_eq!(refusal(declare_attack(&mut sink, &gifted_now, &hero(P2))), None);
        let gifted_now = instance_in(sink.state, &gifted.id);
        assert!(has_exertion(sink.state, &gifted_now, ExertionKind::Switch));
        assert_eq!(refusal(switch_position(&mut sink, &gifted_now, Default::default())), None);
    }

    #[test]
    fn r50_discovers_from_the_actual_graveyard_so_a_spell_token_sitting_there_is_eligible() {
        let mut state = game("r50");
        let token = new_instance(&mut state, SPELL_TOKEN, P1, Zone::Graveyard { player: P1 });
        state.players.p1.graveyard.push(token.clone());

        let mut sink = sink_for(&mut state);
        apply_effects(
            &[effects::discover_from_graveyard(json_as(json!({ "step": "pick" })))],
            &mut make_context(&mut sink, None, controlled(P1)),
        );

        let pending = sink.state.pending.clone();
        assert_eq!(pending.as_ref().map(|prompt| prompt.kind), Some(PromptKind::Discover));
        let selections: Vec<Selection> = pending
            .map(|prompt| prompt.options.iter().map(|option| option.selection.clone()).collect())
            .unwrap_or_default();
        assert!(selections.contains(&sel(&token)));
        // M4: cards/test/72-reminisce.test.ts proves the card half.
    }

    #[test]
    fn r51_gives_all_enemies_one_damage_instance_to_every_enemy_unit_and_one_to_the_enemy_hero() {
        let mut state = game("r51");
        let first = put(&mut state, BIG_BODY, slot(P2, UNITS, 1), json!({}));
        let second = put(&mut state, BIG_BODY, slot(P2, UNITS, 3), json!({}));
        let mut sink = sink_for(&mut state);

        let spell = new_instance(&mut *sink.state, ALL_ENEMIES, P1, Zone::Resolving { player: P1 });
        run_hook(&mut sink, &spell, HookName::Cry, HookOptions::default());

        let hits = of_type(sink.events, GameEventType::Damage);
        assert_eq!(
            pluck(&hits, "targetId"),
            vec![json!(first.id), json!(second.id), json!("hero-p2")]
        );
        assert!(hits.iter().all(|event| event["amount"] == 2));
        assert_eq!(sink.state.players.p1.hero.health, HERO_HEALTH);
    }

    #[test]
    fn r52_gives_the_end_of_turn_token_to_the_trap_s_controller_whoever_ended_the_turn_with_mana() {
        let mut state = playing("r52");
        put(&mut state, BREAD_TRAP, slot(P2, BACKROW, 1), json!({})); // the trap belongs to p2
        state.players.p1.mana.current = 2; // p1 is the one ending with unspent mana

        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));

        assert_eq!(def_ids(active_units_of(&state, P2)), vec![UNIT_TOKEN]);
        assert!(active_units_of(&state, P1).is_empty());
        // M4: cards/test/18-bread-and-butter.test.ts proves the card half.
    }

    #[test]
    fn r53_forces_an_attack_past_the_validator_spends_no_exertion_and_stops_once_the_target_is_gone() {
        let mut state = playing("r53");
        let attacker = put(&mut state, BIG_BODY, slot(P1, UNITS, 1), json!({})); // 5/5
        let turn = state.turn;
        edit(&mut state, &attacker, |card| {
            card.position = Some(Position::Def);
            card.summoned_turn = Some(turn); // and summoning sick
        });
        let target = put(&mut state, PLAIN, slot(P2, UNITS, 1), json!({})); // 3/3

        let attacker = instance_in(&state, &attacker.id);
        assert!(refusal(why_cannot_attack(&state, &attacker, &unit_target(&target))).is_some());

        let mut sink = sink_for(&mut state);
        force_attack(&mut sink, &attacker, &unit_target(&target));

        assert_eq!(
            pluck(&of_type(sink.events, GameEventType::Destroyed), "instanceId"),
            vec![json!(target.id)]
        );
        // The target still struck back, through the Armor 1 that Defense Position grants (§4.1).
        let struck = instance_in(sink.state, &attacker.id);
        assert_eq!(struck.damage, 2);
        assert_eq!(to_json(&struck.exertion), json!({ "attacked": false, "switched": false }));

        // Each forced attack is its own combat and its own state check, so the next one is skipped.
        let second = put(sink.state, PLAIN, slot(P1, UNITS, 2), json!({}));
        let third = put(sink.state, PLAIN, slot(P1, UNITS, 3), json!({}));
        let victim = put(sink.state, SMALL, slot(P2, UNITS, 2), json!({})); // 1/2
        force_attacks_on(&mut sink, &[second.clone(), third.clone()], &unit_target(&victim), None);

        assert_ne!(instance_in(sink.state, &victim.id).zone.z(), ZoneName::Field);
        assert_eq!(instance_in(sink.state, &second.id).damage, 1);
        assert_eq!(instance_in(sink.state, &third.id).damage, 0);
    }

    #[test]
    fn r54_never_offers_a_token_index_or_the_generating_card_s_own_index_in_a_random_pool() {
        let mut state = game("r54");
        // §5.1: tokens are out unless the query asks for the Token tag.
        assert!(!query(&json_as(json!({ "set": "Core" })))
            .iter()
            .any(|def| def.token || def.tags.contains(&Tag::Token)));
        assert!(!query(&json_as(json!({ "tags": ["Token"] }))).is_empty());

        // And a pool never offers the card that generated it: KY's Trial rerolls its own index.
        let me = put(&mut state, PLAIN, slot(P1, UNITS, 1), json!({}));
        let mut sink = sink_for(&mut state);
        apply_effects(
            &[effects::discover_from_catalog(json_as(json!({ "step": "pick", "query": { "type": "Unit" } })))],
            &mut make_context(&mut sink, Some(&me), controlled(P1)),
        );

        let options: Vec<PromptOption> = sink.state.pending.as_ref().map(|prompt| prompt.options.clone()).unwrap_or_default();
        assert_eq!(options.len(), 3);
        assert!(!options
            .iter()
            .any(|option| matches!(&option.selection, Selection::Mode { option } if option == PLAIN)));
        // M4: cards/test/82-kys-trial.test.ts proves the 1–100 index roll.
    }

    #[test]
    fn r55_counts_both_players_draws_plays_destructions_and_exiles_from_the_start_of_the_game() {
        let mut state = game("r55");
        assert_eq!(
            to_json(&state.counters),
            json!({ "drawn": 0, "played": 0, "destroyed": 0, "exiled": 0 })
        );

        let mut sink = sink_for(&mut state);
        set_library(sink.state, P2, &[PLAIN, SMALL]);
        draw::draw(&mut sink, P2, 1);
        assert_eq!(sink.state.counters.drawn, 1);

        let spell = new_instance(&mut *sink.state, NOOP, P1, Zone::Hand { player: P1 });
        sink.state.players.p1.hand.push(spell.clone());
        cast_card(&mut sink, &spell, CastOptions::default());
        assert_eq!(sink.state.counters.played, 1);

        let doomed = put(sink.state, PLAIN, slot(P2, UNITS, 1), json!({}));
        apply_effects(
            &[effects::destroy(json_as(json!({ "target": { "of": "chosen" } })))],
            &mut make_context(&mut sink, None, targeted(P1, pick(&doomed))),
        );
        state_check(&mut sink);
        assert_eq!(sink.state.counters.destroyed, 1);

        let banished = put(sink.state, SMALL, slot(P2, UNITS, 2), json!({}));
        apply_effects(
            &[effects::exile(json_as(json!({ "target": { "of": "chosen" } })))],
            &mut make_context(&mut sink, None, targeted(P1, pick(&banished))),
        );
        assert_eq!(sink.state.counters.exiled, 1);
        // M4: cards/test/100-ceaseless-void.test.ts proves the card half.
    }

    #[test]
    fn r56_reports_the_cost_actually_paid_after_modifiers_which_is_what_a_cost_threshold_reads() {
        let mut state = playing("r56");
        state.players.p1.hand = vec![];
        let card = hand_card(&mut state, PRICEY, P1); // printed 3
        edit(&mut state, &card, |live| live.cost_mod = -2);
        assert_eq!(price_now(&state, &card), 1);

        let result = act_result(&state, json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }));
        assert_eq!(result.error, None);
        assert_eq!(only(&of_type(&result.events, GameEventType::CardPlayed))["costPaid"], json!(1));
        assert_eq!(result.state.players.p1.mana.current, 0);
    }

    #[test]
    fn r57_makes_a_shuffled_in_copy_a_fresh_instance_carrying_the_radiant_flag_and_a_field_copy_keep_statsoverride(
    ) {
        let mut state = game("r57");
        state.players.p1.library = vec![];

        let me = put(&mut state, PLAIN, slot(P1, UNITS, 1), json!({ "radiant": true }));
        edit(&mut state, &me, |card| {
            card.buffs = AttackHealth { attack: 1, health: 1 };
            card.damage = 2;
            card.counters.plague = Some(3);
            card.granted_keywords = vec![json_as(json!({ "kind": "Taunt" }))];
            card.exertion = Exertion {
                attacked: true,
                switched: false,
                attacks: None,
            };
        });
        let me = instance_in(&state, &me.id);

        let mut sink = sink_for(&mut state);
        let mut ctx = make_context(&mut sink, Some(&me), controlled(P1));
        apply_effects(&[effects::shuffle_copies_of_self(json_as(json!({ "count": 1 })))], &mut ctx);

        let copy = only(&ctx.sink.state.players.p1.library);
        assert_eq!(copy.def_id, PLAIN);
        assert!(copy.radiant);
        assert_eq!(copy.buffs, AttackHealth { attack: 0, health: 0 });
        assert_eq!(copy.damage, 0);
        assert_eq!(to_json(&copy.counters), json!({}));
        assert!(copy.granted_keywords.is_empty());
        assert_eq!(to_json(&copy.exertion), json!({ "attacked": false, "switched": false }));

        // A copy that arrives on the field keeps the radiant flag and the §7 stats override, reset.
        apply_effects(
            &[effects::summon(json_as(json!({
                "defId": PLAIN,
                "radiant": true,
                "statsOverride": { "attack": 5, "health": 5 },
            })))],
            &mut ctx,
        );
        let on_field = card_at(ctx.sink.state, slot(P1, UNITS, 2)).cloned();
        assert_eq!(on_field.as_ref().map(|card| card.radiant), Some(true));
        assert_eq!(
            on_field.as_ref().and_then(|card| card.stats_override),
            Some(AttackHealth { attack: 5, health: 5 })
        );
        assert_eq!(on_field.as_ref().map(|card| card.damage), Some(0));
        assert_eq!(
            on_field.as_ref().map(|card| to_json(&card.exertion)),
            Some(json!({ "attacked": false, "switched": false }))
        );
        // M4: cards/test/12-prejudiced-postdoc.test.ts and cards/test/33-unstable-clone-machine.test.ts
        // prove the card halves.
    }

    #[test]
    fn r58_casts_at_most_cast_on_draw_chain_cap_cards_in_one_draw_casts_on_a_full_hand_and_draws_n_times_for_draw_n(
    ) {
        let mut state = game("r58");
        let mut sink = sink_for(&mut state);
        sink.state.players.p1.hand = vec![];
        let casters = vec![CASTER; (CAST_ON_DRAW_CHAIN_CAP + 5) as usize];
        set_library(sink.state, P1, &casters);

        draw::draw_one(&mut sink, P1, Some(ChainLinkOrCount::from(0)));
        assert_eq!(
            of_type(sink.events, GameEventType::CardPlayed).len(),
            CAST_ON_DRAW_CHAIN_CAP as usize
        );
        assert_eq!(sink.state.players.p1.hand.len(), 1); // the next one ends the chain, uncast
        assert_eq!(sink.state.players.p1.library.len(), 4);

        // A cast-on-draw card is cast even with a full hand, since it never enters the hand.
        let mut full = game("r58-full");
        let mut full_sink = sink_for(&mut full);
        full_sink.state.players.p1.hand = vec![];
        in_hand(full_sink.state, NOOP, P1, HAND_CAP);
        set_library(full_sink.state, P1, &[CASTER]);
        draw::draw_one(&mut full_sink, P1, Some(ChainLinkOrCount::from(0)));
        assert_eq!(of_type(full_sink.events, GameEventType::CardPlayed).len(), 1);
        assert_eq!(full_sink.state.players.p1.hand.len(), HAND_CAP as usize);

        // "Draw N" is N separate draws.
        let mut many = game("r58-n");
        let mut many_sink = sink_for(&mut many);
        many_sink.state.players.p1.hand = vec![];
        set_library(many_sink.state, P1, &[PLAIN, SMALL, NOOP]);
        draw::draw(&mut many_sink, P1, 3);
        assert_eq!(of_type(many_sink.events, GameEventType::Drawn).len(), 3);
    }

    #[test]
    fn r59_runs_the_state_check_after_a_whole_effect_never_between_its_hits_and_calls_two_dead_heroes_a_draw() {
        let mut state = game("r59");
        let mut sink = sink_for(&mut state);
        let first = put(sink.state, SMALL, slot(P2, UNITS, 1), json!({}));
        let second = put(sink.state, SMALL, slot(P2, UNITS, 2), json!({}));

        let spell = new_instance(&mut *sink.state, SPLITTER, P1, Zone::Resolving { player: P1 });
        run_hook(&mut sink, &spell, HookName::Cry, HookOptions::default());

        assert!(unit_view(sink.state, &instance_in(sink.state, &first.id)).health <= 0);
        assert!(unit_view(sink.state, &instance_in(sink.state, &second.id)).health <= 0);
        assert!(of_type(sink.events, GameEventType::Destroyed).is_empty());

        state_check(&mut sink);
        assert_eq!(of_type(sink.events, GameEventType::Destroyed).len(), 2);

        for player in PLAYER_IDS {
            sink.state.players[player].hero.health = 0;
        }
        state_check(&mut sink);
        assert_eq!(
            sink.state.result,
            Some(GameResult {
                winner: Winner::Draw,
                reason: GameOverReason::BothHeroesDead,
            })
        );
    }

    #[test]
    fn r60_picks_different_cards_among_the_non_radiant_ones_all_of_them_when_fewer_exist_and_none_when_none_are_left(
    ) {
        let mut state = game("r60");
        state.players.p1.hand = vec![];
        let cards = in_hand(&mut state, NOOP, P1, 3);
        for card in &cards {
            edit(&mut state, card, |live| live.radiant = true);
        }

        let mut sink = sink_for(&mut state);
        let mut ctx = make_context(&mut sink, None, controlled(P1));
        let cursor = ctx.sink.rng.cursor();
        apply_effects(&[effects::set_radiant_random(json_as(json!({ "zones": "hand", "count": 2 })))], &mut ctx);
        // None left: nothing changes and nothing is drawn (R129); the hidden hand is still cued as a
        // pick of two would cue it, on its Radiant cards (R177).
        assert_eq!(ctx.sink.rng.cursor(), cursor);
        assert_eq!(
            pluck(&of_type(ctx.sink.events, GameEventType::RadiantSet), "instanceId"),
            cards.iter().take(2).map(|card| json!(card.id)).collect::<Vec<_>>()
        );

        let first = must(cards.first().cloned(), "fixture");
        let second = must(cards.get(1).cloned(), "fixture");
        edit(ctx.sink.state, &first, |live| live.radiant = false);
        edit(ctx.sink.state, &second, |live| live.radiant = false);

        let from = ctx.sink.events.len();
        apply_effects(&[effects::set_radiant_random(json_as(json!({ "zones": "hand", "count": 5 })))], &mut ctx);
        // Fewer non-Radiant cards than the pick: both change, and they are different cards.
        assert!(instance_in(ctx.sink.state, &first.id).radiant && instance_in(ctx.sink.state, &second.id).radiant);
        let set = of_type(&ctx.sink.events[from..], GameEventType::RadiantSet);
        let picked: BTreeSet<String> = set
            .iter()
            .take(2)
            .map(|event| event["instanceId"].as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(picked, BTreeSet::from([first.id.clone(), second.id.clone()]));
        // The picks it could not make are cued on the rest of the hand (R177): three cards, three cues.
        assert_eq!(set.len(), 3);

        // Discover options are always different.
        apply_effects(
            &[effects::discover_from_catalog(json_as(json!({ "step": "pick", "query": { "type": "Unit" } })))],
            &mut ctx,
        );
        let keys: Vec<String> = ctx
            .sink
            .state
            .pending
            .as_ref()
            .map(|prompt| prompt.options.iter().map(|option| option.key.clone()).collect())
            .unwrap_or_default();
        assert_eq!(keys.len(), 3);
        assert_eq!(keys.iter().collect::<BTreeSet<_>>().len(), 3);
    }

    #[test]
    fn r61_emits_cardplayed_only_for_a_play_or_a_cast_refuses_an_immutable_fuse_target_and_consumes_a_trap_that_does_nothing(
    ) {
        let mut state = playing("r61");
        let mut sink = sink_for(&mut state);
        let mut ctx = make_context(&mut sink, None, controlled(P1));

        // Summon, Recruit and Transform never set a "plays a permanent" trap off.
        apply_effects(&[effects::summon(json_as(json!({ "defId": PLAIN })))], &mut ctx);
        set_library(ctx.sink.state, P1, &[SMALL]);
        apply_effects(&[effects::recruit(json_as(json!({})))], &mut ctx);
        let summoned = card_at(ctx.sink.state, slot(P1, UNITS, 1)).cloned();
        assert!(summoned.is_some());
        let summoned_id = summoned.map(|card| card.id).unwrap_or_default();
        apply_effects(
            &[effects::transform(json_as(json!({ "instanceId": summoned_id, "defId": BIG_BODY })))],
            &mut ctx,
        );
        assert!(!of_type(ctx.sink.events, GameEventType::Summoned).is_empty());
        assert!(of_type(ctx.sink.events, GameEventType::CardPlayed).is_empty());

        // A Field Trap counts as a Trap.
        let trap = put(ctx.sink.state, EMPTY_TRAP, slot(P1, BACKROW, 1), json!({}));
        let field_trap = put(ctx.sink.state, WINDOW_TRAP, slot(P1, BACKROW, 2), json!({}));
        assert!(is_trap_type(ctx.sink.state, &trap));
        assert!(is_trap_type(ctx.sink.state, &field_trap));

        // An Immutable permanent is never chosen as the Fuse target (R23).
        let warded = put(ctx.sink.state, IMMUTABLE, slot(P2, UNITS, 1), json!({}));
        let food = put(ctx.sink.state, PLAIN, slot(P2, UNITS, 2), json!({}));
        drop(ctx);
        assert!(fuse(&mut sink, json_as(json!({ "ingredients": [warded, food], "target": warded }))).is_none());

        // With no legal target the trap fires, is consumed and does nothing; the permanent stays.
        let mut live = playing("r61-trap");
        put(&mut live, EMPTY_TRAP, slot(P2, BACKROW, 1), json!({}));
        live.players.p1.hand = vec![];
        let unit = hand_card(&mut live, PLAIN, P1);
        let result = act_result(
            &live,
            json!({
                "type": "play",
                "instanceId": unit.id,
                "playerId": "p1",
                "zone": { "row": "units", "lane": 1 },
            }),
        );
        assert_eq!(result.error, None);
        assert_eq!(of_type(&result.events, GameEventType::TrapFired).len(), 1);
        let live = result.state;
        assert!(live.players.p2.graveyard.iter().any(|card| card.def_id == EMPTY_TRAP));
        assert_eq!(def_at(&live, slot(P1, UNITS, 1)), Some(PLAIN.to_string()));
        // M4: cards/test/85-unlicensed-experimentation.test.ts proves the card half.
    }

    #[test]
    fn r62_ends_a_turn_as_triggers_then_the_trap_window_on_both_sides_then_delayed_effects_then_cleanup() {
        let mut state = playing("r62");
        put(&mut state, LOG_CARD, slot(P1, BACKROW, 5), json!({}));
        let clock_card = put(&mut state, CLOCK, slot(P1, UNITS, 1), json!({}));
        put(&mut state, WINDOW_TRAP, slot(P1, BACKROW, 1), json!({}));
        put(&mut state, WINDOW_TRAP, slot(P2, BACKROW, 1), json!({}));

        {
            let mut sink = sink_for(&mut state);
            schedule_delayed(
                &mut sink,
                P1,
                DelayedAt {
                    phase: Phase::End,
                    player: P1,
                },
                delayed_resume(CLOCK, Some(&clock_card.id), json!({})),
                None,
                None,
            );
            let turn = sink.state.turn;
            add_modifier(&mut sink, P1, ModifierExpiry::ThisTurn { turn }, cost_discount(1, None));
        }

        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(rb_steps(&state), vec!["end", "trap:p1", "trap:p2", "delayed"]);
        assert!(state.players.p1.mods.is_empty()); // cleanup came after all of it

        // And the start half: refresh, start-of-turn triggers, then the draw.
        let library_before = state.players.p1.library.len();
        state = act(&state, json!({ "type": "endTurn", "playerId": "p2" }));
        assert_eq!(
            rb_steps(&state),
            vec![
                "end".to_string(),
                "trap:p1".to_string(),
                "trap:p2".to_string(),
                "delayed".to_string(),
                "trap:p2".to_string(),
                "trap:p1".to_string(),
                format!("start:lib{library_before}"),
            ]
        );
        assert_eq!(state.players.p1.library.len(), library_before - 1);
    }

    #[test]
    fn r63_tramples_only_the_excess_cleaves_past_a_stopped_hit_ignores_zero_hits_and_lifesteals_the_total_once() {
        let mut state = game("r63");
        let mut sink = sink_for(&mut state);

        // Trample: up to the unit's health lands on it, the rest on its controller's hero.
        let tramp = put(sink.state, TRAMPLER, slot(P1, UNITS, 1), json!({})); // 6/4 Trample
        let blocker = put(sink.state, SMALL, slot(P2, UNITS, 1), json!({})); // 1/2
        deal_damage(&mut sink, hit(Some(tramp.clone()), unit_hit(&blocker), 6));
        assert_eq!(instance_in(sink.state, &blocker.id).damage, 2);
        assert_eq!(sink.state.players.p2.hero.health, HERO_HEALTH - 4);

        // A hit of 0 before step 1 is not a damage instance, so Divine Shield stays.
        let shield = put(sink.state, SHIELDED, slot(P2, UNITS, 2), json!({}));
        assert_eq!(deal_damage(&mut sink, hit(None, unit_hit(&shield), 0)), 0);
        assert_eq!(instance_in(sink.state, &shield.id).divine_shield_spent, None);

        // A hit reduced to 0 by Armor emits no event and triggers nothing.
        let armour = put(sink.state, ARMOURED, slot(P2, UNITS, 3), json!({})); // Armor 5
        let before = sink.events.len();
        assert_eq!(deal_damage(&mut sink, hit(None, unit_hit(&armour), 2)), 0);
        assert_eq!(sink.events.len(), before);

        // Cleave belongs to the attack, so it lands even when the hit on the defender was stopped.
        let mut cleave_game = game("r63-cleave");
        let mut cleave_sink = sink_for(&mut cleave_game);
        let cleaver_unit = put(cleave_sink.state, CLEAVER, slot(P1, UNITS, 1), json!({})); // 3/6 Cleave
        let defender = put(cleave_sink.state, SHIELDED, slot(P2, UNITS, 2), json!({}));
        let left = put(cleave_sink.state, PLAIN, slot(P2, UNITS, 1), json!({}));
        let right = put(cleave_sink.state, PLAIN, slot(P2, UNITS, 3), json!({}));
        force_attack(&mut cleave_sink, &cleaver_unit, &unit_target(&defender));
        assert_eq!(instance_in(cleave_sink.state, &defender.id).divine_shield_spent, Some(true));
        let cleaves: Vec<Value> = of_type(cleave_sink.events, GameEventType::Damage)
            .into_iter()
            .filter(|event| event["targetId"] == left.id.as_str() || event["targetId"] == right.id.as_str())
            .map(|event| event["amount"].clone())
            .collect();
        assert_eq!(cleaves, vec![json!(3), json!(3)]);

        // A zero-attack unit striking back is not a damage instance either.
        let mut zero_game = game("r63-zero");
        let mut zero_sink = sink_for(&mut zero_game);
        let striker = put(zero_sink.state, SHIELDED, slot(P1, UNITS, 1), json!({}));
        let dummy = put(zero_sink.state, ZERO_ATTACK, slot(P2, UNITS, 1), json!({}));
        force_attack(&mut zero_sink, &striker, &unit_target(&dummy));
        assert_eq!(instance_in(zero_sink.state, &striker.id).divine_shield_spent, None);
        assert_eq!(instance_in(zero_sink.state, &dummy.id).damage, 2);

        // Lifesteal heals the total of a Trample hit once, not the unit's share twice.
        let mut leech_game = game("r63-lifesteal");
        let mut leech_sink = sink_for(&mut leech_game);
        let leech = put(leech_sink.state, TRAMPLE_LEECH, slot(P1, UNITS, 1), json!({}));
        let chump = put(leech_sink.state, SMALL, slot(P2, UNITS, 1), json!({}));
        deal_damage(&mut leech_sink, hit(Some(leech.clone()), unit_hit(&chump), 6));
        assert_eq!(leech_sink.state.players.p1.hero.health, HERO_HEALTH + 6);

        // Poisonous only affects units.
        let mut poison_game = game("r63-poison");
        let mut poison_sink = sink_for(&mut poison_game);
        let poison = put(poison_sink.state, POISONER, slot(P1, UNITS, 1), json!({}));
        let victim = put(poison_sink.state, BIG_BODY, slot(P2, UNITS, 1), json!({}));
        deal_damage(&mut poison_sink, hit(Some(poison.clone()), unit_hit(&victim), 1));
        assert_eq!(instance_in(poison_sink.state, &victim.id).marked_destroyed, Some(true));
        deal_damage(&mut poison_sink, hit(Some(poison.clone()), DamageTarget::Hero { player: P2 }, 1));
        assert_eq!(poison_sink.state.players.p2.hero.health, HERO_HEALTH - 1);
        assert!(poison_sink.state.result.is_none());
    }

    #[test]
    fn r64_summons_into_the_leftmost_open_zone_fills_the_board_left_to_right_and_reserves_a_reborn_unit_s_zone() {
        let mut state = game("r64");
        let mut sink = sink_for(&mut state);
        let mut ctx = make_context(&mut sink, None, controlled(P1));

        put(ctx.sink.state, PLAIN, slot(P1, UNITS, 1), json!({}));
        lock_zone(ctx.sink.state, slot(P1, UNITS, 2));
        apply_effects(&[effects::summon(json_as(json!({ "defId": SMALL })))], &mut ctx);
        assert_eq!(def_at(ctx.sink.state, slot(P1, UNITS, 3)), Some(SMALL.to_string()));

        apply_effects(&[effects::fill_board(json_as(json!({ "defId": UNIT_TOKEN })))], &mut ctx);
        assert_eq!(
            slots_of(P1, UNITS)
                .into_iter()
                .map(|zone| def_at(ctx.sink.state, zone))
                .collect::<Vec<_>>(),
            vec![
                Some(PLAIN.to_string()),
                None,
                Some(SMALL.to_string()),
                Some(UNIT_TOKEN.to_string()),
                Some(UNIT_TOKEN.to_string()),
            ]
        );

        let rb = put(ctx.sink.state, REBORN_UNIT, slot(P2, UNITS, 3), json!({}));
        edit(ctx.sink.state, &rb, |card| card.damage = 99);
        drop(ctx);
        state_check(&mut sink);
        assert_eq!(id_at(sink.state, slot(P2, UNITS, 3)), Some(rb.id.clone()));
        assert_eq!(unit_view(sink.state, &instance_in(sink.state, &rb.id)).health, 1);
        assert!(!is_reserved(sink.state, slot(P2, UNITS, 3)));
    }

    #[test]
    fn r65_reads_costoverride_then_costmod_then_the_player_s_discounts_then_curvature_floored_at_0() {
        let mut state = game("r65");
        let mut sink = sink_for(&mut state);
        let card = hand_card(sink.state, GIGA, P1); // printed 6

        assert_eq!(price_now(sink.state, &card), 6);
        edit(sink.state, &card, |live| live.cost_mod = -1);
        assert_eq!(price_now(sink.state, &card), 5);
        edit(sink.state, &card, |live| live.cost_override = Some(5));
        assert_eq!(price_now(sink.state, &card), 4);

        add_modifier(&mut sink, P1, ModifierExpiry::Never, cost_discount(1, Some(4)));
        assert_eq!(price_now(sink.state, &card), 3);
        add_modifier(&mut sink, P1, ModifierExpiry::Never, cost_discount(9, None));
        assert_eq!(price_now(sink.state, &card), 0);

        // An X-cost card costs exactly X, ignores modifiers, and an override makes it free.
        let x = hand_card(sink.state, X_CARD, P1);
        edit(sink.state, &x, |live| {
            live.x = Some(3);
            live.cost_mod = -2;
        });
        assert_eq!(price_now(sink.state, &x), 3);
        edit(sink.state, &x, |live| live.cost_override = Some(4));
        assert_eq!(price_now(sink.state, &x), 0);

        // Outside play an X card reads 0 and an embiggen card its base price.
        assert_eq!(query_cost(def_of(Some(&*sink.state), X_CARD)), 0);
        assert_eq!(query_cost(def_of(Some(&*sink.state), EMBIGGEN_CARD)), 2);
        let emb = hand_card(sink.state, EMBIGGEN_CARD, P1);
        assert_eq!(printed_cost(sink.state, &emb), 2);
        edit(sink.state, &emb, |live| live.embiggened = Some(true));
        assert_eq!(printed_cost(sink.state, &instance_in(sink.state, &emb.id)), 4);
    }

    #[test]
    fn r66_reads_genn_s_greed_costs_per_r65_at_resolution_and_exempts_x_cost_cards_from_both_halves() {
        assert_eq!(GENN_GREED_EXILES, "odd");

        let mut state = game("r66");
        let card = hand_card(&mut state, PRICEY, P1); // printed 3
        edit(&mut state, &card, |live| live.cost_mod = -1);
        assert_eq!(price_now(&state, &card), 2); // the 2-cost draw reads the modified cost

        let x = hand_card(&mut state, X_CARD, P1);
        assert!(is_x_cost(&state, &x));
        assert_eq!(price_now(&state, &x), 0);
        assert_eq!(query_cost(def_of(Some(&state), X_CARD)), 0); // neither odd nor 2: exempt from both
        // M4: cards/test/94-genns-greed.test.ts proves the card half.
    }

    #[test]
    fn r67_r429_takes_ky_s_math_equation_s_fib_index_from_the_times_it_has_been_played_this_play_included_and_never_from_its_cost(
    ) {
        let mut state = game("r67");
        state.turn = 3;
        state.active = P1;
        state.phase = Phase::Main;
        let mut sink = sink_for(&mut state);
        let card = new_instance(&mut *sink.state, EQUATION, P1, Zone::Resolving { player: P1 });
        sink.state.players.p1.resolving.push(card.clone());
        // A price that moved every way it can: costMod up, a player discount down (R65).
        edit(sink.state, &card, |live| live.cost_mod = 3);
        add_modifier(&mut sink, P1, ModifierExpiry::Never, cost_discount(2, None));

        // R70: a cast is a play, counted at §10.5 step 4 — the one under way included.
        let resolving = instance_in(sink.state, &card.id);
        cast_card(&mut sink, &resolving, CastOptions::default());
        let played = instance_in(sink.state, &card.id);
        assert_eq!(times_played_of(&played), 1);

        // R429: the index is the plays + 1 (Radiant + 3); the cost reads nowhere in it (R67).
        assert_eq!(fib(times_played_of(&played) + 1), 1);
        assert_eq!(fib(times_played_of(&played) + 3), 3);
        // M4: cards/test/31-kys-math-equation.test.ts proves the card half.
    }

    #[test]
    fn r68_orders_triggers_active_side_first_units_by_lane_then_backrow_hand_and_graveyard_delayed_by_creation() {
        let mut state = game("r68");
        state.active = P2;

        let theirs = put(&mut state, PLAIN, slot(P2, UNITS, 1), json!({}));
        let lane2 = put(&mut state, PLAIN, slot(P1, UNITS, 2), json!({}));
        let lane1 = put(&mut state, SMALL, slot(P1, UNITS, 1), json!({}));
        let backrow = put(&mut state, LOG_CARD, slot(P1, BACKROW, 3), json!({}));
        let in_hand_card = hand_card(&mut state, NOOP, P1);
        let buried = new_instance(&mut state, NOOP, P1, Zone::Graveyard { player: P1 });
        state.players.p1.graveyard.push(buried.clone());

        assert_eq!(
            cards_in_trigger_order(&state).iter().map(|holder| holder.card.id.clone()).collect::<Vec<_>>(),
            vec![
                theirs.id.clone(),
                lane1.id.clone(),
                lane2.id.clone(),
                backrow.id.clone(),
                in_hand_card.id.clone(),
                buried.id.clone(),
            ]
        );

        let mut sink = sink_for(&mut state);
        let stub = delayed_resume(CLOCK, None, json!({}));
        let at_start = || DelayedAt {
            phase: Phase::Start,
            player: P1,
        };
        let first = schedule_delayed(&mut sink, P1, at_start(), stub.clone(), None, None);
        let second = schedule_delayed(&mut sink, P1, at_start(), stub.clone(), None, None);
        sink.state.delayed.reverse(); // the order in the array is not the order they resolve in
        assert_eq!(
            due_delayed(sink.state, Phase::Start, P1).iter().map(|effect| effect.id.clone()).collect::<Vec<_>>(),
            vec![first.id.clone(), second.id.clone()]
        );
    }

    #[test]
    fn r69_collects_an_indestructible_unit_whose_max_health_falls_to_0_and_leaves_one_merely_at_0_health() {
        let mut state = game("r69");
        let mut sink = sink_for(&mut state);
        let warded = put(sink.state, WARDED_PINGER, slot(P1, UNITS, 1), json!({})); // 4/4 Indestructible

        edit(sink.state, &warded, |card| card.damage = 10); // 0 or less health, but max health is still 4
        state_check(&mut sink);
        assert_eq!(id_at(sink.state, slot(P1, UNITS, 1)), Some(warded.id.clone()));
        assert_eq!(sink.state.players.p2.hero.health, HERO_HEALTH);

        let destroyed_before = sink.state.counters.destroyed;
        // Max health falls to 0: no destroy effect is involved, so it dies.
        edit(sink.state, &warded, |card| card.buffs.health = -4);
        state_check(&mut sink);
        assert!(card_at(sink.state, slot(P1, UNITS, 1)).is_none());
        assert_eq!(sink.state.counters.destroyed, destroyed_before + 1);
        assert_eq!(sink.state.players.p2.hero.health, HERO_HEALTH - 1); // its Death fired
    }

    #[test]
    fn r70_makes_a_cast_free_counts_it_as_a_play_fires_the_card_s_cry_and_sends_a_spell_to_the_graveyard() {
        let mut state = playing("r70");
        let mut sink = sink_for(&mut state);
        add_modifier(&mut sink, P1, ModifierExpiry::Never, cost_discount(2, None));

        let spell = new_instance(&mut *sink.state, PRICEY, P1, Zone::Hand { player: P1 }); // printed 3
        sink.state.players.p1.hand.push(spell.clone());
        let mana_before = sink.state.players.p1.mana.current;

        cast_card(&mut sink, &spell, CastOptions::default());

        let played = only(&of_type(sink.events, GameEventType::CardPlayed));
        assert_eq!(played["costPaid"], json!(0)); // free, and no discount was consumed to get there
        assert_eq!(sink.state.players.p1.mana.current, mana_before);
        assert!(sink.state.players.p1.turn_log.played_ids.contains(&spell.id));
        assert_eq!(sink.state.counters.played, 1);
        assert_eq!(sink.state.players.p2.hero.health, HERO_HEALTH - 1); // the Cry resolved
        assert!(sink.state.players.p1.graveyard.iter().any(|card| card.id == spell.id));
        // M4: cards/test/79-twinspell.test.ts proves that a cast Spell still uses Echo.
    }

    #[test]
    fn r71_copies_every_other_card_played_this_turn_at_end_of_turn_including_the_ones_played_after_it() {
        let mut state = playing("r71");
        state.players.p1.hand = vec![];
        put(&mut state, PLAIN, slot(P1, UNITS, 1), json!({})); // keeps the turn from auto-ending (R82)

        let recycler_card = hand_card(&mut state, RECYCLER, P1);
        let later = hand_card(&mut state, NOOP, P1);

        state = act(
            &state,
            json!({
                "type": "play",
                "instanceId": recycler_card.id,
                "playerId": "p1",
                "zone": { "row": "backrow", "lane": 1 },
            }),
        );
        state = act(&state, json!({ "type": "play", "instanceId": later.id, "playerId": "p1" }));
        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));

        // The card played after it is copied; the recycler never copies itself.
        assert_eq!(def_ids(&state.players.p1.hand), vec![NOOP]);
        // M4: cards/test/39-recycling-initiative.test.ts proves the card half.
    }

    #[test]
    fn r72_keeps_exile_piles_with_their_owners_and_lets_a_hero_climb_above_the_30_that_missing_health_counts_from() {
        let mut state = game("r72");
        let mut sink = sink_for(&mut state);
        assert_eq!(HERO_HEALTH, 30);

        let mine = put(sink.state, PLAIN, slot(P1, UNITS, 1), json!({}));
        let theirs = put(sink.state, PLAIN, slot(P2, UNITS, 1), json!({}));

        apply_effects(
            &[effects::exile(json_as(json!({ "target": { "of": "chosen" } })))],
            &mut make_context(&mut sink, None, targeted(P1, pick(&theirs))),
        );
        apply_effects(
            &[effects::exile(json_as(json!({ "target": { "of": "chosen" } })))],
            &mut make_context(&mut sink, None, targeted(P1, pick(&mine))),
        );

        // "Cards in exile" is your own pile: each card went to its owner's (R12).
        assert_eq!(ids(&sink.state.players.p1.exile), vec![mine.id.clone()]);
        assert_eq!(ids(&sink.state.players.p2.exile), vec![theirs.id.clone()]);

        // A hero has no maximum, so missing health has to count from HERO_HEALTH, not from current.
        heal_hero(&mut sink, P1, 10);
        assert_eq!(sink.state.players.p1.hero.health, HERO_HEALTH + 10);
        assert_eq!(0.max(HERO_HEALTH - sink.state.players.p1.hero.health), 0);
        // M4: cards/test/40-echoes-of-the-forgotten.test.ts and cards/test/70-spiteful-stab.test.ts
        // prove the card halves.
    }

    #[test]
    fn r73_gives_pocket_chaos_its_three_swaps_armor_apart_from_health_locks_with_the_zone_a_trap_read_by_its_controller_and_owner_routed_libraries(
    ) {
        let mut state = game("r73");

        // The three swaps each have their own event (§10.3).
        assert!(GameEventType::ALL.iter().any(|event_type| event_type.as_str() == "swapped"));
        let swaps: Vec<GameEvent> = vec![
            json_as(json!({ "type": "swapped", "what": "health" })),
            json_as(json!({ "type": "swapped", "what": "board" })),
            json_as(json!({ "type": "swapped", "what": "library" })),
        ];
        assert_eq!(
            pluck(&of_type(&swaps, GameEventType::Swapped), "what"),
            vec![json!("health"), json!("board"), json!("library")]
        );

        // Health and armor are separate fields, so swapping the health values leaves armor alone.
        let HeroState { health: _, armor: _ } = state.players.p1.hero;
        let mut hero_fields: Vec<String> = to_json(&state.players.p1.hero)
            .as_object()
            .map(|fields| fields.keys().cloned().collect())
            .unwrap_or_default();
        hero_fields.sort();
        assert_eq!(hero_fields, vec!["armor", "health"]);

        // Locks stay with their zones, not with the card that occupied them (§3.2).
        let mut occupant = put(&mut state, PLAIN, slot(P1, UNITS, 1), json!({}));
        lock_zone(&mut state, slot(P1, UNITS, 1));
        move_to_zone(&mut state, &mut occupant, OffFieldZone::Graveyard, Default::default());
        assert!(card_at(&state, slot(P1, UNITS, 1)).is_none());
        assert!(!is_open(&state, slot(P1, UNITS, 1)));

        // A face-down trap is readable by its controller only, so control is what a swap moves (R33).
        let trap = put(&mut state, EMPTY_TRAP, slot(P1, BACKROW, 1), json!({}));
        assert_eq!(
            to_json(&view_for(&state, P2).opponent.backrow[0]),
            json!({ "faceDown": true, "cost": 0 })
        );
        edit(&mut state, &trap, |card| card.controller = P2);
        assert_matches(
            to_json(&view_for(&state, P2).opponent.backrow[0]),
            json!({ "faceDown": false, "defId": EMPTY_TRAP }),
        );
        assert_eq!(
            to_json(&view_for(&state, P1).you.backrow[0]),
            json!({ "faceDown": true, "cost": 0 })
        );

        // The library swap is R12's one exception: a card follows its owner off the field.
        let mut card = only(&state.players.p1.library);
        state.players.p1.library = vec![];
        card.owner = P2;
        move_to_zone(&mut state, &mut card, OffFieldZone::Graveyard, Default::default());
        assert_eq!(ids(&state.players.p2.graveyard), vec![card.id.clone()]);
        assert!(!state.players.p1.graveyard.iter().any(|c| c.id == card.id));

        // Fatigue counters belong to the player, so swapped libraries leave them behind.
        state.players.p1.fatigue_count = 4;
        let players = &mut state.players;
        std::mem::swap(&mut players.p1.library, &mut players.p2.library);
        assert_eq!(state.players.p1.fatigue_count, 4);
        assert_eq!(state.players.p2.fatigue_count, 0);
        // M4: cards/test/87-pocket-chaos.test.ts proves the card half.
    }

    #[test]
    fn r74_models_radiant_as_a_flag_that_never_unsets_swaps_the_layer_in_place_and_rides_copies_and_formless_cards() {
        let mut state = game("r74");
        let mut sink = sink_for(&mut state);
        let mut ctx = make_context(&mut sink, None, controlled(P1));
        let set_radiant = |card: &CardInstance| effects::set_radiant(json_as(json!({ "instanceId": card.id })));

        // In hand: the flag is the whole model, and setting it twice changes nothing — though the cue
        // is repeated, since the hand card is hidden from the opponent (R177).
        let held = hand_card(ctx.sink.state, PLAIN, P1);
        apply_effects(&[set_radiant(&held)], &mut ctx);
        assert!(instance_in(ctx.sink.state, &held.id).radiant);
        assert_eq!(of_type(ctx.sink.events, GameEventType::RadiantSet).len(), 1);
        apply_effects(&[set_radiant(&held)], &mut ctx);
        assert!(instance_in(ctx.sink.state, &held.id).radiant);
        assert_eq!(of_type(ctx.sink.events, GameEventType::RadiantSet).len(), 2);

        // On the field: the base-stat layer swaps at once, damage and buffs stay, no Cry re-fires.
        let unit = put(ctx.sink.state, FUSE_A, slot(P1, UNITS, 1), json!({})); // 2/3, and its Cry pings the hero
        edit(ctx.sink.state, &unit, |card| {
            card.damage = 1;
            card.buffs = AttackHealth { attack: 1, health: 0 };
        });
        assert_eq!(unit_view(ctx.sink.state, &instance_in(ctx.sink.state, &unit.id)).attack, 3);
        apply_effects(&[set_radiant(&unit)], &mut ctx);
        assert_eq!(unit_view(ctx.sink.state, &instance_in(ctx.sink.state, &unit.id)).attack, 5); // 4 printed radiant + 1 buff
        assert_eq!(instance_in(ctx.sink.state, &unit.id).damage, 1);
        assert_eq!(ctx.sink.state.players.p2.hero.health, HERO_HEALTH);

        // A card an effect generates "Radiant" is Radiant.
        apply_effects(
            &[effects::add_to_hand(json_as(json!({ "defId": PLAIN, "radiant": true })))],
            &mut ctx,
        );
        let generated = ctx.sink.state.players.p1.hand.iter().find(|card| card.id != held.id).cloned();
        assert_eq!(generated.map(|card| card.radiant), Some(true));

        // A definition whose radiant face is its base face is unchanged, but the flag still sets. No
        // Core card is one any more (R276); the engine still reads such a face, as this fixture shows.
        let formless = put(ctx.sink.state, SAME_FACE, slot(P1, UNITS, 2), json!({}));
        apply_effects(&[set_radiant(&formless)], &mut ctx);
        let formless_now = instance_in(ctx.sink.state, &formless.id);
        assert!(formless_now.radiant);
        assert_eq!(unit_view(ctx.sink.state, &formless_now).attack, 2);
    }

    #[test]
    fn r75_keeps_the_5_3_corrections_in_the_catalog_shape_a_set_and_type_on_every_def_the_felinor_tag_n_1_indexes_and_cost_6(
    ) {
        let state = game("r75");
        let catalog = registered_catalog();
        let types = [CardType::Unit, CardType::Spell, CardType::FieldSpell, CardType::Trap, CardType::FieldTrap];

        // Every def carries the set and the type the source left out.
        assert!(catalog.values().all(|def| !def.set.as_str().is_empty()));
        assert!(catalog.values().all(|def| types.contains(&def.type_)));
        assert_eq!(def_of(Some(&state), SAME_FACE).type_, CardType::Unit);

        // The tribe tag is Felinor, not "Felinors".
        let felinor: Tag = Tag::Felinor;
        assert!(def_of(Some(&state), FUSE_B).tags.contains(&felinor));

        // A token index of the "N.1" form addresses a card (#90.1 CN-Virus).
        assert_eq!(
            def_by_index(SetName::Core, "51.1").map(|def| def.id.clone()),
            Some(DOTTED.to_string())
        );

        // #29 keeps cost 6 even though MAX_MANA is 4: castable only after a mana gain.
        assert_eq!(MAX_MANA, 4);
        let mut live = playing("r75-giga");
        live.players.p1.hand = vec![];
        let six = hand_card(&mut live, GIGA, P1);
        assert_eq!(price_now(&live, &six), 6);
        assert!(!legal_actions(&live, P1).iter().any(|action| plays_card(action, &six.id)));
        live.players.p1.mana.current = 6;
        assert!(legal_actions(&live, P1).iter().any(|action| plays_card(action, &six.id)));
    }

    #[test]
    fn r76_fires_the_delayed_steal_at_your_next_start_of_turn_even_though_the_unit_died_and_fizzles_on_a_card_already_yours(
    ) {
        let mut state = playing("r76");
        let fanatic = put(&mut state, KPOP, slot(P1, UNITS, 1), json!({}));
        let prize = put(&mut state, PLAIN, slot(P2, UNITS, 1), json!({}));

        {
            let mut sink = sink_for(&mut state);
            schedule_delayed(
                &mut sink,
                P1,
                DelayedAt {
                    phase: Phase::Start,
                    player: P1,
                },
                delayed_resume(KPOP, Some(&fanatic.id), json!({ "target": prize.id })),
                None,
                None,
            );

            edit(sink.state, &fanatic, |card| card.damage = 99);
            state_check(&mut sink);
            assert!(card_at(sink.state, slot(P1, UNITS, 1)).is_none());
        }

        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        state = act(&state, json!({ "type": "endTurn", "playerId": "p2" }));
        assert_eq!(def_at(&state, slot(P1, UNITS, 1)), Some(PLAIN.to_string()));

        // It fizzles when the target is already under your control.
        let mut own = playing("r76-own");
        let holder = put(&mut own, KPOP, slot(P1, UNITS, 1), json!({}));
        let mine = put(&mut own, PLAIN, slot(P1, UNITS, 2), json!({}));
        {
            let mut own_sink = sink_for(&mut own);
            schedule_delayed(
                &mut own_sink,
                P1,
                DelayedAt {
                    phase: Phase::Start,
                    player: P1,
                },
                delayed_resume(KPOP, Some(&holder.id), json!({ "target": mine.id })),
                None,
                None,
            );
        }
        own = act(&own, json!({ "type": "endTurn", "playerId": "p1" }));
        let back = act_result(&own, json!({ "type": "endTurn", "playerId": "p2" }));
        assert_eq!(back.error, None);
        assert!(of_type(&back.events, GameEventType::ControlChanged).is_empty());
        // M4: cards/test/050-k-pop-fanatic.test.ts proves the card half.
    }

    #[test]
    fn r77_fuses_the_base_forms_keeps_the_target_s_instance_sums_buffs_and_crafts_a_free_non_radiant_hand_card() {
        let mut state = game("r77");
        let mut sink = sink_for(&mut state);
        let target = put(sink.state, FUSE_A, slot(P1, UNITS, 1), json!({})); // 2/3 Taunt, cost 2, Human
        let food = put(sink.state, FUSE_B, slot(P1, UNITS, 2), json!({})); // 1/1 Rush, cost 3, Felinor
        edit(sink.state, &target, |card| {
            card.damage = 1;
            card.buffs = AttackHealth { attack: 1, health: 0 };
            card.granted_keywords = vec![json_as(json!({ "kind": "Lifesteal" }))];
        });
        edit(sink.state, &food, |card| card.buffs = AttackHealth { attack: 0, health: 2 });

        let target_now = instance_in(sink.state, &target.id);
        let food_now = instance_in(sink.state, &food.id);
        let result = fuse(
            &mut sink,
            json_as(json!({ "ingredients": [target_now, food_now], "target": target_now })),
        );
        assert_eq!(result.as_ref().map(|card| card.id.clone()), Some(target.id.clone()));

        let kept = instance_in(sink.state, &target.id);
        let def = def_of(Some(&*sink.state), &kept.def_id).clone();
        assert_eq!(def.base.attack, Some(3));
        assert_eq!(def.base.health, Some(4));
        assert_eq!(def.radiant.attack, Some(6));
        assert_eq!(def.cost, CardCost::Fixed(FUSE_COST_CAP)); // min(2 + 3, 4)
        assert_eq!(def.type_, CardType::Unit);
        let mut tags: Vec<&str> = def.tags.iter().map(|tag| tag.as_str()).collect();
        tags.sort();
        assert_eq!(tags, vec!["Felinor", "Human"]);
        let mut kinds: Vec<&str> = def.base.keywords.iter().map(|keyword| keyword.kind().as_str()).collect();
        kinds.sort();
        assert_eq!(kinds, vec!["Rush", "Taunt"]);

        // The kept instance keeps everything; the other ingredient ceases to exist.
        assert_eq!(kept.damage, 1);
        assert_eq!(kept.buffs, AttackHealth { attack: 1, health: 2 });
        assert_eq!(
            kept.granted_keywords.iter().map(|keyword| keyword.kind().as_str()).collect::<Vec<_>>(),
            vec!["Lifesteal"]
        );
        assert!(card_at(sink.state, slot(P1, UNITS, 2)).is_none());
        assert!(sink.state.players.p1.graveyard.is_empty());
        assert_eq!(sink.state.counters.destroyed, 0);

        // The scripts are concatenated, so both Cry lists run.
        run_hook(&mut sink, &kept, HookName::Cry, HookOptions::default());
        assert_eq!(sink.state.players.p2.hero.health, HERO_HEALTH - 3);

        // Craft a Card: no target on the field, a fresh non-Radiant hand card at cost 0.
        let a = hand_card(sink.state, FUSE_A, P1);
        let b = hand_card(sink.state, FUSE_B, P1);
        let crafted = fuse(&mut sink, json_as(json!({ "ingredients": [a, b], "toHand": "p1" })));
        assert!(crafted.is_some());
        assert_eq!(crafted.as_ref().map(|card| card.radiant), Some(false));
        assert_eq!(crafted.as_ref().and_then(|card| card.cost_override), Some(0));
        assert_eq!(crafted.as_ref().map_or(-1, |card| price_now(sink.state, card)), 0);
    }

    #[test]
    fn r78_resets_an_instance_as_it_leaves_the_field_while_costmod_costoverride_and_radiant_persist() {
        let mut state = game("r78");
        let mut sink = sink_for(&mut state);
        let unit = put(sink.state, PLAIN, slot(P1, UNITS, 1), json!({ "radiant": true }));
        edit(sink.state, &unit, |card| {
            card.damage = 2;
            card.buffs = AttackHealth { attack: 1, health: 1 };
            card.granted_keywords = vec![json_as(json!({ "kind": "Taunt" }))];
            card.vanilla = true;
            card.counters.plague = Some(2);
            card.memory.insert("note".to_string(), json!("eaten"));
            card.exertion = Exertion {
                attacked: true,
                switched: true,
                attacks: None,
            };
            card.position = Some(Position::Def);
            card.summoned_turn = Some(7);
            card.stats_override = Some(AttackHealth { attack: 9, health: 9 });
            card.taunt_suppressed_turn = Some(3);
            card.controller = P2;
            card.cost_mod = -1;
            card.cost_override = Some(2);
        });

        let mut leaving = instance_in(sink.state, &unit.id);
        move_to_zone(sink.state, &mut leaving, OffFieldZone::Hand, Default::default());

        let unit_now = instance_in(sink.state, &unit.id);
        assert_matches(
            to_json(&unit_now),
            json!({
                "damage": 0,
                "buffs": { "attack": 0, "health": 0 },
                "grantedKeywords": [],
                "vanilla": false,
                "counters": {},
                "memory": {},
                "exertion": { "attacked": false, "switched": false },
                "controller": "p1",
                "radiant": true,
                "costMod": -1,
                "costOverride": 2,
            }),
        );
        assert_eq!(unit_now.position, None);
        assert_eq!(unit_now.summoned_turn, None);
        assert_eq!(unit_now.stats_override, None);
        assert_eq!(unit_now.taunt_suppressed_turn, None);

        // A Death trigger reads the card as it was just before it left.
        let dying = put(sink.state, REMEMBERER, slot(P1, UNITS, 2), json!({}));
        edit(sink.state, &dying, |card| {
            card.memory.insert("note".to_string(), json!("remembered"));
            card.damage = 99;
        });
        state_check(&mut sink);
        assert_eq!(sink.state.players.p2.hero.health, HERO_HEALTH - 4);

        // A Reborn unit returns reset, at 1 health, without Reborn.
        let rb = put(sink.state, REBORN_UNIT, slot(P2, UNITS, 1), json!({}));
        edit(sink.state, &rb, |card| {
            card.buffs = AttackHealth { attack: 3, health: 0 };
            card.damage = 99;
        });
        state_check(&mut sink);
        let rb_now = instance_in(sink.state, &rb.id);
        assert_eq!(unit_view(sink.state, &rb_now).health, 1);
        assert!(!unit_has(sink.state, &rb_now, KeywordKind::Reborn));
        assert_eq!(rb_now.buffs, AttackHealth { attack: 0, health: 0 });
    }

    #[test]
    fn r79_answers_only_the_timed_out_player_s_prompt_loses_on_a_disconnect_draws_at_the_ceiling_and_leaves_the_clocks_to_the_server(
    ) {
        // A prompt held by the non-active player: their own clock answers it and the turn stays open.
        let mut prompted = playing("r79-prompt");
        {
            let mut sink = sink_for(&mut prompted);
            open_prompt(
                &mut sink,
                OpenPromptArgs {
                    player: P2,
                    kind: PromptKind::Mode,
                    aim: None,
                    prompt: "pick one".into(),
                    options: vec![json_as(json!({
                        "key": "mode:a",
                        "label": "a",
                        "selection": { "pick": "mode", "option": "a" },
                    }))],
                    min: None,
                    max: None,
                    budget: None,
                    owner: None,
                    resume: json_as(json!({ "defId": "", "hook": "resume", "step": "none", "radiant": false, "data": {} })),
                },
            );
        }
        let turn = prompted.turn;
        let answered = act_result(&prompted, json!({ "type": "timeout", "playerId": "p2" }));
        assert_eq!(answered.error, None);
        assert!(answered.state.pending.is_none());
        assert_eq!(answered.state.active, P1);
        assert_eq!(answered.state.turn, turn);

        // The active player's clock ends the turn instead.
        let clean = playing("r79-clean");
        let timed_out = act_result(&clean, json!({ "type": "timeout", "playerId": "p1" }));
        assert_eq!(timed_out.error, None);
        assert_eq!(timed_out.state.active, P2);

        // A disconnect is a loss; the hard ceiling is a draw.
        assert_eq!(
            act_result(&clean, json!({ "type": "disconnectExpired", "player": "p1", "playerId": "p1" }))
                .state
                .result,
            Some(GameResult {
                winner: Winner::P2,
                reason: GameOverReason::Disconnect,
            })
        );
        assert_eq!(
            act_result(&clean, json!({ "type": "ceilingReached", "playerId": "p1" })).state.result,
            Some(GameResult {
                winner: Winner::Draw,
                reason: GameOverReason::MatchCeiling,
            })
        );

        // The engine carries the clock the server runs, and none of R79's numbers.
        assert_eq!(view_for_with_clock(&clean, P1, 75_000).clock_ms, Some(75_000));
        assert_eq!(view_for(&clean, P1).clock_ms, None);
        // TS also asserted that `config.ts` exports none of the server's constant names (its
        // `Object.keys(engineConfig)` over SERVER_CONSTANTS); Rust has no run-time list of a module's
        // names, so that one assertion is in `.fullsend/notes/spec-gaps-part-25-5.md`.
        // M6/M7: apps/server/src/config.ts carries the clock, grace, ceiling, room-code and rating values.
    }

    #[test]
    fn r80_caps_a_library_at_library_cap_a_new_card_is_never_created_and_an_existing_one_lands_in_the_graveyard() {
        let mut state = game("r80");
        let library: Vec<CardInstance> = (0..LIBRARY_CAP)
            .map(|_| new_instance(&mut state, PLAIN, P1, Zone::Library { player: P1 }))
            .collect();
        state.players.p1.library = library;
        let mut sink = sink_for(&mut state);

        let mut fresh = new_instance(&mut *sink.state, PLAIN, P1, Zone::Resolving { player: P1 });
        assert_eq!(
            draw::shuffle_into_library(&mut sink, &mut fresh, false, None),
            ShuffleInOutcome::Dropped
        );
        assert_eq!(sink.state.players.p1.library.len(), LIBRARY_CAP as usize);
        assert!(of_type(sink.events, GameEventType::ShuffledIn).is_empty());

        let mut existing = put(sink.state, PLAIN, slot(P1, UNITS, 1), json!({}));
        assert_eq!(
            draw::shuffle_into_library(&mut sink, &mut existing, true, None),
            ShuffleInOutcome::Dropped
        );
        assert_eq!(ids(&sink.state.players.p1.graveyard), vec![existing.id.clone()]);

        // A unit-token card ceases to exist instead of reaching a graveyard (R11).
        let mut token = put(sink.state, UNIT_TOKEN, slot(P1, UNITS, 2), json!({}));
        assert_eq!(
            draw::shuffle_into_library(&mut sink, &mut token, true, None),
            ShuffleInOutcome::Dropped
        );
        assert_eq!(ids(&sink.state.players.p1.graveyard), vec![existing.id.clone()]);
        assert!(!sink.state.players.p1.exile.iter().any(|card| card.id == token.id));
    }

    #[test]
    fn r81_carries_zone_x_targets_and_modes_in_the_play_action_and_opens_a_pendingchoice_only_during_resolution() {
        let mut state = playing("r81");
        state.players.p1.hand = vec![];
        let unit = hand_card(&mut state, PLAIN, P1);
        let x = hand_card(&mut state, X_CARD, P1);

        let plays: Vec<ActionBody> = legal_actions(&state, P1)
            .into_iter()
            .filter(|action| action.action_type() == ActionType::Play)
            .collect();
        assert_eq!(
            plays.iter().filter(|action| plays_card(action, &unit.id)).count(),
            UNIT_ZONES as usize
        );
        // R348: X from 1 up to current mana (1 here).
        assert_eq!(
            plays
                .iter()
                .filter_map(|action| match action {
                    ActionBody::Play { instance_id, x: chosen, .. } if *instance_id == x.id => Some(*chosen),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            vec![Some(1)]
        );

        // A declared target travels in `targets` and a declared direction in `modes`: nothing pauses.
        let enemy = put(&mut state, BIG_BODY, slot(P2, UNITS, 1), json!({}));
        let spell = hand_card(&mut state, MODE_SPELL, P1);
        let resolved = act_result(
            &state,
            json!({
                "type": "play",
                "instanceId": spell.id,
                "playerId": "p1",
                "targets": [{ "pick": "instance", "instanceId": enemy.id }],
                "modes": ["left"],
            }),
        );
        assert_eq!(resolved.error, None);
        assert!(resolved.state.pending.is_none());
        assert_eq!(
            pluck(&of_type(&resolved.events, GameEventType::Damage), "amount"),
            vec![json!(1)]
        );
        assert_eq!(
            to_json(&scripts_for(&state, MODE_SPELL).base.targets),
            json!([{ "kind": "target", "min": 1, "max": 1 }])
        );
        assert_eq!(
            to_json(&scripts_for(&state, MODE_SPELL).base.modes),
            json!([{ "kind": "direction", "options": ["left", "right"] }])
        );

        // A choice made during resolution opens a PendingChoice instead.
        let discover = hand_card(&mut state, DISCOVER_SPELL, P1);
        let paused = act_result(&state, json!({ "type": "play", "instanceId": discover.id, "playerId": "p1" }));
        assert_eq!(paused.error, None);
        assert_eq!(paused.state.pending.map(|prompt| prompt.kind), Some(PromptKind::Discover));
    }

    #[test]
    fn r82_ends_the_turn_by_itself_when_only_ending_conceding_and_offering_a_draw_are_left() {
        let mut state = playing("r82");
        state.players.p1.hand = vec![];
        let card = hand_card(&mut state, NOOP, P1);
        assert!(legal_actions(&state, P1).iter().any(|action| action.action_type() == ActionType::OfferDraw));

        let result = act_result(&state, json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }));
        assert_eq!(result.error, None);
        // The draw offer was still on the table and did not hold the turn open (R36).
        assert_eq!(of_type(&result.events, GameEventType::TurnAutoEnded).len(), 1);
        assert_eq!(result.state.active, P2);
    }

    #[test]
    fn r83_brings_a_reborn_unit_back_summoning_sick_so_it_cannot_attack_twice_but_it_may_still_switch() {
        let mut state = playing("r83");
        let rb = put(&mut state, REBORN_UNIT, slot(P1, UNITS, 1), json!({})); // 2/2 Reborn
        let wall = put(&mut state, BIG_BODY, slot(P2, UNITS, 1), json!({})); // 5/5

        let mut sink = sink_for(&mut state);
        assert_eq!(refusal(declare_attack(&mut sink, &rb, &unit_target(&wall))), None);

        // It died in that combat and came back in its reserved zone, at 1 health.
        assert_eq!(id_at(sink.state, slot(P1, UNITS, 1)), Some(rb.id.clone()));
        let back = instance_in(sink.state, &rb.id);
        assert_eq!(unit_view(sink.state, &back).health, 1);
        assert!(!unit_has(sink.state, &back, KeywordKind::Reborn));

        // It entered the field again this turn, so it is sick and cannot take a second attack.
        assert_eq!(back.summoned_turn, Some(sink.state.turn));
        let wall_now = instance_in(sink.state, &wall.id);
        assert_eq!(
            refusal(why_cannot_attack(sink.state, &back, &unit_target(&wall_now))),
            says("that unit is summoning sick")
        );

        // R78 cleared its exertion, so it may switch position, as any unit summoned this turn may.
        assert!(has_exertion(sink.state, &back, ExertionKind::Switch));
        assert_eq!(refusal(switch_position(&mut sink, &back, Default::default())), None);
        assert_eq!(instance_in(sink.state, &rb.id).position, Some(Position::Def));
    }

    #[test]
    fn r84_keeps_concede_offerdraw_and_answerdraw_out_of_the_policy_so_a_random_game_ends_by_death_or_the_cap() {
        let state = playing("r84");
        assert_eq!(
            AI_SKIPPED_ACTIONS.to_vec(),
            vec![ActionType::Concede, ActionType::OfferDraw, ActionType::AnswerDraw]
        );

        let offered: Vec<ActionType> = legal_actions(&state, P1).iter().map(ActionBody::action_type).collect();
        assert!(offered.contains(&ActionType::Concede));
        assert!(offered.contains(&ActionType::OfferDraw));
        let policy: Vec<ActionType> = policy_actions(&state, P1, PolicyOptions::default())
            .iter()
            .map(ActionBody::action_type)
            .collect();
        for skipped in AI_SKIPPED_ACTIONS.iter() {
            assert!(!policy.contains(skipped));
        }
        assert_eq!(
            policy_actions(&state, P1, PolicyOptions { skip: Some(vec![]) })
                .iter()
                .map(ActionBody::action_type)
                .collect::<Vec<_>>(),
            offered
        );

        // The random-game harness filters the same set, so a fuzz game never resigns itself.
        let finished = play_random_game("rb-r84", None).state;
        let reason = finished.result.as_ref().map(|result| result.reason);
        assert!([GameOverReason::HeroDeath, GameOverReason::BothHeroesDead, GameOverReason::TurnCap]
            .iter()
            .any(|ended| reason == Some(*ended)));
    }
}
