//! M #97.6 University (SPEC §8.8 row 97.6). (4) Unit, Token (printed Epic), 0/12 → 0/24.
//!   Base:    "Can't attack\nActivate: Buff each of your permanents {times|time|times}."
//!   Radiant: "Can't attack\nActivate: Buff each of your permanents {times|time|times}."
//!   Engine:  "Activate (R384): separate Buffs (Upgrade, R386) on each permanent acting on your side (unit
//!            tops, backrow cards, face-down ones, this one included), each application its own draw; an
//!            Immutable card is left alone (R23); a face-down card's applications are reported as R440
//!            reports hidden ones. Tunes: times 2 ↑ (Radiant 5)."
//!
//! One script serves both faces: only the declared `times` (2, Radiant 5) and the health differ.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-097-6";

/// §8.8: "Activate" is once a turn.
const USES: i32 = 1;

fn study() -> ActivationDecl {
    ActivationDecl {
        id: "study".to_string(),
        label: "Buff your permanents".to_string(),
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| {
            vec![upgrade(json_as(json!({
                "scope": { "side": "self", "zones": ["field"] },
                "times": param(&*ctx, "times"),
            })))]
        }),
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        activations: vec![study()],
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}
