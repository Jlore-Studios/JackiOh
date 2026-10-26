//! Test-only cards for the Starship (SPEC §6.1 Starship Piece and Starship, §6.2 When launched, §6.3
//! Launch; R1852000–R1852007): a fixture definition of the Dormant Starship under the id the engine
//! summons (`DORMANT_STARSHIP_DEF_ID`), Starship Pieces in the shapes Stinkyscape will print, and the
//! cards that kill, steal, discard, equip, stack onto and launch them through the engine's own verbs.
//! No shipped card is a piece yet (the real Dormant Starship and its pieces come with the set), so the
//! engine never imports `crates/cards` for this.
//!
//! Ids are prefixed `ss-` and indexed from 18520, so they collide with no other fixture file's.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects;
// The verbs and reads as a card script names them, through the prelude (CLAUDE.md rule 5).
use jackioh_engine::prelude::{
    awaken, dormant_starship_of, launched_starship_of, starship_pieces_of, when_launched,
};
use jackioh_engine::testkit::*;

use super::harness::both;

fn def(
    id: &str,
    index: u32,
    type_: &str,
    cost: i32,
    stats: (i32, i32),
    keywords: Value,
    tags: Value,
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
        "name": format!("{id} (Starship fixture)"),
        "set": "Core",
        "type": type_,
        "tags": tags,
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": face,
        "radiant": radiant,
    }))
}

fn piece(name: &str, index: u32, stats: (i32, i32), more: Value, tags: Value) -> CardDef {
    let mut keywords = vec![json!({ "kind": "Starship Piece" })];
    keywords.extend(more.as_array().cloned().unwrap_or_default());
    def(
        &format!("ss-{name}"),
        index,
        "Unit",
        1,
        stats,
        json!(keywords),
        tags,
    )
}

/// The Dormant Starship, as Stinkyscape prints it: "(3) Unit, 0/0. Dormant. Activate: pay (3) to
/// launch." Its Activate is the engine's (`starship::launch_decl`), so its script prints Dormant and the
/// hull and nothing else.
pub static hull: LazyLock<CardDef> = LazyLock::new(|| {
    let mut card = def(
        DORMANT_STARSHIP_DEF_ID,
        18520,
        "Unit",
        3,
        (0, 0),
        json!([]),
        json!([]),
    );
    card.name = "Dormant Starship (Starship fixture)".to_string();
    card
});
/// A 2/3 Human piece: "Starship Piece. When launched: deal 2 damage to the enemy hero."
pub static booster: LazyLock<CardDef> =
    LazyLock::new(|| piece("booster", 18521, (2, 3), json!([]), json!(["Human"])));
/// A 1/1 piece: "Starship Piece. Cry: deal 1 damage to a Unit." — a Cry that needs a target.
pub static gunner: LazyLock<CardDef> = LazyLock::new(|| piece("gunner", 18522, (1, 1), json!([]), json!([])));
/// A 1/1 Felinor piece with Reborn.
pub static phoenix: LazyLock<CardDef> = LazyLock::new(|| {
    piece(
        "phoenix",
        18523,
        (1, 1),
        json!([{ "kind": "Reborn" }]),
        json!(["Felinor"]),
    )
});
/// A 1/2 piece with Equip.
pub static plate: LazyLock<CardDef> =
    LazyLock::new(|| piece("plate", 18524, (1, 2), json!([{ "kind": "Equip" }]), json!([])));
/// A 1/1 Equip Unit that is no piece.
pub static clamp: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "ss-clamp",
        18525,
        "Unit",
        1,
        (1, 1),
        json!([{ "kind": "Equip" }]),
        json!([]),
    )
});
/// A 2/2 Stack Unit: "Stack. Cry: deal 3 damage to the enemy hero."
pub static lander: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "ss-lander",
        18526,
        "Unit",
        1,
        (2, 2),
        json!([{ "kind": "Stack" }]),
        json!([]),
    )
});

/// A Spell: "Deal 1 damage to every Unit."
pub static quake: LazyLock<CardDef> =
    LazyLock::new(|| def("ss-quake", 18527, "Spell", 0, (0, 0), json!([]), json!([])));
/// A Spell: "Deal 5 damage to a Unit."
pub static bolt: LazyLock<CardDef> =
    LazyLock::new(|| def("ss-bolt", 18528, "Spell", 0, (0, 0), json!([]), json!([])));
/// A Spell: "Discard your hand."
pub static dump: LazyLock<CardDef> =
    LazyLock::new(|| def("ss-dump", 18529, "Spell", 0, (0, 0), json!([]), json!([])));
/// A Spell: "Take control of every enemy Unit."
pub static grab: LazyLock<CardDef> =
    LazyLock::new(|| def("ss-grab", 18530, "Spell", 0, (0, 0), json!([]), json!([])));
/// A Spell: "Your Starship launch costs (0)." (#144 Prepare For Liftoff's text.)
pub static liftoff: LazyLock<CardDef> =
    LazyLock::new(|| def("ss-liftoff", 18531, "Spell", 0, (0, 0), json!([]), json!([])));
/// A Spell: "Add to your hand a copy of each Starship Piece your launched Starship holds." (#137.4 Space
/// Doctrine's read of the memory.)
pub static salvage: LazyLock<CardDef> =
    LazyLock::new(|| def("ss-salvage", 18532, "Spell", 0, (0, 0), json!([]), json!([])));
