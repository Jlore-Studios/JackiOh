//! SPEC §8.13 IA #188 Overfitting ((3) Trap, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reveals when your opponent plays a Unit: Summon a copy of it for yourself with +2/+2."
//!   Radiant: "Reveals when your opponent plays a Unit: Summon a Radiant copy of it for yourself with +2/+2."
//!
//! Built by C8 (#4592). R4592000.

use jackioh_engine::effects::{
    ForEachCardArgs, SummonCopyArgs, buff, for_each_card, set_radiant, summon_copy,
};
use jackioh_engine::prelude::*;
use jackioh_engine::prompts::summoned_so_far;
use jackioh_engine::query::left_field_since_resolved;

pub const ID: &str = "infiniteassault-188";

fn overfitting(radiant: bool) -> Script {
    let trigger = TriggerDef::new("overfitting", &[GameEventType::CardResolved], move |_ctx, event| {
        let GameEvent::CardResolved { instance_id, .. } = event else {
            return vec![];
        };
        let mut effects = vec![summon_copy(SummonCopyArgs {
            of: TargetSpec::Instance {
                instance_id: instance_id.clone(),
            },
            player: Some(PlayerSpec::SelfSide),
            lane: None,
            stack: None,
            vanilla: None,
            granted_keywords: None,
        })];
        if radiant {
            effects.push(for_each_card(ForEachCardArgs {
                cards: Arc::new(|ctx| summoned_so_far(ctx)),
                each: Arc::new(|id| set_radiant(json_as(json!({ "instanceId": id })))),
            }));
        }
        effects.push(for_each_card(ForEachCardArgs {
            cards: Arc::new(|ctx| summoned_so_far(ctx)),
            each: Arc::new(|id| {
                buff(json_as(json!({
                    "target": { "of": "instance", "instanceId": id },
                    "attack": 2,
                    "health": 2,
                })))
            }),
        }));
        effects
    })
    .with_when(|ctx, event| {
        let GameEvent::CardResolved {
            player, permanent, ..
        } = event
        else {
            return false;
        };
        *player != ctx.controller
            && played_type_of(&*ctx.state, event) == Some(CardType::Unit)
            && *permanent
            && !left_field_since_resolved(&*ctx.state, event)
            && !fill_board_zones(ctx.state, ctx.controller).is_empty()
    });

    Script {
        triggers: vec![trigger],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: overfitting(false),
        radiant: overfitting(true),
    }
}

// IA #188 Overfitting — SPEC §8.13 row 188.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 1/1 R2/2";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": false, "lane": 1 }],
                "mana": 10,
            },
            "p2": { "hand": [UNIT], "mana": 10 },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn summons_copy_with_plus_2_plus_2() {
            let mut s = board(false);
            s.play(UNIT, json!({ "zone": 1 }));
            s.expect_in_zone(ID, "graveyard");
            let summoned = s.unit_at(P1, 1);
            assert_eq!(summoned.def_id, UNIT);
            assert!(!summoned.radiant);
            s.expect_stats(&summoned, json!({ "attack": 3, "health": 3 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn summons_radiant_copy_with_plus_2_plus_2() {
            let mut s = board(true);
            s.play(UNIT, json!({ "zone": 1 }));
            s.expect_in_zone(ID, "graveyard");
            let summoned = s.unit_at(P1, 1);
            assert_eq!(summoned.def_id, UNIT);
            assert!(summoned.radiant);
            s.expect_stats(&summoned, json!({ "attack": 4, "health": 4 }));
        }
    }

    /// Overfitting set in p1's lane 1; p1's unit zones hold `held` stand-ins (lanes 1..=held);
    /// p2 holds the Unit to play.
    fn crowded(radiant: bool, held: usize) -> Scenario {
        let field: Vec<_> = (1..=held)
            .map(|lane| json!({ "def": UNIT, "lane": lane }))
            .collect();
        scenario(json!({
            "active": "p2",
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": false, "lane": 1 }],
                "field": field,
                "mana": 10,
            },
            "p2": { "hand": [UNIT], "mana": 10 },
        }))
    }

    fn reserve_lane_5(s: &mut Scenario) {
        reserve_zone(s.state_mut(), ZoneSlot { player: P1, row: Row::Units, lane: 5 });
    }

    /// R4985000: the reveal gate reads `fill_board_zones`, the unit zones a summon can take.
    mod gate {
        use super::*;

        #[test]
        fn r4985000_stays_set_when_every_unit_zone_is_occupied() {
            let mut s = crowded(false, 5);
            s.play(UNIT, json!({ "zone": 1 }));
            s.expect_in_zone(ID, "field");
            assert_eq!(active_units_of(s.state(), P1).len(), 5);
        }

        #[test]
        fn r4985000_stays_set_when_the_only_empty_zone_is_reserved() {
            let mut s = crowded(false, 4);
            reserve_lane_5(&mut s);
            s.play(UNIT, json!({ "zone": 1 }));
            s.expect_in_zone(ID, "field");
            assert_eq!(active_units_of(s.state(), P1).len(), 4);
        }

        #[test]
        fn r4985000_stays_set_when_the_only_empty_zone_is_in_the_hole() {
            let mut s = crowded(false, 4);
            s.state_mut().black_hole = Some(BlackHole {
                lane: 5,
                swallowed: None,
                pulls: None,
                capacity: None,
                at_end: None,
            });
            s.play(UNIT, json!({ "zone": 1 }));
            s.expect_in_zone(ID, "field");
            assert_eq!(active_units_of(s.state(), P1).len(), 4);
        }

        #[test]
        fn r4985000_reveals_while_an_open_zone_remains_beside_a_reserved_one() {
            let mut s = crowded(false, 3);
            reserve_lane_5(&mut s);
            s.play(UNIT, json!({ "zone": 1 }));
            s.expect_in_zone(ID, "graveyard");
            let summoned = s.unit_at(P1, 4);
            assert_eq!(summoned.def_id, UNIT);
            s.expect_stats(&summoned, json!({ "attack": 3, "health": 3 }));
            assert_eq!(active_units_of(s.state(), P1).len(), 4);
        }

        #[test]
        fn r4985000_a_radiant_overfitting_obeys_the_same_gate() {
            let mut s = crowded(true, 5);
            s.play(UNIT, json!({ "zone": 1 }));
            s.expect_in_zone(ID, "field");
            assert_eq!(active_units_of(s.state(), P1).len(), 5);
        }

        /// R4985000: the gate counts unit zones only; a Flying copy that could stand in the backrow
        /// (R2248000) does not open it.
        #[test]
        fn r4985000_a_full_unit_row_keeps_the_trap_set_even_for_a_flying_unit() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": {
                    "backrow": [{ "def": ID, "faceUp": false, "lane": 1 }],
                    "field": (1..=5).map(|lane| json!({ "def": UNIT, "lane": lane })).collect::<Vec<_>>(),
                    "mana": 10,
                },
                "p2": { "hand": ["stand-in (1) 0/40 Flying"], "mana": 10 },
            }));
            s.play("stand-in (1) 0/40 Flying", json!({ "zone": 1 }));
            s.expect_in_zone(ID, "field");
            assert_eq!(active_units_of(s.state(), P1).len(), 5);
        }
    }
}
