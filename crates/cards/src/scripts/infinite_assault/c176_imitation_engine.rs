//! SPEC §8.13 IA #176 Imitation Engine ((3) Unit, Common, 4/6 → 8/12), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Set this's Attack to the highest Attack among enemy Units."
//!   Radiant: "Cry: Set this's Attack to the highest Attack and its Health to the highest Health among enemy Units."
//!
//! Built by C8 (#4592). R4592000.

use jackioh_engine::damage::DamageTarget;
use jackioh_engine::effects::{SetStatsArgs, set_stats};
use jackioh_engine::layers::{stats_with_buffs, unit_view};
use jackioh_engine::prelude::*;
use jackioh_engine::zones::active_units_of;

pub const ID: &str = "infiniteassault-176";

fn imitation(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let opp = opponent_of(ctx.controller);
            let enemy_units = active_units_of(ctx.state, opp);
            if enemy_units.is_empty() {
                return vec![];
            }
            let max_attack = enemy_units
                .iter()
                .map(|u| unit_view(ctx.state, u).attack)
                .max()
                .unwrap_or(0);
            let Some(DamageTarget::Unit { instance: this }) =
                resolve_target(ctx, &TargetSpec::SelfCard)
            else {
                return vec![];
            };
            let health = if radiant {
                enemy_units
                    .iter()
                    .map(|u| unit_view(ctx.state, u).health)
                    .max()
                    .unwrap_or(0)
            } else {
                stats_with_buffs(ctx.state, &this).max_health
            };
            vec![set_stats(SetStatsArgs {
                target: TargetSpec::SelfCard,
                attack: max_attack,
                health,
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: imitation(false),
        radiant: imitation(true),
    }
}

// IA #176 Imitation Engine — SPEC §8.13 row 176.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005";
    const BIG_UNIT: &str = "core-019"; // 9/9

    fn board(radiant: bool, enemy_field: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "mana": 10 },
            "p2": { "field": enemy_field },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn sets_attack_to_highest_among_enemy_units() {
            let mut s = board(false, json!([{ "def": BIG_UNIT, "lane": 1 }]));
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 9, "health": 6 }));
        }

        #[test]
        fn with_no_enemy_units_keeps_printed_stats() {
            let mut s = board(false, json!([]));
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 4, "health": 6 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn sets_attack_and_health_to_highest_among_enemy_units() {
            let mut s = board(true, json!([{ "def": BIG_UNIT, "lane": 1 }]));
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 9, "health": 9 }));
        }
    }
}
