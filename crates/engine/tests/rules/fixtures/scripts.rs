//! Test-only cards for M1-M3 engine tests (BUILD §0): each reproduces one behaviour of a real card
//! through the effects library.

use std::sync::LazyLock;

use jackioh_engine::effects;
use jackioh_engine::subsystems::hero_power::{POWER_RESUME, STEADY_SHOT_PARAM, hero_power, power_abilities};
use jackioh_engine::testkit::*;

/// `b`'s keys replace `a`'s.
fn spread(mut base: Value, extra: Value) -> Value {
    if let (Some(into), Value::Object(from)) = (base.as_object_mut(), extra) {
        for (key, value) in from {
            into.insert(key, value);
        }
    }
    base
}

/// "All enemies": every enemy unit plus the enemy hero, one instance each (R51).
fn damage_all_enemies(amount: i32) -> Effect {
    Effect::new("fixture:damageAllEnemies", move |ctx| {
        let enemy = opponent_of(ctx.controller);
        let units: Vec<CardInstance> = zones::active_units_of(&*ctx.state, enemy)
            .into_iter()
            .cloned()
            .collect();
        for unit in units {
            let args = damage::DamageArgs {
                source: ctx.self_.clone(),
                target: damage::DamageTarget::Unit { instance: unit },
                amount,
                flags: None,
            };
            damage::deal_damage(ctx, args);
        }
        let args = damage::DamageArgs {
            source: ctx.self_.clone(),
            target: damage::DamageTarget::Hero { player: enemy },
            amount,
            flags: None,
        };
        damage::deal_damage(ctx, args);
    })
}

/// `id`, `index`, `name` and `type` are the overrides' own.
fn def(overrides: Value) -> CardDef {
    json_as(spread(
        json!({
            "set": "Core",
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": { "keywords": [], "text": "" },
            "radiant": { "keywords": [], "text": "" },
        }),
        overrides,
    ))
}

fn flags(static_flags: Value) -> Option<StaticFlags> {
    Some(json_as(static_flags))
}

/// #21 Hinder: cast on draw, the opponent's next refresh is 1 lower (2 radiant).
pub fn hinder() -> CardDef {
    def(json!({ "id": "fx-hinder", "index": "21", "name": "Hinder (fixture)", "type": "Spell", "cost": 0 }))
}

fn hinder_scripts() -> CardScripts {
    CardScripts {
        base: Script {
            static_flags: flags(json!({ "castOnDraw": true })),
            cry: Some(hook(|_ctx| {
                vec![effects::next_turn_mana(json_as(
                    json!({ "player": "enemy", "amount": -1 }),
                ))]
            })),
            ..Script::default()
        },
        radiant: Script {
            static_flags: flags(json!({ "castOnDraw": true })),
            cry: Some(hook(|_ctx| {
                vec![effects::next_turn_mana(json_as(
                    json!({ "player": "enemy", "amount": -2 }),
                ))]
            })),
            ..Script::default()
        },
    }
}

/// #90.1 CN-Virus: cast on draw, 1 damage to your hero, 2 copies shuffled in (3 radiant).
pub fn cn_virus() -> CardDef {
    def(json!({
        "id": "fx-cn-virus",
        "index": "90.1",
        "name": "CN-Virus (fixture)",
        "type": "Spell",
        "tags": ["Token", "CN"],
        "rarity": "Token",
        "token": true,
    }))
}

fn cn_virus_scripts() -> CardScripts {
    CardScripts {
        base: Script {
            static_flags: flags(json!({ "castOnDraw": true })),
            cry: Some(hook(|_ctx| {
                vec![
                    effects::damage(json_as(json!({ "to": { "of": "selfHero" }, "amount": 1 }))),
                    effects::shuffle_copies_of_self(json_as(json!({ "count": 2 }))),
                ]
            })),
            ..Script::default()
        },
        radiant: Script {
            static_flags: flags(json!({ "castOnDraw": true })),
            cry: Some(hook(|_ctx| {
                vec![
                    effects::damage(json_as(json!({ "to": { "of": "selfHero" }, "amount": 1 }))),
                    effects::shuffle_copies_of_self(json_as(json!({ "count": 3 }))),
                ]
            })),
            ..Script::default()
        },
    }
}

