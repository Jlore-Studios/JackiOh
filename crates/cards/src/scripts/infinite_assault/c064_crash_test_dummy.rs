//! SPEC §8.13 IA #64 Crash Test Dummy ((1) Unit, Rare, 1/6 → 2/12), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Taunt. Death: Deal {damage} damage to the enemy hero." (damage 5)
//!   Radiant: "Taunt. Death: Deal {damage} damage to the enemy hero." (damage 10)
//!
//! Taunt is the catalog's. Its Death deals the declared `damage` to the enemy hero (§4.4).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-064";

pub fn script() -> CardScripts {
    let base = Script {
        death: Some(hook(|ctx| {
            vec![damage(json_as(
                json!({ "to": { "of": "enemyHero" }, "amount": param(&*ctx, "damage") }),
            ))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 10 is its declared `damage`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #64 Crash Test Dummy — SPEC §8.13 row 64.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [HIT_JOB], "field": [{ "def": ID, "radiant": radiant }], "mana": 10 },
            "p2": { "field": ["stand-in (1) 1/1"] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn its_death_deals_5_to_the_enemy_hero() {
            let mut s = board(false);
            let dummy = s.unit_at(P1, 1);
            assert!(has_keyword(&s.stats(&dummy).keywords, KeywordKind::Taunt));
            s.play(HIT_JOB, pick::targets(pick::instance(&dummy)));
            s.expect_in_zone(&dummy, "graveyard");
            s.expect_health(P2, HERO_HEALTH - 5);
            s.expect_health(P1, HERO_HEALTH);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_death_deals_10_to_the_enemy_hero() {
            let mut s = board(true);
            let dummy = s.unit_at(P1, 1);
            s.play(HIT_JOB, pick::targets(pick::instance(&dummy)));
            s.expect_health(P2, HERO_HEALTH - 10);
        }
    }
}
