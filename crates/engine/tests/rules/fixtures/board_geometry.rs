//! Test-only cards for board geometry and damage routing (issue #3554, R3554000–R3554009):
//! plain Units, an Armored one, a Cleaver, a Taunt, a GIGA, a Crater, a +1-attack aura, a hole Spell
//! (Radiant: opponent only), a slash Spell (Radiant: the lane pick), an untargeted blast, a targeted
//! bolt, Units whose Cry sweeps the enemy or summons a plain one, a Lancer (Flexible, Discovers its
//! tile), a Vela (every tile, Cleave, laser), an Arc (chain lightning) and a Fuse (Ranged, sweep
//! 5/3), so the engine never imports `crates/cards`. Ids are `geo-` from 35540, clear of every other
//! fixture file's.

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

use super::harness::both;

/// A (0) 1/3 Unit with no text.
pub const PLAIN: &str = "geo-plain";
/// A (0) 1/6 Unit with Armor 2.
pub const ARMORED: &str = "geo-armored";
/// A (0) 3/8 Unit with Cleave.
pub const CLEAVER: &str = "geo-cleaver";
/// A (0) 1/5 Unit with Taunt.
pub const TAUNT: &str = "geo-taunt";
/// A (0) 2/6 Unit with GIGA.
pub const GIGA: &str = "geo-giga";
/// A (0) 1/3 Unit with Crater.
pub const CRATER: &str = "geo-crater";
/// A (0) 1/3 Unit: "Aura: your Units have +1 attack."
pub const AURA: &str = "geo-aura";
/// A (0) Spell: "Open a Black Hole in midlane." Radiant: opponent only.
pub const HOLE: &str = "geo-hole";
/// A (0) Spell: "Slash the board." Radiant: choose the lane.
pub const SLASH: &str = "geo-slash";
/// A (0) Spell: "Deal 2 damage to every enemy Unit."
pub const BLAST: &str = "geo-blast";
/// A (0) Spell: "Deal 3 damage to a Unit."
pub const BOLT: &str = "geo-bolt";
/// A (0) 1/3 Unit whose Cry deals 1 to each enemy Unit.
pub const CRIER: &str = "geo-crier";
/// A (0) 1/3 Unit whose Cry summons a plain one.
pub const MOTHER: &str = "geo-mother";
/// A (0) 2/4 Unit with Flexible whose Cry draws a line to a Discovered enemy tile.
pub const LANCER: &str = "geo-lancer";
/// A (0) 2/4 Unit whose Cry draws the line to any enemy tile, with Cleave, as a laser.
pub const VELA: &str = "geo-vela";
/// A (0) 1/5 Unit whose Cry deals 2 to a Unit, chaining at random to adjacent Units.
pub const ARC: &str = "geo-arc";
/// A (0) 2/20 Unit with Ranged: its attack hits its lane 5 times, adjacent lanes 3 times.
pub const FUSE: &str = "geo-fuse";
/// A (0) 1/20 Unit with no text: what a sweep lands on without dying.
pub const BULWARK: &str = "geo-bulwark";

/// The resume step the slash lane answer re-enters.
pub const SLASH_STEP: &str = "slashLane";

/// Lancer's line: 3 damage, Discovered from three tiles, no Cleave, no laser.
pub const LANCER_RULES: effects::LineRules = effects::LineRules {
    amount: 3,
    offers: Some(DISCOVER_OPTIONS),
    cleave: false,
    laser: false,
    pierce: true,
};

/// Vela's line: 3 damage, every enemy tile offered, Cleave, laser.
pub const VELA_RULES: effects::LineRules = effects::LineRules {
    amount: 3,
    offers: None,
    cleave: true,
    laser: true,
    pierce: true,
};

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
        "name": format!("{id} (board geometry fixture)"),
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

/// A Spell aimed at one Unit, its Cry the effects `make` builds.
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

/// The +1-attack aura both faces carry.
fn aura_script() -> Script {
    Script {
        aura: Some(aura_hook(move |a| {
            let controller = a.self_.controller;
            vec![AuraEntry {
                applies: Box::new(move |unit: &CardInstance| {
                    unit.controller == controller && matches!(unit.zone, Zone::Field { .. })
                }),
                mod_: StatMod {
                    attack: Some(1),
                    ..StatMod::default()
                },
            }]
        })),
        ..Script::default()
    }
}

/// The untargeted blast: 2 to every enemy Unit.
fn blast_script() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![effects::damage_all(json_as(
                json!({ "amount": 2, "side": "enemy" }),
            ))]
        })),
        ..Script::default()
    }
}

/// The targeted bolt: 3 to the chosen Unit.
fn bolt_script() -> Script {
    aimed(|| vec![effects::damage(json_as(json!({ "to": chosen(), "amount": 3 })))])
}

/// The crier: 1 to each enemy Unit.
fn crier_script() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![effects::damage_all(json_as(
                json!({ "amount": 1, "side": "enemy" }),
            ))]
        })),
        ..Script::default()
    }
}

