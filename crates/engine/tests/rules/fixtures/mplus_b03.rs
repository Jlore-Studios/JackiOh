//! Test-only cards for the systems Meditative+ batch 03 builds (issue #1181): an effect made "in N
//! turns" with its countdown badge (`delay`'s `turns` and `label`, R1181000), Mountaintop Garden's
//! aura over embiggen prices (the `embiggenGarden` static flag, R1181002) and a price for the
//! Embiggened version alone (`add_to_hand`'s `embiggenCostOverride`, R1181003). Each does it through
//! the engine's own verbs and flags, as a card script would, so the engine never imports
//! `crates/cards`; the real cards (M+ #4 Supply Drop, #22 Mountaintop Garden, #23 Enormous
//! Elderberry) test the same cases again.
//!
//! Ids are prefixed `b3-` and indexed from 11840, so they collide with no other fixture file's.

use std::sync::LazyLock;

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

/// The fixture "in N turns" counts.
pub const LATER_TURNS: i32 = 3;
/// What the delayed hit deals to each enemy.
pub const LATER_DAMAGE: i32 = 2;
/// The countdown's caption.
pub const LATER_LABEL: &str = "later";
/// The swell fixture's printed prices: "2 embiggen 4".
pub const SWELL_BASE: i32 = 2;
pub const SWELL_EMBIGGEN: i32 = 4;
/// The garden fixture's premium on its two faces (Mountaintop Garden's 2 and 1).
pub const GARDEN_PREMIUM: i32 = 2;
pub const GARDEN_PREMIUM_RADIANT: i32 = 1;
/// The price the elder fixture gives the Embiggened version alone (Enormous Elderberry's (0)).
pub const ELDER_COST: i32 = 0;

const HIT: &str = "hit";

fn card(index: u32, name: &str, type_: &str, cost: Value) -> CardDef {
    json_as(json!({
        "id": format!("b3-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (batch 03 fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    }))
}

/// A (0) Spell: "In 3 turns, deal 2 damage to each enemy, with Pierce and Lifesteal" (M+ #4's Combat).
pub static LATER: LazyLock<CardDef> = LazyLock::new(|| card(11840, "later", "Spell", json!(0)));
/// A (2, Embiggened 4) Field Spell with no text: the embiggen card the garden reaches.
pub static SWELL: LazyLock<CardDef> = LazyLock::new(|| {
    card(
        11841,
        "swell",
        "Field Spell",
        json!({ "base": SWELL_BASE, "embiggen": SWELL_EMBIGGEN }),
    )
});
/// A (0) Field Spell: "Aura: Embiggen cards play their Embiggened version at their normal price; for
/// (2) more (Radiant: (1)), one that isn't Radiant is also played Radiant" (M+ #22's shape).
pub static GARDEN: LazyLock<CardDef> = LazyLock::new(|| card(11842, "garden", "Field Spell", json!(0)));
/// A (0) Spell: "Cast a swell" (R545's cast, which the garden does not reach).
pub static CASTER: LazyLock<CardDef> = LazyLock::new(|| card(11843, "caster", "Spell", json!(0)));
/// A (0) Spell: "Add a swell to your hand; its Embiggened version costs (0)" (M+ #23's shape).
pub static ELDER: LazyLock<CardDef> = LazyLock::new(|| card(11844, "elder", "Spell", json!(0)));

fn both(script: impl Fn() -> Script) -> CardScripts {
    CardScripts {
        base: script(),
        radiant: script(),
    }
}

fn garden(premium: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            embiggen_garden: Some(premium),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            LATER.id.clone(),
            both(|| Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::delay(json_as(json!({
                        "at": { "phase": "start", "player": "self" },
                        "step": HIT,
                        "hook": RESUME_HOOK,
                        "turns": LATER_TURNS,
                        "label": LATER_LABEL,
                    })))]
                })),
                resume: IndexMap::from([(
                    HIT,
                    hook(|_ctx| {
                        vec![effects::damage_all(json_as(json!({
                            "amount": LATER_DAMAGE,
                            "side": "enemy",
                            "heroes": true,
                            "ignoreArmor": true,
                            "lifesteal": true,
                        })))]
                    }),
                )]),
                ..Script::default()
            }),
        ),
        (
            GARDEN.id.clone(),
            CardScripts {
                base: garden(GARDEN_PREMIUM),
                radiant: garden(GARDEN_PREMIUM_RADIANT),
            },
        ),
        (
            CASTER.id.clone(),
            both(|| Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::cast_new(json_as(json!({ "def": SWELL.id })))]
                })),
                ..Script::default()
            }),
        ),
        (
            ELDER.id.clone(),
            both(|| Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::add_to_hand(effects::AddToHandArgs {
                        def_id: Some(SWELL.id.clone()),
                        embiggen_cost_override: Some(ELDER_COST),
                        ..effects::AddToHandArgs::default()
                    })]
                })),
                ..Script::default()
            }),
        ),
    ])
});

pub static B03_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        LATER.clone(),
        SWELL.clone(),
        GARDEN.clone(),
        CASTER.clone(),
        ELDER.clone(),
    ]
});

/// The engine's fixture catalog (`harness::setup_catalog`) with these cards and scripts on top.
pub fn register_b03_fixtures() {
    super::harness::setup_catalog();
    let mut defs: CardDefs = registered_catalog().clone();
    for def in B03_DEFS.iter() {
        defs.insert(def.id.clone(), def.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}
