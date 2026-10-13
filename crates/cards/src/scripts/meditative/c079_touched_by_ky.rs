//! M #79 Touched by KY (SPEC §8.8 row 79): (3) Unit, KY, Legendary, 4/4 → 8/8.
//!
//! Base:    "Cry: For the rest of the game, your hand size is {handSize}. Draw {draw|card|cards}."
//! Radiant: "Cry: For the rest of the game, your hand size is {handSize}. Draw {draw|card|cards}. Each
//!          becomes Radiant."
//! Engine:
//! - **The hand size** (ME-HANDCAP, R1143): `set_hand_cap` sets this player's hand cap to `handSize`
//!   for the rest of the game, whatever becomes of this card. A setting, not an increase: a second
//!   Touched by KY changes nothing and the latest setting wins. Every rule that reads the hand cap
//!   reads it (`hand_cap_of`), and both views show it. The designer's order is reversed so the cap is
//!   set before the draws, which then use the new room.
//! - **The draws:** `draw` separate §2.4 draws, each with its own cast-on-draw chain. The Radiant
//!   face's are `draw_priced` draws whose card, if the draw put it in the hand, becomes Radiant
//!   (§5.2); a card cast on draw, burned, or a fatigue or limited draw gets nothing (R596).
//! - **Voice line** (R204, `card-audio.json5`): "We must expand our minds."

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-079";

