//! Test-only cards for Dormant (SPEC §6.1, §6.2 "When this awakens", §6.3 Awaken, R1839000–R1839003):
//! the Dormant Units Stinkyscape will print, in their shapes, and the cards that try to reach them.
//! Ids `sl-…` from 18390, so they collide with no other fixture file's.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects;
// The verbs and reads as a card script names them, through the prelude (CLAUDE.md rule 5).
use jackioh_engine::prelude::{active_units_of, awaken, units_on_field_of, when_this_awakens};
use jackioh_engine::testkit::*;

use super::combat::{big_body, plain};
use super::harness::both;

fn def(index: u32, name: &str, type_: &str, cost: i32, stats: (i32, i32), keywords: Value) -> CardDef {
    let (attack, health) = stats;
    let (face, radiant) = if type_ == "Unit" {
        (
            json!({ "attack": attack, "health": health, "keywords": keywords, "text": name }),
            json!({ "attack": 2 * attack, "health": 2 * health, "keywords": keywords, "text": name }),
        )
    } else {
        (
            json!({ "keywords": keywords, "text": name }),
            json!({ "keywords": keywords, "text": name }),
        )
    };
    json_as(json!({
        "id": format!("sl-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (Dormant fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": face,
        "radiant": radiant,
    }))
}

/// Stinkyscape's Imprisoned: "Dormant for 2 turns. Aura: your other Units have +1 Attack. Start of
/// turn: deal 1 damage to the enemy hero. When this awakens, deal 3 damage to the enemy hero."
pub static imprisoned: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        18390,
        "imprisoned",
        "Unit",
        0,
        (3, 3),
        json!([{ "kind": "Taunt" }]),
    )
});
/// The Dormant Starship: "Dormant. Activate: pay (3) to awaken. Activate: deal 1 damage to the enemy hero."
pub static starship: LazyLock<CardDef> =
    LazyLock::new(|| def(18391, "starship", "Unit", 0, (4, 4), json!([])));
/// "Dormant for 1 turn. Cry: deal 2 damage to the enemy hero, then give this +2/+2."
pub static napper: LazyLock<CardDef> = LazyLock::new(|| def(18392, "napper", "Unit", 0, (2, 2), json!([])));
/// "Aura: every Unit has +1/+1."
pub static banner: LazyLock<CardDef> = LazyLock::new(|| def(18393, "banner", "Unit", 0, (1, 1), json!([])));
/// "Stack. Cry: the cards beneath this become copies of it" (Classic+ #4).
pub static copycat: LazyLock<CardDef> =
    LazyLock::new(|| def(18394, "copycat", "Unit", 0, (1, 1), json!([{ "kind": "Stack" }])));

/// "Deal 2 damage to a Unit."
pub static snipe: LazyLock<CardDef> = LazyLock::new(|| def(18395, "snipe", "Spell", 0, (0, 0), json!([])));
/// "Deal 1 damage to every Unit."
pub static quake: LazyLock<CardDef> = LazyLock::new(|| def(18396, "quake", "Spell", 0, (0, 0), json!([])));
/// "Destroy every Unit."
pub static wipe: LazyLock<CardDef> = LazyLock::new(|| def(18397, "wipe", "Spell", 0, (0, 0), json!([])));
/// "Exile every Unit."
pub static void: LazyLock<CardDef> = LazyLock::new(|| def(18398, "void", "Spell", 0, (0, 0), json!([])));
/// "Return every Unit to its controller's hand."
pub static flood: LazyLock<CardDef> = LazyLock::new(|| def(18399, "flood", "Spell", 0, (0, 0), json!([])));
/// "Freeze every Unit."
pub static frost: LazyLock<CardDef> = LazyLock::new(|| def(18400, "frost", "Spell", 0, (0, 0), json!([])));
/// "Give every Unit +1/+1."
pub static cheer: LazyLock<CardDef> = LazyLock::new(|| def(18401, "cheer", "Spell", 0, (0, 0), json!([])));
/// "Take control of every enemy Unit."
pub static heist: LazyLock<CardDef> = LazyLock::new(|| def(18402, "heist", "Spell", 0, (0, 0), json!([])));
/// "Transform each enemy Unit into a Big Body" — each named by its id, read off the board.
pub static morph: LazyLock<CardDef> = LazyLock::new(|| def(18403, "morph", "Spell", 0, (0, 0), json!([])));
/// "Allure: steal the enemy Units at the start of your next turn" (§6.3 Allure).
pub static lure: LazyLock<CardDef> = LazyLock::new(|| def(18404, "lure", "Spell", 0, (0, 0), json!([])));
/// "Deal damage to the enemy hero equal to the number of Units you control."
pub static census: LazyLock<CardDef> = LazyLock::new(|| def(18405, "census", "Spell", 0, (0, 0), json!([])));
/// "Fill your board with Plain Bodies."
pub static swarm: LazyLock<CardDef> = LazyLock::new(|| def(18406, "swarm", "Spell", 0, (0, 0), json!([])));
/// "Awaken every card on your field" — the `awaken` verb from another card, each named by id.
pub static rouse: LazyLock<CardDef> = LazyLock::new(|| def(18407, "rouse", "Spell", 0, (0, 0), json!([])));
/// "Summon an Imprisoned" — a Dormant card arriving by a summon.
pub static call: LazyLock<CardDef> = LazyLock::new(|| def(18408, "call", "Spell", 0, (0, 0), json!([])));