/// #37 Gravedigger: at the start of your turn, a random graveyard card returns to your hand.
pub fn gravedigger() -> CardDef {
    def(json!({
        "id": "fx-gravedigger",
        "index": "37",
        "name": "Gravedigger (fixture)",
        "type": "Unit",
        "base": { "attack": 4, "health": 5, "keywords": [], "text": "" },
        "radiant": { "attack": 8, "health": 10, "keywords": [], "text": "" },
        "cost": 2,
    }))
}

fn gravedigger_scripts() -> CardScripts {
    let script = || Script {
        start_of_turn: Some(hook(|_ctx| {
            vec![effects::add_random_from_graveyard(Default::default())]
        })),
        ..Script::default()
    };
    CardScripts {
        base: script(),
        radiant: script(),
    }
}

/// #5 Stockpile: draw 2.
pub fn stockpile() -> CardDef {
    def(json!({ "id": "fx-stockpile", "index": "5", "name": "Stockpile (fixture)", "type": "Spell" }))
}

fn stockpile_scripts() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| vec![effects::draw(json_as(json!({ "count": 2 })))])),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| vec![effects::draw(json_as(json!({ "count": 5 })))])),
            ..Script::default()
        },
    }
}

/// #75 Infinite Reserves: an empty-library draw gives a Rush Token card instead of fatigue.
pub fn infinite_reserves() -> CardDef {
    def(json!({
        "id": "fx-infinite-reserves",
        "index": "75",
        "name": "Infinite Reserves (fixture)",
        "type": "Field Spell",
        "cost": 0,
    }))
}

fn infinite_reserves_scripts() -> CardScripts {
    let script = || Script {
        static_flags: flags(json!({ "infiniteReserves": true })),
        ..Script::default()
    };
    CardScripts {
        base: script(),
        radiant: script(),
    }
}

/// #73 Anti-oneshot Armor: your hero takes at most 5 damage per instance (3 radiant).
pub fn anti_oneshot() -> CardDef {
    def(json!({
        "id": "fx-anti-oneshot",
        "index": "73",
        "name": "Anti-oneshot Armor (fixture)",
        "type": "Field Spell",
        "cost": 2,
    }))
}

fn anti_oneshot_scripts() -> CardScripts {
    let script = || Script {
        static_flags: flags(json!({ "antiOneshot": true })),
        ..Script::default()
    };
    CardScripts {
        base: script(),
        radiant: script(),
    }
}

/// #6 Mana Well: at the start of your turn, gain 1 temporary mana.
pub fn mana_well() -> CardDef {
    def(json!({
        "id": "fx-mana-well",
        "index": "6",
        "name": "Mana Well (fixture)",
        "type": "Field Spell",
        "cost": 3,
    }))
}

fn mana_well_scripts() -> CardScripts {
    CardScripts {
        base: Script {
            start_of_turn: Some(hook(|_ctx| {
                vec![effects::gain_mana(json_as(json!({ "amount": 1 })))]
            })),
            ..Script::default()
        },
        radiant: Script {
            start_of_turn: Some(hook(|_ctx| {
                vec![effects::gain_mana(json_as(json!({ "amount": 2 })))]
            })),
            ..Script::default()
        },
    }
}

/// #84 Going Long: a Quickdraw Field Spell that gives the hero Armor.
pub fn going_long() -> CardDef {
    def(json!({
        "id": "fx-going-long",
        "index": "84",
        "name": "Going Long (fixture)",
        "type": "Field Spell",
        "tags": ["Quickdraw"],
        "cost": { "base": 2, "embiggen": 4 },
    }))
}

fn going_long_scripts() -> CardScripts {
    let script = || Script {
        static_flags: flags(json!({ "quickdraw": true })),
        ..Script::default()
    };
    CardScripts {
        base: script(),
        radiant: script(),
    }
}

pub use jackioh_engine::subsystems::hero_power::HERO_POWER_NAMES as HERO_POWERS;

/// #98 Heroic Power (R752): costs (0), rolls one of the thirteen powers at start of game, and each power
/// is one of its Activate abilities. The roll is the generic `rememberRandom` over the stored names
/// (R103), which is all setup's tests need; the abilities are the subsystem's own.
pub fn heroic_power() -> CardDef {
    def(json!({
        "id": "fx-heroic-power",
        "index": "98",
        "name": "Heroic Power (fixture)",
        "type": "Field Spell",
        "tags": ["Quickdraw"],
        "cost": 0,
        "params": [{ "key": STEADY_SHOT_PARAM, "base": 2, "radiant": 4, "better": "up", "step": 2, "min": 1 }],
    }))
}

