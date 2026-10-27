//! Stinkyscape #151 Call to Chaos (Scape Edition) (SPEC §8.10 row 151, R28, R87, R423, R436,
//! R1882005–R1882009): its table of ten effects, which Core #95's subsystem (`call_to_chaos.rs`) rolls,
//! announces and resolves — `call_to_chaos({ radiant, table: CHAOS_SCAPE_EFFECTS })` — as it does every
//! other edition's: the base face rolls one entry, the Radiant three different ones resolved in the
//! list's order (R423), the recursion counts casts of every edition against `CALL_TO_CHAOS_CHAIN_CAP`
//! (R28), and both players are told what was rolled (`chaosRolled`, R436).
//!
//! Every entry is built when it resolves (`lazy_part`), as the other editions' are (R87, R113); the one
//! that draws as it is built (entry 4's two Systems) keeps its draw as the part's memo, so a cast that
//! asks pauses the second and the resume casts the same one.

use serde_json::{Value, json};

use crate::catalog::{def_by_index, query};
use crate::config::{
    CHAOS_SCAPE_CORVETTES, CHAOS_SCAPE_FUSED_PIECES, CHAOS_SCAPE_HAND_PIECES, CHAOS_SCAPE_LOT_PRICE,
    CHAOS_SCAPE_SYSTEMS, CHAOS_SCAPE_WEAPONS, CHAOS_STINKYSCAPE_COST, UNIT_ZONES,
};
use crate::effects::cast::{CastHow, CastNewArgs, CastNewDef};
use crate::effects::tray::AddMarketLotArgs;
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext, EffectPart};
use crate::subsystems::call_to_chaos::{CHAOS_TAG, ChaosEffectDef, cast_random_call_to_chaos};
use crate::wire::{KeywordKind, SetName, Tag};

/// §8.10: the set whose indices the table names (B2.2).
const SET: SetName = SetName::Stinkyscape;
/// Entries 2, 3 and 6: the drones, at their shared T- indices (§7).
const CORVETTE_INDEX: &str = "T-drone-corvette";
const INTERCEPTOR_INDEX: &str = "T-drone-interceptor";
/// Entry 3: the Drone Battleship's layers, bottom first (R1842000: the Shield is fought first).
const BATTLESHIP_LAYERS: [&str; 3] = ["T-meltdown-layer", "T-propagation-layer", "T-shield-layer"];
/// Entry 4: S #117 Core System, #118 Secure System, #119 Unsecure System and #120 Wild System.
const SYSTEM_INDICES: [&str; 4] = ["117", "118", "119", "120"];

/// `callToChaosScape:<name>`, the kind of an entry's part (an effect's kind is a static name).
fn entry_kind(name: &str) -> &'static str {
    match name {
        "anomaly" => "callToChaosScape:anomaly",
        "corvettes" => "callToChaosScape:corvettes",
        "battleship" => "callToChaosScape:battleship",
        "systems" => "callToChaosScape:systems",
        "pieces" => "callToChaosScape:pieces",
        "interceptors" => "callToChaosScape:interceptors",
        "launch" => "callToChaosScape:launch",
        "weapons" => "callToChaosScape:weapons",
        "market" => "callToChaosScape:market",
        _ => "callToChaosScape",
    }
}

