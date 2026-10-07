//! Fixture cards for a kill credited to another unit (R42, R412: `effects/killCredit.ts`) and for
//! `isCastOnDraw` (R58: `castOnDrawNow.ts`), shaped like Classic+ #19.2 Jungle Loser, #19.5 Bot Loser
//! and #26 Tommy Tempo. The engine never imports `packages/cards` (CLAUDE.md). Ids `kc-…`, indexed from
//! 5800 so they collide with no other fixture file.
//!
//! Port of `packages/engine/test/fixtures/killCredit.ts`. TS's module counter (`nextIndex`, from 5800,
//! one per `unit` call in file order) is each def's stated index.

#![allow(non_upper_case_globals)]

use std::sync::{Arc, LazyLock};

use jackioh_engine::effects::{WithKillCreditArgs, buff, forced_attack_random, go_berserk, remember, with_kill_credit};
use jackioh_engine::testkit::*;

/// TS `unit(name, attack, health)`: a Core Common Unit at cost 0, both faces alike.
fn unit(name: &str, index: i32, attack: i32, health: i32) -> CardDef {
    let face = json!({ "attack": attack, "health": health, "keywords": [], "text": name });
    json_as(json!({
        "id": format!("kc-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (kill credit)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face,
        "radiant": face,
    }))
}

/// Classic+ #19.5's shape: "Whenever this destroys a Unit, it gets +5 Attack."
pub static bot: LazyLock<CardDef> = LazyLock::new(|| unit("bot", 5801, 5, 5));
/// Classic+ #19.2's shape with no roll: at end of turn it attacks a random enemy Unit; base sends the bot across Berserk, Radiant credits it the kill.
pub static jungle: LazyLock<CardDef> = LazyLock::new(|| unit("jungle", 5802, 5, 5));
/// Classic+ #26's shape: it casts itself on draw and remembers whether its Cry ran as that cast.
pub static tempo: LazyLock<CardDef> = LazyLock::new(|| unit("tempo", 5803, 1, 1));

/// Each `bot` of the controller's with the enemy Unit across from it.
fn across_from_bots(state: &GameState, controller: PlayerId) -> Vec<KillCredit> {
    slots_of(controller, Row::Units)
        .into_iter()
        .flat_map(|at| {
            let Some(mine) = card_at(state, &at) else {
                return vec![];
            };
            if mine.def_id != bot.id {
                return vec![];
            }
            let across = ZoneSlot {
                player: opponent_of(controller),
                row: Row::Units,
                lane: at.lane,
            };
            match card_at(state, &across) {
                None => vec![],
                Some(victim) => vec![KillCredit {
                    victim_id: victim.id.clone(),
                    to_id: mine.id.clone(),
                }],
            }
        })
        .collect()
}

fn jungle_face(transfer: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |_ctx| {
            vec![with_kill_credit(WithKillCreditArgs {
                killer: json_as(json!({ "of": "self" })),
                pairs: Arc::new(|ctx: &mut EffectContext<'_>, _killer: &CardInstance| {
                    across_from_bots(ctx.state, ctx.controller)
                }),
                transfer,
                during: forced_attack_random(json_as(json!({ "attacker": { "of": "self" }, "among": "enemyUnits" }))),
                then: if transfer {
                    None
                } else {
                    Some(Arc::new(|pair: &KillCredit| {
                        vec![go_berserk(json_as(json!({
                            "target": { "of": "instance", "instanceId": pair.to_id },
                        })))]
                    }))
                },
            })]
        })),
        ..Script::default()
    }
}

fn bot_script() -> Script {
    Script {
        triggers: vec![TriggerDef::new("kc-bot-kill", &[GameEventType::Destroyed], |ctx, event| {
            let killed_by_self = match (event, ctx.self_.as_ref()) {
                (GameEvent::Destroyed { killer_id, .. }, Some(me)) => killer_id.as_deref() == Some(me.id.as_str()),
                _ => false,
            };
            if killed_by_self {
                vec![buff(json_as(json!({ "target": { "of": "self" }, "attack": 5 })))]
            } else {
                vec![]
            }
        })],
        ..Script::default()
    }
}

fn tempo_script() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| {
            let value = match ctx.self_.as_ref() {
                Some(me) => is_cast_on_draw(ctx.state, me),
                None => false,
            };
            vec![remember(json_as(json!({ "key": "castOnDraw", "value": value })))]
        })),
        ..Script::default()
    }
}

pub static KC_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            bot.id.clone(),
            CardScripts {
                base: bot_script(),
                radiant: bot_script(),
            },
        ),
        (
            jungle.id.clone(),
            CardScripts {
                base: jungle_face(false),
                radiant: jungle_face(true),
            },
        ),
        (
            tempo.id.clone(),
            CardScripts {
                base: tempo_script(),
                radiant: tempo_script(),
            },
        ),
    ])
});

/// Adds these cards to whatever catalog and scripts are registered (after the base fixture's setup).
pub fn register_kill_credit() {
    let mut defs: CardDefs = registered_catalog().clone();
    for card in [&*bot, &*jungle, &*tempo] {
        defs.insert(card.id.clone(), card.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(KC_SCRIPTS.clone());
    register_scripts(merged);
}

/// Part 24's brief, step 2: this file's test catalog (the three defs).
pub fn catalog() -> CardDefs {
    [&*bot, &*jungle, &*tempo]
        .into_iter()
        .map(|card| (card.id.clone(), card.clone()))
        .collect()
}

/// Part 24's brief, step 2: this file's scripts (`KC_SCRIPTS`).
pub fn scripts() -> IndexMap<String, CardScripts> {
    KC_SCRIPTS.clone()
}
