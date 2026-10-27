//! S #168 Drone Hanger (SPEC §8.10 row 168): (2) Field Spell, Rare, a card of the held Stinkyscape set
//! (R1420).
//!   Base:    "Start of turn: Summon a Drone Interceptor."
//!   Radiant: "Start of turn: Summon a Radiant Drone Interceptor."
//! Engine: Start of turn (R62), its controller's: a summon of the Drone Interceptor into their leftmost
//!   open unit zone (R64), no Cry (R1); with no open zone nothing. The Interceptor draws its own combat
//!   keyword as it arrives (R781004). Radiant: a Radiant Interceptor.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-168";

/// The Drone Interceptor token (§8.10 T-drone-interceptor).
const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";

fn face(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| {
            vec![summon(json_as(json!({ "defId": INTERCEPTOR, "radiant": radiant })))]
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

// S #168 Drone Hanger — SPEC §8.10 row 168, BUILD M12 row S 168: "at its controller's start of turn
// summons a Drone Interceptor, none at the opponent's; a full unit row summons nothing; radiant a Radiant
// one".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const HANGER: &str = "stinkyscape-168";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

    fn fielded(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [FILLER],
                "backrow": [{ "def": HANGER, "radiant": radiant, "faceUp": true }],
                "field": field,
                "library": filler(6),
            },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }))
    }

    fn units(s: &Scenario, player: PlayerId) -> Vec<(String, bool)> {
        (1..=5)
            .filter_map(|lane| s.unit(player, lane).map(|unit| (unit.def_id, unit.radiant)))
            .collect()
    }

    #[test]
    fn r62_at_its_controllers_start_of_turn_only_it_summons_an_interceptor() {
        let mut s = fielded(false, json!([]));
        s.end_turn();
        assert!(units(&s, P1).is_empty(), "the opponent's start of turn summons none");
        assert!(units(&s, P2).is_empty());
        s.end_turn();
        assert_eq!(units(&s, P1), vec![(INTERCEPTOR.to_string(), false)]);
        assert!(units(&s, P2).is_empty());
        s.end_turn();
        s.end_turn();
        assert_eq!(units(&s, P1).len(), 2, "one each start of its controller's turn");
    }

    #[test]
    fn a_full_unit_row_summons_nothing() {
        let mut s = fielded(false, json!(vec![VANILLA; 5]));
        s.end_turn();
        s.end_turn();
        assert_eq!(units(&s, P1), vec![(VANILLA.to_string(), false); 5]);
    }

    #[test]
    fn radiant_a_radiant_one() {
        let mut s = fielded(true, json!([]));
        s.end_turn();
        s.end_turn();
        assert_eq!(units(&s, P1), vec![(INTERCEPTOR.to_string(), true)]);
    }
}
