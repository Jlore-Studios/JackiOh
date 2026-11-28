//! FvH #19 Trowel (SPEC §8.14 row 19): (2) Field Spell, Roblox-Tool, Common.
//!   Base:    "Activate: Summon a Wall of Bricks."
//!   Radiant: "Activate: Summon a Radiant Wall of Bricks."
//!
//! Engine: Activate, once a turn (R384), its controller's, in their main phase. It prints no Durability
//! (FV2 #19), so its uses have no lifetime cap: it serves every turn it stays. The use is a `summon` of
//! FvH #19.1 Wall of Bricks into its controller's leftmost open unit zone (R64), summoned so no Cry; with
//! no open unit zone nothing is summoned (R129) and the use is spent all the same. Radiant: a Radiant
//! Wall. Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-019";

/// FvH #19.1 Wall of Bricks, the token it summons.
const WALL: &str = "felinorsvshumans-019-1";
/// "Activate" is once a turn (R384).
const USES: i32 = 1;

fn face(radiant: bool) -> Script {
    let build = ActivationDecl {
        id: "wall".to_string(),
        label: if radiant {
            "Summon a Radiant Wall of Bricks".to_string()
        } else {
            "Summon a Wall of Bricks".to_string()
        },
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(move |_ctx| vec![summon(json_as(json!({ "defId": WALL, "radiant": radiant })))]),
    };
    Script {
        activations: vec![build],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// FvH #19 Trowel — SPEC §8.14 row 19 (R384, R64, R129; FV2 #19).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    /// A textless Spell each seat can always play, so no turn ends by itself (§2.5).
    const SPELL: &str = "stand-in (1) Spell";
    const BODY: &str = "stand-in (1) 1/1";

    fn trowel(radiant: bool, field: Value) -> Scenario {
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

    /// How many Walls of Bricks have been summoned so far.
    fn walls_summoned(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == WALL))
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn r384_r64_its_activate_summons_a_wall_of_bricks_into_the_leftmost_open_unit_zone_once_a_turn() {
            let mut s = trowel(false, json!([]));
            s.activate(ID, json!({}));
            let wall = s.unit(P1, 1).expect("the Wall of Bricks");
            assert_eq!(wall.def_id, WALL);
            assert!(!wall.radiant);
            s.expect_refused(|s| s.activate(ID, json!({})));
            assert_eq!(walls_summoned(&s), 1);
        }

        #[test]
        fn r384_it_prints_no_durability_so_it_serves_on_every_turn_of_its_controllers() {
            let mut s = trowel(false, json!([]));
            assert_eq!(durability_left(s.state(), s.card(ID)), None);
            for turn in 1..=3 {
                s.activate(ID, json!({}));
                assert_eq!(walls_summoned(&s), turn);
                s.end_turn();
                s.end_turn();
            }
            s.expect_in_zone(ID, "field");
        }

        #[test]
        fn r129_with_no_open_unit_zone_it_summons_nothing_and_the_use_is_spent() {
            let mut s = trowel(false, json!([BODY, BODY, BODY, BODY, BODY]));
            s.activate(ID, json!({}));
            assert_eq!(walls_summoned(&s), 0);
            s.expect_refused(|s| s.activate(ID, json!({})));
        }

        #[test]
        fn the_wall_it_summons_wears_its_printed_taunt_cant_attack_and_brittle() {
            let mut s = trowel(false, json!([]));
            s.activate(ID, json!({}));
            let wall = s.unit(P1, 1).expect("the Wall of Bricks");
            let kinds: Vec<KeywordKind> = s.stats(&wall.id).keywords.iter().map(|keyword| keyword.kind()).collect();
            for kind in [KeywordKind::Taunt, KeywordKind::CantAttack, KeywordKind::Brittle] {
                assert!(kinds.contains(&kind), "{kind:?} in {kinds:?}");
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r384_its_activate_summons_a_radiant_wall_of_bricks() {
            let mut s = trowel(true, json!([]));
            s.activate(ID, json!({}));
            let wall = s.unit(P1, 1).expect("the Wall of Bricks");
            assert_eq!(wall.def_id, WALL);
            assert!(wall.radiant);
            s.expect_stats(&wall.id, json!({ "attack": 0, "health": 10 }));
            s.expect_refused(|s| s.activate(ID, json!({})));
        }
    }
}
