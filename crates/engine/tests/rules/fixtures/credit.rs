//! Fixture cards for the credit line and the Untributable keyword (R1220–R1225, Meditative
//! #89 Jlarna's systems). Ids are `cr-*`, indices from 8200 so they cannot collide (BUILD §0).
//!
//! The fixtures reproduce the shapes the real cards use, through JSON defs and the effects
//! library: a lending Field Spell (`creditLine` 4, `creditInstalments` 4, with and without the
//! lapsing face), a (3) vanilla Unit, a Field Spell with a mana-2 Activate, an Untributable Unit
//! (with a "Tribute this" activation), an Untributable Field Spell whose `tributeWhen` always
//! holds, and a Unit with a Tribute 1 play cost.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use super::harness::new_game;

pub use super::harness::{in_hand, put, slot};

/// `{ ...defaults, ...extra }`: the keys of `extra` replace the defaults' (TS's object spread).
fn spread(mut base: Value, extra: Value) -> Value {
    if let (Some(fields), Value::Object(over)) = (base.as_object_mut(), extra) {
        fields.extend(over);
    }
    base
}

/// TS `def(name, type, extra)`; indices run from 8200 in declaration order.
fn def(name: &str, type_: &str, index: i32, extra: Value) -> CardDef {
    json_as(spread(
        json!({
            "id": format!("cr-{name}"),
            "index": index.to_string(),
            "name": format!("{name} (credit fixture)"),
            "set": "Core",
            "type": type_,
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 0,
            "base": { "keywords": [], "text": name },
            "radiant": { "keywords": [], "text": name },
        }),
        extra,
    ))
}

fn unit(name: &str, attack: i32, health: i32, index: i32, extra: Value) -> CardDef {
    def(
        name,
        "Unit",
        index,
        spread(
            json!({
                "base": { "attack": attack, "health": health, "keywords": [], "text": name },
                "radiant": { "attack": attack * 2, "health": health * 2, "keywords": [], "text": name },
            }),
            extra,
        ),
    )
}

/// A lending Field Spell: `creditLine` 4, `creditInstalments` 4, and the lapsing face (Jlarna's
/// base face shape).
pub fn lender() -> CardDef {
    def("lender", "Field Spell", 8200, json!({}))
}

/// The same lender without `creditLapses` (Jlarna's Radiant face shape).
pub fn lender_open() -> CardDef {
    def("lender-open", "Field Spell", 8201, json!({}))
}

/// A (3) vanilla Unit: the body plays borrow against.
pub fn body() -> CardDef {
    unit("body", 2, 2, 8202, json!({ "cost": 3 }))
}

/// A Field Spell with a mana-2 Activate, whose cost may be borrowed (R1223).
pub fn tapper() -> CardDef {
    def("tapper", "Field Spell", 8203, json!({}))
}

/// An Untributable Unit, with a "Tribute this" activation that can never be used (R1220).
pub fn safe() -> CardDef {
    unit(
        "safe",
        2,
        2,
        8204,
        json!({
            "cost": 1,
            "base": { "attack": 2, "health": 2, "keywords": [{ "kind": "Untributable" }], "text": "Untributable" },
            "radiant": { "attack": 4, "health": 4, "keywords": [{ "kind": "Untributable" }], "text": "Untributable" },
        }),
    )
}

/// An Untributable Field Spell whose `tributeWhen` always holds: it must never fire (R1220).
pub fn safe_field() -> CardDef {
    def(
        "safe-field",
        "Field Spell",
        8205,
        json!({
            "base": { "keywords": [{ "kind": "Untributable" }], "text": "Untributable" },
            "radiant": { "keywords": [{ "kind": "Untributable" }], "text": "Untributable" },
        }),
    )
}

/// A Unit with a Tribute 1 play cost: the play a Tribute test pays with.
pub fn tribute_one() -> CardDef {
    unit("tribute-one", 2, 2, 8206, json!({}))
}

fn flags(value: Value) -> Option<StaticFlags> {
    Some(json_as(value))
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(
        lender().id,
        both(Script {
            static_flags: flags(json!({
                "creditLine": 4,
                "creditInstalments": 4,
                "creditLapses": true,
            })),
            ..Script::default()
        }),
    );
    scripts.insert(
        lender_open().id,
        both(Script {
            static_flags: flags(json!({ "creditLine": 4, "creditInstalments": 4 })),
            ..Script::default()
        }),
    );
    scripts.insert(
        tapper().id,
        both(Script {
            activations: vec![ActivationDecl {
                id: "tap".to_string(),
                label: "Pay (2)".to_string(),
                uses: ActivationUses::Count(1),
                cost: Some(ActivationCost {
                    mana: Some(2),
                    ..ActivationCost::default()
                }),
                targets: vec![],
                modes: vec![],
                can_activate: None,
                has: None,
                run: hook(|_ctx| vec![]),
            }],
            ..Script::default()
        }),
    );
    scripts.insert(
        safe().id,
        both(Script {
            activations: vec![ActivationDecl {
                id: "tribute".to_string(),
                label: "Tribute this".to_string(),
                uses: ActivationUses::Count(1),
                cost: Some(ActivationCost {
                    tribute_self: Some(true),
                    ..ActivationCost::default()
                }),
                targets: vec![],
                modes: vec![],
                can_activate: None,
                has: None,
                run: hook(|_ctx| vec![]),
            }],
            ..Script::default()
        }),
    );
    scripts.insert(
        safe_field().id,
        both(Script {
            tribute_when: Some(read_hook(|_| true)),
            ..Script::default()
        }),
    );
    scripts.insert(
        tribute_one().id,
        both(Script {
            static_flags: flags(json!({ "tribute": 1 })),
            ..Script::default()
        }),
    );
    scripts
}

fn defs() -> Vec<CardDef> {
    vec![
        lender(),
        lender_open(),
        body(),
        tapper(),
        safe(),
        safe_field(),
        tribute_one(),
    ]
}

/// This file's fixtures registered on top of the shared fixture catalog.
pub fn register() {
    let mut catalog = registered_catalog().clone();
    for card in defs() {
        catalog.insert(card.id.clone(), card);
    }
    register_catalog(catalog);
    let mut all = registered_scripts().clone();
    all.extend(scripts());
    register_scripts(all);
}

/// Past setup, in p1's main phase, with this file's fixtures registered and 4 mana.
pub fn playing(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    state.phase = Phase::Main;
    register();
    state.players.p1.mana = ManaState {
        current: 4,
        max: 4,
        next_turn_mod: 0,
        perm_mod: 0,
    };
    state
}

static NONCE: AtomicU32 = AtomicU32::new(0);

/// `reduce` on a hand-built action, panicking on refusal (TS's `act`).
pub fn act(state: &GameState, body: Value) -> GameState {
    let result = act_result(state, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

pub fn act_result(state: &GameState, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let input: ActionInput = json_as(body);
    reduce(state, &input.with_nonce(format!("cr{nonce}")))
}

/// The card as it stands in the state now (TS held the live object).
pub fn live(state: &GameState, card: &CardInstance) -> CardInstance {
    find_instance(state, &card.id)
        .cloned()
        .expect("the card is in the state")
}
