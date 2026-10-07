//! Test-only definitions and scripts for `effects/fruit.ts` (the Classic+ Fruit verbs: C+ #65, #65.2,
//! #65.3, #65.5, #66). The engine does not depend on `packages/cards`, so the five Grapes are stand-ins
//! under the ids `GRAPE_ODDS` names, with the Fruit tag and the Token rarity the real ones carry; the
//! real cards' tests cover the same cases again (packages/cards/test/classic-plus/065-*.test.ts).
//!
//! Port of `packages/engine/test/fixtures/fruit.ts`: each exported def is a `pub static` under TS's name
//! snake_cased (`grape_roller`, `priced_draw`, `cast_on_draw`).

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects::{
    add_rolled_grapes, damage_enemy_or_heal_friend, draw_priced, replace_hand_with_random,
};
use jackioh_engine::testkit::*;

use super::catalog::spell_def;

/// A Grape stand-in: a Fruit Spell token, as §7 prints each of the five.
fn grape_def(def_id: &str, at: i32) -> CardDef {
    spell_def(
        900 + at,
        json!({
            "id": def_id,
            "index": format!("65.{}", at + 1),
            "name": format!("Fixture Grape {}", at + 1),
            "set": "Classic+",
            "tags": ["Fruit", "Token"],
            "rarity": "Token",
            "token": true,
        }),
    )
}

/// Two Grapes' stand-in: "Add 3 Grapes", the Radiant face printing Lucky 1.
pub static grape_roller: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        910,
        json!({
            "id": "fx-fruit-roller",
            "name": "Fixture Grape Roller",
            "radiant": { "keywords": [{ "kind": "Lucky", "n": 1 }], "text": "Lucky 1. Add 3 Radiant Grapes." },
        }),
    )
});

/// Normal Grape's stand-in: a target, then one draw priced −1.
pub static priced_draw: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        911,
        json!({ "id": "fx-fruit-priced", "name": "Fixture Priced Draw" }),
    )
});

/// A cast-on-draw Spell that does nothing, to end a priced draw with no card in hand (R58).
pub static cast_on_draw: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        912,
        json!({ "id": "fx-fruit-cod", "name": "Fixture Cast On Draw" }),
    )
});

/// Mythic Grape's stand-in, and the one non-token "Mythic" its pool holds besides.
pub static replacer: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        913,
        json!({ "id": "fx-fruit-replacer", "name": "Fixture Replacer", "rarity": "Mythic" }),
    )
});
pub static mythic: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        914,
        json!({ "id": "fx-fruit-mythic", "name": "Fixture Mythic", "rarity": "Mythic" }),
    )
});

/// The five Grape stand-ins (under `GRAPE_ODDS`' ids, in its order) and the fixtures above, on top of `base`.
pub fn fruit_catalog(base: CardDefs) -> CardDefs {
    let mut defs = base;
    for (at, odds) in GRAPE_ODDS.iter().enumerate() {
        defs.insert(odds.def_id.to_string(), grape_def(odds.def_id, at as i32));
    }
    for entry in [
        &*grape_roller,
        &*priced_draw,
        &*cast_on_draw,
        &*replacer,
        &*mythic,
    ] {
        defs.insert(entry.id.clone(), entry.clone());
    }
    defs
}

fn priced_draw_targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(
        1,
        1,
        json!({ "side": "any", "of": ["unit", "hero"] }),
    )]
}

fn cast_on_draw_script() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub static FRUIT_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            grape_roller.id.clone(),
            CardScripts {
                base: Script {
                    cry: Some(hook(|_ctx| {
                        vec![add_rolled_grapes(json_as(json!({ "count": 3 })))]
                    })),
                    ..Script::default()
                },
                radiant: Script {
                    cry: Some(hook(|_ctx| {
                        vec![add_rolled_grapes(json_as(json!({ "count": 3, "radiant": true })))]
                    })),
                    ..Script::default()
                },
            },
        ),
        (
            priced_draw.id.clone(),
            CardScripts {
                base: Script {
                    targets: priced_draw_targets(),
                    cry: Some(hook(|_ctx| {
                        vec![
                            damage_enemy_or_heal_friend(json_as(json!({ "amount": 2 }))),
                            draw_priced(json_as(json!({ "costMod": -1 }))),
                        ]
                    })),
                    ..Script::default()
                },
                radiant: Script {
                    targets: priced_draw_targets(),
                    cry: Some(hook(|_ctx| {
                        vec![
                            damage_enemy_or_heal_friend(json_as(json!({ "amount": 4 }))),
                            draw_priced(json_as(json!({ "costOverride": 0 }))),
                            draw_priced(json_as(json!({ "costOverride": 0 }))),
                        ]
                    })),
                    ..Script::default()
                },
            },
        ),
        (
            cast_on_draw.id.clone(),
            CardScripts {
                base: cast_on_draw_script(),
                radiant: cast_on_draw_script(),
            },
        ),
        (
            replacer.id.clone(),
            CardScripts {
                base: Script {
                    cry: Some(hook(|_ctx| {
                        vec![replace_hand_with_random(json_as(json!({
                            "query": { "rarity": "Mythic" },
                            "costOverride": 0,
                        })))]
                    })),
                    ..Script::default()
                },
                radiant: Script {
                    cry: Some(hook(|_ctx| {
                        vec![replace_hand_with_random(json_as(json!({
                            "query": { "rarity": "Mythic" },
                            "costOverride": 0,
                            "radiant": true,
                        })))]
                    })),
                    ..Script::default()
                },
            },
        ),
    ])
});
