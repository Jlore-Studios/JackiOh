//! C+ #73 Call to Chaos (Classic+ Edition) (SPEC §8.7 row 73, R28, R87, R380, R382, R387, R423, R436): its
//! table of ten effects, which Core #95's subsystem (`call_to_chaos.rs`) rolls, announces and resolves —
//! `call_to_chaos({ radiant, table: CHAOS_PLUS_EFFECTS })` — so one rule serves both editions: the base face
//! rolls one entry, the Radiant three different ones resolved in the list's order (R423), the recursion
//! counts casts of either edition against `CALL_TO_CHAOS_CHAIN_CAP` (R28) and at the cap resolves into
//! nothing (R87), and both players are told what was rolled (`chaosRolled`, R436).
//!
//! Every entry is built when it resolves, not when the Cry returns (a part of the list, `lazy_part`), as
//! Core's are: a recursion rolled before it resolves its whole chain first and changes the hand, the deck
//! and the board each later entry reads (R87), and a pause inside one parks the rest of it (R113).
//!
//! Port of `packages/engine/src/subsystems/callToChaosPlus.ts`. TS's `addFree(name, pool, count)`
//! returned the entry's builder; here each entry's builder is a named function (`build_fruits`, …), so
//! the table is a constant like Core's.

use std::borrow::Borrow;
use std::sync::Arc;

use serde_json::{Value, json};

use crate::config::{
    CHAOS_PLUS_BOOKS, CHAOS_PLUS_CLASSIC_CARDS, CHAOS_PLUS_COST, CHAOS_PLUS_DEGRADES, CHAOS_PLUS_FRUITS,
    CHAOS_PLUS_UPGRADES,
};
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext, EffectPart, Memo};
use crate::state::CardInstance;
use crate::subsystems::call_to_chaos::{CHAOS_TAG, ChaosEffectDef, cast_random_call_to_chaos};
use crate::wire::{CardDef, SetName};

/// §7: the Classic Golem (C+ #73.1), by its index in its set (B2.2).
const GOLEM_SET: SetName = SetName::ClassicPlus;
const GOLEM_INDEX: &str = "73.1";

/// The definitions a catalog read hands back, as owned copies.
fn owned_defs<R: Borrow<CardDef>>(defs: impl IntoIterator<Item = R>) -> Vec<CardDef> {
    defs.into_iter().map(|def| def.borrow().clone()).collect()
}

/// `callToChaosPlus:<name>`, the kind of an entry's part (an effect's kind is a static name).
fn entry_kind(name: &str) -> &'static str {
    match name {
        "fruits" => "callToChaosPlus:fruits",
        "books" => "callToChaosPlus:books",
        "destroy" => "callToChaosPlus:destroy",
        "classic" => "callToChaosPlus:classic",
        "upgrade" => "callToChaosPlus:upgrade",
        "fuse" => "callToChaosPlus:fuse",
        "degrade" => "callToChaosPlus:degrade",
        "golem" => "callToChaosPlus:golem",
        "replace" => "callToChaosPlus:replace",
        _ => "callToChaosPlus",
    }
}

/// Apply an effect list in order; once a state check inside it has ended the game, the rest does not
/// resolve (R216). A private copy of `resolve::apply_effects`.
fn apply_effects(effects: &[Effect], ctx: &mut EffectContext<'_>) {
    for effect in effects {
        if ctx.sink.state.result.is_some() {
            return;
        }
        (effect.apply)(ctx);
    }
}

