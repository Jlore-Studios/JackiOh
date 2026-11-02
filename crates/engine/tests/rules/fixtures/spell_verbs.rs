//! Test-only cards for Stinkyscape's spell verbs (issue #1848, R1848000–R1848006): each is the shape of
//! the set card that will use a verb. Ids `sv-…` from 18480 (#1868's from 18680), so they collide with
//! no other fixture file's.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects;
// The verbs as a card script names them, through the prelude (CLAUDE.md rule 5).
use jackioh_engine::prelude::{
    INFLATION_CHANCE, freeze, give_next_spell_keyword, inflate, inflation_chance_of, melee,
    next_inflation_chance, target_check, unit_view, x_of,
};
use jackioh_engine::testkit::*;

use super::damage_combat::ask_controller;
use super::harness::both;

fn def(index: u32, name: &str, type_: &str, cost: Value, base: Value, radiant: Value) -> CardDef {
    json_as(json!({
        "id": format!("sv-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (spell verbs fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": base,
        "radiant": radiant,
    }))
}

fn spell(index: u32, name: &str, cost: Value, keywords: Value, radiant_keywords: Value) -> CardDef {
    def(
        index,
        name,
        "Spell",
        cost,
        json!({ "keywords": keywords, "text": name }),
        json!({ "keywords": radiant_keywords, "text": name }),
    )
}

fn unit(index: u32, name: &str, attack: i32, health: i32, keywords: Value) -> CardDef {
    def(
        index,
        name,
        "Unit",
        json!(1),
        json!({ "attack": attack, "health": health, "keywords": keywords, "text": name }),
        json!({ "attack": 2 * attack, "health": 2 * health, "keywords": keywords, "text": name }),
    )
}

// R1848000: Freeze forever

/// Stinkyscape #2 Absolute Zero: "Freeze a Unit forever."
pub static absolute_zero: LazyLock<CardDef> =
    LazyLock::new(|| spell(18480, "absolute-zero", json!(2), json!([]), json!([])));

// R1848001: a Spell's Trample onto adjacent Units

/// Stinkyscape #3 Meteor: "Deal 15 damage to a Unit. Tramples onto adjacent Units"; Radiant
/// "Deal 20 damage to a Unit. Tramples onto adjacent Units and the enemy hero".
pub static meteor: LazyLock<CardDef> =
    LazyLock::new(|| spell(18481, "meteor", json!(3), json!([]), json!([])));

// R1848002: Poisonous for a Spell

/// Stinkyscape #13 Asphyxiants Cloud: "Poisonous. Your next Spell this turn has Poisonous.
/// Deal 1 damage to all enemies."
pub static cloud: LazyLock<CardDef> = LazyLock::new(|| {
    spell(
        18482,
        "cloud",
        json!(1),
        json!([{ "kind": "Poisonous" }]),
        json!([{ "kind": "Poisonous" }]),
    )
});
/// "Deal 1 damage to a Unit": the next Spell the Cloud's grant reaches.
pub static ping: LazyLock<CardDef> = LazyLock::new(|| spell(18483, "ping", json!(0), json!([]), json!([])));

// R1848003: copies with a fixed X

/// Stinkyscape #99 No-No Words' shape on a number a test reads: "(X) Deal X damage to the enemy hero."
pub static nono: LazyLock<CardDef> = LazyLock::new(|| spell(18484, "nono", json!("X"), json!([]), json!([])));
/// Stinkyscape #102 Book of Nono-Words: "Add 4 copies of No-No Words to your hand, their prices
/// are set at 1, 2, 3, 4."
pub static nono_book: LazyLock<CardDef> =
    LazyLock::new(|| spell(18485, "nono-book", json!(1), json!([]), json!([])));
/// "Add a Ping to your hand, its X set at 2": a set X on a card that has no X.
pub static not_x_book: LazyLock<CardDef> =
    LazyLock::new(|| spell(18486, "not-x-book", json!(0), json!([]), json!([])));

// R1848004: lose or gain N mana next turn

/// Stinkyscape #117 Core System's mana half: "Lose 2 mana next turn."
pub static core_system: LazyLock<CardDef> =
    LazyLock::new(|| spell(18487, "core-system", json!(0), json!([]), json!([])));
/// "Lose 5 mana next turn", more than a refresh gives.
pub static blackout: LazyLock<CardDef> =
    LazyLock::new(|| spell(18488, "blackout", json!(0), json!([]), json!([])));

// R1848005: the melee

/// Stinkyscape #120 Wild System: "All units attack each other until only one survives (max 20
/// attacks). Gain 1 mana next turn."
pub static wild_system: LazyLock<CardDef> =
    LazyLock::new(|| spell(18489, "wild-system", json!(0), json!([]), json!([])));
/// A 1/1 whose Death asks its controller something, so a melee has to wait for the answer.
pub static asker: LazyLock<CardDef> = LazyLock::new(|| unit(18490, "asker", 1, 1, json!([])));
/// A 2/2 with Flying: only a Unit with Flying or Ranged may attack it while it is in Attack Position.
pub static flyer: LazyLock<CardDef> =
    LazyLock::new(|| unit(18491, "flyer", 2, 2, json!([{ "kind": "Flying" }])));
/// A 2/4 with Immune to Spells (§6.1).
pub static warded: LazyLock<CardDef> =
    LazyLock::new(|| unit(18492, "warded", 2, 4, json!([{ "kind": "Immune to Spells" }])));
/// A 1/1 that can't be attacked (§6.1, a static flag), so a melee never draws it as a target.
pub static untouchable: LazyLock<CardDef> = LazyLock::new(|| unit(18498, "untouchable", 1, 1, json!([])));
/// A 1/1 with Spell Damage +2 (§4.4 step 0), so a test can see a Meteor raised once.
pub static adept: LazyLock<CardDef> =
    LazyLock::new(|| unit(18497, "adept", 1, 1, json!([{ "kind": "Spell Damage", "n": 2 }])));

// R1848006: Inflation

/// Stinkyscape #153 Stimmycheck: "Inflation (15%). Add a Stimmycheck to your hand with +10%
/// chance at Inflation"; its Radiant face prints Lucky 1. The copy is added at once rather than at the
/// end of the turn, so a test reads the chance it carries.
pub static stimmy: LazyLock<CardDef> = LazyLock::new(|| {
    spell(
        18493,
        "stimmy",
        json!(0),
        json!([]),
        json!([{ "kind": "Lucky", "n": 1 }]),
    )
});
/// Inflation at 100%, so every card it can reach costs (1) more and no roll is made.
pub static hyperinflation: LazyLock<CardDef> =
    LazyLock::new(|| spell(18494, "hyperinflation", json!(0), json!([]), json!([])));
/// Inflation at 0%, which does nothing.
pub static stable: LazyLock<CardDef> =
    LazyLock::new(|| spell(18495, "stable", json!(0), json!([]), json!([])));
/// Inflation on the opponent's hand and deck (15%); its Radiant face prints Lucky 1.
pub static sanctions: LazyLock<CardDef> = LazyLock::new(|| {
    spell(
        18496,
        "sanctions",
        json!(0),
        json!([]),
        json!([{ "kind": "Lucky", "n": 1 }]),
    )
});

// R1868000: a declared target that reads the play's X

/// Stinkyscape #99 No-No Words: "(X) Destroy a Unit with less than 4X Attack", pick needed (R703).
pub static nono_x: LazyLock<CardDef> =
    LazyLock::new(|| spell(18680, "nono-x", json!("X"), json!([]), json!([])));
/// A 3/3 gaining +2 Attack whenever the other player plays a card (§10.5 step 4, before it resolves).
pub static riser: LazyLock<CardDef> = LazyLock::new(|| unit(18681, "riser", 3, 3, json!([])));

fn a_unit() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "of": ["unit"] }))]
}

