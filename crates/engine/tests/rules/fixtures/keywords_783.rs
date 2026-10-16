//! Test-only cards for #783's five keywords: Ranged, Flying, Cloaked, Crater and Flashback
//! (R783000–R783013).
//!
//! No catalog card prints them until #781's part 2, so these are the engine's proof: plain bodies
//! for the combat keywords, a Cloaked Unit, Spell, Field Spell and Trap, Field Spells whose auras lend
//! Cloaked to the cards in their controller's hand, the Trap price rules a Cloaked card must miss,
//! Crater on every kind of card and on the Spells that summon, Recruit and Flicker, and a Flashback
//! Spell and Unit.
//!
//! Ids are prefixed `kw-` and indexed from 7830 up, clear of every other fixture file (BUILD §0).

use std::sync::LazyLock;

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

/// TS's `{ ...a, ...b }` on two object literals: `b`'s keys replace `a`'s.
fn spread(mut base: Value, extra: Value) -> Value {
    if let (Some(into), Value::Object(from)) = (base.as_object_mut(), extra) {
        for (key, value) in from {
            into.insert(key, value);
        }
    }
    base
}

/// A Core Common card at cost 1 whose faces both print `keywords`.
fn def(index: u32, name: &str, type_: &str, cost: i32, keywords: Value) -> CardDef {
    json_as(json!({
        "id": format!("kw-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (#783)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": { "keywords": keywords, "text": name },
        "radiant": { "keywords": keywords, "text": name },
    }))
}

/// A Unit whose faces both print `keywords`; the Radiant face doubles the stats.
fn unit(index: u32, name: &str, cost: i32, attack: i32, health: i32, keywords: Value) -> CardDef {
    let faces = json!({
        "base": { "attack": attack, "health": health, "keywords": keywords, "text": name },
        "radiant": { "attack": attack * 2, "health": health * 2, "keywords": keywords, "text": name },
    });
    json_as(spread(
        serde_json::to_value(def(index, name, "Unit", cost, keywords)).expect("a def serialises"),
        faces,
    ))
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

// ---------------------------------------------------------------------------
// Ranged and Flying (R783000–R783003)
// ---------------------------------------------------------------------------

/// A Ranged 2/3.
pub fn ranged() -> CardDef {
    unit(7830, "ranged", 1, 2, 3, json!([{ "kind": "Ranged" }]))
}

/// A Ranged 2/3 with Cleave.
pub fn ranged_cleaver() -> CardDef {
    unit(
        7831,
        "ranged-cleaver",
        1,
        2,
        3,
        json!([{ "kind": "Ranged" }, { "kind": "Cleave" }]),
    )
}

/// A Ranged 4/2 with Trample.
pub fn ranged_trampler() -> CardDef {
    unit(
        7832,
        "ranged-trampler",
        1,
        4,
        2,
        json!([{ "kind": "Ranged" }, { "kind": "Trample" }]),
    )
}

/// A Ranged 2/2 with First Strike and Poisonous.
pub fn ranged_striker() -> CardDef {
    unit(
        7833,
        "ranged-striker",
        1,
        2,
        2,
        json!([{ "kind": "Ranged" }, { "kind": "First Strike" }, { "kind": "Poisonous" }]),
    )
}

/// A First Strike 3/1.
pub fn first_striker() -> CardDef {
    unit(
        7834,
        "first-striker",
        1,
        3,
        1,
        json!([{ "kind": "First Strike" }]),
    )
}

/// A Flying 2/3.
pub fn flyer() -> CardDef {
    unit(7835, "flyer", 1, 2, 3, json!([{ "kind": "Flying" }]))
}

/// A Flying 1/4 with Taunt.
pub fn flying_taunter() -> CardDef {
    unit(
        7836,
        "flying-taunter",
        1,
        1,
        4,
        json!([{ "kind": "Flying" }, { "kind": "Taunt" }]),
    )
}

/// A plain 3/3.
pub fn grunt() -> CardDef {
    unit(7837, "grunt", 1, 3, 3, json!([]))
}

/// A Spell that deals 1 to every unit.
pub fn quake() -> CardDef {
    def(7838, "quake", "Spell", 1, json!([]))
}

/// A Unit whose Cry deals 2 to a chosen enemy unit.
pub fn sniper() -> CardDef {
    unit(7839, "sniper", 1, 1, 1, json!([]))
}

// ---------------------------------------------------------------------------
// Cloaked (R783004–R783009)
// ---------------------------------------------------------------------------

/// A Cloaked 3/3 at (3) whose Cry deals 2 to a chosen unit or hero.
pub fn cloaked_unit() -> CardDef {
    unit(7840, "cloaked-unit", 3, 3, 3, json!([{ "kind": "Cloaked" }]))
}

/// A Cloaked Spell at (1) with a required unit target: 2 damage to it.
pub fn cloaked_bolt() -> CardDef {
    def(7841, "cloaked-bolt", "Spell", 1, json!([{ "kind": "Cloaked" }]))
}

/// A Cloaked Field Spell at (2) whose aura gives its controller's Units +1 Attack, and whose Cry deals 1
/// to the enemy hero.
pub fn cloaked_field() -> CardDef {
    def(
        7842,
        "cloaked-field",
        "Field Spell",
        2,
        json!([{ "kind": "Cloaked" }]),
    )
}

/// A Trap that prints Cloaked, which a Trap may never use.
pub fn cloaked_trap() -> CardDef {
    def(7843, "cloaked-trap", "Trap", 1, json!([{ "kind": "Cloaked" }]))
}

/// "Aura: The Units in your hand have Cloaked."
pub fn cloak_units() -> CardDef {
    def(7844, "cloak-units", "Field Spell", 1, json!([]))
}

/// "Aura: The Spells in your hand have Cloaked."
pub fn cloak_spells() -> CardDef {
    def(7845, "cloak-spells", "Field Spell", 1, json!([]))
}

/// A vanilla 2/2 at (2).
pub fn plain_unit() -> CardDef {
    unit(7846, "plain-unit", 2, 2, 2, json!([]))
}

/// A Spell at (1) that deals 1 to the enemy hero.
pub fn plain_spell() -> CardDef {
    def(7847, "plain-spell", "Spell", 1, json!([]))
}

/// Classic #6 Cloaked Toe Cracker's shape: "Aura: Your Traps cost (0)".
pub fn toe_cracker() -> CardDef {
    unit(7848, "toe-cracker", 2, 3, 4, json!([]))
}

/// "Cry: Your next Trap costs (2) less." — Classic #2's rule, narrowed to Traps and Field Traps.
pub fn trap_discount() -> CardDef {
    unit(7849, "trap-discount", 1, 1, 1, json!([]))
}

// ---------------------------------------------------------------------------
// Crater (R783010, R783011)
// ---------------------------------------------------------------------------

/// A Crater 2/2 whose Cry deals 1 to the enemy hero.
pub fn crater_unit() -> CardDef {
    unit(7850, "crater-unit", 1, 2, 2, json!([{ "kind": "Crater" }]))
}

/// A Crater 1/1 with Reborn.
pub fn crater_reborn() -> CardDef {
    unit(
        7851,
        "crater-reborn",
        1,
        1,
        1,
        json!([{ "kind": "Crater" }, { "kind": "Reborn" }]),
    )
}

/// A Crater 2/2 with Cloaked.
pub fn crater_cloaked() -> CardDef {
    unit(
        7852,
        "crater-cloaked",
        1,
        2,
        2,
        json!([{ "kind": "Crater" }, { "kind": "Cloaked" }]),
    )
}

/// A Crater Trap: when the enemy plays a card, it deals 1 to the enemy hero.
pub fn crater_trap() -> CardDef {
    def(7853, "crater-trap", "Trap", 1, json!([{ "kind": "Crater" }]))
}

/// A Crater Trap that fires as it replaces (§5.1): an enemy's heal becomes Pierce damage (Blood Moon's
/// shape).
pub fn crater_moon() -> CardDef {
    def(7867, "crater-moon", "Trap", 1, json!([{ "kind": "Crater" }]))
}

/// A Spell that summons a Crater 2/2.
pub fn summon_crater() -> CardDef {
    def(7854, "summon-crater", "Spell", 1, json!([]))
}

/// A Spell that Recruits.
pub fn recruiter() -> CardDef {
    def(7855, "recruiter", "Spell", 1, json!([]))
}

/// A Spell that Flickers every unit of its controller's.
pub fn flicker_all() -> CardDef {
    def(7856, "flicker-all", "Spell", 1, json!([]))
}

/// A Stack 1/1.
pub fn stacker() -> CardDef {
    unit(7857, "stacker", 1, 1, 1, json!([{ "kind": "Stack" }]))
}

/// A Flying 2/2 with Stack (R1081000: offered every zone of its side).
pub fn flying_stacker() -> CardDef {
    unit(
        7868,
        "flying-stacker",
        1,
        2,
        2,
        json!([{ "kind": "Flying" }, { "kind": "Stack" }]),
    )
}

/// A Crater Spell, on which Crater does nothing: 1 to the enemy hero.
pub fn crater_spell() -> CardDef {
    def(7858, "crater-spell", "Spell", 1, json!([{ "kind": "Crater" }]))
}

/// A Crater Field Spell, on which Crater does nothing.
pub fn crater_field() -> CardDef {
    def(
        7859,
        "crater-field",
        "Field Spell",
        1,
        json!([{ "kind": "Crater" }]),
    )
}

// ---------------------------------------------------------------------------
// Flashback (R783012, R783013)
// ---------------------------------------------------------------------------

/// A Flashback Spell at (2): 2 to the enemy hero.
pub fn flashback_spell() -> CardDef {
    def(
        7860,
        "flashback-spell",
        "Spell",
        2,
        json!([{ "kind": "Flashback" }]),
    )
}

/// A Flashback 2/2 at (2) whose Cry deals 1 to the enemy hero.
pub fn flashback_unit() -> CardDef {
    unit(7861, "flashback-unit", 2, 2, 2, json!([{ "kind": "Flashback" }]))
}

/// "You may play cards from your graveyard" (Classic #28 Second Wind's shape).
pub fn second_wind() -> CardDef {
    unit(7862, "second-wind", 1, 1, 3, json!([]))
}

/// "You may play cards from your graveyard that cost (3) or more" (Radiant Second Wind's shape, at a
/// higher floor than the Flashback cards' (2)).
pub fn picky_wind() -> CardDef {
    unit(7865, "picky-wind", 1, 1, 3, json!([]))
}

/// "You may play Units from your graveyard, paid with this card's Plague Counters" (Classic #74 Corpse
/// Plantation's shape).
pub fn plague_field() -> CardDef {
    def(7866, "plague-field", "Field Spell", 1, json!([]))
}

/// A Field Trap that prints Cloaked, which a Field Trap may never use.
pub fn cloaked_field_trap() -> CardDef {
    def(
        7864,
        "cloaked-field-trap",
        "Field Trap",
        1,
        json!([{ "kind": "Cloaked" }]),
    )
}

/// A Cloaked 1/1 whose Cry and Death each deal 1 to the enemy hero.
pub fn cloaked_mourner() -> CardDef {
    unit(7863, "cloaked-mourner", 1, 1, 1, json!([{ "kind": "Cloaked" }]))
}

pub static KW_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        ranged(),
        ranged_cleaver(),
        ranged_trampler(),
        ranged_striker(),
        first_striker(),
        flyer(),
        flying_taunter(),
        grunt(),
        quake(),
        sniper(),
        cloaked_unit(),
        cloaked_bolt(),
        cloaked_field(),
        cloaked_trap(),
        cloak_units(),
        cloak_spells(),
        plain_unit(),
        plain_spell(),
        toe_cracker(),
        trap_discount(),
        crater_unit(),
        crater_reborn(),
        crater_cloaked(),
        crater_trap(),
        crater_moon(),
        summon_crater(),
        recruiter(),
        flicker_all(),
        stacker(),
        flying_stacker(),
        crater_spell(),
        crater_field(),
        flashback_spell(),
        flashback_unit(),
        second_wind(),
        picky_wind(),
        plague_field(),
        cloaked_mourner(),
        cloaked_field_trap(),
    ]
});

