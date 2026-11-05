//! S #168 Drone Hanger (SPEC §8.10 row 168): (3) Field Spell, Rare, a card of the Stinkyscape set. It was (2)
//! with a start-of-turn summon until #3041.
//!   Base:    "Activate: Summon a Drone Interceptor."
//!   Radiant: "Activate: Summon a Radiant Drone Interceptor."
//! Engine: Activate (R384), once a turn, its controller's, in their main phase: a summon of the Drone
//!   Interceptor into their leftmost open unit zone (R64), no Cry (R1), with a full unit row their
//!   leftmost open backrow zone (R2248000); with no open zone nothing, and the use is spent all the same.
//!   The Interceptor draws its own combat keyword as it arrives (R781004). Radiant: a Radiant Interceptor.
//!   Through v0.5.11p (`script_v0_5_11o`, kept, R767000) it summoned one at its controller's start of turn
//!   (R62), the opponent's summoning none.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-168";

/// The Drone Interceptor token (§8.10 T-drone-interceptor).
const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
/// §8.10: "Activate" is once a turn (R384).
const USES: i32 = 1;

fn summons(radiant: bool) -> Effect {
    summon(json_as(json!({ "defId": INTERCEPTOR, "radiant": radiant })))
}

fn face(radiant: bool) -> Script {
    let launch = ActivationDecl {
        id: "interceptor".to_string(),
        label: if radiant {
            "Summon a Radiant Drone Interceptor".to_string()
        } else {
            "Summon a Drone Interceptor".to_string()
        },
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(move |_ctx| vec![summons(radiant)]),
    };
    Script {
        activations: vec![launch],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

/// Through v0.5.11p: "Start of turn: Summon a Drone Interceptor.", the same summon with no Activate.
fn start_of_turn_face(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| vec![summons(radiant)])),
        ..Script::default()
    }
}

/// R767000: through v0.5.11p the Hanger summoned its Interceptor at its controller's start of turn;
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        base: start_of_turn_face(false),
        radiant: start_of_turn_face(true),
    }
}

// S #168 Drone Hanger — SPEC §8.10 row 168, BUILD M12 row S 168 (R384, R2248000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::{ID, script_v0_5_11o};
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
    fn r384_its_activate_summons_an_interceptor_once_a_turn() {
        let mut s = fielded(false, json!([]));
        s.activate(HANGER, json!({}));
        assert_eq!(units(&s, P1), vec![(INTERCEPTOR.to_string(), false)]);
        s.expect_refused(|s| s.activate(HANGER, json!({})));
        assert_eq!(units(&s, P1).len(), 1);
        s.end_turn();
        s.end_turn();
        s.activate(HANGER, json!({}));
        assert_eq!(units(&s, P1).len(), 2, "one each turn of its controller's");
    }

    #[test]
    fn r384_only_its_controller_activates_it() {
        let mut s = fielded(false, json!([]));
        s.end_turn();
        s.expect_refused(|s| s.activate(HANGER, json!({})));
        assert!(units(&s, P1).is_empty() && units(&s, P2).is_empty());
    }

    #[test]
    fn r62_since_issue_3041_neither_start_of_turn_summons_anything() {
        let mut s = fielded(false, json!([]));
        s.end_turn();
        s.end_turn();
        assert!(units(&s, P1).is_empty() && units(&s, P2).is_empty());
    }

    #[test]
    fn r2248000_a_full_unit_row_puts_the_interceptor_in_the_backrow() {
        let mut s = fielded(false, json!(vec![VANILLA; 5]));
        s.activate(HANGER, json!({}));
        assert_eq!(units(&s, P1), vec![(VANILLA.to_string(), false); 5]);
        let drone = s.backrow(P1, 2).expect("the Interceptor beside the Hanger");
        assert_eq!(drone.def_id, INTERCEPTOR);
    }

    #[test]
    fn radiant_a_radiant_one() {
        let mut s = fielded(true, json!([]));
        s.activate(HANGER, json!({}));
        assert_eq!(units(&s, P1), vec![(INTERCEPTOR.to_string(), true)]);
    }

    /// R767000: through v0.5.11p its controller's start of turn summoned the Interceptor, and it had no
    /// Activate.
    #[test]
    fn r62_r767000_the_kept_script_summons_at_its_controllers_start_of_turn_only() {
        for radiant in [false, true] {
            crate::under_kept(ID, script_v0_5_11o(), || {
                let mut s = fielded(radiant, json!([]));
                s.expect_refused(|s| s.activate(HANGER, json!({})));
                s.end_turn();
                assert!(units(&s, P1).is_empty(), "the opponent's start of turn summons none");
                assert!(units(&s, P2).is_empty());
                s.end_turn();
                assert_eq!(units(&s, P1), vec![(INTERCEPTOR.to_string(), radiant)]);
                s.end_turn();
                s.end_turn();
                assert_eq!(units(&s, P1).len(), 2, "one each start of its controller's turn");
            });
        }
    }
}
