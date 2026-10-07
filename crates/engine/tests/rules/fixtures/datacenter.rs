//! Test-only definitions and scripts for `effects/datacenter.ts` (T-AI-4 Chain of Thought's chained draw,
//! T-AI-6 Datacenter Fire's sweep). The engine does not depend on `packages/cards`; the real cards' tests
//! cover the same cases again (packages/cards/test/classic-plus/t-ai-04-*.test.ts, t-ai-06-*.test.ts).
//!
//! Port of `packages/engine/test/fixtures/datacenter.ts`: each exported def is a `pub static` under TS's
//! name snake_cased (`hard_field`, `x_cost`, `cast_on_draw`).

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects::damage;
use jackioh_engine::testkit::*;

use super::catalog::{spell_def, unit_def};

/// The running Spell (either verb is a Spell's).
pub static runner: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        940,
        json!({ "id": "fx-dc-runner", "name": "Fixture Datacenter Spell", "tags": ["AI", "Token"], "token": true, "rarity": "Token" }),
    )
});

/// Field Spells: a plain one, an Indestructible one (#98's keyword), and one whose Death hits the enemy hero.
pub static field: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        941,
        json!({ "id": "fx-dc-field", "name": "Fixture Field Spell", "type": "Field Spell" }),
    )
});
pub static hard_field: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        942,
        json!({
            "id": "fx-dc-hard",
            "name": "Fixture Indestructible Field Spell",
            "type": "Field Spell",
            "base": { "keywords": [{ "kind": "Indestructible" }], "text": "Indestructible" },
            "radiant": { "keywords": [{ "kind": "Indestructible" }], "text": "Indestructible" },
        }),
    )
});
pub static dying_field: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        943,
        json!({ "id": "fx-dc-dying", "name": "Fixture Dying Field Spell", "type": "Field Spell" }),
    )
});
/// DYING_FIELD's Death: 3 damage to the enemy hero of its controller.
pub const DYING_FIELD_DAMAGE: i32 = 3;

/// An Animated Field Spell, which prints the attack and health of the Unit it becomes (B3.1 rule 1).
pub static animated_field: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        951,
        json!({
            "id": "fx-dc-animated",
            "name": "Fixture Animated Field Spell",
            "type": "Field Spell",
            "base": { "attack": 2, "health": 3, "keywords": [{ "kind": "Animated" }], "text": "Animated" },
            "radiant": { "attack": 4, "health": 6, "keywords": [{ "kind": "Animated" }], "text": "Animated" },
        }),
    )
});

/// The backrow cards that are no Field Spells.
pub static trap: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        944,
        json!({ "id": "fx-dc-trap", "name": "Fixture Trap", "type": "Trap" }),
    )
});
pub static field_trap: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        945,
        json!({ "id": "fx-dc-ftrap", "name": "Fixture Field Trap", "type": "Field Trap" }),
    )
});

/// Library cards for the chained draw: (0), (1), (2) and (X) Spells, a (2) Unit and a cast-on-draw Spell.
pub static free: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        946,
        json!({ "id": "fx-dc-free", "name": "Fixture (0) Spell", "cost": 0 }),
    )
});
pub static one: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        947,
        json!({ "id": "fx-dc-one", "name": "Fixture (1) Spell", "cost": 1 }),
    )
});
pub static two: LazyLock<CardDef> = LazyLock::new(|| {
    unit_def(
        948,
        json!({ "id": "fx-dc-two", "name": "Fixture (2) Unit", "cost": 2 }),
    )
});
pub static x_cost: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        949,
        json!({ "id": "fx-dc-x", "name": "Fixture (X) Spell", "cost": "X" }),
    )
});
pub static cast_on_draw: LazyLock<CardDef> = LazyLock::new(|| {
    spell_def(
        950,
        json!({ "id": "fx-dc-cod", "name": "Fixture Cast On Draw", "cost": 0 }),
    )
});

/// A Unit whose aura caps each hit on its hero at 1, as C+ #11 Anime Armor's does (E6, `heroGuard`).
pub static guard: LazyLock<CardDef> = LazyLock::new(|| {
    unit_def(
        952,
        json!({ "id": "fx-dc-guard", "name": "Fixture Hero Guard", "cost": 2 }),
    )
});
pub const GUARD_CAP: i32 = 1;

/// Every fixture above on top of `base`.
pub fn datacenter_catalog(base: CardDefs) -> CardDefs {
    let mut defs = base;
    for entry in [
        &*runner,
        &*field,
        &*animated_field,
        &*hard_field,
        &*dying_field,
        &*trap,
        &*field_trap,
        &*free,
        &*one,
        &*two,
        &*x_cost,
        &*cast_on_draw,
        &*guard,
    ] {
        defs.insert(entry.id.clone(), entry.clone());
    }
    defs
}

fn dying_field_script() -> Script {
    Script {
        death: Some(hook(|_ctx| {
            vec![damage(json_as(
                json!({ "to": { "of": "enemyHero" }, "amount": DYING_FIELD_DAMAGE }),
            ))]
        })),
        ..Script::default()
    }
}

fn cast_on_draw_script() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

fn guard_script() -> Script {
    Script {
        hero_guard: Some(read_hook(|_a| {
            vec![HeroGuard {
                cap: Some(GUARD_CAP),
                divisor: None,
            }]
        })),
        ..Script::default()
    }
}

pub static DATACENTER_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            dying_field.id.clone(),
            CardScripts {
                base: dying_field_script(),
                radiant: dying_field_script(),
            },
        ),
        (
            cast_on_draw.id.clone(),
            CardScripts {
                base: cast_on_draw_script(),
                radiant: cast_on_draw_script(),
            },
        ),
        (
            guard.id.clone(),
            CardScripts {
                base: guard_script(),
                radiant: guard_script(),
            },
        ),
    ])
});

/// Part 24's brief, step 2: this file's scripts (`DATACENTER_SCRIPTS`).
pub fn scripts() -> IndexMap<String, CardScripts> {
    DATACENTER_SCRIPTS.clone()
}
