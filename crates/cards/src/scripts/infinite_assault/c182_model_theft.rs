//! SPEC §8.13 IA #182 Model Theft ((3) Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Choose an enemy Unit. Summon a copy of it for yourself."
//!   Radiant: "Choose an enemy Unit. Summon a Radiant copy of it for yourself."
//!
//! Built by C8 (#4592). R4592000.

use jackioh_engine::effects::{
    ForEachCardArgs, SummonCopyArgs, for_each_card, set_radiant, summon_copy,
};
use jackioh_engine::prelude::*;
use jackioh_engine::prompts::summoned_so_far;

pub const ID: &str = "infiniteassault-182";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Harm),
        ..TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))
    }]
}

fn model_theft(radiant: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |_ctx| {
            let mut effects = vec![summon_copy(SummonCopyArgs {
                of: TargetSpec::Chosen { index: None },
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
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: model_theft(false),
        radiant: model_theft(true),
    }
}

// IA #182 Model Theft — SPEC §8.13 row 182.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const TARGET_UNIT: &str = "core-019"; // 9/9 Taunt

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 },
            "p2": { "field": [{ "def": TARGET_UNIT, "lane": 1 }] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn summons_copy_of_enemy_unit() {
            let mut s = board(false);
            let enemy = s.unit_at(PlayerId::P2, 1).id.clone();
            s.play(
                ID,
                json!({ "targets": [{ "pick": "instance", "instanceId": enemy }] }),
            );
            let summoned = s.unit_at(P1, 1);
            assert_eq!(summoned.def_id, TARGET_UNIT);
            assert!(!summoned.radiant);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn summons_radiant_copy() {
            let mut s = board(true);
            let enemy = s.unit_at(PlayerId::P2, 1).id.clone();
            s.play(
                ID,
                json!({ "targets": [{ "pick": "instance", "instanceId": enemy }] }),
            );
            let summoned = s.unit_at(P1, 1);
            assert_eq!(summoned.def_id, TARGET_UNIT);
            assert!(summoned.radiant);
        }
    }
}