fn meteor_hit(amount: i32, onto: &str) -> Effect {
    effects::damage(json_as(
        json!({ "to": { "of": "chosen" }, "amount": amount, "trampleOnto": onto }),
    ))
}

fn next_turn_mana(amount: i32) -> Effect {
    effects::next_turn_mana(json_as(json!({ "amount": amount })))
}

/// R1868000: a needed pick of one Unit, either side, whose Attack is below 4 × the play's X.
fn below_four_x() -> Vec<TargetDecl> {
    let mut decl = TargetDecl::target(1, 1, json!({ "of": ["unit"], "check": "belowFourX" }));
    decl.required = Some(true);
    vec![decl]
}

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            absolute_zero.id.clone(),
            both(Script {
                targets: a_unit(),
                cry: Some(hook(|_ctx| {
                    vec![freeze(json_as(
                        json!({ "target": { "of": "chosen" }, "forever": true }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            meteor.id.clone(),
            CardScripts {
                base: Script {
                    targets: a_unit(),
                    cry: Some(hook(|_ctx| vec![meteor_hit(15, "adjacent")])),
                    ..Script::default()
                },
                radiant: Script {
                    targets: a_unit(),
                    cry: Some(hook(|_ctx| vec![meteor_hit(20, "adjacentAndHero")])),
                    ..Script::default()
                },
            },
        ),
        (
            cloud.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![
                        give_next_spell_keyword(json_as(json!({ "keyword": { "kind": "Poisonous" } }))),
                        effects::damage_all(json_as(json!({ "amount": 1, "side": "enemy", "heroes": true }))),
                    ]
                })),
                ..Script::default()
            }),
        ),
        (
            ping.id.clone(),
            both(Script {
                targets: a_unit(),
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "chosen" }, "amount": 1 }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            nono.id.clone(),
            both(Script {
                cry: Some(hook(|ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": ctx.x }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            nono_book.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    (1..=4)
                        .map(|x| effects::add_to_hand(json_as(json!({ "defId": "sv-nono", "fixedX": x }))))
                        .collect()
                })),
                ..Script::default()
            }),
        ),
        (
            not_x_book.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::add_to_hand(json_as(
                        json!({ "defId": "sv-ping", "fixedX": 2 }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            core_system.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![next_turn_mana(-2)])),
                ..Script::default()
            }),
        ),
        (
            blackout.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![next_turn_mana(-5)])),
                ..Script::default()
            }),
        ),
        (
            wild_system.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![melee(Default::default()), next_turn_mana(1)])),
                ..Script::default()
            }),
        ),
        (
            asker.id.clone(),
            both(Script {
                death: Some(hook(|_ctx| vec![ask_controller("answered")])),
                resume: IndexMap::from([("answered", hook(|_ctx| Vec::new()))]),
                ..Script::default()
            }),
        ),
        (
            untouchable.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    cant_be_attacked: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            }),
        ),
        (
            stimmy.id.clone(),
            both(Script {
                cry: Some(hook(|ctx| {
                    let chance = inflation_chance_of(ctx.self_.as_ref(), INFLATION_CHANCE);
                    vec![
                        inflate(Default::default()),
                        effects::add_to_hand(json_as(json!({
                            "defId": "sv-stimmy",
                            "memory": { "inflationChance": next_inflation_chance(chance) },
                        }))),
                    ]
                })),
                ..Script::default()
            }),
        ),
        (
            hyperinflation.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![inflate(json_as(json!({ "chance": 100 })))])),
                ..Script::default()
            }),
        ),
        (
            stable.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![inflate(json_as(json!({ "chance": 0 })))])),
                ..Script::default()
            }),
        ),
        (
            sanctions.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![inflate(json_as(json!({ "player": "enemy" })))])),
                ..Script::default()
            }),
        ),
        (
            nono_x.id.clone(),
            both(Script {
                targets: below_four_x(),
                target_checks: IndexMap::from([(
                    "belowFourX",
                    target_check(|args| {
                        args.candidate
                            .is_some_and(|unit| unit_view(args.state, unit).attack < 4 * x_of(args.self_))
                    }),
                )]),
                cry: Some(hook(|_ctx| {
                    vec![effects::destroy(json_as(json!({ "target": { "of": "chosen" } })))]
                })),
                ..Script::default()
            }),
        ),
        (
            riser.id.clone(),
            both(Script {
                triggers: vec![
                    TriggerDef::new("sv-riser-rise", &[GameEventType::CardPlayed], |_ctx, _event| {
                        vec![effects::buff(json_as(
                            json!({ "target": { "of": "self" }, "attack": 2 }),
                        ))]
                    })
                    .with_when(|ctx, event| {
                        matches!(event, GameEvent::CardPlayed { player, .. } if *player != ctx.controller)
                    }),
                ],
                ..Script::default()
            }),
        ),
    ])
});

pub static SPELL_VERB_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        absolute_zero.clone(),
        meteor.clone(),
        cloud.clone(),
        ping.clone(),
        nono.clone(),
        nono_book.clone(),
        not_x_book.clone(),
        core_system.clone(),
        blackout.clone(),
        wild_system.clone(),
        asker.clone(),
        flyer.clone(),
        warded.clone(),
        untouchable.clone(),
        adept.clone(),
        stimmy.clone(),
        hyperinflation.clone(),
        stable.clone(),
        sanctions.clone(),
        nono_x.clone(),
        riser.clone(),
    ]
});

pub fn register_spell_verb_fixtures() {
    super::freeze::register_freeze_fixtures();
    let mut defs: CardDefs = registered_catalog().clone();
    for card in SPELL_VERB_DEFS.iter() {
        defs.insert(card.id.clone(), card.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}
