//! IA #35 Fata Morgana (SPEC §8.13 row 35), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Whenever a Figment of yours reveals, summon a Mirage."
//!   Radiant: "Whenever a Figment of yours reveals, summon a Radiant Mirage."
//!
//! A Figment's own firing is an ordinary trigger that runs after the combat (§10.3): this answers
//! its `trapFired` — one of mine (`controller`), the Figment's (`def_id`), and not a later firing
//! (`again`, R2657001) — with a Mirage, Radiant on the Radiant face.
//!
//! Engine: `summon` (R4586007). Built by C2 (#4586).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-035";

/// What each Figment's reveal summons (§7).
const MIRAGE: &str = "infiniteassault-035-1";
/// The Figment whose reveal this answers.
const FIGMENT: &str = "infiniteassault-025-1";

/// A reveal of one of mine Figments: its `trapFired` names no refire.
fn figment_reveal(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::TrapFired {
        controller,
        def_id,
        again,
        ..
    } = event
    else {
        return false;
    };
    *controller == ctx.controller && *def_id == FIGMENT && *again != Some(true)
}

fn morgana(radiant: bool) -> Script {
    Script {
        triggers: vec![TriggerDef::new("fata-morgana", &[GameEventType::TrapFired], move |ctx, event| {
            if !figment_reveal(ctx, event) {
                return vec![];
            }
            vec![summon(json_as(json!({ "defId": MIRAGE, "radiant": radiant })))]
        })
        .with_when(figment_reveal)],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: morgana(false),
        radiant: morgana(true),
    }
}

// IA #35 Fata Morgana — SPEC §8.13 row 35 (R4586007).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::{FIGMENT, ID, MIRAGE};
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const ATTACKER: &str = "core-008"; // Mr. Vanilla: a 4/4.

    fn standing(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "active": "p2",
            "p1": {
                "field": [{ "def": ID, "radiant": radiant }],
                "backrow": [FIGMENT],
                "hand": [STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": { "field": [ATTACKER], "hand": [STOCKPILE], "library": crate::filler(4) },
        }))
    }

    fn mirages(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .filter(|card| card.def_id == MIRAGE)
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4586007_a_figments_reveal_summons_a_mirage() {
            let mut s = standing(false, "morgana-base");
            s.attack(ATTACKER, "hero");
            let summoned = mirages(&s);
            assert_eq!(summoned.len(), 1);
            assert!(!summoned[0].radiant, "a plain Mirage");
            s.expect_in_zone(FIGMENT, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586007_a_figments_reveal_summons_a_radiant_mirage() {
            let mut s = standing(true, "morgana-radiant");
            s.attack(ATTACKER, "hero");
            let summoned = mirages(&s);
            assert_eq!(summoned.len(), 1);
            assert!(summoned[0].radiant, "a Radiant Mirage");
        }
    }
}
