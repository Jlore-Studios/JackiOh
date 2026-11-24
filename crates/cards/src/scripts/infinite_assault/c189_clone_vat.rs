//! SPEC §8.13 IA #189 Clone Vat ((4) Field Spell, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "End of turn: Summon a copy of a random enemy Unit. Give it Brittle 2."
//!   Radiant: "End of turn: Summon a copy of a random enemy Unit."
//!
//! Built by C8 (#4592). R4592000.

use jackioh_engine::effects::brittle::give_brittle;
use jackioh_engine::effects::{ForEachCardArgs, SummonCopyArgs, for_each_card, summon_copy};
use jackioh_engine::prelude::*;
use jackioh_engine::prompts::summoned_so_far;
use jackioh_engine::zones::active_units_of;

pub const ID: &str = "infiniteassault-189";

fn clone_vat(radiant: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            let opp = opponent_of(ctx.controller);
            let enemy_ids: Vec<String> = active_units_of(ctx.state, opp)
                .into_iter()
                .map(|u| u.id.clone())
                .collect();
            let Some(chosen_id) = ctx.rng.pick(&enemy_ids).cloned() else {
                return vec![];
            };
            let mut effects = vec![summon_copy(SummonCopyArgs {
                of: TargetSpec::Instance {
                    instance_id: chosen_id,
                },
                player: Some(PlayerSpec::SelfSide),
                lane: None,
                stack: None,
                vanilla: None,
                granted_keywords: None,
            })];
            if !radiant {
                effects.push(for_each_card(ForEachCardArgs {
                    cards: Arc::new(|ctx| summoned_so_far(ctx)),
                    each: Arc::new(|id| give_brittle(json_as(json!({ "instanceId": id, "n": 2 })))),
                }));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: clone_vat(false),
        radiant: clone_vat(true),
    }
}

// IA #189 Clone Vat — SPEC §8.13 row 189.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const ENEMY_UNIT: &str = "core-019"; // 9/9 Taunt

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": true, "lane": 1 }],
                "mana": 10,
            },
            "p2": { "field": [{ "def": ENEMY_UNIT, "lane": 1 }] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn summons_copy_with_brittle_2_at_end_of_turn() {
            let mut s = board(false);
            s.end_turn(); // p1 end of turn fires Clone Vat
            let summoned = s.unit_at(P1, 1);
            assert_eq!(summoned.def_id, ENEMY_UNIT);
            assert_eq!(summoned.brittle.as_ref().map(|b| b.count), Some(2));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn summons_copy_without_brittle() {
            let mut s = board(true);
            s.end_turn();
            let summoned = s.unit_at(P1, 1);
            assert_eq!(summoned.def_id, ENEMY_UNIT);
            assert_eq!(summoned.brittle, None);
        }
    }
}
