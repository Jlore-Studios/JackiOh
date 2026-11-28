//! SPEC §8.14 FvH #107 KY, Himself ((1) Unit, Human, KY, Acclaimed, Mythic; 1/3 → 2/3), a card of the
//! Felinors vs Humans set (R5235000).
//!   Base:    "Cry: Swap hands with your opponent until the end of this turn."
//!   Radiant: "Cry: Swap hands with your opponent until the end of this turn. The cards you get cost
//!            ({discount}) less until then." (1)
//!
//! Cry (R1): `swap_hands` with `until_end_of_turn` (R5238013): the whole hands change holders, owner and
//! controller, with no burn (R4586008); at the end of the swapping player's turn every swapped card still
//! in the hand the swap put it in goes back to the seat it came from, and a card that joined a hand
//! later stays (FV2 #107). The Radiant face's swap takes the declared `discount` off the cards its
//! player gets (IA #48 Ego Death's `costMod`), which they give back as they go back. The list's "until
//! the end of your turn" prints as "this turn" (R366).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-107";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| vec![swap_hands(json_as(json!({ "untilEndOfTurn": true })))])),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|ctx| {
                let discount = param(&*ctx, "discount");
                vec![swap_hands(json_as(json!({ "untilEndOfTurn": true, "costMod": -discount })))]
            })),
            ..Script::default()
        },
    }
}

// FvH #107 KY, Himself — SPEC §8.14 row 107 (R5238013, R4586008).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{def_ids, weapon_board};

    const MINE: &str = "core-005"; // Stockpile: held by p1.
    /// Held by p2: a (0) Unit, so p1 can play one it takes.
    const THEIRS: &str = "stand-in (0) 1/1";

    /// p1 plays KY (`radiant` its face) holding two MINE; p2 holds three THEIRS.
    fn swapped(seed: &str, radiant: bool) -> Scenario {
        let mut s = weapon_board(
            seed,
            json!({ "hand": [{ "def": ID, "radiant": radiant }, MINE, MINE] }),
            json!({ "hand": [THEIRS, THEIRS, THEIRS] }),
        );
        s.play(ID, json!({}));
        s
    }

    /// The first `count` cards of `player`'s hand by def id.
    fn first_of(s: &Scenario, player: PlayerId, count: usize) -> Vec<String> {
        def_ids(&s.hand(player)).into_iter().take(count).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r5238013_the_hands_swap_until_the_end_of_this_turn() {
            let mut s = swapped("fvh107-base", false);
            assert_eq!(def_ids(&s.hand(P1)), vec![THEIRS; 3]);
            assert_eq!(def_ids(&s.hand(P2)), vec![MINE; 2]);
            for card in nonempty(s.hand(P1), "the cards p1 took") {
                assert_eq!((card.owner, card.controller, card.cost_mod), (P1, P1, 0));
            }
            s.end_turn();
            // Back at p1's end of turn, before p2's draw lands at the right end of p2's hand.
            assert_eq!(def_ids(&s.hand(P1)), vec![MINE; 2]);
            assert_eq!(first_of(&s, P2, 3), vec![THEIRS; 3]);
            for card in nonempty(s.hand(P1), "the cards back with p1") {
                assert_eq!((card.owner, card.controller), (P1, P1));
            }
        }

        #[test]
        fn r5238013_a_card_played_from_the_swapped_hand_stays_where_it_went() {
            let mut s = swapped("fvh107-played", false);
            let taken = s.hand(P1)[0].clone();
            s.play(&taken, json!({}));
            s.expect_in_zone(&taken, "field");
            s.end_turn();
            s.expect_in_zone(&taken, "field");
            assert_eq!(s.card(&taken).controller, P1);
            assert_eq!(first_of(&s, P2, 2), vec![THEIRS; 2]);
            assert_eq!(def_ids(&s.hand(P2)).iter().filter(|id| id.as_str() == THEIRS).count(), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5238013_the_cards_you_get_cost_1_less_until_they_go_back() {
            let mut s = swapped("fvh107-radiant", true);
            assert_eq!(def_ids(&s.hand(P1)), vec![THEIRS; 3]);
            for card in nonempty(s.hand(P1), "the cards p1 took") {
                assert_eq!(card.cost_mod, -1);
            }
            for card in nonempty(s.hand(P2), "the cards p2 took") {
                assert_eq!(card.cost_mod, 0, "only the swapper's cards are discounted");
            }
            s.end_turn();
            let back: Vec<CardInstance> = s.hand(P2).into_iter().filter(|card| card.def_id == THEIRS).collect();
            assert_eq!(back.len(), 3);
            for card in nonempty(back, "the cards back with p2") {
                assert_eq!(card.cost_mod, 0, "the discount went back with them");
            }
        }

        #[test]
        fn r1748000_the_discount_moves_with_a_buff() {
            let mut s = weapon_board(
                "fvh107-buff",
                json!({ "hand": [{ "def": ID, "radiant": true }, MINE] }),
                json!({ "hand": [THEIRS] }),
            );
            assert_eq!(crate::upgrade_number(&mut s, ID, "discount"), 2);
            s.play(ID, json!({}));
            assert_eq!(s.hand(P1)[0].cost_mod, -2);
        }
    }
}
