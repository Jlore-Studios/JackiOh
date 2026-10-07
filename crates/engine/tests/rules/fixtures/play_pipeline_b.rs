//! Port of `packages/engine/test/fixtures/playPipelineB.ts`.
//!
//! Test-only cards for play pipeline B (docs/classic-sets.md B5 E11, E12, E15, B4.5; R391, R396,
//! R452–R455), each reproducing through the effects library the shape of a Classic or Classic+ card
//! that the systems exist for. The engine never imports `packages/cards` (CLAUDE.md), so these are
//! the engine's proof; the real cards' tests prove the same cases again.
//!
//! Ids are prefixed `pb-` and indexed from 4520 up, clear of every other fixture file (BUILD §0).
//! (TS numbered them from a module counter, one step per `def`; each index is written out here in the
//! order TS made them.)

use std::cell::Cell;
use std::sync::{Arc, LazyLock};

use jackioh_engine::effects;
use jackioh_engine::testkit::*;
use serde::Serialize;

use super::harness::new_game;

/// TS's `{ ...a, ...b }` on two object literals: `b`'s keys replace `a`'s.
fn spread(mut base: Value, extra: Value) -> Value {
    if let (Some(into), Value::Object(from)) = (base.as_object_mut(), extra) {
        for (key, value) in from {
            into.insert(key, value);
        }
    }
    base
}

fn def_json(index: u32, name: &str, type_: &str, extra: Value) -> Value {
    spread(
        json!({
            "id": format!("pb-{name}"),
            "index": index.to_string(),
            "name": format!("{name} (pipeline B)"),
            "set": "Core",
            "type": type_,
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": { "keywords": [], "text": name },
            "radiant": { "keywords": [], "text": name },
        }),
        extra,
    )
}

fn def(index: u32, name: &str, type_: &str, extra: Value) -> CardDef {
    json_as(def_json(index, name, type_, extra))
}

