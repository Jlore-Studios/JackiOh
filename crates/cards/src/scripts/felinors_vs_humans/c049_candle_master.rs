//! FvH #49 Candle Master (SPEC §8.14 row 49): (1) Unit, Human, Common, 2/1 → 4/2.
//!   Base:    "Cry: Summon a Candle Shot."
//!   Radiant: "Cry: Summon a Radiant Candle Shot."
//!
//! Engine: Cry (R1): `summon` of FvH #49.1 Candle Shot, a Field Spell token, into its controller's leftmost
//! open backrow zone (R64), summoned so it runs no Cry; a full backrow, nothing (R129). Radiant: a Radiant
//! one, whose face deals 2 (FV2 #49.1). Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-049";

/// FvH #49.1 Candle Shot, the Field Spell token it summons.
const CANDLE_SHOT: &str = "felinorsvshumans-049-1";

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![summon(json_as(json!({ "defId": CANDLE_SHOT, "radiant": radiant })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// FvH #49 Candle Master — SPEC §8.14 row 49 (R1, R64, R129).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    /// A textless Spell each seat can always play, so no turn ends by itself (§2.5).
    const SPELL: &str = "stand-in (1) Spell";
    const FIELD_SPELL: &str = "stand-in (1) Field Spell";

    fn master(radiant: bool, backrow: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, SPELL],
                "backrow": backrow,
                "library": filler(6),
            },
            "p2": { "hand": [SPELL], "library": filler(6) },
        }))
    }

    /// The Candle Shots summoned so far: their backrow lane and face.
    fn shots(s: &Scenario) -> Vec<(i32, bool)> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { instance_id, def_id, row, lane, .. }
                    if def_id == CANDLE_SHOT && *row == Row::Backrow =>
                {
                    Some((*lane, s.card(instance_id.as_str()).radiant))
                }
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1_r64_its_cry_summons_a_candle_shot_into_the_leftmost_open_backrow_zone() {
            let mut s = master(false, json!([{ "def": FIELD_SPELL, "lane": 1 }]));
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(shots(&s), vec![(2, false)]);
            let shot = s.backrow(P1, 2).expect("the Candle Shot");
            assert_eq!(shot.def_id, CANDLE_SHOT);
            // Summoned, not played: the Shot runs no play of its own.
            assert!(!s.events().iter().any(|event| matches!(
                event,
                GameEvent::CardPlayed { def_id, .. } if def_id == CANDLE_SHOT
            )));
        }

        #[test]
        fn r129_with_a_full_backrow_nothing_is_summoned() {
            let full = vec![FIELD_SPELL; 5];
            let mut s = master(false, json!(full));
            s.play(ID, json!({ "zone": 1 }));
            assert!(shots(&s).is_empty());
            s.expect_in_zone(ID, "field");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1_its_cry_summons_a_radiant_candle_shot() {
            let mut s = master(true, json!([]));
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(shots(&s), vec![(1, true)]);
        }
    }
}
