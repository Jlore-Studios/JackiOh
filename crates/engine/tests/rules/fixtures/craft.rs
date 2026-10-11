//! Test-only cards for ME-CRAFT (Meditative #17 True Craft a Card, R880–R883): `pm-crafter`
//! runs the card's own chain — a `number` prompt for the cost, then a `craft` prompt, then the
//! mint — so the engine is proved without `crates/cards` (CLAUDE.md: the engine never imports it).

use std::sync::LazyLock;

use jackioh_engine::effects::{choose_craft, choose_number, chosen_number, chosen_recipe, craft_card};
use jackioh_engine::testkit::*;

/// The card's script on a fixture Spell: the cost prompt, then the craft prompt, then the mint.
fn crafter_script(radiant: bool) -> Script {
    let open_editor: Hook = hook(move |ctx| match chosen_number(ctx) {
        Some(cost) => vec![choose_craft(json_as(json!({
            "step": "craft",
            "cost": cost,
            "radiant": radiant,
        })))],
        None => vec![],
    });
    let mint: Hook = hook(move |ctx| match chosen_recipe(ctx) {
        Some(recipe) => vec![craft_card(json_as(json!({
            "recipe": recipe,
            "radiant": radiant,
        })))],
        None => vec![],
    });
    Script {
        cry: Some(hook(|_ctx| {
            vec![choose_number(json_as(json!({
                "step": "cost",
                "from": 0,
                "to": CRAFT_MAX_COST,
                "prompt": "Choose the crafted card's Cost",
            })))]
        })),
        resume: IndexMap::from([("cost", open_editor), ("craft", mint)]),
        ..Script::default()
    }
}

/// `pm-crafter`: the fixture Spell the craft tests cast.
pub fn crafter() -> CardDef {
    json_as(json!({
        "id": "pm-crafter",
        "index": "3401",
        "name": "crafter (craft)",
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": "crafter" },
        "radiant": { "keywords": [], "text": "crafter" },
    }))
}

/// `pm-crafter-qd`: the same Spell with Quickdraw, so a replayed game deals it in the opening hand
/// (as `prompts.rs`'s `quickdraw_of` cards are).
pub fn quickdraw_crafter() -> CardDef {
    CardDef {
        id: "pm-crafter-qd".to_string(),
        index: "3402".to_string(),
        name: "crafter (craft, quickdraw)".to_string(),
        ..crafter()
    }
}

/// The crafter's script with `quickdraw` set on both faces.
fn quickdraw_crafter_script(radiant: bool) -> Script {
    let script = crafter_script(radiant);
    let mut flags = script.flags();
    flags.quickdraw = Some(true);
    Script {
        static_flags: Some(flags),
        ..script
    }
}

static CRAFTER_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            "pm-crafter".to_string(),
            CardScripts {
                base: crafter_script(false),
                radiant: crafter_script(true),
            },
        ),
        (
            "pm-crafter-qd".to_string(),
            CardScripts {
                base: quickdraw_crafter_script(false),
                radiant: quickdraw_crafter_script(true),
            },
        ),
    ])
});

/// This file's defs, by id.
pub fn catalog() -> IndexMap<String, CardDef> {
    IndexMap::from([
        (crafter().id.clone(), crafter()),
        (quickdraw_crafter().id.clone(), quickdraw_crafter()),
    ])
}

/// This file's scripts, by id.
pub fn scripts() -> IndexMap<String, CardScripts> {
    CRAFTER_SCRIPTS.clone()
}

/// Add these fixtures to whatever the harness registered (`newGame` registers its own first).
pub fn register_craft_fixtures() {
    let mut all_defs = registered_catalog().clone();
    all_defs.extend(catalog());
    register_catalog(all_defs);
    let mut all_scripts = registered_scripts().clone();
    all_scripts.extend(scripts());
    register_scripts(all_scripts);
}
