//! Stinkyscape #151 Call to Chaos (Scape Edition) (SPEC §8.10 row 151, R28, R87, R423, R436,
//! R1882005–R1882009): the builders of its table's entries, which Core #95's subsystem
//! (`call_to_chaos.rs`) rolls, announces and resolves as it does every other edition's.
//!
//! The table itself is the card's (`CHAOS_SCAPE_EFFECTS`), since its first entry lays an Anomaly board,
//! data the cards crate holds (S20's `t_anomaly_found.rs`). Every entry is built when it resolves
//! (`lazy_part`, R87, R113); entry 4 keeps its draw as the part's memo, so a resume casts the same one.

use serde_json::{Value, json};

use crate::catalog::{def_id_by_index, query};
use crate::config::{
    CHAOS_SCAPE_CORVETTES, CHAOS_SCAPE_FUSED_PIECES, CHAOS_SCAPE_HAND_PIECES, CHAOS_SCAPE_LOT_PRICE,
    CHAOS_SCAPE_SYSTEMS, CHAOS_SCAPE_WEAPONS, CHAOS_STINKYSCAPE_COST, UNIT_ZONES,
};
use crate::effects::cast::{CastHow, CastNewArgs, CastNewDef};
use crate::effects::tray::AddMarketLotArgs;
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext, EffectPart};
use crate::subsystems::call_to_chaos::CHAOS_TAG;
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

/// One entry, built as it resolves and kept by name across a pause (R87); the card builds its board
/// swap ("anomaly") with it.
pub fn scape_entry(
    name: &'static str,
    build: impl Fn(&mut EffectContext<'_>) -> Vec<Effect> + Send + Sync + 'static,
) -> Effect {
    crate::resolve::lazy_part(entry_kind(name), move |ctx, _memo| EffectPart {
        effects: build(ctx),
        memo: None,
    })
}

fn def_id_at(index: &str) -> Option<String> {
    def_id_by_index(SET, index)
}

/// R1878000: the Starship Piece pool, the same query as the cards crate's `starship_pieces()`.
fn starship_pieces_query() -> Value {
    json!({ "set": SET, "keywords": [KeywordKind::StarshipPiece] })
}

/// 2. Three Drone Corvettes, each placed per R64; a full row takes fewer.
pub fn scape_corvettes() -> Effect {
    scape_entry("corvettes", |_ctx| {
        let Some(def_id) = def_id_at(CORVETTE_INDEX) else {
            return vec![];
        };
        (0..CHAOS_SCAPE_CORVETTES)
            .map(|_| crate::effects::summon::summon(json_as(json!({ "defId": def_id }))))
            .collect()
    })
}

/// 3. R1842000: the Drone Battleship, its three layers one pile in one unit zone, the Shield on top.
pub fn scape_battleship() -> Effect {
    scape_entry("battleship", |_ctx| {
        let def_ids: Vec<String> = BATTLESHIP_LAYERS
            .iter()
            .filter_map(|index| def_id_at(index))
            .collect();
        vec![crate::effects::summon::summon_pile(json_as(
            json!({ "defIds": def_ids }),
        ))]
    })
}

/// 4. R1882006: different Systems drawn from the match rng (R60), each cast in the order drawn as a new
///    card of its caster's (R453), its choices theirs (R70).
pub fn scape_systems() -> Effect {
    crate::resolve::lazy_part(entry_kind("systems"), |ctx, memo| {
        let kept: Option<Vec<String>> = memo.as_ref().and_then(Value::as_array).map(|list| {
            list.iter()
                .filter_map(|id| id.as_str().map(str::to_string))
                .collect()
        });
        let picked = match kept {
            Some(picked) => picked,
            None => {
                let mut left: Vec<String> = SYSTEM_INDICES
                    .iter()
                    .filter_map(|index| def_id_at(index))
                    .collect();
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

/// 5. Independent picks (R60) of the Starship Pieces, each a fresh (0) hand card (a full hand: §2.4).
pub fn scape_pieces() -> Effect {
    scape_entry("pieces", |_ctx| {
        vec![crate::effects::add_random_from_catalog(json_as(json!({
            "query": starship_pieces_query(),
            "count": CHAOS_SCAPE_HAND_PIECES,
            "costOverride": CHAOS_STINKYSCAPE_COST
        })))]
    })
}

/// 6. R1882007: a Drone Interceptor into each of its caster's unit zones in lane order, on top of a pile
///    (§3.2), a Locked zone taking it (R688); a zone held for a Reborn return takes none (R64).
pub fn scape_interceptors() -> Effect {
    scape_entry("interceptors", |_ctx| {
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

/// 7. R1882008: independent picks (R60) of the Starship Pieces fused into its caster's Dormant Starship,
///    the first summoning one if none (R1852000); then it awakens, launching for free (R1852003,
///    R1852006). A Starship buried under a Stack pile cannot awaken and stays Dormant.
pub fn scape_launch() -> Effect {
    scape_entry("launch", |_ctx| {
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
pub fn scape_weapons() -> Effect {
    scape_entry("weapons", |_ctx| {
        vec![crate::effects::add_random_from_catalog(json_as(json!({
            "query": { "tags": [Tag::Weapon], "set": SET },
            "count": CHAOS_SCAPE_WEAPONS,
            "radiant": true,
            "costOverride": CHAOS_STINKYSCAPE_COST
        })))]
    })
}

/// 9. R1882009, R1853005: its caster's own rest-of-game Marketplace lot, a Call to Chaos of the
///    recursion's pool (R380) at a fixed price, never Radiant; the Marketplace opens if it is closed.
pub fn scape_market() -> Effect {
    scape_entry("market", |_ctx| {
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
