//! S #35 Drone Core IX (SPEC §8.10 row 35): (6) Spell, Drone Core, Legendary, a card of the
//! Stinkyscape set.
//!   Base:    "Float reduces cost.\nSummon {corvettes|Drone Corvette|Drone Corvettes}." — corvettes 3
//!   Radiant: "Float reduces cost.\nSummon {corvettes|Radiant Drone Corvette|Radiant Drone Corvettes}."
//!            — corvettes 3
//! Engine: "Float reduces cost" as S #34 (R1563000): a hand trigger on its owner's Float (a deck trigger too through v0.5.9f, R2657002), a
//!   `costMod` kept until a graveyard or exile (R766). Then one summon per Corvette, each into the caster's
//!   leftmost open unit zone in turn (R64), then the backrow's once the unit row is full, where each stands
//!   (R2248000), no Cry (R1), as many as fit; the count is the declared
//!   `corvettes` (R386). Each Corvette draws its own combat keywords as it arrives (R781004). Radiant:
//!   summoned Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-035";

/// The Drone Corvette token (§8.10 T-drone-corvette).
const CORVETTE: &str = "stinkyscape-t-drone-corvette";

/// One face. `deck` adds the Float deck trigger it had through v0.5.9f (R2657002).
fn face(radiant: bool, deck: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            (0..param(&*ctx, "corvettes"))
                .map(|_| summon(json_as(json!({ "defId": CORVETTE, "radiant": radiant }))))
                .collect()
        })),
        hand_triggers: vec![float_reduces_cost("drone-core-ix-float-hand")],
        deck_triggers: if deck {
            vec![float_reduces_cost("drone-core-ix-float-deck")]
        } else {
            vec![]
        },
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false, false),
        radiant: face(true, false),
    }
}

/// R767000: through v0.5.9f a Float lowered its cost in the deck too, before R2657002; `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_5_9f() -> CardScripts {
    CardScripts {
        base: face(false, true),
        radiant: face(true, true),
    }
}

// S #35 Drone Core IX — SPEC §8.10 row 35, BUILD M12 row S 35 (R2248000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::stinkyscape_family as fam;

    const CORE: &str = "stinkyscape-035";
    const CORVETTE: &str = "stinkyscape-t-drone-corvette";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005";

    use crate::{filler, scenario};

    fn setup(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": CORE, "radiant": radiant }, FILLER], "field": field, "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn corvette(radiant: bool) -> Option<(String, bool)> {
        fam::drone_cell(CORVETTE, radiant)
    }

    #[test]
    fn r1563000_r2657002_a_float_of_2_makes_it_cost_two_less_in_hand_never_in_the_deck() {
        fam::drone_float_two_less(CORE, 6);
    }

    #[test]
    fn r1563000_the_opponents_float_lowers_nothing() {
        fam::drone_float_opponent(CORE, 6);
    }

    #[test]
    fn r64_r2248000_summons_three_corvettes_the_unit_row_first_then_the_backrow() {
        let mut s = setup(false, json!([]));
        s.play(CORE, json!({}));
        assert_eq!(
            fam::drone_unit_row(&s),
            vec![corvette(false), corvette(false), corvette(false), None, None]
        );
        assert_eq!(fam::drone_backrow_row(&s), vec![None; 5]);

        let mut s = setup(false, json!(vec![VANILLA; 4]));
        s.play(CORE, json!({}));
        assert_eq!(
            fam::drone_unit_row(&s)[4],
            corvette(false),
            "one zone, one Corvette"
        );
        assert_eq!(
            fam::drone_backrow_row(&s),
            vec![corvette(false), corvette(false), None, None, None]
        );
    }

    #[test]
    fn corvettes_reads_through_param() {
        let mut s = setup(false, json!([]));
        set_param(s.card_mut(CORE), "corvettes", 4);
        s.play(CORE, json!({}));
        assert_eq!(
            fam::drone_unit_row(&s),
            vec![
                corvette(false),
                corvette(false),
                corvette(false),
                corvette(false),
                None
            ]
        );
    }

    #[test]
    fn radiant_summons_three_radiant_ones() {
        let mut s = setup(true, json!([]));
        s.play(CORE, json!({}));
        assert_eq!(
            fam::drone_unit_row(&s),
            vec![corvette(true), corvette(true), corvette(true), None, None]
        );
    }
}
