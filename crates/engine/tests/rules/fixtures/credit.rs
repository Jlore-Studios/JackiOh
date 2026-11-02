//! Fixture cards for the credit line and the Untributable keyword (R1220–R1225, Meditative
//! #89 Jlarna's systems). Ids are `cr-*`, indices from 8200 so they cannot collide (BUILD §0).

use jackioh_engine::testkit::*;

use super::harness::{both, new_game, spread};

pub use super::harness::{in_hand, put, slot};

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