fn unit(index: u32, name: &str, attack: i32, health: i32, extra: Value, keywords: Value) -> CardDef {
    let faces = json!({
        "base": { "attack": attack, "health": health, "keywords": keywords, "text": name },
        "radiant": { "attack": attack * 2, "health": health * 2, "keywords": keywords, "text": name },
    });
    def(index, name, "Unit", spread(faces, extra))
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

// ---------------------------------------------------------------------------
// E11: permissions to play from the graveyard (R454)
// ---------------------------------------------------------------------------

/// Classic #28 Second Wind's shape: every card from your graveyard; Radiant, only a price of (1)+.
pub fn second_wind() -> CardDef {
    def(4521, "second-wind", "Field Spell", json!({ "cost": 0 }))
}

/// Classic #74 Corpse Plantation's shape: Units from your graveyard, paid with its Plague Counters.
pub fn plantation() -> CardDef {
    def(4522, "plantation", "Field Spell", json!({ "cost": 2 }))
}

/// Classic #90 In Too Deep's reward L: the same permission, while its memory says it was earned.
pub fn quest_card() -> CardDef {
    def(4523, "quest", "Field Spell", json!({ "cost": 1 }))
}

pub const QUEST_REWARD_KEY: &str = "rewardL";

/// A Unit with a Cry that is easy to see fire: 2 damage to the enemy hero.
pub fn grave_unit() -> CardDef {
    unit(4524, "grave-unit", 3, 3, json!({ "cost": 2 }), json!([]))
}

/// A Spell with the same visible effect: 1 damage to the enemy hero.
pub fn grave_spell() -> CardDef {
    def(4525, "grave-spell", "Spell", json!({ "cost": 1 }))
}

/// A (0) Spell, the loop Second Wind's Radiant face stops.
pub fn zero_spell() -> CardDef {
    def(4526, "zero-spell", "Spell", json!({ "cost": 0 }))
}

/// A Trap, set face-down wherever it is played from (R227).
pub fn grave_trap() -> CardDef {
    def(4527, "grave-trap", "Trap", json!({ "cost": 1 }))
}

// ---------------------------------------------------------------------------
// E12: casts (R452, R453)
// ---------------------------------------------------------------------------

/// A Spell with one declared target, any unit or hero: 3 damage to it.
pub fn target_spell() -> CardDef {
    def(4528, "target-spell", "Spell", json!({ "cost": 2 }))
}

/// A Spell with a declared mode: "a" deals 1 to the enemy hero, "b" heals nothing and deals 2.
pub fn mode_spell() -> CardDef {
    def(4529, "mode-spell", "Spell", json!({ "cost": 1 }))
}

/// A Spell whose resolution opens a Discover among three fixture units.
pub fn discover_spell() -> CardDef {
    def(4530, "discover-spell", "Spell", json!({ "cost": 1 }))
}

/// An X Spell: X damage to the enemy hero.
pub fn x_spell() -> CardDef {
    def(4531, "x-spell", "Spell", json!({ "cost": "X" }))
}

/// An X Spell with a declared target: X damage to it.
pub fn x_target() -> CardDef {
    def(4532, "x-target", "Spell", json!({ "cost": "X" }))
}

/// A Field Spell with a Cry — a cast one with no zone fizzles (R453).
pub fn cast_field() -> CardDef {
    def(4533, "cast-field", "Field Spell", json!({ "cost": 1 }))
}

/// A Trap a cast sets face-down.
pub fn cast_trap() -> CardDef {
    def(4534, "cast-trap", "Trap", json!({ "cost": 1 }))
}

/// A Spell whose resolution asks the OTHER player to choose (Classic #9's shape).
pub fn their_choice() -> CardDef {
    def(4535, "their-choice", "Spell", json!({ "cost": 1 }))
}

/// A Spell whose resolution asks its controller to choose a target (a prompt, not a declaration).
pub fn ask_target() -> CardDef {
    def(4536, "ask-target", "Spell", json!({ "cost": 1 }))
}

/// The pool the random casters below cast from, named card by card.
pub const RANDOM_POOL: &[&str] = &["pb-target-spell", "pb-mode-spell", "pb-discover-spell", "pb-ask-target"];

/// Classic+ #47 Jogg's Box's shape: cast 3 random Spells from `RANDOM_POOL` (itself excluded).
pub fn jogg_box() -> CardDef {
    def(4537, "jogg-box", "Spell", json!({ "cost": 4 }))
}

/// Classic+ #38.1 Solarius Prime's shape: its Cry casts 2 random Spells that target enemies.
pub fn solarius() -> CardDef {
    unit(4538, "solarius", 9, 5, json!({ "cost": 4 }), json!([]))
}

/// A random caster whose pool is itself and a Jogg's Box: random casts that cast at random (R452).
pub fn chain_caster() -> CardDef {
    def(4539, "chain-caster", "Spell", json!({ "cost": 1 }))
}

/// Classic #56 Spell Tyrant's Radiant shape: cast every Spell in your graveyard, then exile them.
pub fn tyrant() -> CardDef {
    unit(4540, "tyrant", 5, 5, json!({ "cost": 4 }), json!([]))
}

/// Classic #47 Recurring Felinor's shape: its Cry casts a named card (`pb-target-spell`).
pub fn named_caster() -> CardDef {
    unit(4541, "named-caster", 3, 2, json!({ "cost": 2 }), json!([]))
}

/// Classic+ #14 Forever&'s shape: the next Spell gains "return after resolving, can't cost < (2)".
pub fn forever() -> CardDef {
    def(4542, "forever", "Spell", json!({ "cost": 1 }))
}

// ---------------------------------------------------------------------------
// E15: price rules (R455)
// ---------------------------------------------------------------------------

/// Classic #6 Cloaked Toe Cracker's shape: "Aura: Your Traps cost (0)".
pub fn toe_cracker() -> CardDef {
    unit(4543, "toe-cracker", 3, 4, json!({ "cost": 2 }), json!([]))
}

/// Classic #68 Small Card Lobbyist's shape: (3)+ Cost cards cost (1) more; Radiant, the enemy can't play them.
pub fn lobbyist() -> CardDef {
    unit(4544, "lobbyist", 11, 13, json!({ "cost": 4 }), json!([]))
}

/// Classic #77 Anti-Magic Monkey's shape: Spells cost (1) more, (2) on the Radiant face.
pub fn monkey() -> CardDef {
    unit(4545, "monkey", 5, 5, json!({ "cost": 2 }), json!([]))
}

/// Classic #2 The Trickster's shape: your next Trap or Field Spell costs (2) less; Radiant, (0).
pub fn trickster() -> CardDef {
    unit(4546, "trickster", 2, 1, json!({ "cost": 1 }), json!([]))
}

/// AI Alignment Tax's shape: your opponent's cards cost (1) more during their next turn.
pub fn tax() -> CardDef {
    def(4547, "tax", "Spell", json!({ "cost": 1 }))
}

/// A (3) Spell and a (5) Unit to price.
pub fn three_spell() -> CardDef {
    def(4548, "three-spell", "Spell", json!({ "cost": 3 }))
}

pub fn five_unit() -> CardDef {
    unit(4549, "five-unit", 5, 5, json!({ "cost": 5 }), json!([]))
}

/// A (2) Field Spell to price.
pub fn two_field() -> CardDef {
    def(4550, "two-field", "Field Spell", json!({ "cost": 2 }))
}

/// An X Unit and an embiggen Field Spell, for R396's reader.
pub fn x_unit() -> CardDef {
    unit(4551, "x-unit", 1, 1, json!({ "cost": "X" }), json!([]))
}

pub fn embiggen_field() -> CardDef {
    def(4552, "embiggen-field", "Field Spell", json!({ "cost": { "base": 2, "embiggen": 4 } }))
}

// ---------------------------------------------------------------------------
// B4.5: Tribute zones (R391)
// ---------------------------------------------------------------------------

/// Classic #45 Nature Titan's shape: Tribute 1 on a big body.
pub fn titan() -> CardDef {
    unit(4553, "titan", 6, 6, json!({ "cost": 2 }), json!([]))
}

/// Tribute 2.
pub fn titan_two() -> CardDef {
    unit(4554, "titan-two", 8, 8, json!({ "cost": 2 }), json!([]))
}

/// A backrow card with a Tribute cost (none in the new sets; B4.5 rule 4).
pub fn tribute_field() -> CardDef {
    def(4555, "tribute-field", "Field Spell", json!({ "cost": 1 }))
}

/// A body with Reborn: its zone is reserved for its return, so its Tribute frees nothing.
pub fn reborn_body() -> CardDef {
    unit(4556, "reborn-body", 1, 1, json!({ "cost": 1 }), json!([{ "kind": "Reborn" }]))
}

/// A Stack body, so a pile of two can be built.
pub fn stack_body() -> CardDef {
    unit(4557, "stack-body", 1, 1, json!({ "cost": 1 }), json!([{ "kind": "Stack" }]))
}

pub static PB_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        second_wind(),
        plantation(),
        quest_card(),
        grave_unit(),
        grave_spell(),
        zero_spell(),
        grave_trap(),
        target_spell(),
        mode_spell(),
        discover_spell(),
        x_spell(),
        x_target(),
        cast_field(),
        cast_trap(),
        their_choice(),
        ask_target(),
        jogg_box(),
        solarius(),
        chain_caster(),
        tyrant(),
        named_caster(),
        forever(),
        toe_cracker(),
        lobbyist(),
        monkey(),
        trickster(),
        tax(),
        three_spell(),
        five_unit(),
        two_field(),
        x_unit(),
        embiggen_field(),
        titan(),
        titan_two(),
        tribute_field(),
        reborn_body(),
        stack_body(),
    ]
});