/// The mother: summon a plain one.
fn mother_script() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![effects::summon(json_as(json!({ "defId": PLAIN })))]
        })),
        ..Script::default()
    }
}

/// The arc: 2 to the chosen Unit, chaining at random to adjacent Units.
fn arc_script() -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "of": ["unit"] }))],
        cry: Some(hook(|_ctx| {
            vec![effects::chain_lightning(json_as(
                json!({ "to": chosen(), "amount": 2 }),
            ))]
        })),
        ..Script::default()
    }
}

/// A line Unit's script: the Cry opens the tile Discover, the resume draws the line.
fn line_script(rules: effects::LineRules) -> Script {
    Script {
        cry: Some(hook(move |_ctx| effects::line_damage_begin(rules))),
        resume: IndexMap::from([(
            effects::LINE_STEP,
            hook(move |ctx| effects::line_damage_answered(rules, ctx)),
        )]),
        ..Script::default()
    }
}

pub fn scripts() -> IndexMap<String, CardScripts> {
    let mut all: IndexMap<String, CardScripts> = IndexMap::new();
    for id in [PLAIN, ARMORED, CLEAVER, TAUNT, GIGA, CRATER, BULWARK] {
        all.insert(id.to_string(), both(Script::default()));
    }
    all.insert(AURA.to_string(), both(aura_script()));
    all.insert(BLAST.to_string(), both(blast_script()));
    all.insert(CRIER.to_string(), both(crier_script()));
    all.insert(MOTHER.to_string(), both(mother_script()));
    all.insert(LANCER.to_string(), both(line_script(LANCER_RULES)));
    all.insert(VELA.to_string(), both(line_script(VELA_RULES)));
    all.insert(BOLT.to_string(), both(bolt_script()));
    all.insert(ARC.to_string(), both(arc_script()));
    all.insert(
        FUSE.to_string(),
        both(flagged(StaticFlags {
            sweep_lane: Some(5),
            sweep_adjacent: Some(3),
            ..StaticFlags::default()
        })),
    );
    all.insert(
        HOLE.to_string(),
        CardScripts {
            base: Script {
                cry: Some(hook(|_ctx| vec![effects::open_black_hole(json_as(json!({})))])),
                ..Script::default()
            },
            radiant: Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::open_black_hole(json_as(json!({ "opponentOnly": true })))]
                })),
                ..Script::default()
            },
        },
    );
    all.insert(
        SLASH.to_string(),
        CardScripts {
            base: Script {
                cry: Some(hook(|_ctx| vec![effects::slash_board(json_as(json!({})))])),
                ..Script::default()
            },
            radiant: Script {
                cry: Some(hook(|_ctx| vec![effects::ask_slash_lane(SLASH_STEP)])),
                resume: IndexMap::from([(
                    SLASH_STEP,
                    hook(|ctx| {
                        let lane = effects::chosen_slash_lane(ctx);
                        vec![effects::slash_board(json_as(json!({ "lane": lane })))]
                    }),
                )]),
                ..Script::default()
            },
        },
    );
    all
}

pub fn defs() -> Vec<CardDef> {
    vec![
        card(35540, PLAIN, "Unit", Some((1, 3)), json!([])),
        card(
            35541,
            ARMORED,
            "Unit",
            Some((1, 6)),
            json!([{ "kind": "Armor", "n": 2 }]),
        ),
        card(
            35542,
            CLEAVER,
            "Unit",
            Some((3, 8)),
            json!([{ "kind": "Cleave" }]),
        ),
        card(35543, TAUNT, "Unit", Some((1, 5)), json!([{ "kind": "Taunt" }])),
        card(35544, GIGA, "Unit", Some((2, 6)), json!([{ "kind": "GIGA" }])),
        card(35545, CRATER, "Unit", Some((1, 3)), json!([{ "kind": "Crater" }])),
        card(35546, AURA, "Unit", Some((1, 3)), json!([])),
        card(35547, HOLE, "Spell", None, json!([])),
        card(35548, SLASH, "Spell", None, json!([])),
        card(35549, BLAST, "Spell", None, json!([])),
        card(35550, BOLT, "Spell", None, json!([])),
        card(35551, CRIER, "Unit", Some((1, 3)), json!([])),
        card(35552, MOTHER, "Unit", Some((1, 3)), json!([])),
        card(
            35553,
            LANCER,
            "Unit",
            Some((2, 4)),
            json!([{ "kind": "Flexible" }]),
        ),
        card(35554, VELA, "Unit", Some((2, 4)), json!([])),
        card(35555, ARC, "Unit", Some((1, 5)), json!([])),
        card(35556, FUSE, "Unit", Some((2, 20)), json!([{ "kind": "Ranged" }])),
        card(35557, BULWARK, "Unit", Some((1, 20)), json!([])),
    ]
}

/// The engine's fixture catalog (`harness::setup_catalog`) with these cards and scripts on top.
pub fn register_geometry_fixtures() {
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