/// A Spell: "Awaken your Dormant Starship." — a launch by an effect.
pub static ignition: LazyLock<CardDef> =
    LazyLock::new(|| def("ss-ignition", 18533, "Spell", 0, (0, 0), json!([]), json!([])));
/// A Trap: "Reveals when either player launches a Starship. Deal 2 damage to the enemy hero."
/// (#131 Spacerace Stimulus's reveal.)
pub static countdown: LazyLock<CardDef> =
    LazyLock::new(|| def("ss-countdown", 18534, "Trap", 0, (0, 0), json!([]), json!([])));
/// A 1/1 Stack Unit piece: "Stack. Starship Piece." — a piece that buries a card.
pub static pod: LazyLock<CardDef> =
    LazyLock::new(|| piece("pod", 18535, (1, 1), json!([{ "kind": "Stack" }]), json!([])));

/// What the booster's "When launched" deals the enemy hero.
pub const LAUNCHED_DAMAGE: i32 = 2;
/// What the gunner's Cry deals the Unit it picks.
pub const CRY_DAMAGE: i32 = 1;
/// What the lander's Cry deals the enemy hero.
pub const LANDER_DAMAGE: i32 = 3;
/// What the countdown trap deals the enemy hero.
pub const TRAP_DAMAGE: i32 = 2;
/// What the bolt deals the Unit it picks.
pub const BOLT_DAMAGE: i32 = 5;

fn to_enemy_hero(amount: i32) -> Effect {
    effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))
}

fn spell(cry: Hook) -> CardScripts {
    both(Script {
        cry: Some(cry),
        ..Script::default()
    })
}

fn hull_text() -> Option<StaticFlags> {
    Some(StaticFlags {
        slumber: Some(SlumberText::default()),
        starship_hull: Some(true),
        ..StaticFlags::default()
    })
}

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            hull.id.clone(),
            both(Script {
                static_flags: hull_text(),
                ..Script::default()
            }),
        ),
        (
            booster.id.clone(),
            both(Script {
                triggers: vec![when_launched("launched", |_ctx, _event| {
                    vec![to_enemy_hero(LAUNCHED_DAMAGE)]
                })],
                ..Script::default()
            }),
        ),
        (
            gunner.id.clone(),
            both(Script {
                targets: vec![TargetDecl::target(1, 1, json!({ "of": ["unit"] }))],
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "chosen" }, "amount": CRY_DAMAGE }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (phoenix.id.clone(), both(Script::default())),
        (plate.id.clone(), both(Script::default())),
        (clamp.id.clone(), both(Script::default())),
        (pod.id.clone(), both(Script::default())),
        (
            lander.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![to_enemy_hero(LANDER_DAMAGE)])),
                ..Script::default()
            }),
        ),
        (
            quake.id.clone(),
            spell(hook(|_ctx| {
                vec![effects::damage_all(json_as(json!({ "amount": 1 })))]
            })),
        ),
        (
            bolt.id.clone(),
            both(Script {
                targets: vec![TargetDecl::target(1, 1, json!({ "of": ["unit"] }))],
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "chosen" }, "amount": BOLT_DAMAGE }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            dump.id.clone(),
            spell(hook(|_ctx| vec![effects::discard_hand(json_as(json!({})))])),
        ),
        (
            grab.id.clone(),
            spell(hook(|_ctx| vec![effects::steal_all(json_as(json!({})))])),
        ),
        (
            liftoff.id.clone(),
            spell(hook(|_ctx| {
                vec![effects::starship_launch_costs(json_as(json!({ "cost": 0 })))]
            })),
        ),
        (
            salvage.id.clone(),
            spell(hook(|ctx| {
                let Some(ship) = launched_starship_of(ctx.state, ctx.controller) else {
                    return Vec::new();
                };
                starship_pieces_of(ship)
                    .into_iter()
                    .map(|piece| {
                        effects::add_to_hand(json_as(
                            json!({ "defId": piece.def_id, "radiant": piece.radiant == Some(true) }),
                        ))
                    })
                    .collect()
            })),
        ),
        (
            ignition.id.clone(),
            spell(hook(|ctx| {
                let Some(ship) = dormant_starship_of(ctx.state, ctx.controller) else {
                    return Vec::new();
                };
                vec![awaken(json_as(
                    json!({ "target": { "of": "instance", "instanceId": ship.id } }),
                ))]
            })),
        ),
        (
            countdown.id.clone(),
            both(Script {
                triggers: vec![TriggerDef::new(
                    "reveal",
                    &[GameEventType::StarshipLaunched],
                    |_ctx, _event| vec![to_enemy_hero(TRAP_DAMAGE)],
                )],
                ..Script::default()
            }),
        ),
    ])
});

pub static STARSHIP_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        hull.clone(),
        booster.clone(),
        gunner.clone(),
        phoenix.clone(),
        plate.clone(),
        clamp.clone(),
        lander.clone(),
        quake.clone(),
        bolt.clone(),
        dump.clone(),
        grab.clone(),
        liftoff.clone(),
        salvage.clone(),
        ignition.clone(),
        countdown.clone(),
        pod.clone(),
    ]
});

/// The engine's fixture catalog (`harness::setup_catalog`) with these cards and scripts on top.
pub fn register_starship_fixtures() {
    super::harness::setup_catalog();
    let mut defs: CardDefs = registered_catalog().clone();
    for card in STARSHIP_DEFS.iter() {
        defs.insert(card.id.clone(), card.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}
