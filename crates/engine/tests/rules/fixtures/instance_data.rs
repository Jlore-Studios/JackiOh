//! Fixture cards for patch v0.2.0's instance data (docs/classic-sets.md B2.7, B3.3, B3.4, E38, E39):
//! Brittle, Degrade and Upgrade, declared numbers, KY's Constant, faces with their own type and X
//! stats, enchantments. Each reproduces the shape of the Classic or Classic+ card named in its doc
//! comment through the effects library (BUILD §0); the real cards and their own tests come with the
//! card workstreams. `instanceGame` registers them on top of the shared fixture catalog.
//!
//! Port of `packages/engine/test/fixtures/instanceData.ts`. Each exported def is a `pub static` under
//! TS's name snake_cased; TS's module counter (`nextIndex`, from 4400, one per `def` call in file
//! order) is each def's stated index.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects::{
    buff_cards, choose_mode, choose_target, chosen_tuning_number, damage, degrade, discover_number, draw,
    enchant, gain_brittle, give_brittle, grant_keyword_cards, set_number, upgrade,
};
use jackioh_engine::testkit::*;

use super::catalog::vanilla_deck;
use super::harness::setup_catalog;

/// TS `def(name, overrides = {})`: a Core Common 2/2 Unit at cost 1 (4/4 Radiant) whose faces print
/// `name`, `overrides` over it.
fn def(name: &str, index: i32, overrides: Value) -> CardDef {
    let mut card = json!({
        "id": format!("id-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (instance-data fixture)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 2, "health": 2, "keywords": [], "text": name },
        "radiant": { "attack": 4, "health": 4, "keywords": [], "text": name },
    });
    if let (Some(target), Value::Object(overrides)) = (card.as_object_mut(), overrides) {
        for (key, value) in overrides {
            target.insert(key, value);
        }
    }
    json_as(card)
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// Classic+ #74's printed Brittle on a Unit: Brittle 2, Radiant Brittle 4.
pub static brittle_unit: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "brittle-unit",
        4401,
        json!({
            "base": { "attack": 2, "health": 3, "keywords": [{ "kind": "Brittle", "n": 2 }], "text": "Brittle 2" },
            "radiant": { "attack": 4, "health": 6, "keywords": [{ "kind": "Brittle", "n": 4 }], "text": "Brittle 4" },
        }),
    )
});

/// Classic+ #74 Twice Forward One Step Backwards' shape: a Field Trap printing Brittle 3 (it never fires here).
pub static brittle_trap: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "brittle-trap",
        4402,
        json!({
            "type": "Field Trap",
            "cost": 2,
            "base": { "keywords": [{ "kind": "Brittle", "n": 3 }], "text": "Brittle 3" },
            "radiant": { "keywords": [{ "kind": "Brittle", "n": 6 }], "text": "Brittle 6" },
        }),
    )
});

/// A plain 3/4 with no keywords, cost 2: every menu row but the keyword add has room on it.
pub static body: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "body",
        4403,
        json!({
            "cost": 2,
            "base": { "attack": 3, "health": 4, "keywords": [], "text": "3/4" },
            "radiant": { "attack": 6, "health": 8, "keywords": [], "text": "6/8" },
        }),
    )
});

/// A Unit whose Death asks its controller something, so a crumble's death pauses the settle after it (R113).
pub static asker: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "asker",
        4404,
        json!({
            "base": { "attack": 1, "health": 1, "keywords": [], "text": "Death: choose" },
            "radiant": { "attack": 2, "health": 2, "keywords": [], "text": "Death: choose" },
        }),
    )
});

/// Numbered keywords that print as `Keyword`s (B3.4 rule 3's X row): Armor 2, Lucky 1, Spell Damage 1, Taunt.
pub static numbered_body: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "numbered",
        4405,
        json!({
            "base": {
                "attack": 2,
                "health": 2,
                "keywords": [{ "kind": "Armor", "n": 2 }, { "kind": "Lucky", "n": 1 }, { "kind": "Spell Damage", "n": 1 }, { "kind": "Taunt" }],
                "text": "Armor 2, Lucky 1, Spell Damage +1, Taunt",
            },
            "radiant": {
                "attack": 4,
                "health": 4,
                "keywords": [{ "kind": "Armor", "n": 4 }, { "kind": "Lucky", "n": 2 }, { "kind": "Spell Damage", "n": 2 }, { "kind": "Taunt" }],
                "text": "Armor 4, Lucky 2, Spell Damage +2, Taunt",
            },
        }),
    )
});

