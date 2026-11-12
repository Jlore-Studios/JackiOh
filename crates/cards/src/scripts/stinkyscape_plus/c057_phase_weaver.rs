//! SPEC §8.12 S+ #57 Phase Weaver (R62, R1420):
//! (1) Field Spell, Mindustrial, Epic, a card of the Stinkyscape+ set.
//!   Base:    "End of your turn: add a Phase Fabric to your hand."
//!   Radiant: "End of your turn: add a Phase Fabric to your hand, it costs (0)."
//!
//! At the end of its controller's turn (R62) it adds S+ #46 Phase Fabric to its controller's hand
//! (`add_to_hand`); on the Radiant face at (0) (`costOverride`).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-057";

/// S+ #46 Phase Fabric, the card it adds.
const PHASE_FABRIC: &str = "stinkyscapeplus-046";
/// "it costs (0)".
const FREE: i32 = 0;

fn phase_weaver(radiant: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |_ctx| {
            let mut args = json!({ "defId": PHASE_FABRIC });
            if radiant {
                args["costOverride"] = json!(FREE);
            }
            vec![add_to_hand(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: phase_weaver(false),
        radiant: phase_weaver(true),
    }
}

// S+ #57 Phase Weaver — SPEC §8.12 row S+ 57 (R62).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, next_turn_of, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// p1 holds the Weaver; `"mana": 10` plays it from hand.
    fn weaver(radiant: bool, seed: &str) -> Scenario {
        let mut s = weapon_board(
            seed,
            json!({
                "mana": 10,
                "hand": [{ "def": ID, "radiant": radiant }, "core-005"],
                "library": filler(6),
            }),
            json!({
                "hand": ["core-005"],
                "library": filler(6),
            }),
        );
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn fabrics(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id == PHASE_FABRIC).collect()
    }

    #[test]
    fn phase_weaver_adds_a_phase_fabric_at_its_controllers_end_of_turn() {
        let mut s = weaver(false, "s57-base");
        assert!(fabrics(&s).is_empty());
        s.end_turn();
        let added = fabrics(&s);
        assert_eq!(added.len(), 1, "one Phase Fabric");
        assert!(!added[0].radiant, "no Radiant face's Radiant card");
        assert_eq!(added[0].cost_override, None, "at its printed price");
    }

    #[test]
    fn phase_weaver_adds_none_on_the_opponents_end_of_turn() {
        let mut s = weaver(false, "s57-turns");
        // Past its controller's end of turn, which adds one; the opponent's end adds none.
        next_turn_of(&mut s, P2);
        assert_eq!(fabrics(&s).len(), 1);
        s.end_turn();
        assert_eq!(fabrics(&s).len(), 1, "the opponent's end of turn adds nothing");
    }

    #[test]
    fn phase_weaver_radiant_one_costs_0() {
        let mut s = weaver(true, "s57-radiant");
        s.end_turn();
        let added = fabrics(&s);
        assert_eq!(added.len(), 1, "one Phase Fabric");
        assert_eq!(added[0].cost_override, Some(0), "at (0)");
        assert_eq!(effective_cost(s.state(), &added[0], Default::default()), 0);
    }
}
