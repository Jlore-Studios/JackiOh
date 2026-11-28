//! SPEC §8.14 FvH #92 KY's Patented Log 2 Utility ((2) Field Spell, KY, Mythic), a card of the Felinors vs
//! Humans set (R5235000).
//!   Base:    "Aura: Every card's Attack, Health and Cost count as the number of digits they have in binary."
//!   Radiant: "Aura: Your cards' Costs and your opponent's cards' Attack and Health count as the number of
//!            digits they have in binary."
//!
//! The engine's flag, nothing to run (FV3 E2): `StaticFlags.log2_utility`, `All` on the base face and
//! `Helpful` on the Radiant. While it acts on the field a card's Attack and maximum Health and its own
//! cost (never an X) count as `log2_utility(n)`, ⌊log₂n⌋ + 1, n ≤ 0 unchanged (R5237011); the Radiant
//! face only its controller's costs and the enemy's Attack and Health (R5237012). Two do not stack, and
//! damage stays on a Unit, so one whose Health falls to 0 under it dies at the state check (R59, FV2
//! #92). Declares no numbers: the base 2 is the engine's.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-092";

/// One face: the flag in `mode`.
fn face(mode: Log2Utility) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            log2_utility: Some(mode),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(Log2Utility::All),
        radiant: face(Log2Utility::Helpful),
    }
}

// FvH #92 KY's Patented Log 2 Utility — SPEC §8.14 row 92 (R5237011, R5237012, R59).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::weapon_board;

    /// A (4) 5/8 to hold: (3) and 3/4 in binary digits.
    const BIG: &str = "stand-in (4) 5/8";
    /// The same stats on the field.
    const BIG_ON_FIELD: &str = "stand-in (1) 5/8";

    /// The price of `player`'s first BIG in hand now.
    fn big_price(s: &Scenario, player: PlayerId) -> i32 {
        let held = must(
            s.hand(player).into_iter().find(|card| card.def_id == BIG),
            "a BIG in hand",
        );
        effective_cost(s.state(), &held, CostOptions::default())
    }

    #[test]
    fn declares_no_numbers() {
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r5237011_every_cards_attack_health_and_cost_count_as_their_binary_digits() {
            let mut s = weapon_board(
                "fvh092-all",
                json!({ "hand": [ID, BIG], "field": [BIG_ON_FIELD] }),
                json!({ "hand": [BIG], "field": [BIG_ON_FIELD] }),
            );
            let mine = s.unit_at(P1, 1).id;
            let theirs = s.unit_at(P2, 1).id;
            assert_eq!((big_price(&s, P1), big_price(&s, P2)), (4, 4));
            s.play(ID, json!({}));
            for id in [&mine, &theirs] {
                s.expect_stats(id.as_str(), json!({ "attack": 3, "health": 4, "maxHealth": 4 }));
            }
            assert_eq!((big_price(&s, P1), big_price(&s, P2)), (3, 3));
        }

        #[test]
        fn r59_a_damaged_unit_whose_health_falls_to_0_under_it_dies() {
            let mut s = weapon_board(
                "fvh092-dies",
                json!({ "hand": [ID, BIG] }),
                json!({ "hand": [BIG], "field": [{ "def": BIG_ON_FIELD, "damage": 4 }, BIG_ON_FIELD] }),
            );
            let damaged = s.unit_at(P2, 1).id;
            let whole = s.unit_at(P2, 2).id;
            s.play(ID, json!({}));
            s.expect_in_zone(damaged.as_str(), "graveyard");
            s.expect_in_zone(whole.as_str(), "field");
        }

        #[test]
        fn r5237011_two_do_not_stack() {
            let mut s = weapon_board(
                "fvh092-two",
                json!({ "hand": [ID, ID], "field": [BIG_ON_FIELD] }),
                json!({ "hand": [BIG] }),
            );
            let mine = s.unit_at(P1, 1).id;
            s.play(ID, json!({}));
            s.play(ID, json!({}));
            s.expect_stats(mine.as_str(), json!({ "attack": 3, "maxHealth": 4 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5237012_only_your_costs_and_the_enemys_attack_and_health() {
            let mut s = weapon_board(
                "fvh092-helpful",
                json!({ "hand": [{ "def": ID, "radiant": true }, BIG], "field": [BIG_ON_FIELD] }),
                json!({ "hand": [BIG], "field": [BIG_ON_FIELD] }),
            );
            let mine = s.unit_at(P1, 1).id;
            let theirs = s.unit_at(P2, 1).id;
            s.play(ID, json!({}));
            s.expect_stats(theirs.as_str(), json!({ "attack": 3, "health": 4, "maxHealth": 4 }));
            s.expect_stats(mine.as_str(), json!({ "attack": 5, "health": 8, "maxHealth": 8 }));
            assert_eq!((big_price(&s, P1), big_price(&s, P2)), (3, 4));
        }
    }
}
