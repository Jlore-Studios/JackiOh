//! Test-only definitions and scripts for ME-JADE and ME-ALLURE (Meditative #39.2 Jade, #39.4 Red
//! Jade, #39.5 Jade Beauty; docs/meditative-set.md M5, Group C's systems). The engine does not depend
//! on `jackioh-cards`, so the Jade Beauty and the three rolled tokens are stand-ins under the ids
//! `JADE_BEAUTY_DEF_ID` and `AUSPICIOUS_ROCK_ODDS` name, with the CN tag and the Token rarity the real
//! ones carry; the real cards' tests cover the same cases again
//! (`crates/cards/src/scripts/meditative/`).

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects::allure_enemy_units;
use jackioh_engine::testkit::*;

use super::catalog::{spell_def, unit_def};

/// A rolled-token stand-in: a CN Spell token, as §7 prints each of the three.
fn rolled_def(def_id: &str, at: i32) -> CardDef {
    spell_def(
        920 + at,
        json!({
            "id": def_id,
            "index": format!("39.{}", at + 2),
            "name": format!("Fixture Rolled {}", at + 2),
            "set": "Meditative",
            "tags": ["CN", "Token"],
            "rarity": "Token",
            "token": true,
            "cost": 0,
        }),
    )
}

/// Jade Beauty's stand-in: a 20/20 CN Unit token (its Radiant face doubles to 40/40, as the real
/// one's does), with no script — ME-JADE summons it with no Cry (R64).
fn beauty_def() -> CardDef {
    unit_def(
        930,
        json!({
            "id": JADE_BEAUTY_DEF_ID,
            "index": "39.5",
            "name": "Fixture Jade Beauty",
            "set": "Meditative",
            "tags": ["CN", "Token"],
            "rarity": "Token",
            "token": true,
            "cost": 10,
            "attack": 20,
            "health": 20,
        }),
    )
}

/// Jade Beauty's Allure as a Spell, so engine tests can drive ME-ALLURE without the counter: "Allure
/// every enemy Unit".
pub static jade_allure: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        931,
        json!({
            "id": "fx-jade-allure",
            "name": "Fixture Jade Allure",
            "cost": 0,
        }),
    )
});

/// The Jade Beauty and rolled-token stand-ins (under `JADE_BEAUTY_DEF_ID` and
/// `AUSPICIOUS_ROCK_ODDS`' ids, in its order) and the fixture above, on top of `base`.
pub fn jade_catalog(base: CardDefs) -> CardDefs {
    let mut defs = base;
    defs.insert(JADE_BEAUTY_DEF_ID.to_string(), beauty_def());
    for (at, odds) in AUSPICIOUS_ROCK_ODDS.iter().enumerate() {
        defs.insert(odds.def_id.to_string(), rolled_def(odds.def_id, at as i32));
    }
    defs.insert(jade_allure.id.clone(), jade_allure.clone());
    defs
}

pub static JADE_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([(
        jade_allure.id.clone(),
        CardScripts {
            base: Script {
                cry: Some(hook(|_ctx| vec![allure_enemy_units()])),
                ..Script::default()
            },
            radiant: Script {
                cry: Some(hook(|_ctx| vec![allure_enemy_units()])),
                ..Script::default()
            },
        },
    )])
});