/// The discover pool: three vanilla fixture units every test catalog holds.
pub const DISCOVER_POOL: &[&str] = &["fx-1", "fx-2", "fx-3"];

/// `openPrompt(ctx, { player, kind, prompt, options, resume: resumeSelf(ctx, step) })`.
fn ask(ctx: &mut EffectContext<'_>, player: PlayerId, kind: PromptKind, prompt: &str, options: Vec<PromptOption>, step: &str) {
    let resume = prompts::resume_self(ctx, step, IndexMap::new());
    let _ = prompts::open_prompt(
        ctx,
        prompts::OpenPromptArgs {
            player,
            kind,
            aim: None,
            prompt: prompt.into(),
            options,
            min: None,
            max: None,
            budget: None,
            owner: None,
            resume,
        },
    );
}

fn option(key: String, label: &str, selection: Selection) -> PromptOption {
    PromptOption {
        key,
        label: label.to_string(),
        selection,
        cost: None,
        radiant: None,
    }
}

/// Opens a prompt for the other player, whose answer deals 1 damage to that player's own hero.
fn ask_the_opponent() -> Effect {
    Effect::new("pb:askTheOpponent", |ctx| {
        let player = opponent_of(ctx.controller);
        let options = ["left", "right"]
            .iter()
            .map(|choice| {
                option(
                    format!("mode:{choice}"),
                    choice,
                    Selection::Mode {
                        option: choice.to_string(),
                    },
                )
            })
            .collect();
        ask(ctx, player, PromptKind::Mode, "Choose", options, "theirs");
    })
}

