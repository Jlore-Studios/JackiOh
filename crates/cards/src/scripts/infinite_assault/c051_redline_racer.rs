//! SPEC §8.13 IA #51 Redline Racer ((2) Unit, Common, 5/2 → 10/4), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Charge. End of turn: Deal {damage} damage to your hero." (damage 1)
//!   Radiant: "Charge, First Strike. End of turn: Deal {damage} damage to your hero." (damage 1)
//!
//! Charge and First Strike are the catalog's. Its end of turn deals the declared `damage` to its
//! controller's hero (§2.2, R68), on every end of turn it sees while face-up on the field.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-051";

pub fn script() -> CardScripts {
    let base = Script {
        end_of_turn: Some(hook(|ctx| {
            vec![damage(json_as(
                json!({ "to": { "of": "selfHero" }, "amount": param(&*ctx, "damage") }),
            ))]
        })),
        ..Script::default()
    };
    // The same script: nothing reads a face but the catalog's keywords and the one number.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #51 Redline Racer — SPEC §8.13 row 51.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: a card in hand, so no turn ends by itself

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "radiant": radiant }], "library": crate::filler(2) },
            "p2": { "hand": [FILLER], "library": crate::filler(2) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn charges_and_its_end_of_turn_deals_1_to_your_hero() {
            let mut s = board(false);
            let racer = s.unit_at(P1, 1);
            assert!(has_keyword(&s.stats(&racer).keywords, KeywordKind::Charge));
            assert!(!has_keyword(&s.stats(&racer).keywords, KeywordKind::FirstStrike));
            // Charge attacks at once: the 5 Attack lands on the enemy hero.
            s.attack(&racer, "hero");
            s.expect_health(P2, HERO_HEALTH - 5);

            s.end_turn();
            s.expect_health(P1, HERO_HEALTH - 1);
            s.expect_health(P2, HERO_HEALTH - 5);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn charges_with_first_strike_and_its_end_of_turn_deals_1_to_your_hero() {
            let mut s = board(true);
            let racer = s.unit_at(P1, 1);
            assert!(has_keyword(&s.stats(&racer).keywords, KeywordKind::Charge));
            assert!(has_keyword(&s.stats(&racer).keywords, KeywordKind::FirstStrike));
            s.attack(&racer, "hero");
            s.expect_health(P2, HERO_HEALTH - 10);

            s.end_turn();
            s.expect_health(P1, HERO_HEALTH - 1);
        }
    }
}
