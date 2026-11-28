//! FvH #20 Bomb (SPEC §8.14 row 20): (2) Field Spell, Roblox-Tool, Common.
//!   Base:    "Activate: Summon a Ticking Time Bomb."
//!   Radiant: "Activate: Summon a Radiant Ticking Time Bomb."
//!
//! Engine: Activate, once a turn (R384), its controller's, in their main phase. It prints no Durability
//! (FV2 #20), so its uses have no lifetime cap: it serves every turn it stays. The use is a `summon` of
//! FvH #20.1 Ticking Time Bomb into its controller's leftmost open unit zone (R64), summoned so no Cry;
//! with no open unit zone nothing is summoned (R129) and the use is spent all the same. Radiant: a
//! Radiant Ticking Time Bomb. Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-020";

/// FvH #20.1 Ticking Time Bomb, the token it summons.
const TICKING_TIME_BOMB: &str = "felinorsvshumans-020-1";
/// "Activate" is once a turn (R384).
const USES: i32 = 1;

fn face(radiant: bool) -> Script {
    let arm = ActivationDecl {
        id: "bomb".to_string(),
        label: if radiant {
            "Summon a Radiant Ticking Time Bomb".to_string()
        } else {
            "Summon a Ticking Time Bomb".to_string()
        },
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(move |_ctx| {
            vec![summon(json_as(json!({ "defId": TICKING_TIME_BOMB, "radiant": radiant })))]
        }),
    };
    Script {
        activations: vec![arm],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// FvH #20 Bomb — SPEC §8.14 row 20 (R384, R64, R129; FV2 #20).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    /// A textless Spell each seat can always play, so no turn ends by itself (§2.5).
    const SPELL: &str = "stand-in (1) Spell";
    const BODY: &str = "stand-in (1) 1/1";

    fn bomb(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [SPELL],
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": true }],
                "field": field,
                "library": filler(6),
            },
            "p2": { "hand": [SPELL], "library": filler(6) },
        }))
    }

    /// How many Ticking Time Bombs have been summoned so far.
    fn bombs_summoned(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == TICKING_TIME_BOMB))
            .count()
    }

    #[test]
    fn declares_no_tuned_numbers() {
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r384_r64_its_activate_summons_a_ticking_time_bomb_into_the_leftmost_open_unit_zone_once_a_turn() {
            let mut s = bomb(false, json!([]));
            s.activate(ID, json!({}));
            let token = s.unit(P1, 1).expect("the Ticking Time Bomb");
            assert_eq!(token.def_id, TICKING_TIME_BOMB);
            assert!(!token.radiant);
            s.expect_refused(|s| s.activate(ID, json!({})));
            assert_eq!(bombs_summoned(&s), 1);
        }

        #[test]
        fn r384_it_prints_no_durability_so_it_serves_again_on_its_controllers_next_turn() {
            let mut s = bomb(false, json!([]));
            assert_eq!(durability_left(s.state(), s.card(ID)), None);
            s.activate(ID, json!({}));
            s.end_turn();
            s.end_turn();
            s.activate(ID, json!({}));
            assert_eq!(bombs_summoned(&s), 2);
            s.expect_in_zone(ID, "field");
        }

        #[test]
        fn r129_with_no_open_unit_zone_it_summons_nothing_and_the_use_is_spent() {
            let mut s = bomb(false, json!([BODY, BODY, BODY, BODY, BODY]));
            s.activate(ID, json!({}));
            assert_eq!(bombs_summoned(&s), 0);
            s.expect_refused(|s| s.activate(ID, json!({})));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r384_its_activate_summons_a_radiant_ticking_time_bomb() {
            let mut s = bomb(true, json!([]));
            s.activate(ID, json!({}));
            let token = s.unit(P1, 1).expect("the Ticking Time Bomb");
            assert_eq!(token.def_id, TICKING_TIME_BOMB);
            assert!(token.radiant);
            s.expect_stats(&token.id, json!({ "attack": 0, "health": 4 }));
            s.expect_refused(|s| s.activate(ID, json!({})));
        }
    }
}
