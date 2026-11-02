//! Test-only cards for the weapon kit (#1841, Stinkyscape's phase 0): an Activate's extra turn cooldown
//! (`ActivationCost.extraTurnCooldown`, R1841000), damage halved to heroes (`halfToHeroes`, R1841001),
//! Equip into this Unit from a pick (`equip_picks`, R1841002) and "Float reduces Tribute requirement by 1"
//! (`float_lowers_tribute`, R1841003), through the engine's own verbs so the engine never imports
//! `crates/cards`. Ids are `wk-` from 18410, so they collide with no other fixture file's.

use jackioh_engine::effects::{damage, equip_picks, gain_mana};
// The verbs as a card script names them, through the prelude (CLAUDE.md rule 5).
use jackioh_engine::prelude::float_lowers_tribute;
use jackioh_engine::testkit::*;

use super::harness::both;

/// What the beam's Activate deals: 3 to a hero once halved and rounded down.
pub const BEAM_DAMAGE: i32 = 7;
/// What the pea's Activate deals: nothing to a hero once halved and rounded down.
pub const PEA_DAMAGE: i32 = 1;
/// What the blade's Cry deals the enemy hero, had it run.
pub const BLADE_CRY_DAMAGE: i32 = 1;
pub const PROTO_TRIBUTE: i32 = 3;
pub const RACK_OPTIONS: i32 = 2;
pub const PICKS: i32 = 2;

pub const BEAM: &str = "wk-beam";
pub const PEA: &str = "wk-pea";
/// A 0/8 Unit with no text: what the beam hits whole.
pub const WALL: &str = "wk-wall";
pub const PLAIN: &str = "wk-plain";
/// A 1/1 Unit with a Cry and an Activate: a pick for the rack.
pub const BLADE: &str = "wk-blade";
/// A 0/3 Unit with no text. A pick for the rack and the chest.
pub const SHIELD: &str = "wk-shield";
pub const RACK: &str = "wk-rack";
pub const CHEST: &str = "wk-chest";
/// A 2/2 Unit whose Cry Equips a random shield to it, twice: its pool is one card.
pub const LOCKER: &str = "wk-locker";
/// A 2/2 Immutable Unit with the rack's Cry (R23).
pub const STATUE: &str = "wk-statue";
pub const PROTO: &str = "wk-proto";

fn card(index: u32, id: &str, stats: (i32, i32), keywords: Value) -> CardDef {
    let (attack, health) = stats;
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (weapon kit fixture)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "attack": attack, "health": health, "keywords": keywords, "text": id },
        "radiant": { "attack": 2 * attack, "health": 2 * health, "keywords": keywords, "text": id },
    }))
}

fn enemy_target() -> TargetDecl {
    TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit", "hero"] }))
}

fn halving_shot(id: &str, uses: i32, amount: i32, cost: Option<ActivationCost>) -> ActivationDecl {
    ActivationDecl {
        id: id.to_string(),
        label: format!("Deal {amount} damage"),
        uses: ActivationUses::Count(uses),
        cost,
        targets: vec![enemy_target()],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |_ctx| {
            vec![damage(json_as(json!({
                "to": { "of": "chosen" },
                "amount": amount,
                "halfToHeroes": true,
            })))]
        }),
    }
}

/// The rack's Cry, which the statue shares: Radiant picks on the Radiant face.
fn rack_cry() -> Hook {
    hook(|ctx| {
        vec![equip_picks(json_as(json!({
            "query": { "defId": [BLADE, SHIELD] },
            "options": RACK_OPTIONS,
            "times": PICKS,
            "radiant": ctx.radiant,
        })))]
    })
}

fn random_cry(pool: &'static [&'static str], radiant: bool) -> Hook {
    hook(move |_ctx| {
        vec![equip_picks(json_as(json!({
            "query": { "defId": pool },
            "pick": "random",
            "times": PICKS,
            "radiant": radiant,
        })))]
    })
}

fn scripts() -> IndexMap<String, CardScripts> {
    IndexMap::from([
        (
            BEAM.to_string(),
            both(Script {
                activations: vec![halving_shot(
                    "beam",
                    1,
                    BEAM_DAMAGE,
                    Some(ActivationCost {
                        extra_turn_cooldown: Some(true),
                        ..ActivationCost::default()
                    }),
                )],
                ..Script::default()
            }),
        ),
        (
            PEA.to_string(),
            both(Script {
                activations: vec![halving_shot("pea", 2, PEA_DAMAGE, None)],
                ..Script::default()
            }),
        ),
        (
            BLADE.to_string(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": BLADE_CRY_DAMAGE }),
                    ))]
                })),
                activations: vec![ActivationDecl {
                    id: "blade".to_string(),
                    label: "Gain 1 mana".to_string(),
                    uses: ActivationUses::Count(1),
                    cost: None,
                    targets: vec![],
                    modes: vec![],
                    can_activate: None,
                    has: None,
                    run: hook(|_ctx| vec![gain_mana(json_as(json!({ "amount": 1 })))]),
                }],
                ..Script::default()
            }),
        ),
        (
            RACK.to_string(),
            both(Script {
                cry: Some(rack_cry()),
                ..Script::default()
            }),
        ),
        (
            CHEST.to_string(),
            both(Script {
                cry: Some(random_cry(&[BLADE, SHIELD], true)),
                ..Script::default()
            }),
        ),
        (
            LOCKER.to_string(),
            both(Script {
                cry: Some(random_cry(&[SHIELD], false)),
                ..Script::default()
            }),
        ),
        (
            STATUE.to_string(),
            both(Script {
                cry: Some(rack_cry()),
                ..Script::default()
            }),
        ),
        (
            PROTO.to_string(),
            both(Script {
                static_flags: Some(StaticFlags {
                    tribute: Some(PROTO_TRIBUTE),
                    ..StaticFlags::default()
                }),
                hand_triggers: vec![float_lowers_tribute("wk-proto-float-hand")],
                deck_triggers: vec![float_lowers_tribute("wk-proto-float-deck")],
                ..Script::default()
            }),
        ),
    ])
}

fn defs() -> Vec<CardDef> {
    vec![
        card(18410, BEAM, (0, 4), json!([])),
        card(18411, PEA, (0, 2), json!([])),
        card(18412, WALL, (0, 8), json!([])),
        card(18413, PLAIN, (1, 1), json!([])),
        card(18414, BLADE, (1, 1), json!([])),
        card(18415, SHIELD, (0, 3), json!([])),
        card(18416, RACK, (2, 2), json!([])),
        card(18417, CHEST, (2, 2), json!([])),
        card(18418, STATUE, (2, 2), json!([{ "kind": "Immutable" }])),
        card(18419, PROTO, (3, 3), json!([])),
        card(18420, LOCKER, (2, 2), json!([])),
    ]
}

/// The engine's fixture catalog (`harness::setup_catalog`) with these cards and scripts on top.
pub fn register_weapon_kit_fixtures() {
    super::harness::setup_catalog();
    let mut catalog: CardDefs = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(scripts());
    register_scripts(merged);
}
