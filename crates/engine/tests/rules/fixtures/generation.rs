//! Fixture cards for the generation systems (docs/classic-sets.md B5 E19, E23, E24, E25): Plague
//! placements, the Fuse variants, the Transform variants and the Recruit extensions. The engine never
//! imports `crates/cards` (CLAUDE.md), so each shape a Classic or Classic+ card will take is written
//! here once, in the smallest script that has it, and the tests drive them through `reduce`.
//!
//! Ids are `gen-*` and indexes 4601 upward, so nothing collides with another fixture file's.

#![allow(non_upper_case_globals)]

use std::cell::Cell;
use std::sync::LazyLock;

use jackioh_engine::effects::{
    choose_mode, chosen_options, damage, discover_from_catalog, fuse_cards, fuse_generated,
    fuse_onto_your_card, fuse_random_into, place_plague, place_plague_each, place_plague_random,
    place_plague_tokens, recruit_all, transform_beneath,
};
use jackioh_engine::testkit::*;

use super::harness::new_game;

/// A face as the JSON a def literal takes.
fn face(text: &str, extra: Value) -> Value {
    let mut printed = json!({ "keywords": [], "text": text });
    if let (Some(target), Value::Object(extra)) = (printed.as_object_mut(), extra) {
        for (key, value) in extra {
            target.insert(key, value);
        }
    }
    printed
}

/// A Core Common at cost 1 named `Gen <name>`, `extra` over it.
fn def(name: &str, index: i32, type_: &str, extra: Value) -> CardDef {
    let mut card = json!({
        "id": format!("gen-{name}"),
        "index": index.to_string(),
        "name": format!("Gen {name}"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": face(&format!("{name} base"), json!({})),
        "radiant": face(&format!("{name} radiant"), json!({})),
    });
    if let (Some(target), Value::Object(extra)) = (card.as_object_mut(), extra) {
        for (key, value) in extra {
            target.insert(key, value);
        }
    }
    json_as(card)
}

/// A Unit whose Radiant face doubles its stats.
fn unit(name: &str, index: i32, attack: i32, health: i32, extra: Value) -> CardDef {
    let mut faces = json!({
        "base": face(&format!("{name} {attack}/{health}"), json!({ "attack": attack, "health": health })),
        "radiant": face(
            &format!("{name} radiant {}/{}", attack * 2, health * 2),
            json!({ "attack": attack * 2, "health": health * 2 }),
        ),
    });
    if let (Some(target), Value::Object(extra)) = (faces.as_object_mut(), extra) {
        for (key, value) in extra {
            target.insert(key, value);
        }
    }
    def(name, index, "Unit", faces)
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

// E19 Plague Counters.

/// Classic #70's shape: "Place 2 Plague Counters" (Radiant 3), then 1 damage to the enemy hero.
pub static plague_book: LazyLock<CardDef> = LazyLock::new(|| def("plague-book", 4601, "Spell", json!({})));
/// Classic #27's shape: Plague Counters placed on this are doubled (Radiant tripled).
pub static slime: LazyLock<CardDef> = LazyLock::new(|| unit("slime", 4602, 1, 1, json!({})));
/// Classic #53's shape: whenever Plague Counters are placed on this, 1 damage to the enemy hero.
pub static crawler: LazyLock<CardDef> = LazyLock::new(|| unit("crawler", 4603, 2, 2, json!({})));
/// Classic #42's aura: your Units +1/+1 per Plague Counter on them, enemy Units −1/−1 per token.
pub static toxins: LazyLock<CardDef> = LazyLock::new(|| def("toxins", 4604, "Field Spell", json!({})));
/// Classic #39's shape: one placement of 1 (Radiant 2) on a declared permanent.
pub static outbreak: LazyLock<CardDef> = LazyLock::new(|| def("outbreak", 4605, "Spell", json!({})));
/// Classic #63's shape: one placement of 1 on each permanent on the field.
pub static dusting: LazyLock<CardDef> = LazyLock::new(|| def("dusting", 4606, "Spell", json!({})));
/// Classic #42's Activate, as a Spell: one token on each of 2 different random Units.
pub static scatter: LazyLock<CardDef> = LazyLock::new(|| def("scatter", 4607, "Spell", json!({})));
/// Classic #69's self layer: +2 attack for each Plague Counter on this.
pub static charger: LazyLock<CardDef> = LazyLock::new(|| unit("charger", 4608, 4, 2, json!({})));
/// A face-down trap with no text, to carry tokens in the backrow.
pub static quiet_trap: LazyLock<CardDef> = LazyLock::new(|| def("quiet-trap", 4609, "Trap", json!({})));
pub static body: LazyLock<CardDef> = LazyLock::new(|| unit("body", 4610, 1, 1, json!({})));
pub static big_body: LazyLock<CardDef> = LazyLock::new(|| unit("big-body", 4611, 3, 5, json!({})));

// E23 Fuse.

/// An ingredient with its own lines of code and a Radiant face that is not a doubling.
pub static fuse_a: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "fuse-a",
        4612,
        2,
        3,
        json!({
            "cost": 2,
            "loc": 10,
            "tags": ["Human"],
            "radiant": face("fuse-a radiant 7/1", json!({ "attack": 7, "health": 1, "keywords": [{ "kind": "Taunt" }] })),
        }),
    )
});
pub static fuse_b: LazyLock<CardDef> =
    LazyLock::new(|| unit("fuse-b", 4613, 1, 1, json!({ "cost": 1, "loc": 7 })));
