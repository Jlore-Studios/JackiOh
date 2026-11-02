//! The Stinkyscape set's two Call to Chaos tables (`subsystems/call_to_chaos_stinky.rs` and
//! `call_to_chaos_scape.rs`, R1882001–R1882009) name cards by index (B2.2): these stand at those indices
//! with no text, so an entry is proved on what it summons or adds; S #150's and #151's tests do the rest.

use jackioh_engine::testkit::*;

use super::harness::{new_game, set_library, setup_catalog};

/// `id`, a Stinkyscape card at `index` with no text: a Unit when `stats` are given.
fn card(
    id: &str,
    index: &str,
    type_: &str,
    stats: Option<(i32, i32)>,
    tags: Value,
    keywords: Value,
) -> CardDef {
    let (base, radiant) = match stats {
        Some((attack, health)) => (
            json!({ "attack": attack, "health": health, "keywords": keywords, "text": id }),
            json!({ "attack": 2 * attack, "health": 2 * health, "keywords": keywords, "text": id }),
        ),
        None => (
            json!({ "keywords": keywords, "text": id }),
            json!({ "keywords": keywords, "text": id }),
        ),
    };
    json_as(json!({
        "id": id,
        "index": index,
        "name": format!("{id} (Stinkyscape Call to Chaos fixture)"),
        "set": "Stinkyscape",
        "type": type_,
        "tags": tags,
        "rarity": "Common",
        "token": index.contains('.') || index.starts_with("T-"),
        "cost": 3,
        "base": base,
        "radiant": radiant,
    }))
}

fn unit(id: &str, index: &str, tags: Value) -> CardDef {
    card(id, index, "Unit", Some((2, 2)), tags, json!([]))
}

fn spell(id: &str, index: &str) -> CardDef {
    card(id, index, "Spell", None, json!([]), json!([]))
}

/// Stinky entry 1: S #159 ETHICS and S #95 Vanguard Coalition.
pub const ETHICS: &str = "sx-ethics";
pub const VANGUARD: &str = "sx-vanguard";
/// Stinky entry 2: S #113 A Very Dandy Lad and S #114 Arctic Citadel.
pub const DANDY_LAD: &str = "sx-dandy-lad";
pub const ARCTIC_CITADEL: &str = "sx-arctic-citadel";
/// Stinky entry 9: S #150.1 Stinkyscape Golem.
pub const GOLEM: &str = "sx-golem";
/// Scape entries 2 and 6: the two drones, at their T- indices (§7).
pub const CORVETTE: &str = "sx-corvette";
pub const INTERCEPTOR: &str = "sx-interceptor";
/// Scape entry 3: the Drone Battleship's three layers.
pub const MELTDOWN: &str = "sx-meltdown-layer";
pub const PROPAGATION: &str = "sx-propagation-layer";
pub const SHIELD: &str = "sx-shield-layer";
/// Scape entry 5's pool: the set's Starship Pieces, two of them.
pub const PIECES: [&str; 2] = ["sx-piece-a", "sx-piece-b"];
/// Scape entry 8's pool: the set's Weapons, two of them.
pub const WEAPONS: [&str; 2] = ["sx-weapon-a", "sx-weapon-b"];

fn defs() -> Vec<CardDef> {
    let mut defs = vec![
        spell(ETHICS, "159"),
        unit(VANGUARD, "95", json!([])),
        unit(DANDY_LAD, "113", json!(["Human"])),
        unit(ARCTIC_CITADEL, "114", json!([])),
        unit(GOLEM, "150.1", json!([])),
        unit(CORVETTE, "T-drone-corvette", json!([])),
        unit(INTERCEPTOR, "T-drone-interceptor", json!([])),
        unit(MELTDOWN, "T-meltdown-layer", json!([])),
        unit(PROPAGATION, "T-propagation-layer", json!([])),
        unit(SHIELD, "T-shield-layer", json!([])),
    ];
    for (at, id) in PIECES.iter().enumerate() {
        defs.push(card(
            id,
            &(200 + at).to_string(),
            "Unit",
            Some((1, 1)),
            json!([]),
            json!([{ "kind": "Starship Piece" }]),
        ));
    }
    for (at, id) in WEAPONS.iter().enumerate() {
        defs.push(unit(id, &(210 + at).to_string(), json!(["Weapon"])));
    }
    defs
}

/// A main phase on p1's turn 4, empty hands and library and these cards registered: where the two
/// Stinkyscape tables' tests start.
pub fn stinkyscape_game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    register_chaos_stinkyscape();
    state.turn = 4;
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    state.players.p1.hand = Vec::new();
    state.players.p2.hand = Vec::new();
    set_library(&mut state, PlayerId::P1, &[] as &[&str]);
    state
}

/// The harness's catalog and scripts, plus these cards.
pub fn register_chaos_stinkyscape() {
    setup_catalog();
    let mut catalog: CardDefs = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
}