/// The ability the Starship keeps while it is Dormant (R1839002).
pub const LAUNCH: &str = "launch";
/// The Starship's other ability, off while it is Dormant.
pub const ZAP: &str = "zap";
pub const LAUNCH_MANA: i32 = 3;
/// What the Imprisoned's awakening deals the enemy hero.
pub const AWAKENING_DAMAGE: i32 = 3;
/// What the Imprisoned's start-of-turn hook deals the enemy hero.
pub const START_OF_TURN_DAMAGE: i32 = 1;
/// What the Napper's Cry deals the enemy hero.
pub const CRY_DAMAGE: i32 = 2;

fn slumber_text(turns: Option<i32>, wakes_by: Option<&str>) -> Option<StaticFlags> {
    Some(StaticFlags {
        slumber: Some(SlumberText {
            turns,
            wakes_by: wakes_by.map(str::to_string),
        }),
        ..StaticFlags::default()
    })
}

fn to_enemy_hero(amount: i32) -> Effect {
    effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))
}

fn ability(id: &str, cost: Option<ActivationCost>, run: Hook) -> ActivationDecl {
    ActivationDecl {
        id: id.to_string(),
        label: id.to_string(),
        uses: ActivationUses::Count(1),
        cost,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run,
    }
}

fn other_friendly_attack_aura() -> AuraHook {
    aura_hook(|a| {
        let controller = a.self_.controller;
        let me = a.self_.id.clone();
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == controller
                    && unit.id != me
                    && matches!(unit.zone, Zone::Field { row: Row::Units, .. })
            }),
            mod_: StatMod {
                attack: Some(1),
                ..StatMod::default()
            },
        }]
    })
}

fn every_unit_aura() -> AuraHook {
    aura_hook(|_a| {
        vec![AuraEntry {
            applies: Box::new(|unit: &CardInstance| matches!(unit.zone, Zone::Field { row: Row::Units, .. })),
            mod_: StatMod {
                attack: Some(1),
                max_health: Some(1),
                ..StatMod::default()
            },
        }]
    })
}

