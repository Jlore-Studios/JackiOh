//! FvH #58 Call of the Shaped Felinors (SPEC §8.14 row 58; §6.3 Summon, R1, R64): (3) Spell, Epic.
//!   Base:    "Summon a Wide Felinor, a Narrow Felinor and a Square Felinor."
//!   Radiant: "Summon a Radiant Wide Felinor, a Radiant Narrow Felinor and a Radiant Square Felinor."
//!
//! Engine: three `summon`s, FvH #57.4 Wide Felinor, #57.5 Narrow Felinor and #57.6 Square Felinor, in
//! that order (S #18 Imitated Instrumentality's shape), each a fresh token into its caster's leftmost
//! open unit zone (R64); one with no zone left is not summoned. Summoned, so no Cry (R1). Radiant: each
//! `radiant: true`, the designer's "Radiant {all}" reaching the whole list. Declares no numbers. Built by
//! S05 (#5255).

use jackioh_engine::prelude::*;

use crate::scripts::felinors_vs_humans::{c057_4_wide_felinor, c057_5_narrow_felinor, c057_6_square_felinor};

pub const ID: &str = "felinorsvshumans-058";

/// FvH #57.4 Wide Felinor, #57.5 Narrow Felinor, #57.6 Square Felinor — the order they are summoned in.
const SHAPES: [&str; 3] = [c057_4_wide_felinor::ID, c057_5_narrow_felinor::ID, c057_6_square_felinor::ID];

fn call(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            SHAPES
                .iter()
                .map(|def_id| summon(json_as(json!({ "defId": def_id, "radiant": radiant }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: call(false),
        radiant: call(true),
    }
}

// FvH #58 Call of the Shaped Felinors — SPEC §8.14 row 58 (R64).
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

    /// p1's unit zones, lane 1 to 5: each card's def id and whether it is Radiant.
    fn lanes(s: &Scenario) -> Vec<Option<(String, bool)>> {
        (1..=5).map(|lane| s.unit(P1, lane).map(|card| (card.def_id, card.radiant))).collect()
    }

    fn shape(def_id: &str, radiant: bool) -> Option<(String, bool)> {
        Some((def_id.to_string(), radiant))
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
        fn r64_summons_a_wide_a_narrow_and_a_square_felinor_in_that_order_into_the_leftmost_open_zones() {
            let s = cast(false, json!([{ "def": BODY, "lane": 2 }]));
            assert_eq!(
                lanes(&s),
                [
                    shape(WIDE, false),
                    shape(BODY, false),
                    shape(NARROW, false),
                    shape(SQUARE, false),
                    None,
                ]
            );
            assert_eq!(summoned(&s), [WIDE, NARROW, SQUARE]);
        }

        #[test]
        fn r64_one_with_no_zone_left_is_not_summoned() {
            let s = cast(false, json!([BODY, BODY, BODY]));
            assert_eq!(summoned(&s), [WIDE, NARROW]);
        }

        #[test]
        fn the_square_felinor_lends_the_other_two_its_attack() {
            let mut s = cast(false, json!([]));
            let (wide, narrow) = (s.unit_at(P1, 1), s.unit_at(P1, 2));
            s.expect_stats(&wide, json!({ "attack": 4 + 1, "health": 4 }));
            s.expect_stats(&narrow, json!({ "attack": 4 + 1, "health": 2 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r64_summons_a_radiant_wide_narrow_and_square_felinor() {
            let s = cast(true, json!([]));
            assert_eq!(
                lanes(&s),
                [shape(WIDE, true), shape(NARROW, true), shape(SQUARE, true), None, None]
            );
        }
    }
}
