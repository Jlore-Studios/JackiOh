//! Test-only cards for Burn and Toxic (issue #3540, R3540000–R3540005): Units that carry a rider or a
//! conversion in their static flags, Spells that run the `burn`, `toxic`, `burnAll` and `toxicAll` verbs,
//! and plain Units for each defence the tick and the conversion meet, so the engine never imports
//! `crates/cards`. Ids are `bt-` from 35400, clear of every other fixture file's.

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

use super::harness::both;

/// What `bt-burner` adds to the Unit it aims at.
pub const BT_BURN: i32 = 4;
/// What `bt-toxicant` adds to the enemy hero.
pub const BT_TOXIC: i32 = 2;
/// What `bt-scourge` adds of each counter to every enemy Unit and the enemy hero.
pub const BT_SWEEP: i32 = 3;
/// What `bt-mace` and `bt-venom` add after each hit their target survives.
pub const BT_RIDER: i32 = 3;
/// What `bt-rounds` adds to its host's riders as an Equip.
pub const BT_ROUNDS: i32 = 2;

/// A (0) 2/20 Unit with no text: what a tick or a hit can land on without dying.
pub const PLAIN: &str = "bt-plain";
/// A (0) 2/2 Unit with no text: what a few points of damage kill.
pub const FRAIL: &str = "bt-frail";
/// A (0) 2/20 Unit with Armor 2.
pub const ARMORED: &str = "bt-armored";
/// A (0) 2/20 Unit with Divine Shield.
pub const SHIELDED: &str = "bt-shielded";
/// A (0) 2/20 Unit with Indestructible.
pub const STONE: &str = "bt-stone";
/// A (0) 3/8 Unit: "Damage also Burns 3."
pub const MACE: &str = "bt-mace";
/// A (0) 3/8 Unit: "Damage also applies Toxic 3."
pub const VENOM: &str = "bt-venom";
/// A (0) 1/1 Unit with Equip: "Damage also Burns 2."
pub const ROUNDS: &str = "bt-rounds";
/// A (0) 4/8 Unit: "Damage dealt Burns."
pub const VOLCANIC: &str = "bt-volcanic";
/// A (0) 4/8 Unit: "Damage dealt is Toxic."
pub const NOXIOUS: &str = "bt-noxious";
/// A (0) 4/8 Unit with both conversions.
pub const BOTH: &str = "bt-both";
/// A (0) Spell: "Burn a Unit 4."
pub const BURNER: &str = "bt-burner";
/// A (0) Spell: "Return a Unit to its owner's hand."
pub const BOUNCER: &str = "bt-bouncer";
/// A (0) Spell: "A Unit loses its text" (a Vanilla).
pub const VANILLA: &str = "bt-vanilla";
/// A (0) Spell: "Nerf a Unit three times."
pub const NERF: &str = "bt-nerf";
/// A (0) Spell: "Toxic the enemy hero 2."
pub const TOXICANT: &str = "bt-toxicant";
/// A (0) Spell: "Burn and Toxic every enemy Unit and the enemy hero 3."
pub const SCOURGE: &str = "bt-scourge";

fn card(index: u32, id: &str, type_: &str, stats: Option<(i32, i32)>, keywords: Value) -> CardDef {
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
        "index": index.to_string(),
        "name": format!("{id} (burn and toxic fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": base,
        "radiant": radiant,
    }))
}

fn flagged(flags: StaticFlags) -> Script {
    Script {
        static_flags: Some(flags),
        ..Script::default()
    }
}

/// A Spell aimed at one Unit, its Cry the effects `make` builds from the chosen one.
fn aimed(make: fn() -> Vec<Effect>) -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "of": ["unit"] }))],
        cry: Some(hook(move |_ctx| make())),
        ..Script::default()
    }
}

fn chosen() -> Value {
    json!({ "of": "chosen" })
}