/// Declared numbers (B3.4 rule 5): `damage` 2 → 4 (more is better), `threshold` 3 (less is better, never
/// below 2), `big` 8 → 16 (default step 2, then 4), `huge` 20 → 40 (default step a quarter: 5, then 10;
/// never above 22 on the base face). Cry: deal {damage} to the enemy hero.
pub static numbered: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "params",
        4406,
        json!({
            "type": "Spell",
            "cost": 2,
            "params": [
                { "key": "damage", "base": 2, "radiant": 4, "better": "up" },
                { "key": "threshold", "base": 3, "radiant": 3, "better": "down", "min": 2 },
                { "key": "big", "base": 8, "radiant": 16, "better": "up" },
                { "key": "huge", "base": 20, "radiant": 40, "better": "up", "max": 44 },
            ],
            "base": { "keywords": [], "text": "Deal {damage} damage. {threshold} {big} {huge}" },
            "radiant": { "keywords": [], "text": "Deal {damage} damage. {threshold} {big} {huge}" },
        }),
    )
});

/// Classic+ #69 Buff Billy's shape: (X) Unit, "[3X/3X]", Radiant "[7X/7X]"; Cry: Upgrade this X (2X) times.
pub static billy: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "billy",
        4407,
        json!({
            "cost": "X",
            "base": { "attack": 0, "health": 0, "xStats": { "attack": 3, "health": 3 }, "keywords": [], "text": "This is a 3X/3X." },
            "radiant": { "attack": 0, "health": 0, "xStats": { "attack": 7, "health": 7 }, "keywords": [], "text": "This is a 7X/7X." },
        }),
    )
});

/// An X-cost Spell dealing X to the enemy hero, so its tuned X shows at resolution (B3.4 rule 3).
pub static x_bolt: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "x-bolt",
        4408,
        json!({
            "type": "Spell",
            "cost": "X",
            "base": { "keywords": [], "text": "Deal X." },
            "radiant": { "keywords": [], "text": "Deal X." },
        }),
    )
});

/// A Spell with Echo 1 (§6.1), dealing 1 to the enemy hero each time it resolves.
pub static echo_bolt: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "echo-bolt",
        4409,
        json!({
            "type": "Spell",
            "cost": 1,
            "base": { "keywords": [], "text": "Echo 1. Deal 1." },
            "radiant": { "keywords": [], "text": "Echo 1. Deal 1." },
        }),
    )
});

/// A Unit with "Tribute 2" (§6.3) as a static flag.
pub static tributer: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "tributer",
        4410,
        json!({
            "cost": 3,
            "base": { "attack": 5, "health": 5, "keywords": [], "text": "Tribute 2" },
            "radiant": { "attack": 10, "health": 10, "keywords": [], "text": "Tribute 2" },
        }),
    )
});

/// A Unit with "Activate 2: deal 1 damage" (B3.2).
pub static activator: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "activator",
        4411,
        json!({
            "base": { "attack": 1, "health": 1, "keywords": [], "text": "Activate 2" },
            "radiant": { "attack": 2, "health": 2, "keywords": [], "text": "Activate 2" },
        }),
    )
});

/// Classic+ #22 Blood Moon's shape: a Trap whose Radiant face is a Field Trap (B2.7).
pub static blood_moon: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "blood-moon",
        4412,
        json!({
            "type": "Trap",
            "base": { "keywords": [], "text": "Trap" },
            "radiant": { "type": "Field Trap", "keywords": [], "text": "Field Trap" },
        }),
    )
});

/// Classic #5 Tesla's shape: a Field Trap printing Animated and a 1/4 unit face (B3.1).
pub static tesla: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "tesla",
        4413,
        json!({
            "type": "Field Trap",
            "base": { "attack": 1, "health": 4, "keywords": [{ "kind": "Animated" }], "text": "Animated" },
            "radiant": { "attack": 2, "health": 8, "keywords": [{ "kind": "Animated" }], "text": "Animated" },
        }),
    )
});

/// An Immutable 3/3 (§6.1, R23): Degrade and Upgrade never change it (B3.4 rule 2).
pub static stoic: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "stoic",
        4414,
        json!({
            "base": { "attack": 3, "health": 3, "keywords": [{ "kind": "Immutable" }], "text": "Immutable" },
            "radiant": { "attack": 6, "health": 6, "keywords": [{ "kind": "Immutable" }], "text": "Immutable" },
        }),
    )
});

/// Cost (0) and cost (4) bodies for the cost row's two bounds (B3.4 rule 3).
pub static free_body: LazyLock<CardDef> = LazyLock::new(|| def("free", 4415, json!({ "cost": 0 })));
pub static dear_body: LazyLock<CardDef> = LazyLock::new(|| def("dear", 4416, json!({ "cost": 4 })));

