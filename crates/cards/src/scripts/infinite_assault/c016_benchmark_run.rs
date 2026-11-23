//! SPEC §8.13 IA #16 Benchmark Run ((1) Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Set a friendly Unit's Attack to the highest Attack and its Health to the highest Health among
//!             enemy Units."
//!   Radiant: "Set a friendly Unit's Attack to the highest Attack and its Health to the highest Health among
//!             enemy Units, and give it Rush."
//!
//! TG #34 Dino-size's `setStats` on a friendly target, its two numbers read off the enemy Units on the
//! field as it resolves: the highest Attack and, apart, the highest current Health (R4577007). With no
//! enemy Unit no stat changes, though the Radiant face still gives Rush; Immutable refuses the set (R23).

use jackioh_engine::effects::grant_keyword;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-016";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Help),
        ..TargetDecl::target(1, 1, json!({ "side": "ally", "of": ["unit"] }))
    }]
}

/// R4577007: the highest Attack and the highest Health among the enemy Units, if there is one.
fn benchmark(ctx: &EffectContext<'_>) -> Option<(i32, i32)> {
    let views: Vec<UnitView> = active_units_of(ctx.state, opponent_of(ctx.controller))
        .into_iter()
        .map(|unit| unit_view(ctx.state, unit))
        .collect();
    let attack = views.iter().map(|view| view.attack).max()?;
    let health = views.iter().map(|view| view.health).max()?;
    Some((attack, health))
}

fn benchmark_run(rush: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let mut effects = Vec::new();
            if let Some((attack, health)) = benchmark(ctx) {
                effects.push(set_stats(json_as(
                    json!({ "target": { "of": "chosen" }, "attack": attack, "health": health }),
                )));
            }
            if rush {
                effects.push(grant_keyword(json_as(
                    json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Rush" } }),
                )));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: benchmark_run(false),
        radiant: benchmark_run(true),
    }
}

// IA #16 Benchmark Run — SPEC §8.13 row 16 (R4577007).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const MINE: &str = "stand-in (1) 1/1";

    fn benchmarked(radiant: bool, enemies: Value) -> (Scenario, CardInstance) {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "field": [MINE] },
            "p2": { "field": enemies },
        }));
        let mine = s.unit_at(P1, 1);
        s.play(ID, pick::targets(pick::instance(&mine)));
        (s, mine)
    }

    fn rushes(s: &Scenario, unit: &CardInstance) -> bool {
        has_keyword(&s.stats(unit).keywords, KeywordKind::Rush)
    }

    mod base {
        use super::*;

        #[test]
        fn takes_the_highest_attack_and_apart_the_highest_health() {
            let (mut s, mine) = benchmarked(false, json!(["stand-in (1) 5/2", "stand-in (1) 2/7"]));
            s.expect_stats(&mine, json!({ "attack": 5, "health": 7 }));
            assert!(!rushes(&s, &mine));
        }

        #[test]
        fn r4577007_the_health_read_is_current_health() {
            let (mut s, mine) = benchmarked(false, json!([{ "def": "stand-in (1) 3/8", "damage": 5 }]));
            s.expect_stats(&mine, json!({ "attack": 3, "health": 3 }));
        }

        #[test]
        fn r4577007_with_no_enemy_unit_no_stat_changes() {
            let (mut s, mine) = benchmarked(false, json!([]));
            s.expect_stats(&mine, json!({ "attack": 1, "health": 1 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn takes_the_highest_stats_and_gives_rush() {
            let (mut s, mine) = benchmarked(true, json!(["stand-in (1) 5/2", "stand-in (1) 2/7"]));
            s.expect_stats(&mine, json!({ "attack": 5, "health": 7 }));
            assert!(rushes(&s, &mine));
        }

        #[test]
        fn r4577007_with_no_enemy_unit_it_still_gives_rush() {
            let (mut s, mine) = benchmarked(true, json!([]));
            s.expect_stats(&mine, json!({ "attack": 1, "health": 1 }));
            assert!(rushes(&s, &mine));
        }
    }
}
