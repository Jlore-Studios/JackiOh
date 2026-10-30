//! Test-only cards for Tradable and the Trade (R1838000–R1838003) and Stinkyscape's trade cards
//! (R1864002–R1864004); the cards that print them (Meditative #53 Prestige, #98 Showdown, Stinkyscape #164
//! The Syndicate and #165 The Trade Union) test them again in their own files. Ids are `trade-` from 18380
//! up, clear of every other fixture file (BUILD §0).

use std::sync::LazyLock;

use jackioh_engine::effects::exile;
use jackioh_engine::testkit::*;

use super::harness::both;

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

/// R1864004: the Trades a turn `two_trades` gives its controller.
pub const FIXTURE_TRADES: i32 = 2;

/// R1864002: a Field Spell whose aura lends Tradable to every card in its controller's hand.
pub fn lender() -> CardDef {
    def(18383, "lender", "Field Spell", 1, json!([]))
}

/// R1864004: a Field Spell that gives its controller `FIXTURE_TRADES` Trades a turn while it acts.
pub fn two_trades() -> CardDef {
    def(18384, "two-trades", "Field Spell", 1, json!([]))
}

/// R1864003: a Field Spell that, on its controller's `traded`, exiles the card the Trade put in the deck.
pub fn trade_watcher() -> CardDef {
    def(18385, "watcher", "Field Spell", 1, json!([]))
}

pub static TRADE_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        tradable_spell(),
        plain_unit(),
        no_draws(),
        lender(),
        two_trades(),
        trade_watcher(),
    ]
});

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
    table.insert(
        lender().id,
        both(Script {
            aura: Some(tradable_hand_aura()),
            ..Script::default()
        }),
    );
    table.insert(
        two_trades().id,
        both(Script {
            trades_per_turn: Some(read_hook(|_args| FIXTURE_TRADES)),
            ..Script::default()
        }),
    );
    table.insert(
        trade_watcher().id,
        both(Script {
            triggers: vec![
                TriggerDef::new("watch", &[GameEventType::Traded], |ctx, _event| {
                    traded_card_of(ctx)
                        .map(|id| {
                            vec![exile(json_as(json!({
                                "target": { "of": "instance", "instanceId": id },
                            })))]
                        })
                        .unwrap_or_default()
                })
                .with_when(
                    |ctx, event| matches!(event, GameEvent::Traded { player } if *player == ctx.controller),
                ),
            ],
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
