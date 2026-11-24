//! SPEC §8.13 IA #59 Voltage Spike ((2) Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Deal {damage} damage to an enemy. If your hero has {health} or less Health, deal {empowered} instead." (damage 3, health 15, empowered 6)
//!   Radiant: "Deal {damage} damage to an enemy. If your hero has {health} or less Health, deal {empowered} instead." (damage 6, health 15, empowered 12)
//!
//! One declared enemy, a Unit or a hero, aimed to harm (R81, as Core #35 Lunar Eclipse's): `damage`
//! of the declared `damage`, or of `empowered` when its controller's hero sits at the declared
//! `health` or less, read as the Cry resolves.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-059";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Harm),
        ..TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit", "hero"] }))
    }]
}

pub fn script() -> CardScripts {
    let spike = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            let amount = if hero_of(ctx.state, ctx.controller).health <= param(&*ctx, "health") {
                param(&*ctx, "empowered")
            } else {
                param(&*ctx, "damage")
            };
            vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 6 and 12 are its declared numbers.
    let radiant = spike.clone();
    CardScripts {
        base: spike,
        radiant,
    }
}

// IA #59 Voltage Spike — SPEC §8.13 row 59.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P2;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const WALL: &str = "stand-in (1) 5/5";

    fn board(radiant: bool, health: i32) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "health": health },
            "p2": { "field": [WALL] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn deals_3_above_15_health() {
            let mut s = board(false, HERO_HEALTH);
            let wall = s.unit_at(P2, 1);
            s.play(ID, pick::targets(pick::instance(&wall)));
            s.expect_stats(&wall, json!({ "attack": 5, "health": 2 }));
        }

        #[test]
        fn deals_6_at_15_health_or_less() {
            for health in [15, 9] {
                let mut s = board(false, health);
                let wall = s.unit_at(P2, 1);
                s.play(ID, pick::targets(pick::instance(&wall)));
                s.expect_in_zone(&wall, "graveyard");
            }
        }

        #[test]
        fn the_enemy_hero_is_a_legal_pick() {
            let mut s = board(false, HERO_HEALTH);
            s.play(ID, json!({ "targets": pick::hero("p2") }));
            s.expect_health(P2, HERO_HEALTH - 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn deals_6_above_15_health() {
            let mut s = board(true, HERO_HEALTH);
            s.play(ID, json!({ "targets": pick::hero("p2") }));
            s.expect_health(P2, HERO_HEALTH - 6);
        }

        #[test]
        fn deals_12_at_15_health_or_less() {
            let mut s = board(true, 15);
            s.play(ID, json!({ "targets": pick::hero("p2") }));
            s.expect_health(P2, HERO_HEALTH - 12);
        }
    }
}