/// An X-cost Unit and an embiggen Unit, for R470's printed forms.
pub static x_unit: LazyLock<CardDef> = LazyLock::new(|| unit("x-unit", 4614, 1, 1, json!({ "cost": "X" })));
pub static big_unit: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "embiggen-unit",
        4615,
        1,
        1,
        json!({ "cost": { "base": 2, "embiggen": 4 } }),
    )
});
/// An Immutable Unit, which no Fuse keeps (R23).
pub static immutable: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "immutable",
        4616,
        2,
        2,
        json!({
            "base": face("immutable 2/2", json!({ "attack": 2, "health": 2, "keywords": [{ "kind": "Immutable" }] })),
            "radiant": face("immutable 4/4", json!({ "attack": 4, "health": 4, "keywords": [{ "kind": "Immutable" }] })),
        }),
    )
});
/// A Field Trap and a Trap, which count as one type for "of its type".
pub static field_trap: LazyLock<CardDef> = LazyLock::new(|| def("field-trap", 4617, "Field Trap", json!({})));
pub static plain_trap: LazyLock<CardDef> = LazyLock::new(|| def("plain-trap", 4618, "Trap", json!({})));
/// Classic+ #31 Fusion Lab's Cry: fuse a random card of the pool into a declared hand card.
pub static lab: LazyLock<CardDef> = LazyLock::new(|| def("lab", 4619, "Field Spell", json!({})));
/// The pool Fusion Lab and the deck fusion draw from: the lab itself (never drawn, B4.1) and two cards.
pub static LAB_POOL: LazyLock<Vec<String>> =
    LazyLock::new(|| vec![lab.id.clone(), fuse_a.id.clone(), fuse_b.id.clone()]);
/// Classic+ #43 AI Slop: fuse 3 random AI cards into the hand, costing (0).
pub static slop: LazyLock<CardDef> = LazyLock::new(|| def("slop", 4620, "Spell", json!({ "cost": 4 })));
pub static ai_unit: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "ai-unit",
        4621,
        "Unit",
        json!({
            "token": true,
            "tags": ["AI", "Token"],
            "rarity": "Token",
            "cost": 1,
            "base": face("ai unit 1/3", json!({ "attack": 1, "health": 3 })),
            "radiant": face("ai unit radiant 2/6", json!({ "attack": 2, "health": 6 })),
        }),
    )
});
pub static ai_spell: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "ai-spell",
        4622,
        "Spell",
        json!({ "token": true, "tags": ["AI", "Token"], "rarity": "Token", "cost": 2 }),
    )
});
/// Classic+ #73's deck fusion: every card of your deck, each keeping its cost.
pub static deck_fusion: LazyLock<CardDef> =
    LazyLock::new(|| def("deck-fusion", 4623, "Spell", json!({ "cost": 4 })));
/// Classic #78 Radiant's shape: fuse the declared enemy card onto one of yours of its type, then 1 damage.
pub static mutate: LazyLock<CardDef> = LazyLock::new(|| def("mutate", 4624, "Spell", json!({})));
/// Classic+ #30 Felinor Fuser: Discover a Felinor Unit, then another, and fuse both into this.
pub static fuser: LazyLock<CardDef> =
    LazyLock::new(|| unit("fuser", 4625, 3, 3, json!({ "tags": ["Felinor"], "cost": 3 })));