fn spell(cry: Hook) -> CardScripts {
    both(Script {
        cry: Some(cry),
        ..Script::default()
    })
}

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            imprisoned.id.clone(),
            both(Script {
                static_flags: slumber_text(Some(2), None),
                aura: Some(other_friendly_attack_aura()),
                start_of_turn: Some(hook(|_ctx| vec![to_enemy_hero(START_OF_TURN_DAMAGE)])),
                triggers: vec![when_this_awakens("awakens", |_ctx, _event| {
                    vec![to_enemy_hero(AWAKENING_DAMAGE)]
                })],
                ..Script::default()
            }),
        ),
        (
            starship.id.clone(),
            both(Script {
                static_flags: slumber_text(None, Some(LAUNCH)),
                activations: vec![
                    ability(
                        LAUNCH,
                        Some(ActivationCost {
                            mana: Some(LAUNCH_MANA),
                            ..ActivationCost::default()
                        }),
                        hook(|_ctx| vec![awaken(json_as(json!({ "target": { "of": "self" } })))]),
                    ),
                    ability(ZAP, None, hook(|_ctx| vec![to_enemy_hero(1)])),
                ],
                ..Script::default()
            }),
        ),
        (
            napper.id.clone(),
            both(Script {
                static_flags: slumber_text(Some(1), None),
                cry: Some(hook(|_ctx| {
                    vec![
                        to_enemy_hero(CRY_DAMAGE),
                        effects::buff(json_as(
                            json!({ "target": { "of": "self" }, "attack": 2, "health": 2 }),
                        )),
                    ]
                })),
                ..Script::default()
            }),
        ),
        (
            banner.id.clone(),
            both(Script {
                aura: Some(every_unit_aura()),
                ..Script::default()
            }),
        ),
        (
            copycat.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![effects::transform_beneath(json_as(json!({})))])),
                ..Script::default()
            }),
        ),
        (
            snipe.id.clone(),
            both(Script {
                targets: vec![TargetDecl::target(1, 1, json!({ "of": ["unit"] }))],
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "chosen" }, "amount": 2 }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            quake.id.clone(),
            spell(hook(|_ctx| {
                vec![effects::damage_all(json_as(json!({ "amount": 1 })))]
            })),
        ),
        (
            wipe.id.clone(),
            spell(hook(|_ctx| vec![effects::destroy_all(json_as(json!({})))])),
        ),
        (
            void.id.clone(),
            spell(hook(|_ctx| vec![effects::exile_all(json_as(json!({})))])),
        ),
        (
            flood.id.clone(),
            spell(hook(|_ctx| vec![effects::bounce_all(json_as(json!({})))])),
        ),
        (
            frost.id.clone(),
            spell(hook(|_ctx| vec![effects::freeze_all(json_as(json!({})))])),
        ),
        (
            cheer.id.clone(),
            spell(hook(|_ctx| {
                vec![effects::buff_all_units(json_as(
                    json!({ "side": "both", "attack": 1, "health": 1 }),
                ))]
            })),
        ),
        (
            heist.id.clone(),
            spell(hook(|_ctx| vec![effects::steal_all(json_as(json!({})))])),
        ),
        (
            morph.id.clone(),
            spell(hook(|ctx| {
                // Every card topping an enemy unit zone, the Dormant one included, named by its id:
                // `transform` is what passes the Dormant one by (R1839000).
                units_on_field_of(ctx.state, opponent_of(ctx.controller))
                    .into_iter()
                    .map(|unit| {
                        effects::transform(json_as(json!({ "instanceId": unit.id, "defId": big_body.id })))
                    })
                    .collect()
            })),
        ),
        (
            lure.id.clone(),
            spell(hook(|_ctx| vec![effects::allure_enemy_units()])),
        ),
        (
            census.id.clone(),
            spell(hook(|ctx| {
                let units = active_units_of(ctx.state, ctx.controller).len() as i32;
                vec![to_enemy_hero(units)]
            })),
        ),
        (
            swarm.id.clone(),
            spell(hook(|_ctx| {
                vec![effects::fill_board(json_as(json!({ "defId": plain.id })))]
            })),
        ),
        (
            call.id.clone(),
            spell(hook(|_ctx| {
                vec![effects::summon(json_as(json!({ "defId": imprisoned.id })))]
            })),
        ),
        (
            rouse.id.clone(),
            spell(hook(|ctx| {
                units_on_field_of(ctx.state, ctx.controller)
                    .into_iter()
                    .map(|unit| {
                        awaken(json_as(
                            json!({ "target": { "of": "instance", "instanceId": unit.id } }),
                        ))
                    })
                    .collect()
            })),
        ),
    ])
});

pub static SLUMBER_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        imprisoned.clone(),
        starship.clone(),
        napper.clone(),
        banner.clone(),
        copycat.clone(),
        snipe.clone(),
        quake.clone(),
        wipe.clone(),
        void.clone(),
        flood.clone(),
        frost.clone(),
        cheer.clone(),
        heist.clone(),
        morph.clone(),
        lure.clone(),
        census.clone(),
        swarm.clone(),
        rouse.clone(),
        call.clone(),
    ]
});

pub fn register_slumber_fixtures() {
    super::harness::setup_catalog();
    let mut defs: CardDefs = registered_catalog().clone();
    for card in SLUMBER_DEFS.iter() {
        defs.insert(card.id.clone(), card.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}
