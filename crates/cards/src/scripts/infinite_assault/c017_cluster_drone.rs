//! SPEC §8.13 IA #17 Cluster Drone ((2) Unit, Rare, 3/2 → 6/4), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Death: Deal {damage} damage to every enemy Unit." (damage 2)
//!   Radiant: "Death: Deal {damage} damage to every enemy Unit." (damage 4)
//!
//! A `damageAll` over the enemy side from its Death: Units only, never the hero (§4.4).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-017";

pub fn script() -> CardScripts {
    let base = Script {
        death: Some(hook(|ctx| {
            vec![damage_all(json_as(json!({ "amount": param(&*ctx, "damage"), "side": "enemy" })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 4 is its declared `damage`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #17 Cluster Drone — SPEC §8.13 row 17.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const WALL: &str = "stand-in (1) 1/9";

    fn after_its_death(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [HIT_JOB], "field": [{ "def": ID, "radiant": radiant }, WALL], "mana": 10 },
            "p2": { "field": [WALL, WALL] },
        }));
        let drone = s.unit_at(P1, 1);
        s.play(HIT_JOB, pick::targets(pick::instance(&drone)));
        s
    }

    fn healths(s: &Scenario, player: PlayerId) -> Vec<i32> {
        active_units_of(s.state(), player)
            .into_iter()
            .map(|unit| s.stats(unit).health)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn death_deals_2_to_every_enemy_unit_and_spares_its_side_and_the_hero() {
            let mut s = after_its_death(false);
            assert_eq!(healths(&s, P2), [7, 7]);
            assert_eq!(healths(&s, P1), [9]);
            s.expect_health(P2, HERO_HEALTH);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn death_deals_4_to_every_enemy_unit() {
            let s = after_its_death(true);
            assert_eq!(healths(&s, P2), [5, 5]);
            assert_eq!(healths(&s, P1), [9]);
        }
    }
}