fn heroic_power_script(radiant: bool) -> Script {
    Script {
        static_flags: flags(json!({ "quickdraw": true })),
        start_of_game: Some(hook(|_ctx| {
            vec![effects::remember_random(json_as(
                json!({ "key": "power", "options": HERO_POWERS }),
            ))]
        })),
        activations: power_abilities(radiant),
        resume: IndexMap::from([(POWER_RESUME, hook(hero_power))]),
        ..Script::default()
    }
}

fn heroic_power_scripts() -> CardScripts {
    CardScripts {
        base: heroic_power_script(false),
        radiant: heroic_power_script(true),
    }
}

/// #74 Adaptive UI, trimmed to the X part: deal X damage to the enemy hero.
pub fn x_bolt() -> CardDef {
    def(json!({ "id": "fx-x-bolt", "index": "74", "name": "X Bolt (fixture)", "type": "Spell", "cost": "X" }))
}

fn x_bolt_scripts() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": ctx.x }),
                ))]
            })),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": ctx.x * 2 }),
                ))]
            })),
            ..Script::default()
        },
    }
}

/// #13 Jlockeed Shredder-10: at your end of turn, 2 damage to every enemy unit and the enemy hero.
pub fn shredder() -> CardDef {
    def(json!({
        "id": "fx-shredder",
        "index": "13",
        "name": "Shredder (fixture)",
        "type": "Unit",
        "cost": 3,
        "base": { "attack": 8, "health": 10, "keywords": [], "text": "" },
        "radiant": { "attack": 16, "health": 20, "keywords": [], "text": "" },
    }))
}

fn shredder_scripts() -> CardScripts {
    CardScripts {
        base: Script {
            end_of_turn: Some(hook(|_ctx| vec![damage_all_enemies(2)])),
            ..Script::default()
        },
        radiant: Script {
            end_of_turn: Some(hook(|_ctx| vec![damage_all_enemies(5)])),
            ..Script::default()
        },
    }
}

/// A fixture with no Core counterpart: it damages the enemy hero and then draws, so one effect can
/// leave both heroes at 0 before the state check runs (R59, BUILD M1-T8).
pub fn double_edge() -> CardDef {
    def(json!({
        "id": "fx-double-edge",
        "index": "999",
        "name": "Double Edge (fixture)",
        "type": "Spell",
        "cost": 0,
    }))
}

fn double_edge_scripts() -> CardScripts {
    let script = || Script {
        cry: Some(hook(|_ctx| {
            vec![
                effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 30 }))),
                effects::draw(json_as(json!({ "count": 1 }))),
            ]
        })),
        ..Script::default()
    };
    CardScripts {
        base: script(),
        radiant: script(),
    }
}

pub static FIXTURE_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        hinder(),
        cn_virus(),
        gravedigger(),
        stockpile(),
        infinite_reserves(),
        anti_oneshot(),
        mana_well(),
        going_long(),
        heroic_power(),
        x_bolt(),
        shredder(),
        double_edge(),
    ]
});

pub static FIXTURE_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let mut table = IndexMap::new();
    table.insert(hinder().id, hinder_scripts());
    table.insert(cn_virus().id, cn_virus_scripts());
    table.insert(gravedigger().id, gravedigger_scripts());
    table.insert(stockpile().id, stockpile_scripts());
    table.insert(infinite_reserves().id, infinite_reserves_scripts());
    table.insert(anti_oneshot().id, anti_oneshot_scripts());
    table.insert(mana_well().id, mana_well_scripts());
    table.insert(going_long().id, going_long_scripts());
    table.insert(heroic_power().id, heroic_power_scripts());
    table.insert(x_bolt().id, x_bolt_scripts());
    table.insert(shredder().id, shredder_scripts());
    table.insert(double_edge().id, double_edge_scripts());
    table
});

/// `base` with every fixture definition over it.
pub fn fixture_catalog(base: CardDefs) -> CardDefs {
    let mut defs = base;
    for entry in FIXTURE_DEFS.iter() {
        defs.insert(entry.id.clone(), entry.clone());
    }
    defs
}

/// This file's scripts, by id.
pub fn scripts() -> IndexMap<String, CardScripts> {
    FIXTURE_SCRIPTS.clone()
}
