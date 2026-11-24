//! SPEC §8.13 IA #56 Afterburner ((2) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Deal {damage} damage to the enemy hero. Deal {selfDamage} damage to your hero." (damage 4, selfDamage 2)
//!   Radiant: "Deal {damage} damage to the enemy hero. Deal {selfDamage} damage to your hero." (damage 8, selfDamage 2)
//!
//! Two hits in the text's order (§4.4): the declared `damage` to the enemy hero, then the declared
//! `selfDamage` to its controller's hero.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-056";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![
                damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": param(&*ctx, "damage") }),
                )),
                damage(json_as(
                    json!({ "to": { "of": "selfHero" }, "amount": param(&*ctx, "selfDamage") }),
                )),
            ]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 8 is its declared `damage`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #56 Afterburner — SPEC §8.13 row 56.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn deals_4_to_the_enemy_hero_and_2_to_yours() {
            let mut s = board(false);
            s.play(ID, json!({}));
            s.expect_health(P2, HERO_HEALTH - 4);
            s.expect_health(P1, HERO_HEALTH - 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn deals_8_to_the_enemy_hero_and_2_to_yours() {
            let mut s = board(true);
            s.play(ID, json!({}));
            s.expect_health(P2, HERO_HEALTH - 8);
            s.expect_health(P1, HERO_HEALTH - 2);
        }
    }
}
