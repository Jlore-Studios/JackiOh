//! SPEC §8.14 FvH #33 Unstoppable Force ((10) Unit, Epic, 50/50 → 100/100), a card of the held Felinors vs
//! Humans set (R5235000). A Unit (FV0's call; the designer's list says Field Spell).
//!   Base:    "Indestructible, Immune to Spells\nFloat reduces cost."
//!   Radiant: "Indestructible, Immune to Spells, Windfury\nFloat reduces cost."
//!
//! Engine: the keywords are catalog data: Indestructible, so Immutable innately and never in Defense
//! Position nor Taunt (R4353000, R2179000); Immune to Spells; the Radiant face's Windfury attacks twice a
//! turn (R636). "Float reduces cost" (§6.2, R1563000) is the engine's `float_reduces_cost` hand trigger, S
//! #34 Drone Core VIII's: each Float of its owner's lowers its cost by that much, so at (10), past the mana
//! cap, it is played only so. Declares no numbers. Built by S01 (#5251).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-033";

fn unstoppable_force() -> Script {
    Script {
        hand_triggers: vec![float_reduces_cost("unstoppable-force-float")],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script on both faces: Windfury and the doubled line are the Radiant face's catalog data.
    CardScripts {
        base: unstoppable_force(),
        radiant: unstoppable_force(),
    }
}

// FvH #33 Unstoppable Force — SPEC §8.14 row 33 (R1563000, R4353000, R636).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::stinkyscape_family as fam;
    use crate::{filler, scenario};

    const HIT_JOB: &str = "core-016"; // Hit Job: Destroy a Unit, the Spell it is immune to
    const VANILLA: &str = "stand-in (1) 4/4";
    const SMALL: &str = "stand-in (1) 1/1";
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    /// The printed price the Float lowers.
    const PRINTED: i32 = 10;

    /// The Force (`radiant` its face) and a Vanilla stand on p1's side; p2 holds a Hit Job on its turn.
    fn facing_a_hit_job(radiant: bool) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": {
                "hand": [FILLER],
                "field": [{ "def": ID, "radiant": radiant }, VANILLA],
                "library": filler(4),
            },
            "p2": { "hand": [HIT_JOB, FILLER], "library": filler(4), "mana": 10 },
        }))
    }

    /// Immune to Spells: the Hit Job may aim at the Vanilla and never at the Force; Indestructible is
    /// on it.
    fn spells_pass_it_by(radiant: bool) {
        let s = facing_a_hit_job(radiant);
        let force = s.unit_at(P1, 1);
        let vanilla = s.unit_at(P1, 2);
        let aimed = pick::instance_ids(&s.play_targets(HIT_JOB));
        assert!(aimed.contains(&vanilla.id), "the Hit Job is playable");
        assert!(!aimed.contains(&force.id), "it is Immune to Spells");
        let keywords = s.stats(&force).keywords;
        assert!(has_keyword(&keywords, KeywordKind::Indestructible));
        assert!(has_keyword(&keywords, KeywordKind::ImmuneToSpells));
    }

    #[test]
    fn declares_no_tuned_numbers() {
        crate::register_all();
        assert!(crate::card_def(ID).params.is_none());
    }

    #[test]
    fn r1563000_a_float_of_2_makes_it_cost_two_less_in_hand_never_in_the_deck() {
        fam::drone_float_two_less(ID, PRINTED);
    }

    #[test]
    fn r1563000_the_opponents_float_lowers_nothing() {
        fam::drone_float_opponent(ID, PRINTED);
    }

    mod base {
        use super::*;

        #[test]
        fn a_spell_cannot_aim_at_it() {
            spells_pass_it_by(false);
        }

        #[test]
        fn without_windfury_it_attacks_once_a_turn() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [ID], "library": filler(4) },
                "p2": { "hand": [FILLER], "field": [SMALL, SMALL], "library": filler(4) },
            }));
            let force = s.unit_at(P1, 1);
            s.attack(&force, s.unit_at(P2, 1));
            assert!(s.unit(P2, 1).is_none(), "the first one died");
            s.expect_refused(|s| s.attack(&force, s.unit_at(P2, 2)));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_spell_cannot_aim_at_it() {
            spells_pass_it_by(true);
        }

        #[test]
        fn r636_windfury_lets_it_attack_twice_a_turn() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [{ "def": ID, "radiant": true }], "library": filler(4) },
                "p2": { "hand": [FILLER], "field": [SMALL, SMALL, SMALL], "library": filler(4) },
            }));
            let force = s.unit_at(P1, 1);
            s.attack(&force, s.unit_at(P2, 1));
            s.attack(&force, s.unit_at(P2, 2));
            assert!(s.unit(P2, 1).is_none() && s.unit(P2, 2).is_none(), "both died");
            s.expect_refused(|s| s.attack(&force, s.unit_at(P2, 3)));
        }
    }
}