pub static felinor_a: LazyLock<CardDef> =
    LazyLock::new(|| unit("felinor-a", 4626, 1, 2, json!({ "tags": ["Felinor"] })));
pub static felinor_b: LazyLock<CardDef> =
    LazyLock::new(|| unit("felinor-b", 4627, 2, 1, json!({ "tags": ["Felinor"] })));
pub static felinor_c: LazyLock<CardDef> =
    LazyLock::new(|| unit("felinor-c", 4628, 3, 3, json!({ "tags": ["Felinor"] })));

// E24 Transform.

/// Classic+ #4 Juhan: Stack; its Cry makes the cards beneath it copies of it.
pub static juhan: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "juhan",
        4629,
        9,
        6,
        json!({
            "cost": 3,
            "base": face("juhan 9/6", json!({ "attack": 9, "health": 6, "keywords": [{ "kind": "Stack" }] })),
            "radiant": face(
                "juhan 18/12",
                json!({ "attack": 18, "health": 12, "keywords": [{ "kind": "Stack" }, { "kind": "First Strike" }] }),
            ),
        }),
    )
});
/// Classic and Classic+ Units for a Classic Golem's pool, and a Classic Spell no Unit pool holds.
pub static classic_unit: LazyLock<CardDef> =
    LazyLock::new(|| unit("classic-unit", 4630, 2, 2, json!({ "set": "Classic" })));
pub static classic_plus_unit: LazyLock<CardDef> =
    LazyLock::new(|| unit("classic-plus-unit", 4631, 3, 3, json!({ "set": "Classic+" })));
pub static classic_spell: LazyLock<CardDef> =
    LazyLock::new(|| def("classic-spell", 4632, "Spell", json!({ "set": "Classic" })));

// E25 Recruit.

/// Classic #60 Pile On's shape: Recruit every permanent in your deck.
pub static pile_on: LazyLock<CardDef> = LazyLock::new(|| def("pile-on", 4633, "Spell", json!({ "cost": 5 })));
/// A Field Spell that asks a question as it arrives anywhere (R151), so a Recruit can pause.
pub static asker: LazyLock<CardDef> = LazyLock::new(|| def("asker", 4634, "Field Spell", json!({})));
pub static deck_spell: LazyLock<CardDef> = LazyLock::new(|| def("deck-spell", 4635, "Spell", json!({})));
pub static cheap_unit: LazyLock<CardDef> =
    LazyLock::new(|| unit("cheap-unit", 4636, 1, 1, json!({ "cost": 1 })));
pub static pricy_unit: LazyLock<CardDef> =
    LazyLock::new(|| unit("pricy-unit", 4637, 4, 4, json!({ "cost": 4 })));

thread_local! {
    static ASKER_ANSWERS: Cell<Vec<String>> = const { Cell::new(Vec::new()) };
    static NONCE: Cell<u32> = const { Cell::new(0) };
}

/// How many times `asker`'s question was answered, by option.
pub fn asker_answers() -> Vec<String> {
    ASKER_ANSWERS.with(|answers| {
        let now = answers.take();
        answers.set(now.clone());
        now
    })
}

/// Forget the answers `asker_answers` reads.
pub fn clear_asker_answers() {
    ASKER_ANSWERS.with(|answers| answers.set(Vec::new()));
}

fn push_asker_answers(more: Vec<String>) {
    ASKER_ANSWERS.with(|answers| {
        let mut now = answers.take();
        now.extend(more);
        answers.set(now);
    });
}

fn tokens_on(card: &CardInstance) -> i32 {
    plague_on(card)
}

/// Classic #53: a placement on this card — `counterChanged` with `placed` naming it.
fn placed_on_self(event: &GameEvent, me: Option<&CardInstance>) -> bool {
    match (event, me) {
        (
            GameEvent::CounterChanged {
                instance_id,
                counter: CounterKind::Plague,
                placed: Some(_),
                ..
            },
            Some(me),
        ) => *instance_id == me.id,
        _ => false,
    }
}

