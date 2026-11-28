//! SPEC §8.14 FvH #18 Villa-D Moneybags ((3) Unit, Human, Epic; 4/4 → 8/8), a card of the Felinors vs
//! Humans set (R5235000).
//!   Base:    "Cry: Raise your max mana's cap by {cap} for the rest of the game. Add {cards} random cards of
//!            (5) Cost or more to your hand." (2, 2)
//!   Radiant: the same with {cap} 4 and random Radiant cards.
//!
//! Cry (R1): `raise_mana_cap` of the declared `cap` (R5237003): the cap goes 4 → 6 (Radiant 8), so max mana
//! keeps growing a crystal a turn past 4, no mana now; two Moneybags stack, and a handicapped seat's cap
//! rises from its own (FV2 #18). Then `add_random_from_catalog` of `cards` non-token cards of any type
//! printed (5) or more, Radiant on the Radiant face; a full hand burns the rest (R317). Both numbers are
//! read at the Cry (R1748000).

use jackioh_engine::effects::{add_random_from_catalog, raise_mana_cap};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-018";

/// "of (5) Cost or more": the pool's printed price floor, a printed threshold (R1748003).
const MIN_COST: i32 = 5;

/// One face; `radiant` is the face the added cards arrive on.
fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![
                raise_mana_cap(json_as(json!({ "amount": param(&*ctx, "cap") }))),
                add_random_from_catalog(json_as(json!({
                    "query": { "costRange": { "min": MIN_COST } },
                    "count": param(&*ctx, "cards"),
                    "radiant": radiant,
                }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// FvH #18 Villa-D Moneybags — SPEC §8.14 row 18 (R5237003, R1748000, R317).
#[cfg(test)]
mod tests {
    use super::{ID, MIN_COST};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{next_turn_of, weapon_board};

    const FILLER: &str = "core-005"; // Stockpile: a Spell, each side's spare card (§2.5).
    /// The Moneybags' own (3).
    const COST: i32 = 3;

    /// p1 holds the Moneybags (`radiant` its face) and nothing else; `mana` p1's current mana.
    fn moneybags(seed: &str, radiant: bool, mana: i32) -> Scenario {
        weapon_board(
            seed,
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "mana": mana }),
            json!({ "hand": [FILLER] }),
        )
    }

    /// p1's max mana at the start of each of their next `turns` turns.
    fn maxes(s: &mut Scenario, turns: usize) -> Vec<i32> {
        (0..turns)
            .map(|_| {
                next_turn_of(s, P1);
                max_mana_of(s.state(), P1)
            })
            .collect()
    }

    /// The cards the Cry added: p1's whole hand, each a non-token printed (5) or more, on `radiant`.
    fn expect_added(s: &Scenario, count: usize, radiant: bool) {
        let added = s.hand(P1);
        assert_eq!(added.len(), count);
        for card in nonempty(added, "the added cards") {
            let def = crate::card_def(&card.def_id);
            assert!(query_cost(&def) >= MIN_COST, "{} costs {}", def.id, query_cost(&def));
            assert!(!def.token, "{}", def.id);
            assert_eq!(card.radiant, radiant, "{}", def.id);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r5237003_your_mana_cap_rises_by_2_for_the_rest_of_the_game_and_no_mana_now() {
            let mut s = moneybags("fvh018-cap", false, MAX_MANA);
            s.play(ID, json!({}));
            s.expect_mana(P1, MAX_MANA - COST);
            assert_eq!(s.state().players[P1].mana_cap_bonus, Some(2));
            assert_eq!(maxes(&mut s, 3), vec![MAX_MANA + 2; 3]);
            assert_eq!(max_mana_of(s.state(), P2), MAX_MANA, "the opponent's cap stays");
        }

        #[test]
        fn adds_2_random_non_token_cards_of_5_or_more() {
            let mut s = moneybags("fvh018-cards", false, MAX_MANA);
            s.play(ID, json!({}));
            expect_added(&s, 2, false);
        }

        #[test]
        fn r5237003_two_moneybags_stack() {
            let mut s = weapon_board(
                "fvh018-two",
                json!({ "hand": [ID, ID], "mana": 10 }),
                json!({ "hand": [FILLER] }),
            );
            s.play(ID, json!({}));
            s.play(ID, json!({}));
            assert_eq!(s.state().players[P1].mana_cap_bonus, Some(4));
        }

        #[test]
        fn r1748000_both_numbers_move_with_a_buff() {
            let mut s = moneybags("fvh018-buff", false, MAX_MANA);
            assert_eq!(crate::upgrade_number(&mut s, ID, "cap"), 3);
            assert_eq!(crate::upgrade_number(&mut s, ID, "cards"), 3);
            s.play(ID, json!({}));
            assert_eq!(s.state().players[P1].mana_cap_bonus, Some(3));
            expect_added(&s, 3, false);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5237003_the_cap_rises_by_4() {
            let mut s = moneybags("fvh018-radiant-cap", true, MAX_MANA);
            s.play(ID, json!({}));
            s.expect_mana(P1, MAX_MANA - COST);
            assert_eq!(s.state().players[P1].mana_cap_bonus, Some(4));
            // One crystal a turn past 4, up to 8.
            assert_eq!(maxes(&mut s, 4), vec![6, 7, 8, 8]);
        }

        #[test]
        fn adds_2_random_radiant_cards_of_5_or_more() {
            let mut s = moneybags("fvh018-radiant-cards", true, MAX_MANA);
            s.play(ID, json!({}));
            expect_added(&s, 2, true);
        }
    }
}
