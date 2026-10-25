//! Test-only cards for the starscape map (#1847, R1847000–R1847002): Stinkyscape #148 Alteran Reach's
//! shape, a Field Spell whose abilities travel its controller's map (`subsystems::starscape_abilities`),
//! and the card the edge adds, #148.1 Edge of the Universe's. The engine does not depend on
//! `jackioh-cards`, and no Stinkyscape card exists before the set ships; the real cards' tests will
//! cover the same cases again.
//!
//! Ids are prefixed `ss-` and indexed from 18470, so they collide with no other fixture file's.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::subsystems;
use jackioh_engine::testkit::*;

use super::catalog::spell_def;

/// The fixture Reach's id.
pub const REACH_ID: &str = "ss-reach";
/// The card the edge adds.
pub const EDGE_ID: &str = "ss-edge";

/// A Field Spell: "Activate ♾️ (1): Travel one node of your starscape map"; Radiant, the first travel
/// each turn free.
pub static reach: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        18470,
        json!({ "id": REACH_ID, "name": "Fixture Starscape Reach", "type": "Field Spell", "cost": 1 }),
    )
});

/// The edge's card: a Spell token that does nothing.
pub static edge: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        18471,
        json!({
            "id": EDGE_ID,
            "name": "Fixture Edge",
            "tags": ["Token"],
            "rarity": "Token",
            "token": true,
            "cost": 0,
        }),
    )
});

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([(
        reach.id.clone(),
        CardScripts {
            base: Script {
                activations: subsystems::starscape_abilities(false, EDGE_ID),
                ..Script::default()
            },
            radiant: Script {
                activations: subsystems::starscape_abilities(true, EDGE_ID),
                ..Script::default()
            },
        },
    )])
});

/// The engine's fixture catalog (`harness::setup_catalog`) with these cards and scripts on top.
pub fn register_starscape_fixtures() {
    super::harness::setup_catalog();
    let mut defs: CardDefs = registered_catalog().clone();
    for def in [reach.clone(), edge.clone()] {
        defs.insert(def.id.clone(), def);
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}
