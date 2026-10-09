//! M #97.3 School (SPEC §8.8 row 97.3). (2) Unit, Token (printed Rare), 0/8 → 0/16.
//!   Base:    "Can't attack\nActivate: Summon a random ({cost}) Cost Unit."
//!   Radiant: "Can't attack\nActivate: Summon a random Radiant ({cost}) Cost Unit."
//!   Engine:  "Activate (R384): one uniform pick of the non-token Units of every set at that printed cost
//!            (R380, R60), placed per R64 (a Locked zone only when no open one is left, R688), with no Cry
//!            (R1); no zone to land in draws nothing (R129). Tunes: cost 1 ↑."

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-097-3";

/// §8.8: "Activate" is once a turn.
const USES: i32 = 1;

fn class(radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: "class".to_string(),
        label: "Summon a Unit".to_string(),
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |ctx| {
            vec![summon_random(json_as(json!({
                "query": { "type": "Unit", "cost": param(&*ctx, "cost") },
                "radiant": radiant,
            })))]
        }),
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            activations: vec![class(false)],
            ..Script::default()
        },
        radiant: Script {
            activations: vec![class(true)],
            ..Script::default()
        },
    }
}
