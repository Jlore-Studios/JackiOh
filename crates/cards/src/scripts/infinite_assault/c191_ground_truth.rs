//! SPEC §8.13 IA #191 Ground Truth ((2) Unit, Legendary, 3/5 → 6/10), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Your opponent plays with their hand revealed to you."
//!   Radiant: "Your opponent plays with their hand revealed to you, and their cards cost ({surcharge}) more."
//!
//! Built by C8 (#4592). R4592000, R4592003.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-191";

fn surcharge_aura() -> CostAuraHook {
    read_hook(|args| {
        vec![CostAura {
            rule: CostRule {
                amount: Some(param(&args, "surcharge")),
                ..CostRule::default()
            },
            whose: CostAuraWhose::Opponents,
            ban: None,
        }]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            reveals_opponents_hand: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    let radiant = Script {
        static_flags: Some(StaticFlags {
            reveals_opponents_hand: Some(true),
            ..StaticFlags::default()
        }),
        cost_aura: Some(surcharge_aura()),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// IA #191 Ground Truth — SPEC §8.13 row 191.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const ENEMY_CARD: &str = "core-005"; // (1) cost

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "field": [{ "def": ID, "radiant": radiant, "lane": 1 }],
                "mana": 10,
            },
            "p2": {
                "hand": [ENEMY_CARD],
                "mana": 10,
            },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn hand_is_revealed_to_opponent() {
            let mut s = board(false);
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 3, "health": 5 }));
            assert!(jackioh_engine::revealed_hand::hand_revealed_to(s.state(), P2, P1));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_6_10_and_surges_opponent_cards() {
            let mut s = board(true);
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 6, "health": 10 }));
            assert!(jackioh_engine::revealed_hand::hand_revealed_to(s.state(), P2, P1));
            let enemy_card = s.hand(P2)[0].clone();
            assert_eq!(cost_now(s.state(), &enemy_card), 2); // 1 + 1 surcharge
        }
    }
}
