//! SPEC §8.13 IA #177 Predictive Text ((2) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Draw two cards. If your opponent has more cards in hand than you, draw one more."
//!   Radiant: "Draw three cards. If your opponent has more cards in hand than you, draw one more."
//!
//! Built by C8 (#4592). R4592000.

use jackioh_engine::effects::{ForEachCardArgs, draw, for_each_card};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-177";

fn predictive_text(radiant: bool) -> Script {
    let count = if radiant { 3 } else { 2 };
    Script {
        cry: Some(hook(move |_ctx| {
            vec![
                draw(json_as(json!({ "count": count }))),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(|each: &mut EffectContext<'_>| {
                        let opp = opponent_of(each.controller);
                        let opp_hand = each.state.players[opp].hand.len();
                        let my_hand = each.state.players[each.controller].hand.len();
                        if opp_hand > my_hand {
                            vec!["one-more".to_string()]
                        } else {
                            vec![]
                        }
                    }),
                    each: Arc::new(|_id: &str| draw(json_as(json!({ "count": 1 })))),
                }),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: predictive_text(false),
        radiant: predictive_text(true),
    }
}

// IA #177 Predictive Text — SPEC §8.13 row 177.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005";

    fn board(radiant: bool, opp_hand_count: usize) -> Scenario {
        let opp_hand: Vec<&str> = (0..opp_hand_count).map(|_| FILLER).collect();
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": crate::filler(5), "mana": 10 },
            "p2": { "hand": opp_hand },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn draws_two_normally() {
            // opponent has 2 cards, p1 plays predictive text -> hand becomes 0 -> draws 2 -> p1 has 2, opp has 2 (not strictly more) -> no extra draw
            let mut s = board(false, 2);
            s.play(ID, json!({}));
            assert_eq!(s.hand(P1).len(), 2);
        }

        #[test]
        fn draws_one_more_if_opponent_has_more_cards() {
            // opponent has 4 cards, p1 plays predictive text -> hand becomes 0 -> draws 2 -> p1 has 2, opp has 4 (opp has more) -> draws 1 more -> total 3
            let mut s = board(false, 4);
            s.play(ID, json!({}));
            assert_eq!(s.hand(P1).len(), 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn draws_three_normally() {
            let mut s = board(true, 3);
            s.play(ID, json!({}));
            assert_eq!(s.hand(P1).len(), 3);
        }

        #[test]
        fn draws_one_more_if_opponent_has_more() {
            let mut s = board(true, 5);
            s.play(ID, json!({}));
            assert_eq!(s.hand(P1).len(), 4);
        }
    }
}