pub fn scripts() -> IndexMap<String, CardScripts> {
    let entries: Vec<(&str, Script)> = vec![
        (
            MACE,
            flagged(StaticFlags {
                burns_on_damage: Some(BT_RIDER),
                ..StaticFlags::default()
            }),
        ),
        (
            VENOM,
            flagged(StaticFlags {
                toxic_on_damage: Some(BT_RIDER),
                ..StaticFlags::default()
            }),
        ),
        (
            ROUNDS,
            flagged(StaticFlags {
                burns_on_damage: Some(BT_ROUNDS),
                ..StaticFlags::default()
            }),
        ),
        (
            VOLCANIC,
            flagged(StaticFlags {
                damage_burns: Some(true),
                ..StaticFlags::default()
            }),
        ),
        (
            NOXIOUS,
            flagged(StaticFlags {
                damage_is_toxic: Some(true),
                ..StaticFlags::default()
            }),
        ),
        (
            BOTH,
            flagged(StaticFlags {
                damage_burns: Some(true),
                damage_is_toxic: Some(true),
                ..StaticFlags::default()
            }),
        ),
        (
            BURNER,
            aimed(|| {
                vec![effects::burn(json_as(
                    json!({ "to": chosen(), "amount": BT_BURN }),
                ))]
            }),
        ),
        (
            BOUNCER,
            aimed(|| vec![effects::bounce(json_as(json!({ "target": chosen() })))]),
        ),
        (VANILLA, aimed(|| vec![effects::vanilla(json_as(chosen()))])),
        (
            NERF,
            aimed(|| {
                vec![effects::degrade(json_as(
                    json!({ "target": chosen(), "times": 3 }),
                ))]
            }),
        ),
        (
            TOXICANT,
            Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::toxic(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": BT_TOXIC }),
                    ))]
                })),
                ..Script::default()
            },
        ),
        (
            SCOURGE,
            Script {
                cry: Some(hook(|_ctx| {
                    let sweep = json!({ "amount": BT_SWEEP, "heroes": true, "side": "enemy" });
                    vec![
                        effects::burn_all(json_as(sweep.clone())),
                        effects::toxic_all(json_as(sweep)),
                    ]
                })),
                ..Script::default()
            },
        ),
    ];
    entries
        .into_iter()
        .map(|(id, script)| (id.to_string(), both(script)))
        .collect()
}

pub fn defs() -> Vec<CardDef> {
    vec![
        card(35400, PLAIN, "Unit", Some((2, 20)), json!([])),
        card(35401, FRAIL, "Unit", Some((2, 2)), json!([])),
        card(
            35402,
            ARMORED,
            "Unit",
            Some((2, 20)),
            json!([{ "kind": "Armor", "n": 2 }]),
        ),
        card(
            35403,
            SHIELDED,
            "Unit",
            Some((2, 20)),
            json!([{ "kind": "Divine Shield" }]),
        ),
        card(
            35404,
            STONE,
            "Unit",
            Some((2, 20)),
            json!([{ "kind": "Indestructible" }]),
        ),
        card(35405, MACE, "Unit", Some((3, 8)), json!([])),
        card(35406, VENOM, "Unit", Some((3, 8)), json!([])),
        card(35407, ROUNDS, "Unit", Some((1, 1)), json!([{ "kind": "Equip" }])),
        card(35408, VOLCANIC, "Unit", Some((4, 8)), json!([])),
        card(35409, NOXIOUS, "Unit", Some((4, 8)), json!([])),
        card(35410, BOTH, "Unit", Some((4, 8)), json!([])),
        card(35411, BURNER, "Spell", None, json!([])),
        card(35412, BOUNCER, "Spell", None, json!([])),
        card(35413, VANILLA, "Spell", None, json!([])),
        card(35414, NERF, "Spell", None, json!([])),
        card(35415, TOXICANT, "Spell", None, json!([])),
        card(35416, SCOURGE, "Spell", None, json!([])),
    ]
}

/// The engine's fixture catalog (`harness::setup_catalog`) with these cards and scripts on top.
pub fn register_burn_toxic_fixtures() {
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