/// A Unit printing "Can't attack" and Rush: a Degrade may take Rush and never "Can't attack".
pub static shackled: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "shackled",
        4417,
        json!({
            "base": { "attack": 2, "health": 2, "keywords": [{ "kind": "Can't attack" }, { "kind": "Rush" }], "text": "Can't attack, Rush" },
            "radiant": { "attack": 4, "health": 4, "keywords": [{ "kind": "Can't attack" }, { "kind": "Rush" }], "text": "Can't attack, Rush" },
        }),
    )
});

/// Classic+ #72 Book of Nerf's shape: choose a unit, then Degrade it {times} times — a prompt, then the change.
pub static nerfer: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "nerfer",
        4418,
        json!({
            "type": "Spell",
            "params": [{ "key": "times", "base": 3, "radiant": 5, "better": "up" }],
            "base": { "keywords": [], "text": "Choose a unit. Degrade it {times} times." },
            "radiant": { "keywords": [], "text": "Choose a unit. Degrade it {times} times." },
        }),
    )
});

/// Classic+ #41 KY's Constant's shape: a hand card, then a random number on it (Radiant: Discover one) to 3.
pub static constant: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "constant",
        4419,
        json!({
            "type": "Spell",
            "base": { "keywords": [], "text": "Change a random number to 3." },
            "radiant": { "keywords": [], "text": "Discover a number and change it to 3." },
        }),
    )
});

/// Classic+ #8 Withering Storm's shape: Degrade 4 random cards in the opponent's deck (Radiant: every one). Draw 1.
pub static withering: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "withering",
        4420,
        json!({
            "type": "Spell",
            "cost": 2,
            "params": [{ "key": "cards", "base": 4, "radiant": 4, "better": "up" }],
            "base": { "keywords": [], "text": "Degrade {cards} random cards in your opponent's deck. Draw 1." },
            "radiant": { "keywords": [], "text": "Degrade every card in your opponent's deck. Draw 1." },
        }),
    )
});

/// Classic+ #70 Chaos Machine's shape: at your start and end of turn, Upgrade one of yours and Degrade one of theirs.
pub static chaos_machine: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "chaos-machine",
        4421,
        json!({
            "type": "Field Spell",
            "cost": 2,
            "base": { "keywords": [], "text": "Chaos" },
            "radiant": { "keywords": [], "text": "Chaos" },
        }),
    )
});

/// Classic+ #23 Dropshipping's shape, cut down: give every card in your hand Brittle {brittle}.
pub static dropship: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "dropship",
        4422,
        json!({
            "type": "Spell",
            "params": [{ "key": "brittle", "base": 2, "radiant": 2, "better": "up" }],
            "base": { "keywords": [], "text": "Give each card in your hand Brittle {brittle}." },
            "radiant": { "keywords": [], "text": "Give each card in your hand Brittle {brittle}." },
        }),
    )
});

/// Classic+ #40 Appropriations' Military: your Units on the field, in hand and in deck get +2 Attack and Rush.
pub static military: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "military",
        4423,
        json!({
            "type": "Spell",
            "cost": 2,
            "base": { "keywords": [], "text": "Military" },
            "radiant": { "keywords": [], "text": "Military" },
        }),
    )
});

/// Classic+ #40's Education: every card in your hand gains Cast on draw (an enchantment).
pub static educator: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "educator",
        4424,
        json!({
            "type": "Spell",
            "base": { "keywords": [], "text": "Education" },
            "radiant": { "keywords": [], "text": "Education" },
        }),
    )
});

/// R749: Classic #46 Divine Favor's and Classic #54 Rewind's shape: a number only the Radiant face prints
/// (`times` 1 → 2, more is better), tuned on that face only.
pub static radiant_number: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "radiant-number",
        4425,
        json!({
            "type": "Spell",
            "params": [{ "key": "times", "base": 1, "radiant": 2, "better": "up", "tunedOn": "radiant" }],
            "base": { "keywords": [], "text": "Do it." },
            "radiant": { "keywords": [], "text": "Do it {times|time|times}." },
        }),
    )
});

/// R1431: the mirror, Classic #11 Mind Melt's shape: a number only the base face prints (`cards` 2,
/// more is better), tuned on that face only.
pub static base_number: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "base-number",
        4426,
        json!({
            "type": "Spell",
            "params": [{ "key": "cards", "base": 2, "radiant": 2, "better": "up", "tunedOn": "base" }],
            "base": { "keywords": [], "text": "Exile {cards|card|cards}." },
            "radiant": { "keywords": [], "text": "Exile them all." },
        }),
    )
});