/// `castNew({ def })`: a new card of that definition, cast by the running card's controller.
fn cast_new_of(def_id: String) -> Effect {
    effects::cast_new(effects::CastNewArgs {
        def: effects::CastNewDef::from(def_id),
        radiant: None,
        how: effects::CastHow::default(),
    })
}

/// `castRandom({ query: { defId: pool }, count, ...how })`.
fn cast_random_of(pool: Vec<String>, count: i32, radiant: Option<bool>, how: Value) -> Effect {
    let how: effects::CastHow = json_as(how);
    effects::cast_random(effects::CastRandomArgs {
        query: effects::CastRandomQuery::Fixed(json_as(json!({ "defId": pool }))),
        count: Some(effects::CastRandomCount::Fixed(count)),
        radiant,
        target_enemies: how.target_enemies,
        afterward: how.afterward,
    })
}

/// The Spells in the running card's controller's graveyard, by id (`castEach`'s `cards`).
fn graveyard_spells(ctx: &mut EffectContext<'_>) -> Vec<String> {
    ctx.state.players[ctx.controller]
        .graveyard
        .iter()
        .filter(|card| catalog::def_of(Some(&*ctx.state), &card.def_id).type_ == CardType::Spell)
        .map(|card| card.id.clone())
        .collect()
}

fn pool_with(extra: &str) -> Vec<String> {
    let mut pool: Vec<String> = RANDOM_POOL.iter().map(|id| id.to_string()).collect();
    pool.push(extra.to_string());
    pool
}

fn static_flags(flags: Value) -> Option<StaticFlags> {
    Some(json_as(flags))
}

fn cost_auras(list: Value) -> Option<CostAuraHook> {
    let auras: Vec<CostAura> = json_as(list);
    Some(read_hook(move |_args| auras.clone()))
}