fn touched_by_ky(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects = vec![set_hand_cap(json_as(json!({ "cap": param(&*ctx, "handSize") })))];
            let draws = param(&*ctx, "draw");
            if radiant {
                effects.extend((0..draws.max(0)).map(|_| draw_priced(json_as(json!({ "radiant": true })))));
            } else {
                effects.push(draw(json_as(json!({ "count": draws }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: touched_by_ky(false),
        radiant: touched_by_ky(true),
    }
}

// M #79 Touched by KY — SPEC §8.8 row 79, BUILD M10 row M 79: "Cry (played or cast): your hand cap
// becomes 12 for the rest of the game, shown on both views, then you draw 4 (R1143): with 9 cards in
// hand none burns; the cap stays after it dies; a second Touched by KY leaves it 12; every draw, add,
// steal and fill reads the new cap; handSize and draw read through `param()`; radiant 8/8, and each
// card the draws put in your hand becomes Radiant".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-016";
    const FLOOD: &str = "core-017"; // Bounce every Unit.

    /// p1 holds Touched by KY (Radiant on `radiant`) and `others` fillers, with a deck of `deck`.
    fn holding(seed: &str, radiant: bool, others: usize, deck: usize) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend((0..others).map(|_| json!(FILLER)));
        crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": vec![FILLER; deck], "mana": 10 },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    fn burned(s: &Scenario) -> usize {
        s.events().iter().filter(|event| event.event_type() == GameEventType::Burned).count()
    }

    mod m79_touched_by_ky {
        use super::*;

        #[test]
        fn shape_and_hand_size_max_is_hand_cap_max() {
            crate::register_all();
            let def = crate::card_def(ID);
            assert_eq!(def.cost, CardCost::Fixed(3));
            assert_eq!(
                [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
                [Some(4), Some(4), Some(8), Some(8)]
            );
            let hand_size = def
                .params
                .iter()
                .flatten()
                .find(|param| param.key == "handSize")
                .expect("the handSize param");
            assert_eq!((hand_size.base, hand_size.radiant), (12, 12));
            assert_eq!(hand_size.max, Some(HAND_CAP_MAX));
        }

        mod base {
            use super::*;

            #[test]
            fn r1143_sets_12_then_draws_4_none_burn_from_8_others() {
                let mut s = holding("touched-draws", false, 8, 6);
                assert_eq!(s.state().players[P1].hand_cap, None);
                s.play(ID, json!({}));
                assert_eq!(s.state().players[P1].hand_cap, Some(12));
                assert_eq!(hand_cap_of(s.state(), P1), 12);
                // Eight left behind, four drawn: 12, and the cap was set first, so none burned.
                assert_eq!(s.hand(P1).len(), 12);
                assert_eq!(burned(&s), 0);
                assert_eq!(s.pile(P1, "library").len(), 2);
                assert!(s.hand(P1).iter().all(|card| !card.radiant));
            }

            #[test]
            fn r1143_outlives_the_card() {
                let mut s = crate::scenario(json!({
                    "seed": "touched-outlives",
                    "p1": { "hand": [ID, FLOOD, FILLER], "library": vec![FILLER; 6], "mana": 10 },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                s.play(ID, json!({}));
                let touched = s.card(ID).clone();
                s.play(FLOOD, json!({}));
                // Bounced off the field, the card is gone from it; the hand size is not.
                s.expect_in_zone(&touched, "hand");
                assert_eq!(s.state().players[P1].hand_cap, Some(12));
                s.end_turn();
                s.end_turn();
                assert_eq!(s.state().players[P1].hand_cap, Some(12));
            }

            #[test]
            fn r1143_a_second_leaves_it_12() {
                let mut s = crate::scenario(json!({
                    "seed": "touched-twice",
                    "p1": { "hand": [ID, ID], "library": vec![FILLER; 10], "mana": 10 },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                s.play(ID, json!({}));
                assert_eq!(s.hand(P1).len(), 5);
                s.play(ID, json!({}));
                // A setting, not an increase: still 12, and the second's draws fill to 8.
                assert_eq!(s.state().players[P1].hand_cap, Some(12));
                assert_eq!(s.hand(P1).len(), 8);
            }

            #[test]
            fn r1143_both_views_show_it() {
                let mut s = holding("touched-views", false, 1, 6);
                assert_eq!(s.view(P1).you.hand_cap, None);
                assert_eq!(s.view(P2).opponent.hand_cap, None);
                s.play(ID, json!({}));
                assert_eq!(s.view(P1).you.hand_cap, Some(12));
                assert_eq!(s.view(P2).opponent.hand_cap, Some(12));
                // p2's own hand size is unset.
                assert_eq!(s.view(P2).you.hand_cap, None);
                assert_eq!(s.view(P1).opponent.hand_cap, None);
            }

            #[test]
            fn hand_size_and_draw_read_through_param() {
                let mut s = holding("touched-params", false, 2, 6);
                set_param(s.card_mut(ID), "handSize", 13);
                set_param(s.card_mut(ID), "draw", 2);
                s.play(ID, json!({}));
                assert_eq!(s.state().players[P1].hand_cap, Some(13));
                assert_eq!(s.hand(P1).len(), 4);
            }

            #[test]
            fn r4_past_the_new_cap_a_draw_burns() {
                // Eleven behind it: the first draw fills the twelfth slot and the other three burn.
                let mut s = holding("touched-burn", false, 11, 6);
                s.play(ID, json!({}));
                assert_eq!(s.hand(P1).len(), 12);
                assert_eq!(burned(&s), 3);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn radiant_each_card_put_in_hand_is_radiant() {
                let mut s = holding("touched-radiant", true, 2, 6);
                s.play(ID, json!({}));
                assert_eq!(s.state().players[P1].hand_cap, Some(12));
                let hand = s.hand(P1);
                assert_eq!(hand.len(), 6);
                // The two kept are as they were; the four drawn are Radiant.
                assert!(hand.iter().take(2).all(|card| !card.radiant));
                assert!(hand.iter().skip(2).all(|card| card.radiant));
                let made: usize =
                    s.events().iter().filter(|event| event.event_type() == GameEventType::RadiantSet).count();
                assert_eq!(made, 4);
            }

            #[test]
            fn r596_a_burned_draw_gets_nothing() {
                // Twelve behind it: the hand is at its new cap, so every draw burns.
                let mut s = holding("touched-radiant-burn", true, 12, 6);
                s.play(ID, json!({}));
                assert_eq!(burned(&s), 4);
                assert!(s.pile(P1, "graveyard").iter().all(|card| !card.radiant));
                assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::RadiantSet));
            }

            #[test]
            fn r596_a_fatigue_draw_gets_nothing() {
                let mut s = holding("touched-radiant-fatigue", true, 1, 2);
                s.play(ID, json!({}));
                let hand = s.hand(P1);
                assert_eq!(hand.len(), 3);
                assert!(hand.iter().skip(1).all(|card| card.radiant));
                let made: usize =
                    s.events().iter().filter(|event| event.event_type() == GameEventType::RadiantSet).count();
                assert_eq!(made, 2);
                assert_eq!(s.state().players[P1].fatigue_count, 2);
            }
        }
    }
}