/// Classic+ #30's two chained Discovers (the Stitching pattern, R352) and the fusion onto this Unit:
/// the picks travel in the next prompt's data, and the second answer fuses both onto the card itself.
fn fuser_script() -> Script {
    fn discover(step: &str, picks: Vec<String>) -> Effect {
        discover_from_catalog(json_as(json!({
            "step": step,
            "query": { "tags": ["Felinor"], "type": "Unit" },
            "data": { "picks": picks },
        })))
    }
    fn picks_of(data: &IndexMap<String, Value>) -> Vec<String> {
        match data.get("picks").and_then(Value::as_array) {
            Some(picks) => picks
                .iter()
                .filter_map(|pick| pick.as_str().map(str::to_string))
                .collect(),
            None => vec![],
        }
    }
    Script {
        cry: Some(hook(|_ctx| vec![discover("first", vec![])])),
        resume: IndexMap::from([
            (
                "first",
                hook(|ctx| {
                    let mut picks = picks_of(&ctx.data);
                    picks.extend(chosen_options(&*ctx));
                    vec![discover("second", picks)]
                }),
            ),
            (
                "second",
                hook(|ctx| {
                    let mut picks = picks_of(&ctx.data);
                    picks.extend(chosen_options(&*ctx));
                    match ctx.self_.as_ref() {
                        None => vec![],
                        Some(me) => vec![fuse_cards(json_as(
                            json!({ "defIds": picks, "targetInstanceId": me.id }),
                        ))],
                    }
                }),
            ),
        ]),
        ..Script::default()
    }
}

/// Classic #42's aura: each Unit on the field with Plague Counters on it gets `per` × its tokens, up on
/// its controller's side and down on the other, as one entry per Unit (the amount is the Unit's own).
fn toxins_aura<'a>(state: &'a GameState, me: &'a CardInstance, per: i32) -> Vec<AuraEntry<'a>> {
    let mut entries: Vec<AuraEntry<'a>> = Vec::new();
    for player in [PlayerId::P1, PlayerId::P2] {
        for card in active_units_of(state, player) {
            let tokens = tokens_on(card);
            if tokens == 0 {
                continue;
            }
            let sign = if card.controller == me.controller { 1 } else { -1 };
            let id = card.id.clone();
            entries.push(AuraEntry {
                applies: Box::new(move |candidate: &CardInstance| candidate.id == id),
                mod_: StatMod {
                    attack: Some(sign * per * tokens),
                    max_health: Some(sign * per * tokens),
                    ..StatMod::default()
                },
            });
        }
    }
    entries
}

fn plague_book_face(count: i32) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![
                place_plague_tokens(json_as(json!({ "count": count }))),
                damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 }))),
            ]
        })),
        ..Script::default()
    }
}

fn outbreak_face(amount: i32) -> Script {
    Script {
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "any", "of": ["unit", "backrow"] }),
        )],
        cry: Some(hook(move |_ctx| {
            vec![place_plague(json_as(
                json!({ "target": { "of": "chosen" }, "amount": amount }),
            ))]
        })),
        ..Script::default()
    }
}

fn lab_face(radiant: bool) -> Script {
    Script {
        targets: vec![TargetDecl::hand(1, 1, Value::Null)],
        cry: Some(hook(move |_ctx| {
            let mut args =
                json!({ "into": { "target": { "of": "chosen" } }, "query": { "defId": *LAB_POOL } });
            if radiant {
                args["radiant"] = json!(true);
            }
            vec![fuse_random_into(json_as(args))]
        })),
        ..Script::default()
    }
}