pub static PB_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let mut table: IndexMap<String, CardScripts> = IndexMap::new();
    table.insert(
        second_wind().id,
        CardScripts {
            base: Script {
                graveyard_play: Some(read_hook(|_args| vec![GraveyardPlayPermission::default()])),
                ..Script::default()
            },
            radiant: Script {
                graveyard_play: Some(read_hook(|_args| {
                    vec![GraveyardPlayPermission {
                        min_price: Some(1),
                        ..GraveyardPlayPermission::default()
                    }]
                })),
                ..Script::default()
            },
        },
    );
    table.insert(
        plantation().id,
        both(Script {
            cry: Some(hook(|_ctx| vec![effects::plague(json_as(json!({ "amount": 2 })))])),
            graveyard_play: Some(read_hook(|_args| {
                vec![GraveyardPlayPermission {
                    units: Some(true),
                    plague: Some(true),
                    ..GraveyardPlayPermission::default()
                }]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        quest_card().id,
        both(Script {
            graveyard_play: Some(read_hook(|args| {
                if args.self_.memory.get(QUEST_REWARD_KEY) == Some(&Value::Bool(true)) {
                    vec![GraveyardPlayPermission::default()]
                } else {
                    vec![]
                }
            })),
            ..Script::default()
        }),
    );
    table.insert(
        grave_unit().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 2 })))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        grave_spell().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 })))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        zero_spell().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 })))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        target_spell().id,
        both(Script {
            targets: vec![TargetDecl::target(1, 1, json!({ "of": ["unit", "hero"] }))],
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 3 })))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        mode_spell().id,
        both(Script {
            modes: vec![json_as::<ModeDecl>(json!({ "kind": "mode", "options": ["a", "b"] }))],
            cry: Some(hook(|ctx| {
                let amount = if ctx.modes.first().map(String::as_str) == Some("b") { 2 } else { 1 };
                vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        discover_spell().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::discover_from_catalog(json_as(json!({
                    "step": "picked",
                    "query": { "defId": DISCOVER_POOL },
                })))]
            })),
            resume: IndexMap::from([(
                "picked",
                hook(|ctx| match effects::chosen_options(ctx).first() {
                    None => vec![],
                    Some(def_id) => vec![effects::add_to_hand(json_as(json!({ "defId": def_id })))],
                }),
            )]),
            ..Script::default()
        }),
    );
    table.insert(
        x_spell().id,
        both(Script {
            cry: Some(hook(|ctx| {
                vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": ctx.x })))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        x_target().id,
        both(Script {
            targets: vec![TargetDecl::target(1, 1, json!({ "of": ["unit", "hero"] }))],
            cry: Some(hook(|ctx| {
                vec![effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": ctx.x })))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        cast_field().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 })))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        their_choice().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![
                    ask_the_opponent(),
                    effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 }))),
                ]
            })),
            resume: IndexMap::from([(
                "theirs",
                hook(|_ctx| vec![effects::damage(json_as(json!({ "to": { "of": "selfHero" }, "amount": 1 })))]),
            )]),
            ..Script::default()
        }),
    );
    table.insert(
        ask_target().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![
                    // A target prompt among every unit and hero: a random cast that targets enemies picks one of theirs.
                    Effect::new("pb:askTarget", |ctx| {
                        let heroes = [PlayerId::P1, PlayerId::P2]
                            .into_iter()
                            .map(|player| {
                                option(format!("hero:{player}"), player.as_str(), Selection::Hero { player })
                            })
                            .collect();
                        let controller = ctx.controller;
                        ask(ctx, controller, PromptKind::Target, "Hit", heroes, "hit");
                    }),
                ]
            })),
            resume: IndexMap::from([(
                "hit",
                hook(|_ctx| vec![effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 2 })))]),
            )]),
            ..Script::default()
        }),
    );
    table.insert(
        jogg_box().id,
        CardScripts {
            base: Script {
                cry: Some(hook(|_ctx| vec![cast_random_of(pool_with(&jogg_box().id), 3, None, json!({}))])),
                ..Script::default()
            },
            radiant: Script {
                static_flags: static_flags(json!({ "echo": 1 })),
                cry: Some(hook(|_ctx| vec![cast_random_of(pool_with(&jogg_box().id), 3, None, json!({}))])),
                ..Script::default()
            },
        },
    );
    table.insert(
        solarius().id,
        CardScripts {
            base: Script {
                cry: Some(hook(|_ctx| {
                    let pool = RANDOM_POOL.iter().map(|id| id.to_string()).collect();
                    vec![cast_random_of(pool, 2, None, json!({ "targetEnemies": true }))]
                })),
                ..Script::default()
            },
            radiant: Script {
                cry: Some(hook(|_ctx| {
                    let pool = RANDOM_POOL.iter().map(|id| id.to_string()).collect();
                    vec![cast_random_of(pool, 2, Some(true), json!({ "targetEnemies": true }))]
                })),
                ..Script::default()
            },
        },
    );
    table.insert(
        chain_caster().id,
        both(Script {
            // Its pool is a copy of itself (excluded, B4.1) and Jogg's Box, whose casts can be chain casters.
            cry: Some(hook(|_ctx| {
                vec![cast_random_of(vec![chain_caster().id, jogg_box().id], 2, None, json!({}))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        tyrant().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::cast_each(effects::CastEachArgs {
                    cards: Arc::new(graveyard_spells),
                    how: json_as(json!({ "afterward": "exile" })),
                })]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        named_caster().id,
        both(Script {
            cry: Some(hook(|_ctx| vec![cast_new_of(target_spell().id)])),
            ..Script::default()
        }),
    );
    table.insert(
        forever().id,
        CardScripts {
            base: Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::enchant_next_spell(json_as(json!({
                        "enchantment": { "kind": "returnAfterResolve", "floor": 2 },
                    })))]
                })),
                ..Script::default()
            },
            radiant: Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::enchant_next_spell(json_as(json!({
                        "enchantment": { "kind": "returnAfterResolve", "floor": 1 },
                    })))]
                })),
                ..Script::default()
            },
        },
    );
    table.insert(
        toe_cracker().id,
        both(Script {
            cost_aura: cost_auras(json!([{ "whose": "yours", "types": ["Trap", "Field Trap"], "setTo": 0 }])),
            ..Script::default()
        }),
    );
    table.insert(
        lobbyist().id,
        CardScripts {
            base: Script {
                cost_aura: cost_auras(json!([{ "whose": "all", "minCost": 3, "amount": 1 }])),
                ..Script::default()
            },
            radiant: Script {
                cost_aura: cost_auras(json!([{ "whose": "opponents", "minCost": 3, "ban": true }])),
                ..Script::default()
            },
        },
    );
    table.insert(
        monkey().id,
        CardScripts {
            base: Script {
                cost_aura: cost_auras(json!([{ "whose": "all", "types": ["Spell"], "amount": 1 }])),
                ..Script::default()
            },
            radiant: Script {
                cost_aura: cost_auras(json!([{ "whose": "all", "types": ["Spell"], "amount": 2 }])),
                ..Script::default()
            },
        },
    );
    table.insert(
        trickster().id,
        CardScripts {
            base: Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::add_cost_rule(json_as(json!({
                        "rule": { "types": ["Trap", "Field Trap", "Field Spell"], "amount": -2 },
                        "lasts": "used",
                    })))]
                })),
                ..Script::default()
            },
            radiant: Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::add_cost_rule(json_as(json!({
                        "rule": { "types": ["Trap", "Field Trap", "Field Spell"], "setTo": 0 },
                        "lasts": "used",
                    })))]
                })),
                ..Script::default()
            },
        },
    );
    table.insert(
        tax().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::add_cost_rule(json_as(json!({
                    "player": "enemy",
                    "rule": { "amount": 1 },
                    "lasts": "theirNextTurn",
                })))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        titan().id,
        both(Script {
            static_flags: static_flags(json!({ "tribute": 1 })),
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 })))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        titan_two().id,
        both(Script {
            static_flags: static_flags(json!({ "tribute": 2 })),
            ..Script::default()
        }),
    );
    table.insert(
        tribute_field().id,
        both(Script {
            static_flags: static_flags(json!({ "tribute": 1 })),
            ..Script::default()
        }),
    );
    table
});

