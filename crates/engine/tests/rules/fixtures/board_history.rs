//! Fixtures for E29's board snapshots (docs/classic-sets.md B5 E29; C+ #35 Rollback, R419, R563): test-only
//! scripts in the card's shape, since the engine never imports `packages/cards`. Defs are prefixed `bh-`
//! and indexed from 9700, so they collide with no other file's catalog (BUILD §0).
//!
//! Port of `packages/engine/test/fixtures/boardHistory.ts`: each exported def is a `pub static` under
//! TS's name (`rewind.id`, `phoenix.id`), registered on top of whatever the testkit override holds.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects::{chosen_number, roll_back};
use jackioh_engine::testkit::*;

/// C+ #35 Rollback's shape: N declared with the play (R81), both sides, at (0).
pub static rewind: LazyLock<CardDef> = LazyLock::new(|| {
    json_as(json!({
        "id": "bh-rewind",
        "index": "9701",
        "name": "rewind (board history fixture)",
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": "" },
        "radiant": { "keywords": [], "text": "" },
    }))
});

/// A Reborn Unit whose Death rolls the board back one turn while its own zone is held for it (R64, R563).
pub static phoenix: LazyLock<CardDef> = LazyLock::new(|| {
    let mut def: Value = serde_json::to_value(&*rewind).expect("a CardDef is JSON");
    if let Some(object) = def.as_object_mut() {
        object.insert("id".into(), json!("bh-phoenix"));
        object.insert("index".into(), json!("9702"));
        object.insert("name".into(), json!("phoenix (board history fixture)"));
        object.insert("type".into(), json!("Unit"));
        object.insert("cost".into(), json!(1));
        object.insert(
            "base".into(),
            json!({ "attack": 2, "health": 2, "keywords": [{ "kind": "Reborn" }], "text": "Reborn" }),
        );
        object.insert(
            "radiant".into(),
            json!({ "attack": 4, "health": 4, "keywords": [{ "kind": "Reborn" }], "text": "Reborn" }),
        );
    }
    json_as(def)
});

fn rewind_script() -> Script {
    Script {
        modes: vec![ModeDecl {
            kind: PromptKind::Number,
            options: (0..ROLLBACK_MAX_TURNS).map(|n| (n + 1).to_string()).collect(),
        }],
        cry: Some(hook(|ctx| {
            let turns_ago = chosen_number(&*ctx).unwrap_or(ROLLBACK_MAX_TURNS);
            vec![roll_back(json_as(
                json!({ "turnsAgo": turns_ago, "sides": "both" }),
            ))]
        })),
        ..Script::default()
    }
}

fn phoenix_script() -> Script {
    Script {
        death: Some(hook(|_ctx| {
            vec![roll_back(json_as(json!({ "turnsAgo": 1, "sides": "both" })))]
        })),
        ..Script::default()
    }
}

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            rewind.id.clone(),
            CardScripts {
                base: rewind_script(),
                radiant: rewind_script(),
            },
        ),
        (
            phoenix.id.clone(),
            CardScripts {
                base: phoenix_script(),
                radiant: phoenix_script(),
            },
        ),
    ])
});

/// Adds the fixtures to whatever catalog and scripts are registered.
pub fn register_board_history_fixtures() {
    let mut defs: CardDefs = registered_catalog().clone();
    defs.insert(rewind.id.clone(), rewind.clone());
    defs.insert(phoenix.id.clone(), phoenix.clone());
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}