fn slop_face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            let mut args = json!({ "count": 3, "query": { "tags": ["AI"], "token": true } });
            if radiant {
                args["radiant"] = json!(true);
            }
            vec![fuse_generated(json_as(args))]
        })),
        ..Script::default()
    }
}

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            plague_book.id.clone(),
            CardScripts {
                base: plague_book_face(2),
                radiant: plague_book_face(3),
            },
        ),
        (
            slime.id.clone(),
            CardScripts {
                base: Script {
                    plague_multiplier: Some(read_hook(|_a| 2)),
                    ..Script::default()
                },
                radiant: Script {
                    plague_multiplier: Some(read_hook(|_a| 3)),
                    ..Script::default()
                },
            },
        ),
        (
            crawler.id.clone(),
            both(Script {
                triggers: vec![
                    TriggerDef::new(
                        "placed-on-this",
                        &[GameEventType::CounterChanged],
                        |ctx, event| {
                            if placed_on_self(event, ctx.self_.as_ref()) {
                                vec![damage(json_as(
                                    json!({ "to": { "of": "enemyHero" }, "amount": 1 }),
                                ))]
                            } else {
                                vec![]
                            }
                        },
                    )
                    .with_when(|ctx, event| placed_on_self(event, ctx.self_.as_ref())),
                ],
                ..Script::default()
            }),
        ),
        (
            toxins.id.clone(),
            CardScripts {
                base: Script {
                    aura: Some(aura_hook(|a| toxins_aura(a.state, a.self_, 1))),
                    ..Script::default()
                },
                radiant: Script {
                    aura: Some(aura_hook(|a| toxins_aura(a.state, a.self_, 2))),
                    ..Script::default()
                },
            },
        ),
        (
            outbreak.id.clone(),
            CardScripts {
                base: outbreak_face(1),
                radiant: outbreak_face(2),
            },
        ),
        (
            dusting.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![place_plague_each(json_as(json!({
                        "scope": { "side": "any", "rows": ["units", "backrow"] },
                        "amount": 1,
                    })))]
                })),
                ..Script::default()
            }),
        ),
        (
            scatter.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![place_plague_random(json_as(json!({ "count": 2, "amount": 1 })))]
                })),
                ..Script::default()
            }),
        ),
        (
            charger.id.clone(),
            both(Script {
                aura: Some(aura_hook(|a| {
                    let id = a.self_.id.clone();
                    vec![AuraEntry {
                        applies: Box::new(move |card: &CardInstance| card.id == id),
                        mod_: StatMod {
                            attack: Some(2 * tokens_on(a.self_)),
                            ..StatMod::default()
                        },
                    }]
                })),
                ..Script::default()
            }),
        ),
        (
            lab.id.clone(),
            CardScripts {
                base: lab_face(false),
                radiant: lab_face(true),
            },
        ),
        (
            slop.id.clone(),
            CardScripts {
                base: slop_face(false),
                radiant: slop_face(true),
            },
        ),
        (
            deck_fusion.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![fuse_random_into(json_as(json!({
                        "into": { "pile": "library" },
                        "query": { "defId": *LAB_POOL },
                    })))]
                })),
                ..Script::default()
            }),
        ),
        (
            mutate.id.clone(),
            both(Script {
                targets: vec![TargetDecl::target(
                    1,
                    1,
                    json!({ "side": "enemy", "of": ["unit", "backrow"] }),
                )],
                cry: Some(hook(|_ctx| {
                    vec![
                        fuse_onto_your_card(json_as(json!({ "target": { "of": "chosen" } }))),
                        damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 }))),
                    ]
                })),
                ..Script::default()
            }),
        ),
        (
            fuser.id.clone(),
            CardScripts {
                base: fuser_script(),
                radiant: fuser_script(),
            },
        ),
        (
            juhan.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![transform_beneath(Default::default())])),
                ..Script::default()
            }),
        ),
        (
            pile_on.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![recruit_all(Default::default())])),
                ..Script::default()
            }),
        ),
        (
            asker.id.clone(),
            both(Script {
                start_of_game: Some(hook(|_ctx| {
                    vec![choose_mode(json_as(
                        json!({ "options": ["left", "right"], "step": "picked" }),
                    ))]
                })),
                resume: IndexMap::from([(
                    "picked",
                    hook(|ctx| {
                        push_asker_answers(chosen_options(&*ctx));
                        vec![]
                    }),
                )]),
                ..Script::default()
            }),
        ),
    ])
});

pub static GEN_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        plague_book.clone(),
        slime.clone(),
        crawler.clone(),
        toxins.clone(),
        outbreak.clone(),
        dusting.clone(),
        scatter.clone(),
        charger.clone(),
        quiet_trap.clone(),
        body.clone(),
        big_body.clone(),
        fuse_a.clone(),
        fuse_b.clone(),
        x_unit.clone(),
        big_unit.clone(),
        immutable.clone(),
        field_trap.clone(),
        plain_trap.clone(),
        lab.clone(),
        slop.clone(),
        ai_unit.clone(),
        ai_spell.clone(),
        deck_fusion.clone(),
        mutate.clone(),
        fuser.clone(),
        felinor_a.clone(),
        felinor_b.clone(),
        felinor_c.clone(),
        juhan.clone(),
        classic_unit.clone(),
        classic_plus_unit.clone(),
        classic_spell.clone(),
        pile_on.clone(),
        asker.clone(),
        deck_spell.clone(),
        cheap_unit.clone(),
        pricy_unit.clone(),
    ]
});

pub static GEN_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| SCRIPTS.clone());

