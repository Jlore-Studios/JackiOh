//! Test-only cards for Training Grounds part 5's systems (#2198): a Unit's Attack and Health set
//! (`set_stats`, R2198000), an exact copy of a permanent into the hand (`add_to_hand`'s `exact` rider,
//! R2198001) and tokens destroyed at the end of the turn they arrive (R2198002), through the engine's own
//! verbs so the engine never imports `crates/cards`. Ids are `tg5-` from 21980, clear of other fixtures'.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

// The verbs as a card script names them, through the prelude (CLAUDE.md rule 5).
use jackioh_engine::prelude::{
    DELAYED_DESTROY_HOOK, ForEachCardArgs, THIS_TURN, add_to_hand, delay, for_each_card, recalled, set_stats,
    summon,
};
use jackioh_engine::testkit::*;

use super::harness::both;

/// The Attack and Health the set fixture sets, Dino-size's 10/10.
pub const SET_TO: i32 = 10;
pub const TOKENS: i32 = 2;
/// The price the priced Seance fixture gives its copy, the Radiant Seance's (0).
pub const COPY_COST: i32 = 0;

fn card(
    index: u32,
    name: &str,
    type_: &str,
    stats: Option<(i32, i32)>,
    keywords: Value,
    token: bool,
) -> CardDef {
    let (face, radiant) = match stats {
        Some((attack, health)) => (
            json!({ "attack": attack, "health": health, "keywords": keywords, "text": name }),
            json!({ "attack": 2 * attack, "health": 2 * health, "keywords": keywords, "text": name }),
        ),
        None => (
            json!({ "keywords": keywords, "text": name }),
            json!({ "keywords": keywords, "text": name }),
        ),
    };
    json_as(json!({
        "id": format!("tg5-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (Training Grounds part 5 fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": token,
        "cost": 2,
        "base": face,
        "radiant": radiant,
    }))
}

/// A 1/1 Unit: "Cry: Set a Unit's Attack and Health to 10/10" (TG #34's set; a Unit, so that it may
/// choose an Immutable Unit, which no Spell may, R781001, and reach R23's guard).
pub static dino: LazyLock<CardDef> =
    LazyLock::new(|| card(21980, "dino", "Unit", Some((1, 1)), json!([]), false));
pub static stoic: LazyLock<CardDef> = LazyLock::new(|| {
    card(
        21981,
        "stoic",
        "Unit",
        Some((5, 5)),
        json!([{ "kind": "Immutable" }]),
        false,
    )
});
/// A Spell: "Add an exact copy of a permanent to your hand" (TG #37's base face).
pub static seance: LazyLock<CardDef> =
    LazyLock::new(|| card(21982, "seance", "Spell", None, json!([]), false));
/// A Spell: "Add an exact copy of a permanent to your hand. It costs (0)" (TG #37's Radiant face).
pub static seance_free: LazyLock<CardDef> =
    LazyLock::new(|| card(21983, "seance-free", "Spell", None, json!([]), false));
/// A Spell: "Add a copy of a permanent to your hand" (R1242's plain `copyOf`, for the difference).
pub static copier: LazyLock<CardDef> =
    LazyLock::new(|| card(21984, "copier", "Spell", None, json!([]), false));
pub static dasher: LazyLock<CardDef> = LazyLock::new(|| {
    card(
        21985,
        "dasher",
        "Unit",
        Some((3, 3)),
        json!([{ "kind": "Rush" }]),
        true,
    )
});
/// A Spell: "Summon 2 dashers. At the end of this turn, destroy them" (TG #15's shape).
pub static hurry: LazyLock<CardDef> = LazyLock::new(|| card(21986, "hurry", "Spell", None, json!([]), false));

static TG5_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        dino.clone(),
        stoic.clone(),
        seance.clone(),
        seance_free.clone(),
        copier.clone(),
        dasher.clone(),
        hurry.clone(),
    ]
});

/// Any permanent on either side's field, Core #34's "a permanent" (R81).
fn a_permanent() -> Vec<TargetDecl> {
    vec![TargetDecl::target(
        1,
        1,
        json!({ "side": "any", "of": ["unit", "backrow"] }),
    )]
}

fn copy_script(exact: bool, cost: Option<i32>) -> CardScripts {
    both(Script {
        targets: a_permanent(),
        cry: Some(hook(move |ctx| {
            let Some(Selection::Instance { instance_id }) = ctx.targets.first() else {
                return vec![];
            };
            let mut args = json!({ "copyOf": instance_id, "player": "self" });
            if exact {
                args["exact"] = json!(true);
            }
            if let Some(cost) = cost {
                args["costOverride"] = json!(cost);
            }
            vec![add_to_hand(json_as(args))]
        })),
        ..Script::default()
    })
}

fn token_key(n: i32) -> String {
    format!("token{n}")
}

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            dino.id.clone(),
            both(Script {
                targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
                cry: Some(hook(|_ctx| {
                    vec![set_stats(json_as(json!({
                        "target": { "of": "chosen" },
                        "attack": SET_TO,
                        "health": SET_TO,
                    })))]
                })),
                ..Script::default()
            }),
        ),
        (seance.id.clone(), copy_script(true, None)),
        (seance_free.id.clone(), copy_script(true, Some(COPY_COST))),
        (copier.id.clone(), copy_script(false, None)),
        (
            hurry.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    let mut effects: Vec<Effect> = (0..TOKENS)
                        .map(|n| summon(json_as(json!({ "defId": dasher.id, "rememberAs": token_key(n) }))))
                        .collect();
                    effects.push(for_each_card(ForEachCardArgs {
                        cards: std::sync::Arc::new(|ctx| {
                            (0..TOKENS)
                                .filter_map(|n| recalled(ctx, &token_key(n)))
                                .filter_map(|id| id.as_str().map(str::to_string))
                                .collect()
                        }),
                        each: std::sync::Arc::new(|id| {
                            delay(json_as(json!({
                                "at": { "phase": "end", "player": THIS_TURN },
                                "step": "destroy",
                                "hook": DELAYED_DESTROY_HOOK,
                                "data": { "instanceId": id },
                                "watch": id,
                            })))
                        }),
                    }));
                    effects
                })),
                ..Script::default()
            }),
        ),
    ])
});

pub fn register_tg5_fixtures() {
    super::harness::setup_catalog();
    let mut defs: CardDefs = registered_catalog().clone();
    for def in TG5_DEFS.iter() {
        defs.insert(def.id.clone(), def.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}
