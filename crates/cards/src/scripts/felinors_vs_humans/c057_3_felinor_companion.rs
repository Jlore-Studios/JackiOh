//! FvH #57.3 Felinor Companion (SPEC §8.14 row 57.3; §6.3 Summon, R60, R64, R129): (1) Spell,
//! Other-Origins, Token (printed Epic), a token FvH #57 Otherworldly Hunting adds.
//!   Base:    "Summon a random Wide Felinor, Narrow Felinor or Square Felinor."
//!   Radiant: "Summon a random Radiant Wide Felinor, Narrow Felinor or Square Felinor."
//!
//! Engine: one `summonRandom` over the pool of FvH #57.4 Wide Felinor, #57.5 Narrow Felinor and #57.6
//! Square Felinor, named by id (so the tokens and the held set are in it), one uniform pick drawn as it
//! applies (R60), into its caster's leftmost open unit zone (R64); with no zone open nothing is drawn
//! or summoned (R129). Hearthstone's Animal Companion, with no "not already on the board" filter.
//! Radiant: the one summoned is Radiant. Declares no numbers. Built by S05 (#5255).

use jackioh_engine::prelude::*;

use crate::scripts::felinors_vs_humans::{c057_4_wide_felinor, c057_5_narrow_felinor, c057_6_square_felinor};

pub const ID: &str = "felinorsvshumans-057-3";

/// FvH #57.4 Wide Felinor, #57.5 Narrow Felinor, #57.6 Square Felinor: the pool.
const COMPANIONS: [&str; 3] = [c057_4_wide_felinor::ID, c057_5_narrow_felinor::ID, c057_6_square_felinor::ID];

fn companion(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![summon_random(json_as(json!({
                "query": { "defId": COMPANIONS },
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: companion(false),
        radiant: companion(true),
    }
}

// FvH #57.3 Felinor Companion — SPEC §8.14 row 57.3 (R60, R64, R129).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const WIDE: &str = "felinorsvshumans-057-4";
    const NARROW: &str = "felinorsvshumans-057-5";
    const SQUARE: &str = "felinorsvshumans-057-6";
    const BODY: &str = "stand-in (1) 1/1";
    /// A textless (0) Spell: something left to play, so no turn ends by itself.
    const FILLER: &str = "stand-in (0) Spell";

    fn cast(radiant: bool, field: Value) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": field },
            "p2": { "hand": [FILLER] },
        }));
        s.play(ID, json!({}));
        s
    }

    /// The def ids the play summoned, in order.
    fn summoned(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { def_id, .. } => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r60_r64_summons_one_of_the_three_felinors_into_the_leftmost_open_zone() {
            let s = cast(false, json!([{ "def": BODY, "lane": 1 }]));
            let landed = summoned(&s);
            assert_eq!(landed.len(), 1, "one Felinor: {landed:?}");
            assert!([WIDE, NARROW, SQUARE].contains(&landed[0].as_str()), "{} is a companion", landed[0]);
            let felinor = s.unit_at(P1, 2);
            assert_eq!(felinor.def_id, landed[0]);
            assert!(!felinor.radiant);
        }

        #[test]
        fn r129_with_no_zone_open_nothing_is_summoned() {
            let s = cast(false, json!([BODY, BODY, BODY, BODY, BODY]));
            assert!(summoned(&s).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn summons_a_radiant_one_of_the_three() {
            let s = cast(true, json!([]));
            let landed = summoned(&s);
            assert_eq!(landed.len(), 1, "one Felinor: {landed:?}");
            assert!([WIDE, NARROW, SQUARE].contains(&landed[0].as_str()), "{} is a companion", landed[0]);
            let felinor = s.unit_at(P1, 1);
            assert_eq!(felinor.def_id, landed[0]);
            assert!(felinor.radiant, "the companion is Radiant");
        }
    }
}
