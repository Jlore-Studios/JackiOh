//! Fixture cards for copying the last Spell's text (docs/classic-sets.md B5 E14; Classic #57 Echo; R399,
//! R545–R547). The engine never imports `packages/cards` (CLAUDE.md), so the copier and the Spells it
//! copies are proved on cards of their shape here. Ids are `ct-*`, indices from 5900, registered on top
//! of the shared fixture catalog by `withCopiedText`.
//!
//! Port of `packages/engine/test/fixtures/copiedText.ts`. TS's `CT` object of defs is the struct
//! `CtDefs` behind the static `CT` (`CT.echo.id`, `CT.x_bolt.id`, field names snake_cased); TS's module
//! counter (`nextIndex`, from 5900, one per `def` call in the object's order) is each def's stated index.

use std::sync::LazyLock;

use jackioh_engine::effects::{cast_new, choose_target, counter_play, damage, heal};
use jackioh_engine::subsystems::copied_text::copied_text_of;
use jackioh_engine::testkit::*;

/// TS `def(id, type, extra = {})`: a Core Common at cost 1 whose faces print `id` (`<id> radiant`).
fn def(id: &str, index: i32, type_: &str, extra: Value) -> CardDef {
    let mut card = json!({
        "id": format!("ct-{id}"),
        "index": index.to_string(),
        "name": format!("{id} (copied text)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": id },
        "radiant": { "keywords": [], "text": format!("{id} radiant") },
    });
    if let (Some(target), Value::Object(extra)) = (card.as_object_mut(), extra) {
        for (key, value) in extra {
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

fn damage_params() -> Value {
    json!([{ "key": "damage", "base": 2, "radiant": 4, "better": "up" }])
}

fn ask_damage_params() -> Value {
    json!([{ "key": "damage", "base": 3, "radiant": 6, "better": "up" }])
}

fn any_target() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "of": ["unit", "hero"] }))]
}

/// TS `CT`: the copier, the Spells it copies and the cards around them.
pub struct CtDefs {
    /// Classic #57 Echo's shape: has the last Spell's text; its Radiant face adds Echo 1.
    pub echo: CardDef,
    /// A Spell that deals {damage} (2, Radiant 4) to a declared unit or hero.
    pub bolt: CardDef,
    /// A Spell with no choices: 1 damage to the enemy hero (Radiant 3).
    pub ping: CardDef,
    /// A Spell whose script asks for an enemy unit, then deals {damage} (3, Radiant 6) to it.
    pub asker: CardDef,
    /// An X-cost Spell: X damage to the enemy hero.
    pub x_bolt: CardDef,
    /// "Choose one": 2 damage to the enemy hero, or heal your hero 2.
    pub modal: CardDef,
    /// A Spell with Echo 1: 1 damage to the enemy hero, twice.
    pub echo_spell: CardDef,
    /// Cast on draw: 1 damage to the enemy hero.
    pub draw_cast: CardDef,
    /// A Field Spell: never a Spell for the record.
    pub field: CardDef,
    /// A Spell whose script casts a ping, then deals 5 to the enemy hero: a newer Spell mid-resolution.
    pub caster: CardDef,
    /// A Spell with a preview and a yellow glow, both reading its declared number.
    pub glow: CardDef,
    /// A Trap that counters any play of the opponent's.
    pub counter: CardDef,
    /// An embiggen Spell: 1 damage to the enemy hero at its base price, 5 at its embiggen price.
    pub bigger: CardDef,
    /// A plain Unit to target.
    pub dummy: CardDef,
}

impl CtDefs {
    /// TS `Object.values(CT)`: every def, in the object's order.
    pub fn values(&self) -> Vec<CardDef> {
        vec![
            self.echo.clone(),
            self.bolt.clone(),
            self.ping.clone(),
            self.asker.clone(),
            self.x_bolt.clone(),
            self.modal.clone(),
            self.echo_spell.clone(),
            self.draw_cast.clone(),
            self.field.clone(),
            self.caster.clone(),
            self.glow.clone(),
            self.counter.clone(),
            self.bigger.clone(),
            self.dummy.clone(),
        ]
    }
}

pub static CT: LazyLock<CtDefs> = LazyLock::new(|| CtDefs {
    echo: def("echo", 5901, "Spell", json!({})),
    bolt: def("bolt", 5902, "Spell", json!({ "params": damage_params() })),
    ping: def("ping", 5903, "Spell", json!({})),
    asker: def("asker", 5904, "Spell", json!({ "params": ask_damage_params() })),
    x_bolt: def("x-bolt", 5905, "Spell", json!({ "cost": "X" })),
    modal: def("modal", 5906, "Spell", json!({})),
    echo_spell: def("echo-spell", 5907, "Spell", json!({})),
    draw_cast: def("draw-cast", 5908, "Spell", json!({})),
    field: def("field", 5909, "Field Spell", json!({})),
    caster: def("caster", 5910, "Spell", json!({})),
    glow: def("glow", 5911, "Spell", json!({ "params": damage_params() })),
    counter: def("counter", 5912, "Trap", json!({})),
    bigger: def("bigger", 5913, "Spell", json!({ "cost": { "base": 1, "embiggen": 3 } })),
    dummy: def(
        "dummy",
        5914,
        "Unit",
        json!({
            "base": { "attack": 1, "health": 9, "keywords": [], "text": "dummy" },
            "radiant": { "attack": 2, "health": 18, "keywords": [], "text": "dummy radiant" },
        }),
    ),
});

