//! Test-only cards for Tradable and the Trade (R1838000–R1838003): a Tradable Spell, a Unit with no
//! text, and a Field Spell that lets nobody draw. The engine's proof; the shipped cards that print the
//! keyword (Meditative #53 Prestige, #98 Showdown) test it again in their own files.
//!
//! Ids are prefixed `trade-` and indexed from 18380 up, clear of every other fixture file (BUILD §0).

use std::sync::LazyLock;

use jackioh_engine::testkit::*;

use super::harness::both;

/// A Core Common card whose faces both print `keywords`; a Unit's are 2/2.
fn def(index: u32, name: &str, type_: &str, cost: i32, keywords: Value) -> CardDef {
    let face = if type_ == "Unit" {
        json!({ "attack": 2, "health": 2, "keywords": keywords, "text": name })
    } else {
        json!({ "keywords": keywords, "text": name })
    };
    json_as(json!({
        "id": format!("trade-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (#1838)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": face,
        "radiant": face,
    }))
}

/// A (3) Tradable Spell with no text of its own.
pub fn tradable_spell() -> CardDef {
    def(18380, "spell", "Spell", 3, json!([{ "kind": "Tradable" }]))
}

/// A (3) Unit with no text, Tradable only when an effect gives it the keyword.
pub fn plain_unit() -> CardDef {
    def(18381, "plain", "Unit", 3, json!([]))
}

/// A Field Spell whose draw limit lets neither player draw (B5 E3, R457).
pub fn no_draws() -> CardDef {
    def(18382, "no-draws", "Field Spell", 1, json!([]))
}

pub static TRADE_DEFS: LazyLock<Vec<CardDef>> =
    LazyLock::new(|| vec![tradable_spell(), plain_unit(), no_draws()]);

pub static TRADE_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let mut table: IndexMap<String, CardScripts> = IndexMap::new();
    table.insert(tradable_spell().id, both(Script::default()));
    table.insert(plain_unit().id, both(Script::default()));
    table.insert(
        no_draws().id,
        both(Script {
            draw_limit: Some(read_hook(|_args| {
                vec![DrawLimit {
                    player: DrawLimitPlayer::Both,
                    count: 0,
                }]
            })),
            ..Script::default()
        }),
    );
    table
});

/// Merge this file's cards into whatever catalog and scripts the test registered first.
pub fn register_tradable() {
    let mut defs = registered_catalog().clone();
    for entry in TRADE_DEFS.iter() {
        defs.insert(entry.id.clone(), entry.clone());
    }
    register_catalog(defs);
    let mut all_scripts = registered_scripts().clone();
    all_scripts.extend(TRADE_SCRIPTS.clone());
    register_scripts(all_scripts);
}
