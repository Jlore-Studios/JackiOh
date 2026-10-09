//! M #97.7 The Great Wall (SPEC §8.8 row 97.7, R1260). (2) Unit, Token (printed Epic), 0/50 → 0/100.
//!   Base:    "Tribute 3, Immutable, Can't attack\nCry: Lock every unit zone on your side."
//!   Radiant: "Tribute 3, Immutable, Armor 2, Can't attack\nCry: Lock every unit zone on your side."
//!   Engine:  "The Cry, when it is played (R1), Locks each of your five unit zones (§6.3 Lock, a `locked`
//!            each, one Locked already skipped), occupied ones included, since a Lock evicts nothing
//!            (§3.2); until one is Unlocked no Unit, a Stack play included, may be played there (R688), and
//!            'fill your board' effects (R64) fill nothing on that side, whoever's effect it is, while
//!            ordinary summons and moves still enter (R688). The Locks outlast the Wall (R1260)."
//!
//! One script serves both faces: Tribute 3 is its static flag, and Immutable (R23) and the Radiant face's
//! Armor 2 and 0/100 are catalog data.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-097-7";

/// §6.3 "Tribute 3": three of your Units, or one Sheep Token plus one more, worth 3 (§3.2).
const TRIBUTE_COST: i32 = 3;

fn the_great_wall() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        // R1260: all five of the Wall's side, its own zone and the occupied ones included; `lock` skips
        // a zone Locked already and evicts no one.
        cry: Some(hook(|_ctx| {
            (1..=UNIT_ZONES)
                .map(|lane| lock(json_as(json!({ "zone": { "of": "lane", "row": "units", "lane": lane } }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = the_great_wall();
    let radiant = the_great_wall();
    CardScripts { base, radiant }
}