/// One entry, built as it resolves (`lazy_part`), kept by name across a pause like Core's (R87).
fn entry(
    name: &'static str,
    build: impl Fn(&mut EffectContext<'_>) -> Vec<Effect> + Send + Sync + 'static,
) -> Effect {
    crate::resolve::lazy_part(entry_kind(name), move |ctx, _memo| EffectPart {
        effects: build(ctx),
        memo: None,
    })
}

/// The set's card at `index`, by its id, when the catalog holds it.
fn def_id_at(index: &str) -> Option<String> {
    def_by_index(SET, index).map(|def| def.id.clone())
}

/// R1878000: the Starship Piece pool, the set's non-token cards that print the keyword (the cards
/// crate's `starship_pieces()`, the same query).
fn starship_pieces_query() -> Value {
    json!({ "set": SET, "keywords": [KeywordKind::StarshipPiece] })
}

/// 1. R1882005: the board swap.
fn build_anomaly() -> Effect {
    entry("anomaly", |_ctx| vec![])
}

/// 2. Three Drone Corvettes, each placed per R64; a full row takes fewer.
fn build_corvettes() -> Effect {
    entry("corvettes", |_ctx| {
        let Some(def_id) = def_id_at(CORVETTE_INDEX) else {
            return vec![];
        };
        (0..CHAOS_SCAPE_CORVETTES)
            .map(|_| crate::effects::summon::summon(json_as(json!({ "defId": def_id }))))
            .collect()
    })
}

/// 3. R1842000: the Drone Battleship, its three layers one pile in one unit zone, the Shield on top.
fn build_battleship() -> Effect {
    entry("battleship", |_ctx| {
        let def_ids: Vec<String> = BATTLESHIP_LAYERS.iter().filter_map(|index| def_id_at(index)).collect();
        vec![crate::effects::summon::summon_pile(json_as(json!({ "defIds": def_ids })))]
    })
}

/// 4. R1882006: two different Systems drawn from the match rng (R60), in the order drawn, each cast as a
///    new card of its caster's (R453), its choices theirs (R70). The draw is the part's memo.
fn build_systems() -> Effect {
    crate::resolve::lazy_part(entry_kind("systems"), |ctx, memo| {
        let kept: Option<Vec<String>> = memo.as_ref().and_then(Value::as_array).map(|list| {
            list.iter()
                .filter_map(|id| id.as_str().map(str::to_string))
                .collect()
        });
        let picked = match kept {
            Some(picked) => picked,
            None => {
                let mut left: Vec<String> = SYSTEM_INDICES.iter().filter_map(|index| def_id_at(index)).collect();
                let mut picked = Vec::new();
                for _ in 0..CHAOS_SCAPE_SYSTEMS {
                    if left.is_empty() {
                        break;
                    }
                    let at = ctx.sink.rng.int(left.len() as i32) as usize;
                    if at < left.len() {
                        picked.push(left.remove(at));
                    }
                }
                picked
            }
        };
        let effects = picked
            .iter()
            .map(|def_id| {
                crate::effects::cast::cast_new(CastNewArgs {
                    def: CastNewDef::Id(def_id.clone()),
                    radiant: None,
                    how: CastHow::default(),
                })
            })
            .collect();
        EffectPart {
            effects,
            memo: Some(json!(picked)),
        }
    })
}

/// 5. Four independent picks (R60) of the Starship Pieces (R1878000), each a fresh hand card costing
///    (0); a full hand burns it (§2.4).
fn build_pieces() -> Effect {
    entry("pieces", |_ctx| {
        vec![crate::effects::add_random_from_catalog(json_as(json!({
            "query": starship_pieces_query(),
            "count": CHAOS_SCAPE_HAND_PIECES,
            "costOverride": CHAOS_STINKYSCAPE_COST
        })))]
    })
}

/// 6. R1882007: a Drone Interceptor summoned into each of its caster's unit zones in lane order, on top
///    of the pile there when one stands in it (§3.2), a Locked zone taking it as any summon's does
///    (R688); a zone held for a Reborn return takes none (R64).
fn build_interceptors() -> Effect {
    entry("interceptors", |_ctx| {
        let Some(def_id) = def_id_at(INTERCEPTOR_INDEX) else {
            return vec![];
        };
        (1..=UNIT_ZONES)
            .map(|lane| {
                crate::effects::summon::summon(json_as(json!({
                    "defId": def_id,
                    "lane": lane,
                    "stack": true
                })))
            })
            .collect()
    })
}

/// 7. R1882008: four independent picks (R60) of the Starship Pieces, each fused into its caster's
///    Dormant Starship, the first summoning one when they have none (R1852000); then the Starship
///    awakens, which launches it and costs nothing (R1852003, R1852006): its fused Cries, then its
///    "When launched" triggers. A Starship buried under a Stack pile cannot awaken and stays Dormant.
fn build_launch() -> Effect {
    entry("launch", |_ctx| {
        vec![Effect::new("callToChaosScape:fuseAndLaunch", |ctx| {
            let pool = query(&json_as(starship_pieces_query()));
            let controller = ctx.controller;
            for _ in 0..CHAOS_SCAPE_FUSED_PIECES {
                let Some(def_id) = ctx.sink.rng.pick(&pool).map(|def| def.id.clone()) else {
                    return;
                };
                crate::starship::fuse_fresh_piece(&mut ctx.sink, controller, &def_id);
            }
            let Some(ship_id) =
                crate::starship::dormant_starship_of(ctx.state, controller).map(|ship| ship.id.clone())
            else {
                return;
            };
            crate::slumber::awaken_card(&mut ctx.sink, &ship_id);
        })]
    })
}

/// 8. Two independent picks (R60) of the Weapons (R1857000), each a fresh Radiant hand card costing (0).
fn build_weapons() -> Effect {
    entry("weapons", |_ctx| {
        vec![crate::effects::add_random_from_catalog(json_as(json!({
            "query": { "tags": [Tag::Weapon], "set": SET },
            "count": CHAOS_SCAPE_WEAPONS,
            "radiant": true,
            "costOverride": CHAOS_STINKYSCAPE_COST
        })))]
    })
}

/// 9. R1882009, R1853005: a Marketplace lot for the rest of the game, its caster's alone: one Call to
///    Chaos of the pool the recursion casts from (R380), drawn as the lot is added, at a fixed
///    `CHAOS_SCAPE_LOT_PRICE` Credits, never Radiant; the Marketplace opens if it is closed.
fn build_market() -> Effect {
    entry("market", |_ctx| {
        let pool: Vec<String> = query(&json_as(json!({ "tags": [CHAOS_TAG] })))
            .iter()
            .map(|def| def.id.clone())
            .collect();
        vec![crate::effects::tray::add_market_lot(AddMarketLotArgs {
            pool,
            price: CHAOS_SCAPE_LOT_PRICE,
            player: None,
        })]
    })
}

/// §8.10 row 151's ten entries, in the order the card prints them. Each `label` is the clause as the
/// card prints it, which `chaosRolled` names to both players (R436).
pub const CHAOS_SCAPE_EFFECTS: &[ChaosEffectDef] = &[
    ChaosEffectDef {
        name: "anomaly",
        label: "Replace your side of the field with a random Anomaly, any of the ten",
        build: build_anomaly,
    },
    ChaosEffectDef {
        name: "corvettes",
        label: "Summon 3 Drone Corvettes",
        build: build_corvettes,
    },
    ChaosEffectDef {
        name: "battleship",
        label: "Summon the Drone Battleship",
        build: build_battleship,
    },
    ChaosEffectDef {
        name: "systems",
        label: "Cast 2 different cards of Core System, Secure System, Unsecure System and Wild System, picked at random",
        build: build_systems,
    },
    ChaosEffectDef {
        name: "pieces",
        label: "Add 4 random Starship Pieces to your hand, which cost (0)",
        build: build_pieces,
    },
    ChaosEffectDef {
        name: "interceptors",
        label: "Summon a Drone Interceptor into each of your unit zones, on top of any Unit there",
        build: build_interceptors,
    },
    ChaosEffectDef {
        name: "launch",
        label: "Fuse 4 random Starship Pieces into your Dormant Starship, then launch it",
        build: build_launch,
    },
    ChaosEffectDef {
        name: "weapons",
        label: "Add 2 random Radiant Weapons to your hand, which cost (0)",
        build: build_weapons,
    },
    ChaosEffectDef {
        name: "market",
        label: "For the rest of the game, the Marketplace offers you a Call to Chaos for 20 Credits each cycle",
        build: build_market,
    },
    // 10. Core's recursion, the chain counting casts of every edition (R28, R87, R452).
    ChaosEffectDef {
        name: "recast",
        label: "Cast a random Call to Chaos",
        build: cast_random_call_to_chaos,
    },
];