/// A part of a composed list (`Effect.expand`), built when the list reaches it rather than when the
/// list is made (R102). Applied on its own it builds and applies its effects in one go; inside
/// `prompts::apply_resumable` it runs as a nested list a prompt can pause. A private copy of
/// `resolve::lazy_part`.
fn lazy_part(
    kind: &'static str,
    expand: impl Fn(&mut EffectContext<'_>, &Memo) -> EffectPart + Send + Sync + 'static,
) -> Effect {
    let expand = Arc::new(expand);
    let build = expand.clone();
    Effect::with_expand(
        kind,
        move |ctx| {
            let part = build(ctx, &None);
            apply_effects(&part.effects, ctx);
        },
        move |ctx, memo| expand(ctx, memo),
    )
}

/// One entry, built as it resolves (`lazy_part`), kept by name across a pause like Core's (R87).
fn entry(
    name: &'static str,
    build: impl Fn(&mut EffectContext<'_>) -> Vec<Effect> + Send + Sync + 'static,
) -> Effect {
    lazy_part(entry_kind(name), move |ctx, _memo| EffectPart {
        effects: build(ctx),
        memo: None,
    })
}

/// Entries 1, 2 and 4: N independent picks of a pool (R60), each a fresh hand card that costs (0); a full hand burns it (§2.4).
fn add_free(name: &'static str, pool: Value, count: i32) -> Effect {
    entry(name, move |_ctx| {
        vec![crate::effects::add_to_hand::add_random_from_catalog(json_as(json!({
            "query": pool,
            "count": count,
            "costOverride": CHAOS_PLUS_COST,
        })))]
    })
}

/// Entry 9: "replace your deck with random Call to Chaos cards, which cost (0)" — each card of the deck as
/// the entry resolves is Replaced (§6.3, R35) one for one, where it lies, by a random card of the "Call
/// to Chaos" pool (both editions, this one included: the text names its pool, R387, R28), each pick its
/// own (R60), the old card ceasing to exist and the new one a card its owner was never shown (R311), with
/// a `costOverride` of (0) it carries in every zone (R78). The deck keeps its size; an empty one draws
/// nothing (R129).
pub fn replace_deck_with_call_to_chaos() -> Effect {
    entry("replace", |ctx| {
        let deck: Vec<CardInstance> = ctx.sink.state.players[ctx.controller].library.clone();
        let pool: Arc<Vec<CardDef>> = Arc::new(owned_defs(crate::catalog::query(&json_as(
            json!({ "tags": [CHAOS_TAG.as_str()] }),
        ))));
        deck.into_iter()
            .map(|old| {
                let pool = pool.clone();
                Effect::new("callToChaosPlus:replaceOne", move |inner| {
                    let controller = inner.controller;
                    let Some(at) = inner.sink.state.players[controller]
                        .library
                        .iter()
                        .position(|card| card.id == old.id)
                    else {
                        return;
                    };
                    // R673: into a deck.
                    let picked = crate::catalog::pick_generated(inner.sink.rng, &pool, Some(&*inner.sink.state))
                        .map(|def| def.clone());
                    let Some(def) = picked else {
                        return;
                    };
                    let replace = crate::effects::transform::transform(json_as(
                        json!({ "instanceId": old.id, "defId": def.id }),
                    ));
                    (replace.apply)(inner);
                    if let Some(replacement) = inner.sink.state.players[controller].library.get_mut(at)
                        && replacement.id != old.id
                    {
                        replacement.cost_override = Some(CHAOS_PLUS_COST);
                    }
                })
            })
            .collect()
    })
}

/// 1. The Fruit pool holds the five Grapes too (R382).
fn build_fruits() -> Effect {
    add_free("fruits", json!({ "tags": ["Fruit"] }), CHAOS_PLUS_FRUITS)
}

/// 2. Non-token Books of every set (R380).
fn build_books() -> Effect {
    add_free("books", json!({ "tags": ["Book"] }), CHAOS_PLUS_BOOKS)
}

/// 3. Every enemy permanent on the field — the top of each pile, face-down cards included — is marked
/// together (R59); Indestructible ones stay (R46).
fn build_destroy() -> Effect {
    entry("destroy", |_ctx| {
        vec![crate::effects::destroy::destroy_all(json_as(
            json!({ "side": "enemy", "rows": ["units", "backrow"] }),
        ))]
    })
}

/// 4. Non-token cards of the Classic set only (the text names it, R380).
fn build_classic() -> Effect {
    add_free("classic", json!({ "set": "Classic" }), CHAOS_PLUS_CLASSIC_CARDS)
}

/// 5. Two separate Upgrades of each card in your hand and your deck (R386), the deck's unseen by its
/// owner until the card leaves it (R311).
fn build_upgrade() -> Effect {
    entry("upgrade", |_ctx| {
        vec![crate::effects::tune::upgrade(json_as(json!({
            "scope": { "side": "self", "zones": ["hand", "library"] },
            "times": CHAOS_PLUS_UPGRADES,
        })))]
    })
}

/// 6. E23: a random non-token card of every set but this one (R387) fused into each deck card, which
/// is the kept instance, keeps its type and keeps its cost (R470); Immutable ones are skipped (R23).
fn build_fuse() -> Effect {
    entry("fuse", |_ctx| {
        vec![crate::effects::fuse::fuse_random_into(json_as(
            json!({ "into": { "pile": "library" } }),
        ))]
    })
}

/// 7. Three separate Degrades of each card on the opponent's field and in their hand, hidden in
/// their hand (R177, R242).
fn build_degrade() -> Effect {
    entry("degrade", |_ctx| {
        vec![crate::effects::tune::degrade(json_as(json!({
            "scope": { "side": "enemy", "zones": ["field", "hand"] },
            "times": CHAOS_PLUS_DEGRADES,
        })))]
    })
}

/// 8. The Classic Golem token, placed per R64.
fn build_golem() -> Effect {
    entry("golem", |_ctx| {
        let def_id = crate::catalog::def_by_index(GOLEM_SET, GOLEM_INDEX).map(|def| def.id.clone());
        match def_id {
            None => vec![],
            Some(def_id) => vec![crate::effects::summon::summon(json_as(json!({ "defId": def_id })))],
        }
    })
}

/// §8.7 row 73's ten entries, in the order the card prints them. Each `label` is the clause as the card
/// prints it (R432's cost words, R373's "deck"), which `chaosRolled` names to both players (R436).
pub const CHAOS_PLUS_EFFECTS: &[ChaosEffectDef] = &[
    ChaosEffectDef {
        name: "fruits",
        label: "Add 5 random Fruits to your hand, which cost (0)",
        build: build_fruits,
    },
    ChaosEffectDef {
        name: "books",
        label: "Add 3 random Books to your hand, which cost (0)",
        build: build_books,
    },
    ChaosEffectDef {
        name: "destroy",
        label: "Destroy all enemy permanents",
        build: build_destroy,
    },
    ChaosEffectDef {
        name: "classic",
        label: "Add 3 random Classic cards to your hand, which cost (0)",
        build: build_classic,
    },
    ChaosEffectDef {
        name: "upgrade",
        label: "Upgrade every card in your hand and deck twice",
        build: build_upgrade,
    },
    ChaosEffectDef {
        name: "fuse",
        label: "Fuse a random card into each card in your deck, each keeping its cost",
        build: build_fuse,
    },
    ChaosEffectDef {
        name: "degrade",
        label: "Degrade every card on your opponent's field and in their hand three times",
        build: build_degrade,
    },
    ChaosEffectDef {
        name: "golem",
        label: "Summon a Classic Golem",
        build: build_golem,
    },
    ChaosEffectDef {
        name: "replace",
        label: "Replace your deck with random Call to Chaos cards, which cost (0)",
        build: replace_deck_with_call_to_chaos,
    },
    // 10. Core's recursion, the chain counting casts of either edition (R28, R87).
    ChaosEffectDef {
        name: "recast",
        label: "Cast a random Call to Chaos",
        build: cast_random_call_to_chaos,
    },
];
