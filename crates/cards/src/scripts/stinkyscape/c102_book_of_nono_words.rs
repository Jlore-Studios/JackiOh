//! S #102 Book of Nono-Words (SPEC §8.10 row S 102; §2.4, R65, R766, R1748003, R1848003): (1) Spell,
//! Book, Epic.
//!   Base:    "Add 4 No-No Words to your hand, their X set at 1, 2, 3 and 4."
//!   Radiant: "Add 4 Radiant No-No Words to your hand, their X set at 1, 2, 3 and 4."
//!
//! Engine: four `add_to_hand` of S #99 No-No Words, in that order, each with its `fixedX`
//! (R1848003): played for that X with none to choose, at exactly that price, which no modifier
//! reaches (R65); a full hand burns the rest, without it (§2.4); a Bounce keeps it and a graveyard
//! takes it off (R766). Radiant: each one Radiant. The four set Xs are the card's structure, no
//! declared number (R1748003).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-102";

/// S #99 No-No Words.
const NO_NO_WORDS: &str = "stinkyscape-099";

/// R1748003: the four set Xs, in the order the copies are added.
const SET_XS: [i32; 4] = [1, 2, 3, 4];

fn book(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            SET_XS
                .iter()
                .map(|x| {
                    add_to_hand(json_as(json!({
                        "defId": NO_NO_WORDS,
                        "fixedX": x,
                        "radiant": radiant,
                    })))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: book(false),
        radiant: book(true),
    }
}

// S #102 Book of Nono-Words — SPEC §8.10 row S 102, BUILD M12 row S 102 (R1848003).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOOK: &str = "stinkyscape-102";
    const NONO: &str = "stinkyscape-099";
    const PARANOIA: &str = "meditative-099"; // (2) Field Spell: Aura: your Spells have Cloaked.
    const CORE_SYSTEM: &str = "stinkyscape-117"; // (3) Spell: Bounce all permanents.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005";

    fn nonos(cards: &[CardInstance]) -> Vec<CardInstance> {
        cards.iter().filter(|card| card.def_id == NONO).cloned().collect()
    }

    fn set_xs(cards: &[CardInstance]) -> Vec<Option<i32>> {
        cards.iter().map(|card| card.fixed_x).collect()
    }

    fn book_board(radiant: bool, hand: Value) -> Scenario {
        crate::register_all();
        let mut held = vec![json!({ "def": BOOK, "radiant": radiant })];
        held.extend(hand.as_array().cloned().unwrap_or_default());
        scenario(json!({
            "p1": { "hand": held, "mana": 10 },
            "p2": { "hand": [FILLER], "field": [VANILLA] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r1848003_four_copies_set_at_1_2_3_4_in_order() {
            let mut s = book_board(false, json!([FILLER]));

            s.play(BOOK, json!({}));

            let made = nonos(&s.hand(P1));
            assert_eq!(set_xs(&made), vec![Some(1), Some(2), Some(3), Some(4)]);
            assert!(made.iter().all(|card| !card.radiant));
        }

        #[test]
        fn r1848003_each_costs_its_x_and_a_discount_does_not_lower_it() {
            let mut s = book_board(false, json!([FILLER]));
            s.play(BOOK, json!({}));
            let turn = s.state().turn;
            s.state_mut().players[P1].mods.push(json_as(json!({
                "id": "m-s102",
                "expiry": { "until": "thisTurn", "turn": turn },
                "kind": "costDiscount",
                "amount": 2,
            })));

            for card in nonos(&s.hand(P1)) {
                assert_eq!(Some(cost_now(s.state(), &card)), card.fixed_x, "R65: no modifier reaches it");
            }
            let two = nonos(&s.hand(P1))[1].clone();
            let target = s.unit_at(P2, 1);
            let before = s.state().players[P1].mana.current;
            s.play(&two, json!({ "targets": pick::instance(&target) }));
            assert_eq!(s.state().players[P1].mana.current, before - 2);
            s.expect_in_zone(&target, "graveyard");
        }

        #[test]
        fn a_hand_with_room_for_two_keeps_1_and_2() {
            let mut s = book_board(false, filler(HAND_CAP as usize - 2));

            s.play(BOOK, json!({}));

            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            assert_eq!(set_xs(&nonos(&s.hand(P1))), vec![Some(1), Some(2)]);
            // The other two burned, and the graveyard takes their set X off (R766).
            let burned = nonos(&s.state().players[P1].graveyard);
            assert_eq!(set_xs(&burned), vec![None, None]);
        }

        #[test]
        fn r1848003_a_bounced_copy_keeps_its_x() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [BOOK, CORE_SYSTEM, FILLER],
                    "field": [{ "def": PARANOIA, "lane": 1 }],
                    "mana": 10,
                },
                "p2": { "hand": [FILLER] },
            }));
            s.play(BOOK, json!({}));
            let two = nonos(&s.hand(P1))[1].clone();
            assert_eq!(two.fixed_x, Some(2));
            s.play(&two, json!({ "zone": 2, "row": "backrow", "faceDown": "cloaked" }));
            // A set card takes a fresh id (R227).
            let set = s.backrow(P1, 2).expect("the set No-No Words");
            assert_eq!(set.fixed_x, Some(2));

            s.play(CORE_SYSTEM, json!({}));

            let mut held = set_xs(&nonos(&s.hand(P1)));
            held.sort();
            assert_eq!(held, vec![Some(1), Some(2), Some(3), Some(4)]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_four_are_radiant() {
            let mut s = book_board(true, json!([FILLER]));

            s.play(BOOK, json!({}));

            let made = nonos(&s.hand(P1));
            assert_eq!(set_xs(&made), vec![Some(1), Some(2), Some(3), Some(4)]);
            assert!(made.iter().all(|card| card.radiant));
        }
    }
}
