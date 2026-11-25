//! SPEC §8.13 IA #19 Infinite Scroll ((3) Spell, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Draw {draw|card|cards}. Shuffle a copy of this into your deck." (draw 3)
//!   Radiant: "Draw {draw|card|cards}. Shuffle a copy of this into your deck. It costs ({discount}) less."
//!             (draw 4, discount 1)
//!
//! The draws, then Core #90.1 Super-Virus's fresh copy of the face that was cast (R57), so the copy is
//! never one of the cards drawn. The Radiant face's copy carries `shuffleInto`'s `costMod`; a fresh copy
//! carries no discount of the card copied, so a copy of a copy costs (1) less again, never (2)
//! (R4577002). A full library refuses the copy (R80, R316).

use jackioh_engine::effects::{draw, shuffle_into};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-019";

/// "a copy of this".
const COPIES: i32 = 1;

fn scroll(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut copy = json!({ "defId": ID, "count": COPIES, "player": "self", "radiant": radiant });
            if let Some(this) = ctx.self_.as_ref() {
                copy["copyOf"] = json!(this.id);
            }
            if radiant {
                copy["costMod"] = json!(-param(&*ctx, "discount"));
            }
            vec![draw(json_as(json!({ "count": param(&*ctx, "draw") }))), shuffle_into(json_as(copy))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: scroll(false),
        radiant: scroll(true),
    }
}

// IA #19 Infinite Scroll — SPEC §8.13 row 19 (R57, R4577002).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn board(radiant: bool, library: usize) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": crate::filler(library), "mana": 10 },
            "p2": { "library": crate::filler(4) },
        }))
    }

    fn copies(s: &Scenario) -> Vec<CardInstance> {
        s.pile(P1, "library").into_iter().filter(|card| card.def_id == ID).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn draws_three_then_shuffles_one_full_price_copy_in() {
            let mut s = board(false, 3);
            s.play(ID, json!({}));
            assert_eq!(s.hand(P1).len(), 3);
            assert!(s.hand(P1).iter().all(|card| card.def_id != ID), "the copy is never drawn");
            let copies = copies(&s);
            assert_eq!(copies.len(), 1);
            for copy in nonempty(copies, "the copy") {
                assert!(!copy.radiant);
                assert_eq!(copy.cost_mod, 0);
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn draws_four_then_shuffles_one_radiant_copy_that_costs_1_less() {
            let mut s = board(true, 4);
            s.play(ID, json!({}));
            assert_eq!(s.hand(P1).len(), 4);
            let copies = copies(&s);
            assert_eq!(copies.len(), 1);
            for copy in nonempty(copies, "the copy") {
                assert!(copy.radiant);
                assert_eq!(copy.cost_mod, -1);
            }
        }

        #[test]
        fn r4577002_the_copy_of_a_copy_costs_1_less_never_2() {
            let mut s = board(true, 4);
            s.play(ID, json!({}));
            s.end_turn().end_turn();
            let drawn = s.hand(P1).into_iter().find(|card| card.def_id == ID).expect("the copy is drawn");
            assert_eq!(cost_now(s.state(), &drawn), 2);
            s.state_mut().players[P1].mana.current = 10;
            s.play(&drawn, json!({}));
            let copies = copies(&s);
            assert_eq!(copies.len(), 1);
            for copy in nonempty(copies, "the copy's copy") {
                assert_eq!(copy.cost_mod, -1, "nothing compounds");
            }
        }
    }
}
