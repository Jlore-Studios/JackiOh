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
            && ctx.state.players[ctx.controller]
                .units
                .iter()
                .any(|pile| pile.is_none())
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

    const UNIT: &str = "core-005"; // 1/1

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
            s.expect_stats(&summoned, json!({ "attack": 3, "health": 3 }));
        }
    }
}
