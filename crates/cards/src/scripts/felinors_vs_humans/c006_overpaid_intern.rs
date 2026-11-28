//! SPEC §8.14 FvH #6 Overpaid Intern ((2) Unit, Human, Common, 2/2 → 4/4), a card of the held Felinors vs
//! Humans set (R5235000). Built by S09 (#5259).
//!   Base:    "Tradable. Cry: Destroy an enemy backrow card."
//!   Radiant: "Tradable. Cry: Steal an enemy backrow card."
//!
//! Engine: Tradable is the catalog's keyword (R1838000). The Cry (R1) takes one enemy backrow card picked
//! with the play (R81, aimed to harm), a face-down Trap among them (IA #159 Antivirus's declaration); the base
//! face destroys it (§6.3), the Radiant face steals it (§6.3 Steal, R15: its own lane on the thief's side if
//! free, else the first free backrow zone; none, and it is destroyed instead, R4205000). With no enemy
//! backrow card the Unit is still played and the Cry does nothing (R90). Declares no numbers.

use jackioh_engine::effects::{destroy, steal};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-006";

/// R81: one enemy backrow card, picked with the play and aimed to harm.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Harm),
        ..TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["backrow"] }))
    }]
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            targets: targets(),
            cry: Some(hook(|_ctx| vec![destroy(json_as(json!({ "target": { "of": "chosen" } })))])),
            ..Script::default()
        },
        radiant: Script {
            targets: targets(),
            cry: Some(hook(|_ctx| vec![steal(json_as(json!({ "target": { "of": "chosen" } })))])),
            ..Script::default()
        },
    }
}

// FvH #6 Overpaid Intern — SPEC §8.14 row 6 (R1838000, R81, R15, R4205000, R90).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";
    const FOE_TRAP: &str = "stand-in (1) Trap";
    const FILLER: &str = "core-005"; // Stockpile: a card kept in hand, so no turn ends by itself

    fn board(radiant: bool, foe_backrow: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": [UNIT],
                "library": crate::filler(3),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "backrow": foe_backrow, "field": [UNIT], "library": crate::filler(3) },
        }))
    }

    fn play_on_the_trap(s: &mut Scenario) -> CardInstance {
        let trap = s.backrow(P2, 1).expect("the enemy backrow card");
        s.play(ID, pick::targets(pick::instance(&trap)));
        trap
    }

    #[test]
    fn declares_no_numbers() {
        assert!(crate::card_def(ID).params.is_none());
    }

    #[test]
    fn r81_only_enemy_backrow_cards_are_legal_picks_on_both_faces() {
        for radiant in [false, true] {
            let s = board(radiant, json!([FOE_TRAP]));
            let picks = pick::instance_ids(&s.play_targets(ID));
            let foe_backrow = s.backrow(P2, 1).expect("the enemy backrow card");
            assert!(picks.contains(&foe_backrow.id), "radiant {radiant}");
            assert!(!picks.contains(&s.unit_at(P2, 1).id), "radiant {radiant}: an enemy Unit is none");
            assert!(!picks.contains(&s.unit_at(P1, 1).id), "radiant {radiant}: a friendly Unit is none");
        }
    }

    #[test]
    fn r90_with_no_enemy_backrow_card_it_is_still_played_and_the_cry_does_nothing() {
        for radiant in [false, true] {
            let mut s = board(radiant, json!([]));
            s.play(ID, json!({}));
            s.expect_in_zone(ID, "field");
            assert!(s.state().pending.is_none(), "radiant {radiant}");
        }
    }

    mod base {
        use super::*;

        #[test]
        fn cry_destroys_an_enemy_backrow_card() {
            let mut s = board(false, json!([FOE_TRAP]));
            let trap = play_on_the_trap(&mut s);
            s.expect_in_zone(&trap, "graveyard");
            assert!(s.backrow(P1, 1).is_none(), "nothing is stolen");
            s.expect_stats(ID, json!({ "attack": 2, "health": 2 }));
        }

        #[test]
        fn r1838000_it_is_tradable_from_the_hand() {
            let mut s = board(false, json!([]));
            let intern = s.card(ID).clone();
            s.trade(&intern);
            s.expect_in_zone(&intern, "library");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r15_cry_steals_an_enemy_backrow_card_into_its_lane_on_your_side() {
            let mut s = board(true, json!([FOE_TRAP]));
            let trap = play_on_the_trap(&mut s);
            assert_eq!(s.backrow(P1, 1).map(|card| card.id), Some(trap.id.clone()));
            assert!(s.backrow(P2, 1).is_none());
            assert_eq!(s.card(trap.id.as_str()).controller, P1);
            assert_eq!(s.card(trap.id.as_str()).owner, P2, "R12: control moved, ownership did not");
            s.expect_stats(ID, json!({ "attack": 4, "health": 4 }));
        }

        #[test]
        fn r1838000_it_is_tradable_from_the_hand() {
            let mut s = board(true, json!([]));
            let intern = s.card(ID).clone();
            s.trade(&intern);
            s.expect_in_zone(&intern, "library");
        }
    }
}