pub fn register_generation() {
    let mut defs: CardDefs = registered_catalog().clone();
    for entry in GEN_DEFS.iter() {
        defs.insert(entry.id.clone(), entry.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}

// Driving a game.

#[derive(Clone, Debug)]
pub struct Run {
    pub start: String,
    pub log: Vec<Action>,
    pub state: GameState,
}

fn next_nonce() -> u32 {
    NONCE.with(|nonce| {
        let next = nonce.get() + 1;
        nonce.set(next);
        next
    })
}

fn action_of(input: impl serde::Serialize, nonce: String) -> Action {
    let parsed: ActionInput = json_as(serde_json::to_value(&input).expect("an action body is JSON"));
    parsed.with_nonce(nonce)
}

/// Past the mulligans, in p1's main phase with 4 mana, this file's fixtures registered.
pub fn playing(seed: &str) -> Run {
    let mut state = begin_game(&new_game(seed, None)).state;
    for player in [PlayerId::P1, PlayerId::P2] {
        let keep: Vec<String> = state.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        let action = Action::new(
            ActionBody::Mulligan { keep },
            player,
            format!("gen-mull-{seed}-{player}"),
        );
        let result = reduce(&state, &action);
        if let Some(error) = result.error {
            panic!("{error}");
        }
        state = result.state;
    }
    register_generation();
    state.players.p1.mana = ManaState {
        current: 4,
        max: 4,
        next_turn_mod: 0,
        perm_mod: 0,
    };
    Run {
        start: String::new(),
        log: vec![],
        state,
    }
}

/// Freeze the run's starting point: everything the test set up by hand is in it from here on.
pub fn frozen(run: &Run) -> Run {
    Run {
        start: serde_json::to_string(&run.state).expect("a state is JSON"),
        log: vec![],
        state: run.state.clone(),
    }
}

pub fn act(run: &Run, input: impl serde::Serialize) -> Run {
    let n = next_nonce();
    let action = action_of(input, format!("gen{n}"));
    let result = reduce(&run.state, &action);
    if let Some(error) = &result.error {
        panic!("{} refused: {error}", action.action_type());
    }
    let mut log = run.log.clone();
    log.push(action);
    Run {
        start: run.start.clone(),
        log,
        state: result.state,
    }
}

/// The refusal an action meets, or `None` when it is legal (the state is left as it was).
pub fn refusal(run: &Run, input: impl serde::Serialize) -> Option<String> {
    let n = next_nonce();
    reduce(&run.state, &action_of(input, format!("gen{n}"))).error
}

/// The run's log applied again from its frozen start: the replay a server folds (§9.3).
pub fn replayed(run: &Run) -> GameState {
    let mut state: GameState = serde_json::from_str(&run.start).expect("a frozen run's start is a state");
    for action in &run.log {
        let result = reduce(&state, action);
        if let Some(error) = &result.error {
            panic!("replay refused {}: {error}", action.action_type());
        }
        state = result.state;
    }
    state
}

/// A card put straight into a player's hand.
pub fn hand_card(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(state, def_id, player, Zone::Hand { player });
    state.players[player].hand.push(card.clone());
    card
}

/// Answer the open prompt with one option, picked by its selection; `None` answers as the prompt's player.
pub fn answer(run: &Run, selection: Selection, player: Option<PlayerId>) -> Run {
    let Some(pending) = run.state.pending.as_ref() else {
        panic!("no prompt is open");
    };
    let input = ActionInput {
        body: ActionBody::Answer {
            choice_id: pending.id.clone(),
            selection: vec![selection],
        },
        player_id: player.unwrap_or(pending.player_id),
    };
    act(run, input)
}

/// What `pick` takes: a card or its instance id.
pub trait CardOrId {
    fn instance_id(&self) -> String;
}

impl CardOrId for CardInstance {
    fn instance_id(&self) -> String {
        self.id.clone()
    }
}

impl CardOrId for &CardInstance {
    fn instance_id(&self) -> String {
        self.id.clone()
    }
}

impl CardOrId for &str {
    fn instance_id(&self) -> String {
        (*self).to_string()
    }
}

impl CardOrId for String {
    fn instance_id(&self) -> String {
        self.clone()
    }
}

impl CardOrId for &String {
    fn instance_id(&self) -> String {
        (*self).clone()
    }
}

pub fn pick(card: impl CardOrId) -> Selection {
    Selection::Instance {
        instance_id: card.instance_id(),
    }
}