/// This file's scripts, by id (the brief's `scripts()`).
pub fn scripts() -> IndexMap<String, CardScripts> {
    PB_SCRIPTS.clone()
}

/// Merge this file's cards into whatever catalog and scripts the test registered first.
pub fn register_pipeline_b() {
    let mut defs = registered_catalog().clone();
    for entry in PB_DEFS.iter() {
        defs.insert(entry.id.clone(), entry.clone());
    }
    register_catalog(defs);
    let mut all_scripts = registered_scripts().clone();
    all_scripts.extend(scripts());
    register_scripts(all_scripts);
}

// ---------------------------------------------------------------------------
// Harness shared by the pipeline B tests
// ---------------------------------------------------------------------------

thread_local! {
    /// TS's module `let nonce`: one counter per test thread, so every action a test sends is fresh.
    static NONCE: Cell<u32> = const { Cell::new(0) };
}

/// Reduce one action with a fresh nonce, returning the whole result. `body` is TS's `ActionInput`
/// (an `ActionInput` or its JSON literal).
pub fn pb_reduce(state: &GameState, body: impl Serialize) -> ReduceResult {
    let nonce = NONCE.with(|n| {
        n.set(n.get() + 1);
        n.get()
    });
    let mut fields = serde_json::to_value(body).expect("an action input serialises");
    if let Some(object) = fields.as_object_mut() {
        object.insert("nonce".to_string(), json!(format!("pb{nonce}")));
    }
    reduce(state, &json_as::<Action>(fields))
}

/// Reduce one action with a fresh nonce; a refusal panics with its text (TS threw it).
pub fn pb_act(state: &GameState, body: impl Serialize) -> GameState {
    let result = pb_reduce(state, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

/// Past the mulligans, in p1's main phase, with these cards registered and p1 at 4 mana.
pub fn pb_playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    let keep: Vec<String> = state.players.p1.hand.iter().map(|card| card.id.clone()).collect();
    state = pb_act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p1" }));
    let keep: Vec<String> = state.players.p2.hand.iter().map(|card| card.id.clone()).collect();
    state = pb_act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }));
    register_pipeline_b();
    state.players.p1.mana = ManaState {
        current: 4,
        max: 4,
        next_turn_mod: 0,
        perm_mod: 0,
    };
    state
}

/// A card of `defId` put straight into `player`'s graveyard. TS's default: `player = "p1"`.
pub fn in_graveyard(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(state, def_id, player, Zone::Graveyard { player });
    state.players[player].graveyard.push(card.clone());
    card
}

pub fn only<T: Clone>(items: &[T]) -> T {
    match items.first() {
        Some(first) => first.clone(),
        None => panic!("expected at least one item"),
    }
}

/// The `play` actions `legalActions` offers for one instance. TS's default: `player = "p1"`.
pub fn plays_of(state: &GameState, instance_id: &str, player: PlayerId) -> Vec<ActionBody> {
    legal_actions(state, player)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Play { instance_id: id, .. } if id == instance_id))
        .collect()
}

/// `JSON.parse(JSON.stringify(state))`, the round trip a paused state must survive (§9.3).
pub fn round_trip(state: &GameState) -> GameState {
    let text = serde_json::to_string(state).expect("a state serialises");
    serde_json::from_str(&text).expect("a state parses back")
}