pub static INSTANCE_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        brittle_unit.clone(),
        brittle_trap.clone(),
        body.clone(),
        asker.clone(),
        numbered_body.clone(),
        numbered.clone(),
        billy.clone(),
        x_bolt.clone(),
        echo_bolt.clone(),
        tributer.clone(),
        activator.clone(),
        blood_moon.clone(),
        tesla.clone(),
        stoic.clone(),
        free_body.clone(),
        dear_body.clone(),
        shackled.clone(),
        nerfer.clone(),
        constant.clone(),
        withering.clone(),
        chaos_machine.clone(),
        dropship.clone(),
        military.clone(),
        educator.clone(),
        radiant_number.clone(),
        base_number.clone(),
    ]
});

/// The step `asker`'s Death opens and the answer re-enters.
pub const ASKER_STEP: &str = "asked";
/// The step `nerfer`'s target prompt re-enters.
pub const NERF_STEP: &str = "nerf";
/// The step `constant`'s Radiant Discover re-enters.
pub const CONSTANT_STEP: &str = "picked";

fn to_enemy_hero(amount: i32) -> Effect {
    damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))
}

fn constant_targets() -> Vec<TargetDecl> {
    vec![TargetDecl::hand(1, 1, json!({ "of": ["hand"] }))]
}

fn chaos_turn() -> Hook {
    hook(|_ctx| {
        vec![
            upgrade(json_as(
                json!({ "scope": { "side": "self", "zones": ["hand", "field"] }, "random": 1 }),
            )),
            degrade(json_as(
                json!({ "scope": { "side": "enemy", "zones": ["hand", "field"] }, "random": 1 }),
            )),
        ]
    })
}

