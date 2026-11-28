//! SPEC §8.14 FvH #147 Plagiarized: Ground Truth, a frozen copy of IA #191 Ground Truth (R2195003, FV4), a
//! card of the held Felinors vs Humans set, built by #5235.
//!
//! The code is a copy of the IA card's as it stood when the set was founded, not a re-export, so a later IA
//! patch does not move it (R2195003). Its `ID` is its own, so every read of it names this card; it is named
//! `Plagiarized: Ground Truth`, tagged Plagiarized only, and a token it makes stays the IA token. Its change
//! from IA #191: none. The IA file's notes, as they stood:
//!
//! SPEC §8.13 IA #191 Ground Truth ((2) Unit, Legendary, 3/5 → 6/10), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Aura: Your opponent plays with their hand revealed to you."
//!   Radiant: "Aura: Your opponent plays with their hand revealed to you, and their cards cost ({surcharge}) more."
//!
//! Built by C8 (#4592). R4592000, R4592003.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-147";

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

// FvH #147 Plagiarized: Ground Truth — SPEC §8.14 row 147; IA #191's tests (SPEC §8.13 row 191), on this
// card.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const ENEMY_CARD: &str = "core-005"; // Stockpile

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
        fn r4592003_hand_is_revealed_to_opponent() {
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
