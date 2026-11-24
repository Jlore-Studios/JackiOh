//! SPEC §8.13 IA #187 Fine-Tuner ((2) Unit, Epic, 3/4 → 6/8), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Your cards that started in your opponent's deck cost ({discount}) less." (discount: 1)
//!   Radiant: "Your cards that started in your opponent's deck cost ({discount}) less." (discount: 2)
//!
//! Built by C8 (#4592). R4592000, R4592002.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-187";

fn discount_aura() -> CostAuraHook {
    read_hook(|args| {
        vec![CostAura {
            rule: CostRule {
                amount: Some(-param(&args, "discount")),
                from_opponents_deck: Some(true),
                ..CostRule::default()
            },
            whose: CostAuraWhose::Yours,
            ban: None,
        }]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        cost_aura: Some(discount_aura()),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #187 Fine-Tuner — SPEC §8.13 row 187.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const TARGET: &str = "core-019"; // (4) cost
    const NORMAL: &str = "core-005"; // (1) cost

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "field": [{ "def": ID, "radiant": radiant, "lane": 1 }],
                "hand": [NORMAL],
                "mana": 10,
            },
            "p2": { "library": crate::filler(2) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn reduces_cost_of_cards_from_opponents_deck_by_one() {
            let mut s = board(false);
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 3, "health": 4 }));
            // Give p1 a card that started in p2's deck
            let mut stolen = new_instance(s.state_mut(), TARGET, P1, Zone::Hand { player: P1 });
            stolen.started_in = Some(PlayerId::P2);
            s.state_mut().players[P1].hand.push(stolen);

            // Stolen card has 1 discount
            let stolen_card = s.hand(P1).into_iter().find(|c| c.def_id == TARGET).unwrap();
            assert_eq!(cost_now(s.state(), &stolen_card), 3); // 4 - 1

            // Normal card has no discount
            let normal_card = s.hand(P1).into_iter().find(|c| c.def_id == NORMAL).unwrap();
            assert_eq!(cost_now(s.state(), &normal_card), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_6_8_and_reduces_cost_by_two() {
            let mut s = board(true);
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 6, "health": 8 }));
            let mut stolen = new_instance(s.state_mut(), TARGET, P1, Zone::Hand { player: P1 });
            stolen.started_in = Some(PlayerId::P2);
            s.state_mut().players[P1].hand.push(stolen);

            let stolen_card = s.hand(P1).into_iter().find(|c| c.def_id == TARGET).unwrap();
            assert_eq!(cost_now(s.state(), &stolen_card), 2); // 4 - 2
        }
    }
}