fn echo_base() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            copies_last_spell: Some(true),
            ..StaticFlags::default()
        }),
        records_play_as: Some(read_hook(|a| copied_text_of(a.state, a.self_))),
        ..Script::default()
    }
}

fn to_enemy_hero(amount: i32) -> Effect {
    damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))
}

fn to_chosen(amount: i32) -> Effect {
    damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))
}

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            CT.echo.id.clone(),
            CardScripts {
                base: echo_base(),
                radiant: Script {
                    static_flags: Some(StaticFlags {
                        copies_last_spell: Some(true),
                        echo: Some(1),
                        ..StaticFlags::default()
                    }),
                    ..echo_base()
                },
            },
        ),
        (
            CT.bolt.id.clone(),
            both(Script {
                targets: any_target(),
                cry: Some(hook(|ctx| vec![to_chosen(param(&*ctx, "damage"))])),
                ..Script::default()
            }),
        ),
        (
            CT.ping.id.clone(),
            CardScripts {
                base: Script {
                    cry: Some(hook(|_ctx| vec![to_enemy_hero(1)])),
                    ..Script::default()
                },
                radiant: Script {
                    cry: Some(hook(|_ctx| vec![to_enemy_hero(3)])),
                    ..Script::default()
                },
            },
        ),
        (
            CT.asker.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![choose_target(json_as(json!({ "step": "hit", "scope": { "side": "enemy", "of": ["unit"] } })))]
                })),
                resume: IndexMap::from([("hit", hook(|ctx| vec![to_chosen(param(&*ctx, "damage"))]))]),
                ..Script::default()
            }),
        ),
        (
            CT.x_bolt.id.clone(),
            both(Script {
                cry: Some(hook(|ctx| vec![to_enemy_hero(ctx.x)])),
                ..Script::default()
            }),
        ),
        (
            CT.modal.id.clone(),
            both(Script {
                modes: vec![ModeDecl {
                    kind: PromptKind::Mode,
                    options: vec!["hit".into(), "heal".into()],
                }],
                cry: Some(hook(|ctx| {
                    if ctx.modes.first().map(String::as_str) == Some("heal") {
                        vec![heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": 2 })))]
                    } else {
                        vec![to_enemy_hero(2)]
                    }
                })),
                ..Script::default()
            }),
        ),
        (
            CT.echo_spell.id.clone(),
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
            CT.draw_cast.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    cast_on_draw: Some(true),
                    ..StaticFlags::default()
                }),
                cry: Some(hook(|_ctx| vec![to_enemy_hero(1)])),
                ..Script::default()
            }),
        ),
        (
            CT.caster.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![cast_new(json_as(json!({ "def": CT.ping.id })), to_enemy_hero(5)]
                })),
                ..Script::default()
            }),
        ),
        (
            CT.glow.id.clone(),
            both(Script {
                targets: any_target(),
                cry: Some(hook(|ctx| vec![to_chosen(param(&*ctx, "damage"))])),
                condition_met: Some(condition_hook(|c| param(&c, "damage") >= 4)),
                preview: Some(condition_hook(|c| {
                    vec![json_as::<PreviewValue>(json!({ "label": "damage", "value": param(&c, "damage") }))]
                })),
                ..Script::default()
            }),
        ),
        (
            CT.counter.id.clone(),
            both(Script {
                triggers: vec![TriggerDef::new("counter", &[GameEventType::CardAnnounced], |_ctx, event| {
                    match event {
                        GameEvent::CardAnnounced { instance_id, .. } => vec![counter_play(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                        })))],
                        _ => vec![],
                    }
                })
                .with_when(|ctx, event| {
                    matches!(event, GameEvent::CardAnnounced { player, .. } if *player != ctx.controller)
                })],
                ..Script::default()
            }),
        ),
        (
            CT.bigger.id.clone(),
            both(Script {
                cry: Some(hook(|ctx| vec![to_enemy_hero(if ctx.embiggened { 5 } else { 1 })])),
                ..Script::default()
            }),
        ),
        (CT.dummy.id.clone(), both(Script::default())),
    ])
});

pub static CT_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| CT.values());

/// Register this file's defs and scripts on top of whatever is registered now.
pub fn register_copied_text() {
    let mut defs: CardDefs = registered_catalog().clone();
    for card in CT_DEFS.iter() {
        defs.insert(card.id.clone(), card.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}

/// `registerCopiedText` for a game `newGame` has just made (its catalog is registered by then).
pub fn with_copied_text(state: GameState) -> GameState {
    register_copied_text();
    state
}

/// Part 24's brief, step 2: this file's test catalog (`CT_DEFS` by id).
pub fn catalog() -> CardDefs {
    CT_DEFS.iter().map(|card| (card.id.clone(), card.clone())).collect()
}

/// Part 24's brief, step 2: this file's scripts.
pub fn scripts() -> IndexMap<String, CardScripts> {
    SCRIPTS.clone()
}
