//! SPEC §8.13 IA #69 Rev Limiter ((4) Unit, Epic, 7/7 → 14/14), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Rush. After this attacks, it gains +{attack} Attack and deals {damage} damage to your hero." (attack 3, damage 2)
//!   Radiant: "Rush. After this attacks, it gains +{attack} Attack and deals {damage} damage to your hero." (attack 6, damage 2)
//!
//! Rush is the catalog's. `after_attack` (S #91 The Propagation's Wasp's hook, R1849001) gains the
//! declared `attack` on itself (`buff`, §6.3) and deals the declared `damage` to its controller's
//! hero (§4.4): the limiter revs up and its hero pays for it.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-069";

pub fn script() -> CardScripts {
    let base = Script {
        after_attack: Some(hook(|ctx| {
            vec![
                buff(json_as(
                    json!({ "target": { "of": "self" }, "attack": param(&*ctx, "attack") }),
                )),
                damage(json_as(
                    json!({ "to": { "of": "selfHero" }, "amount": param(&*ctx, "damage") }),
                )),
            ]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's +6 is its declared `attack`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #69 Rev Limiter — SPEC §8.13 row 69.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "field": [{ "def": ID, "radiant": radiant }] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn after_it_attacks_it_gains_3_attack_and_deals_2_to_your_hero() {
            let mut s = board(false);
            let limiter = s.unit_at(P1, 1);
            assert!(has_keyword(&s.stats(&limiter).keywords, KeywordKind::Rush));
            s.attack(&limiter, "hero");
            s.expect_health(P2, HERO_HEALTH - 7);
            s.expect_stats(&limiter, json!({ "attack": 10, "health": 7 }));
            s.expect_health(P1, HERO_HEALTH - 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn after_it_attacks_it_gains_6_attack_and_deals_2_to_your_hero() {
            let mut s = board(true);
            let limiter = s.unit_at(P1, 1);
            s.attack(&limiter, "hero");
            s.expect_health(P2, HERO_HEALTH - 14);
            s.expect_stats(&limiter, json!({ "attack": 20, "health": 14 }));
            s.expect_health(P1, HERO_HEALTH - 2);
        }
    }
}
