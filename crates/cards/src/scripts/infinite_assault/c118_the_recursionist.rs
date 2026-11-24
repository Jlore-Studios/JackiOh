//! SPEC §8.13 IA #118 The Recursionist ((3) Unit, Legendary, 4/7 → 8/14), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Your Death triggers and Reborns happen twice."
//!   Radiant: "Your Death triggers and Reborns happen twice, and your Units have Reborn."
//!
//! `death_extra` and `reborn_extra` both read 1 (`read_hook`, R4589003): its controller's Deaths run
//! once more (the highest multiplier holds with Double Counting's, R820) and its Reborns come back
//! twice. A kept Reborn returns with Reborn again instead of spending it, so it comes back twice and
//! stays dead; a Recursionist dying in the same pass doubles nothing (R823), and two bodies never
//! share the one reserved zone (R64). The Radiant face's aura gives its controller's Units Reborn,
//! C #14 Jlockeed's Weapons' shape (`aura_hook`).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-118";

/// "happen twice": one extra run.
const EXTRA: i32 = 1;

fn recursionist(aura: bool) -> Script {
    let mut script = Script {
        death_extra: Some(read_hook(|_| EXTRA)),
        reborn_extra: Some(read_hook(|_| EXTRA)),
        ..Script::default()
    };
    if aura {
        script.aura = Some(aura_hook(|args: HookArgs<'_>| {
            let controller = args.self_.controller;
            let state = args.state;
            vec![AuraEntry {
                // C #14 Jlockeed's Weapons' shape: the controller's Units, itself among them.
                applies: Box::new(move |unit: &CardInstance| {
                    unit.controller == controller && in_units_row_or_standing(state, unit)
                }),
                mod_: StatMod {
                    keywords: Some(vec![Keyword::Reborn]),
                    ..StatMod::default()
                },
            }]
        }));
    }
    script
}

pub fn script() -> CardScripts {
    CardScripts {
        base: recursionist(false),
        radiant: recursionist(true),
    }
}

// IA #118 The Recursionist — SPEC §8.13 row 118 (R4589003).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const PAWN: &str = "stand-in (1) 1/1, Reborn";
    const FILLER: &str = "stand-in (0) Spell";

    /// The Recursionist watches p1's Reborn Pawn die three times.
    fn watched(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [HIT_JOB, HIT_JOB, HIT_JOB],
                "field": [{ "def": ID, "radiant": radiant }, PAWN],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        for _ in 0..3 {
            let pawn = s.unit_at(P1, 2);
            s.play(HIT_JOB, pick::targets(pick::instance(&pawn)));
        }
        s
    }

    fn pawns(s: &Scenario) -> Vec<CardInstance> {
        active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == PAWN)
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4589003_deaths_and_reborns_happen_twice() {
            let s = watched(false);
            // Three kills: the first two each return the Pawn, the third leaves it dead.
            assert!(pawns(&s).is_empty(), "two returns, then dead");
        }
    }

    const PLAIN: &str = "stand-in (1) 2/2";

    mod radiant {
        use super::*;

        #[test]
        fn r4589003_the_aura_gives_reborn_and_the_returns_happen_twice() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [HIT_JOB, HIT_JOB, HIT_JOB],
                    "field": [{ "def": ID, "radiant": true }, PLAIN],
                    "library": crate::filler(4),
                    "mana": 10,
                },
                "p2": { "hand": [FILLER], "library": crate::filler(4) },
            }));
            // The aura gives the plain Unit Reborn.
            let plain = s.unit_at(P1, 2);
            assert!(has_keyword(&s.stats(&plain).keywords, KeywordKind::Reborn));
            for _ in 0..3 {
                let body = s.unit_at(P1, 2);
                s.play(HIT_JOB, pick::targets(pick::instance(&body)));
            }
            assert!(
                active_units_of(s.state(), P1)
                    .iter()
                    .all(|unit| unit.def_id != PLAIN),
                "two returns, then dead"
            );
        }
    }
}
