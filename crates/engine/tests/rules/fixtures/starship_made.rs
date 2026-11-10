//! Test-only cards for Starships a card makes (SPEC §6.3 Summon Starships, Launch a random
//! Starship, Exile a whole board; R3539000–R3539004): a sealed hull with no launch by hand, a priced
//! one, a shipyard, two Stinkyscape-set pieces and the spells that summon, launch and exile them.
//! Ids `sm-…` from 35390, so they collide with no other fixture file's.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects;
use jackioh_engine::effects::StarshipBlueprint;
use jackioh_engine::prelude::starship_pieces_of;
// The verbs and reads as a card script names them, through the prelude (CLAUDE.md rule 5).
use jackioh_engine::prelude::{condition_hook, hook, launch_as_starship};
use jackioh_engine::testkit::*;

use super::harness::both;
use super::starship::{booster, gunner, register_starship_fixtures};

fn def(
    id: &str,
    index: u32,
    set: &str,
    type_: &str,
    cost: i32,
    stats: (i32, i32),
    keywords: Value,
    tags: Value,
    token: bool,
) -> CardDef {
    let (attack, health) = stats;
    let (face, radiant) = if type_ == "Unit" {
        (
            json!({ "attack": attack, "health": health, "keywords": keywords, "text": id }),
            json!({ "attack": 2 * attack, "health": 2 * health, "keywords": keywords, "text": id }),
        )
    } else {
        (
            json!({ "keywords": keywords, "text": id }),
            json!({ "keywords": keywords, "text": id }),
        )
    };
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (Starship made fixture)"),
        "set": set,
        "type": type_,
        "tags": tags,
        "rarity": "Common",
        "token": token,
        "cost": cost,
        "base": face,
        "radiant": radiant,
    }))
}

fn unit(id: &str, index: u32, stats: (i32, i32), keywords: Value, tags: Value) -> CardDef {
    def(id, index, "Core", "Unit", 0, stats, keywords, tags, false)
}

fn spell(id: &str, index: u32) -> CardDef {
    def(id, index, "Core", "Spell", 0, (0, 0), json!([]), json!([]), false)
}

/// A sealed hull: "Dormant until launched. It can't be launched by hand." (R3539000).
pub static sealed: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "sm-sealed",
        35390,
        (0, 0),
        json!([{ "kind": "Flying" }, { "kind": "Ranged" }]),
        json!(["Starship"]),
    )
});
/// A shipyard: "Start of turn: Fuse a random Starship Piece into this. Dormant until launched.
/// Activate 3: Exile the shipyard and launch as a Starship." (R3539004).
pub static yard: LazyLock<CardDef> =
    LazyLock::new(|| unit("sm-yard", 35391, (0, 0), json!([]), json!(["Starship"])));
/// A Stinkyscape Starship Piece, (1, 2).
pub static cell_a: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "sm-cell-a",
        35392,
        "Stinkyscape",
        "Unit",
        1,
        (1, 2),
        json!([{ "kind": "Starship Piece" }]),
        json!([]),
        false,
    )
});
/// A Stinkyscape Starship Piece, (2, 1).
pub static cell_b: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "sm-cell-b",
        35393,
        "Stinkyscape",
        "Unit",
        1,
        (2, 1),
        json!([{ "kind": "Starship Piece" }]),
        json!([]),
        false,
    )
});
/// "Summon 2 sealed Starships, each holding a booster and a gunner." (R3539001).
pub static maker: LazyLock<CardDef> = LazyLock::new(|| spell("sm-maker", 35394));
/// "Summon 1 sealed Starship. It launches for (3)." (R3539000).
pub static vagabond: LazyLock<CardDef> = LazyLock::new(|| spell("sm-vagabond", 35395));
/// "Launch a random sealed Starship." (R3539002).
pub static voyage: LazyLock<CardDef> = LazyLock::new(|| spell("sm-voyage", 35396));
/// "Exile your board." (R3539002).
pub static wipe: LazyLock<CardDef> = LazyLock::new(|| spell("sm-wipe", 35397));
/// A Unit with Immune to Spells.
pub static shield: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "sm-shield",
        35398,
        (1, 1),
        json!([{ "kind": "Immune to Spells" }]),
        json!([]),
    )
});
/// "Your Starship launch costs (1)."
pub static cheap: LazyLock<CardDef> = LazyLock::new(|| spell("sm-cheap", 35399));