/// 1 to the enemy hero.
fn ping() -> Vec<Effect> {
    vec![effects::damage(json_as(
        json!({ "to": { "of": "enemyHero" }, "amount": 1 }),
    ))]
}

/// "Aura: The cards of `type_` in your hand have Cloaked."
fn cloak_hand(type_: CardType) -> AuraHook {
    aura_hook(move |a| {
        let controller = a.self_.controller;
        let state = a.state;
        vec![AuraEntry {
            applies: Box::new(move |card: &CardInstance| {
                matches!(card.zone, Zone::Hand { player } if player == controller)
                    && jackioh_engine::faces::card_type_of_face(state, &card.def_id, card.radiant) == type_
            }),
            mod_: StatMod {
                keywords: Some(vec![Keyword::Cloaked]),
                ..StatMod::default()
            },
        }]
    })
}

pub static KW_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let mut table: IndexMap<String, CardScripts> = IndexMap::new();
    for card in [
        ranged(),
        ranged_cleaver(),
        ranged_trampler(),
        ranged_striker(),
        first_striker(),
        flyer(),
        flying_taunter(),
        grunt(),
        plain_unit(),
        cloaked_trap(),
        cloaked_field_trap(),
        crater_reborn(),
        crater_cloaked(),
        stacker(),
        flying_stacker(),
        crater_field(),
    ] {
        table.insert(card.id, both(Script::default()));
    }
    table.insert(
        quake().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::damage_all(json_as(
                    json!({ "amount": 1, "side": "any" }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        sniper().id,
        both(Script {
            targets: vec![TargetDecl::target(
                1,
                1,
                json!({ "side": "enemy", "of": ["unit"] }),
            )],
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "chosen" }, "amount": 2 }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        cloaked_unit().id,
        both(Script {
            targets: vec![TargetDecl::target(1, 1, json!({ "of": ["unit", "hero"] }))],
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "chosen" }, "amount": 2 }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        cloaked_bolt().id,
        both(Script {
            targets: vec![TargetDecl {
                required: Some(true),
                ..TargetDecl::target(1, 1, json!({ "of": ["unit"] }))
            }],
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "chosen" }, "amount": 2 }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        cloaked_field().id,
        both(Script {
            cry: Some(hook(|_ctx| ping())),
            aura: Some(aura_hook(|a| {
                let controller = a.self_.controller;
                vec![AuraEntry {
                    applies: Box::new(move |unit: &CardInstance| {
                        unit.controller == controller
                            && matches!(unit.zone, Zone::Field { row: Row::Units, .. })
                    }),
                    mod_: StatMod {
                        attack: Some(1),
                        ..StatMod::default()
                    },
                }]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        cloak_units().id,
        both(Script {
            aura: Some(cloak_hand(CardType::Unit)),
            ..Script::default()
        }),
    );
    table.insert(
        cloak_spells().id,
        both(Script {
            aura: Some(cloak_hand(CardType::Spell)),
            ..Script::default()
        }),
    );
    table.insert(
        plain_spell().id,
        both(Script {
            cry: Some(hook(|_ctx| ping())),
            ..Script::default()
        }),
    );
    table.insert(
        toe_cracker().id,
        both(Script {
            cost_aura: Some(read_hook(|_args| {
                json_as::<Vec<CostAura>>(
                    json!([{ "whose": "yours", "types": ["Trap", "Field Trap"], "setTo": 0 }]),
                )
            })),
            ..Script::default()
        }),
    );
    table.insert(
        trap_discount().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::add_cost_rule(json_as(json!({
                    "rule": { "types": ["Trap", "Field Trap"], "amount": -2 },
                    "lasts": "used",
                })))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        crater_unit().id,
        both(Script {
            cry: Some(hook(|_ctx| ping())),
            ..Script::default()
        }),
    );
    table.insert(
        crater_trap().id,
        both(Script {
            triggers: vec![
                TriggerDef::new("kw-crater-trap", &[GameEventType::CardPlayed], |_ctx, _event| ping())
                    .with_when(|ctx, event| {
                        matches!(event, GameEvent::CardPlayed { player, .. } if *player != ctx.controller)
                    }),
            ],
            ..Script::default()
        }),
    );
    table.insert(
        crater_moon().id,
        both(Script {
            replacements: vec![ReplacementDef {
                id: "crater-moon".to_string(),
                on: ReplacementMoment::Healed,
                where_: None,
                when: None,
                instead: json_as(json!({ "damage": "pierce" })),
                then: None,
                by: None,
            }],
            ..Script::default()
        }),
    );
    table.insert(
        summon_crater().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::summon(json_as(json!({ "defId": crater_unit().id })))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        recruiter().id,
        both(Script {
            cry: Some(hook(|_ctx| vec![effects::recruit(Default::default())])),
            ..Script::default()
        }),
    );
    table.insert(
        flicker_all().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::flicker::flicker(json_as(
                    json!({ "scope": { "side": "self" } }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        crater_spell().id,
        both(Script {
            cry: Some(hook(|_ctx| ping())),
            ..Script::default()
        }),
    );
    table.insert(
        flashback_spell().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": 2 }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        flashback_unit().id,
        both(Script {
            cry: Some(hook(|_ctx| ping())),
            ..Script::default()
        }),
    );
    table.insert(
        cloaked_mourner().id,
        both(Script {
            cry: Some(hook(|_ctx| ping())),
            death: Some(hook(|_ctx| ping())),
            ..Script::default()
        }),
    );
    table.insert(
        second_wind().id,
        both(Script {
            graveyard_play: Some(read_hook(|_args| vec![GraveyardPlayPermission::default()])),
            ..Script::default()
        }),
    );
    table.insert(
        picky_wind().id,
        both(Script {
            graveyard_play: Some(read_hook(|_args| {
                vec![GraveyardPlayPermission {
                    min_price: Some(3),
                    ..GraveyardPlayPermission::default()
                }]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        plague_field().id,
        both(Script {
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
    table
});

/// Merge this file's cards into whatever catalog and scripts the test registered first.
pub fn register_keywords_783() {
    let mut defs = registered_catalog().clone();
    for entry in KW_DEFS.iter() {
        defs.insert(entry.id.clone(), entry.clone());
    }
    register_catalog(defs);
    let mut all_scripts = registered_scripts().clone();
    all_scripts.extend(KW_SCRIPTS.clone());
    register_scripts(all_scripts);
}