pub static INSTANCE_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            asker.id.clone(),
            both(Script {
                death: Some(hook(|_ctx| {
                    vec![choose_mode(json_as(
                        json!({ "options": ["one", "two"], "step": ASKER_STEP }),
                    ))]
                })),
                resume: IndexMap::from([(
                    ASKER_STEP,
                    hook(|ctx| {
                        let two = matches!(ctx.targets.first(), Some(Selection::Mode { option }) if option == "two");
                        vec![to_enemy_hero(if two { 2 } else { 1 })]
                    }),
                )]),
                ..Script::default()
            }),
        ),
        (
            numbered.id.clone(),
            both(Script {
                cry: Some(hook(|ctx| vec![to_enemy_hero(param(&*ctx, "damage"))])),
                ..Script::default()
            }),
        ),
        (
            billy.id.clone(),
            CardScripts {
                base: Script {
                    cry: Some(hook(|ctx| {
                        vec![upgrade(json_as(
                            json!({ "target": { "of": "self" }, "times": ctx.x }),
                        ))]
                    })),
                    ..Script::default()
                },
                radiant: Script {
                    cry: Some(hook(|ctx| {
                        vec![upgrade(json_as(
                            json!({ "target": { "of": "self" }, "times": 2 * ctx.x }),
                        ))]
                    })),
                    ..Script::default()
                },
            },
        ),
        (
            x_bolt.id.clone(),
            both(Script {
                cry: Some(hook(|ctx| vec![to_enemy_hero(ctx.x)])),
                ..Script::default()
            }),
        ),
        (
            echo_bolt.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    echo: Some(1),
                    ..StaticFlags::default()
                }),
                cry: Some(hook(|_ctx| vec![to_enemy_hero(1)])),
                ..Script::default()
            }),
        ),
        (
            tributer.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    tribute: Some(2),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            }),
        ),
        (
            activator.id.clone(),
            both(Script {
                activations: vec![ActivationDecl {
                    id: "ping".to_string(),
                    label: "Deal 1 damage".to_string(),
                    uses: ActivationUses::Count(2),
                    cost: None,
                    targets: vec![],
                    modes: vec![],
                    can_activate: None,
                    has: None,
                    run: hook(|_ctx| vec![to_enemy_hero(1)]),
                }],
                ..Script::default()
            }),
        ),
        (
            nerfer.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![choose_target(json_as(
                        json!({ "step": NERF_STEP, "scope": { "side": "any" } }),
                    ))]
                })),
                resume: IndexMap::from([(
                    NERF_STEP,
                    hook(|ctx| {
                        let times = param(&*ctx, "times");
                        vec![degrade(json_as(
                            json!({ "target": { "of": "chosen" }, "times": times }),
                        ))]
                    }),
                )]),
                ..Script::default()
            }),
        ),
        (
            constant.id.clone(),
            CardScripts {
                base: Script {
                    targets: constant_targets(),
                    cry: Some(hook(|_ctx| {
                        vec![set_number(json_as(
                            json!({ "target": { "of": "chosen" }, "which": "random", "value": 3 }),
                        ))]
                    })),
                    ..Script::default()
                },
                radiant: Script {
                    targets: constant_targets(),
                    cry: Some(hook(|_ctx| {
                        vec![discover_number(json_as(json!({
                            "target": { "of": "chosen" },
                            "value": 3,
                            "count": 3,
                            "step": CONSTANT_STEP,
                        })))]
                    })),
                    resume: IndexMap::from([(
                        CONSTANT_STEP,
                        hook(|ctx| match chosen_tuning_number(&*ctx) {
                            None => vec![],
                            Some(chosen) => vec![set_number(json_as(json!({
                                "instanceId": chosen.instance_id,
                                "which": chosen.which,
                                "value": 3,
                            })))],
                        }),
                    )]),
                    ..Script::default()
                },
            },
        ),
        (
            withering.id.clone(),
            CardScripts {
                base: Script {
                    cry: Some(hook(|ctx| {
                        let cards = param(&*ctx, "cards");
                        vec![
                            degrade(json_as(
                                json!({ "scope": { "side": "enemy", "zones": ["library"] }, "random": cards }),
                            )),
                            draw(json_as(json!({ "count": 1 }))),
                        ]
                    })),
                    ..Script::default()
                },
                radiant: Script {
                    cry: Some(hook(|_ctx| {
                        vec![
                            degrade(json_as(
                                json!({ "scope": { "side": "enemy", "zones": ["library"] } }),
                            )),
                            draw(json_as(json!({ "count": 1 }))),
                        ]
                    })),
                    ..Script::default()
                },
            },
        ),
        (
            chaos_machine.id.clone(),
            both(Script {
                start_of_turn: Some(chaos_turn()),
                end_of_turn: Some(chaos_turn()),
                ..Script::default()
            }),
        ),
        (
            dropship.id.clone(),
            both(Script {
                cry: Some(hook(|ctx| {
                    let n = param(&*ctx, "brittle");
                    vec![
                        give_brittle(json_as(json!({ "scope": { "zones": ["hand"] }, "n": n }))),
                        gain_brittle(json_as(json!({ "scope": { "zones": ["field"] }, "n": 1 }))),
                    ]
                })),
                ..Script::default()
            }),
        ),
        (
            military.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![
                        buff_cards(json_as(json!({
                            "scope": { "zones": ["field", "hand", "library"], "types": ["Unit"] },
                            "attack": 2,
                        }))),
                        grant_keyword_cards(json_as(json!({
                            "scope": { "zones": ["field", "hand", "library"], "types": ["Unit"] },
                            "keyword": { "kind": "Rush" },
                        }))),
                    ]
                })),
                ..Script::default()
            }),
        ),
        (
            educator.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![enchant(json_as(
                        json!({ "scope": { "zones": ["hand"] }, "enchantment": { "kind": "castOnDraw" } }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
    ])
});

/// The fixture catalog with these cards, registered on top of the shared one; returns the whole catalog.
pub fn register_instance_fixtures() -> CardDefs {
    setup_catalog();
    let mut defs: CardDefs = registered_catalog().clone();
    for card in INSTANCE_DEFS.iter() {
        defs.insert(card.id.clone(), card.clone());
    }
    register_catalog(defs.clone());
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(INSTANCE_SCRIPTS.clone());
    register_scripts(merged);
    defs
}

/// A game in setup with these fixtures registered; decks default to the vanilla fixture decks. TS
/// `instanceGame(seed = "instance-data", decks?)`.
pub fn instance_game(seed: &str, decks: Option<(Vec<String>, Vec<String>)>) -> GameState {
    register_instance_fixtures();
    create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: decks.unwrap_or_else(|| (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21))),
        ..CreateGameOptions::default()
    })
}

/// A 20-card deck of these fixtures and vanilla fillers, for the played-out replay games.
pub fn instance_deck(player: PlayerId) -> Vec<String> {
    let mut own: Vec<String> = [
        &*brittle_unit,
        &*body,
        &*asker,
        &*numbered_body,
        &*numbered,
        &*billy,
        &*x_bolt,
        &*echo_bolt,
        &*nerfer,
        &*constant,
        &*withering,
        &*chaos_machine,
        &*dropship,
        &*military,
        &*educator,
        &*brittle_trap,
        &*stoic,
        &*shackled,
    ]
    .iter()
    .map(|card| card.id.clone())
    .collect();
    let filler = vanilla_deck(
        DECK_SIZE - own.len() as i32,
        if player == PlayerId::P1 { 1 } else { 21 },
    );
    own.extend(filler);
    own
}