/// What `maker` and `vagabond` summon: a sealed hull holding a booster and a gunner.
fn made_blueprints() -> Vec<StarshipBlueprint> {
    vec![StarshipBlueprint {
        def_id: sealed.id.clone(),
        pieces: vec![booster.id.clone(), gunner.id.clone()],
    }]
}

fn sealed_text() -> Option<StaticFlags> {
    Some(StaticFlags {
        slumber: Some(SlumberText::default()),
        starship_hull: Some(true),
        no_hand_launch: Some(true),
        ..StaticFlags::default()
    })
}

fn yard_text() -> Option<StaticFlags> {
    Some(StaticFlags {
        slumber: Some(SlumberText {
            wakes_by: Some("yard".to_string()),
            ..SlumberText::default()
        }),
        starship_hull: Some(true),
        starship_yard: Some(true),
        ..StaticFlags::default()
    })
}

/// R3539004: the shipyard's Activate, as S+ #157 prints it: three uses a turn, refused while the
/// shipyard holds no piece, exiling it to launch as a Starship holding its pieces.
fn yard_activate() -> ActivationDecl {
    ActivationDecl {
        id: "yard".to_string(),
        label: "Exile the shipyard and launch as a Starship".to_string(),
        uses: ActivationUses::Count(3),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: Some(condition_hook(|ctx| {
            !starship_pieces_of(ctx.self_).is_empty()
        })),
        has: None,
        run: hook(|_ctx| vec![launch_as_starship()]),
    }
}

fn spell_cry(cry: Hook) -> CardScripts {
    both(Script {
        cry: Some(cry),
        ..Script::default()
    })
}

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let blueprints = made_blueprints();
    let voyage_hulls = vec![sealed.id.clone()];
    IndexMap::from([
        (
            sealed.id.clone(),
            both(Script {
                static_flags: sealed_text(),
                ..Script::default()
            }),
        ),
        (
            yard.id.clone(),
            both(Script {
                static_flags: yard_text(),
                activations: vec![yard_activate()],
                ..Script::default()
            }),
        ),
        (cell_a.id.clone(), both(Script::default())),
        (cell_b.id.clone(), both(Script::default())),
        (
            maker.id.clone(),
            spell_cry(hook(move |_ctx| {
                vec![effects::summon_starships(json_as(json!({
                    "blueprints": blueprints,
                    "count": 2,
                })))]
            })),
        ),
        (
            vagabond.id.clone(),
            spell_cry(hook(move |_ctx| {
                vec![effects::summon_starships(json_as(json!({
                    "blueprints": made_blueprints(),
                    "count": 1,
                    "price": 3,
                })))]
            })),
        ),
        (
            voyage.id.clone(),
            spell_cry(hook(move |_ctx| {
                vec![effects::launch_random_starship(json_as(json!({
                    "hulls": voyage_hulls,
                })))]
            })),
        ),
        (
            wipe.id.clone(),
            spell_cry(hook(|_ctx| {
                vec![effects::exile_board(json_as(json!({ "player": "self" })))]
            })),
        ),
        (shield.id.clone(), both(Script::default())),
        (
            cheap.id.clone(),
            spell_cry(hook(|_ctx| {
                vec![effects::starship_launch_costs(json_as(json!({ "cost": 1 })))]
            })),
        ),
    ])
});

pub static MADE_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        sealed.clone(),
        yard.clone(),
        cell_a.clone(),
        cell_b.clone(),
        maker.clone(),
        vagabond.clone(),
        voyage.clone(),
        wipe.clone(),
        shield.clone(),
        cheap.clone(),
    ]
});

pub fn register_starship_made_fixtures() {
    register_starship_fixtures();
    let mut defs: CardDefs = registered_catalog().clone();
    for card in MADE_DEFS.iter() {
        defs.insert(card.id.clone(), card.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}
