//! Test-only cards for Stinkyscape's Resources and Credits (#1840, R1840000–R1840006): six Resources,
//! one of each rarity and two Commons, and an asteroid, the mining node's shape. Ids `fx-res-…` from
//! 18410, so they collide with no other fixture file's.

use std::sync::LazyLock;

use jackioh_engine::effects::{damage, gain_resources, sacrifice};
use jackioh_engine::testkit::*;

use super::catalog::{spell_def, unit_def};

/// What every fixture Resource sells for on its base face, and what the Marketplace asks.
pub const BASE_SELL: (i32, i32) = (1, 3);
pub const BASE_BUY: i32 = 3;
/// The same on the Radiant face.
pub const RADIANT_SELL: (i32, i32) = (10, 20);
pub const RADIANT_BUY: i32 = 20;
/// What `fx-res-common-a`'s Cry deals the enemy hero.
pub const COMMON_A_DAMAGE: i32 = 1;
/// What the asteroid's base face takes from its controller's hero.
pub const ASTEROID_DAMAGE: i32 = 2;
/// The Resources the asteroid's Death gains.
pub const ASTEROID_YIELD: i32 = 2;
/// The asteroid's ability.
pub const MINE: &str = "mine";

pub const COMMON_A: &str = "fx-res-common-a";
pub const COMMON_B: &str = "fx-res-common-b";
pub const RARE: &str = "fx-res-rare";
pub const EPIC: &str = "fx-res-epic";
pub const LEGENDARY: &str = "fx-res-legendary";
pub const MYTHIC: &str = "fx-res-mythic";
pub const ASTEROID: &str = "fx-res-asteroid";

/// The six Resources, with the rarity each rolls as.
pub const RESOURCES: [(&str, &str); 6] = [
    (COMMON_A, "Common"),
    (COMMON_B, "Common"),
    (RARE, "Rare"),
    (EPIC, "Epic"),
    (LEGENDARY, "Legendary"),
    (MYTHIC, "Mythic"),
];

/// A (0) Spell token that is a Resource of `rarity`, printing it too.
fn resource_def(index: i32, id: &str, rarity: &str) -> CardDef {
    spell_def(
        index,
        json!({
            "id": id,
            "name": format!("Fixture Resource {id}"),
            "tags": ["Token"],
            "rarity": "Token",
            "printedRarity": rarity,
            "token": true,
            "cost": 0,
            "resource": {
                "rarity": rarity,
                "base": { "sellMin": BASE_SELL.0, "sellMax": BASE_SELL.1, "buy": BASE_BUY },
                "radiant": { "sellMin": RADIANT_SELL.0, "sellMax": RADIANT_SELL.1, "buy": RADIANT_BUY },
            },
        }),
    )
}

pub static RESOURCE_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    let mut defs: Vec<CardDef> = RESOURCES
        .iter()
        .zip(18410..)
        .map(|((id, rarity), index)| resource_def(index, id, rarity))
        .collect();
    // A 1/1 mining node: "Activate: Take 2 damage. Tribute this. Death: gain 2 random Resources."
    defs.push(unit_def(
        18416,
        json!({ "id": ASTEROID, "name": "Fixture Asteroid", "cost": 0, "attack": 1, "health": 1 }),
    ));
    defs
});

pub fn resources_catalog(base: CardDefs) -> CardDefs {
    let mut defs = base;
    for def in RESOURCE_DEFS.iter() {
        defs.insert(def.id.clone(), def.clone());
    }
    defs
}

/// The asteroid's ability: "Take N damage. Tribute this", N dropped on the Radiant face.
fn mine(radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: MINE.to_string(),
        label: "Mine".to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |_ctx| {
            let tribute = sacrifice(json_as(json!({ "target": { "of": "self" } })));
            if radiant {
                vec![tribute]
            } else {
                vec![
                    damage(json_as(
                        json!({ "to": { "of": "selfHero" }, "amount": ASTEROID_DAMAGE }),
                    )),
                    tribute,
                ]
            }
        }),
    }
}

fn asteroid(radiant: bool) -> Script {
    Script {
        activations: vec![mine(radiant)],
        death: Some(hook(move |_ctx| {
            vec![gain_resources(json_as(if radiant {
                json!({ "count": ASTEROID_YIELD, "radiant": true })
            } else {
                json!({ "count": ASTEROID_YIELD })
            }))]
        })),
        ..Script::default()
    }
}

pub static RESOURCE_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let cry = || Script {
        cry: Some(hook(|_ctx| {
            vec![damage(json_as(
                json!({ "to": { "of": "enemyHero" }, "amount": COMMON_A_DAMAGE }),
            ))]
        })),
        ..Script::default()
    };
    IndexMap::from([
        (
            COMMON_A.to_string(),
            CardScripts {
                base: cry(),
                radiant: cry(),
            },
        ),
        (
            ASTEROID.to_string(),
            CardScripts {
                base: asteroid(false),
                radiant: asteroid(true),
            },
        ),
    ])
});
