//! SPEC §8.14 FvH #2 M.O.T.H.E.R ((4) Unit, Human, Legendary, 9/7 → 18/14), a card of the held Felinors vs
//! Humans set (R5235000). Built by S09 (#5259).
//!   Base:    "Tribute 2. Cry: Reduce the Cost of a card in your hand by ({discount}), and of the cards outward
//!            from it on both sides by (1) less at each step." (3)
//!   Radiant: the same (5)
//!
//! Engine: Tribute 2 (`StaticFlags.tribute`, R101). The Cry (R1) asks its controller for one card of their
//! hand as it resolves (`choose_from_hand`, C #81 The Power to Thrive's prompt), the hand read then, with
//! M.O.T.H.E.R out of it; the answer's step takes `discount` off the chosen card and `discount − d` off each
//! card d places to its left and to its right, while that stays above 0 and the hand lasts (FV2 #2): 3, 2, 1
//! (Radiant 5, 4, 3, 2, 1), each a `costMod` on the card in hand (R78). No price falls below (0) and an X
//! cost is left alone (R65); an empty hand asks nothing and does nothing (R90, R831009). Tunes: discount
//! 3 ↑ (Radiant 5), read by its Cry (R1748000).

use jackioh_engine::effects::{choose_from_hand, set_cost_mod};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-002";

/// §6.3 "Tribute 2": two of your Units.
const TRIBUTE_COST: i32 = 2;

/// The resume step the hand prompt's answer comes back to (§10.6).
const REDUCE: &str = "reduce";

/// FV2 #2: each card's reduction, `discount` on the chosen card and one less a place outward, none at 0.
fn reductions(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(Selection::Instance { instance_id }) = ctx.targets.first().cloned() else {
        return vec![];
    };
    let discount = param(ctx, "discount");
    let hand = zone_cards(ctx.state, ctx.controller, OffFieldZone::Hand);
    let Some(chosen) = hand.iter().position(|card| card.id == instance_id) else {
        return vec![];
    };
    hand.iter()
        .enumerate()
        .filter_map(|(at, card)| {
            let places = (at as i32 - chosen as i32).abs();
            let amount = discount - places;
            (amount > 0).then(|| {
                set_cost_mod(json_as(json!({
                    "target": { "of": "instance", "instanceId": card.id },
                    "amount": -amount,
                    "inHandOnly": true,
                })))
            })
        })
        .collect()
}

fn mother() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|_ctx| {
            vec![choose_from_hand(json_as(json!({
                "step": REDUCE,
                "count": 1,
                "prompt": "M.O.T.H.E.R: choose a card in your hand",
            })))]
        })),
        resume: IndexMap::from([(REDUCE, hook(reductions))]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The Radiant face's (5) is its declared `discount`, which `param` reads off the running face.
    let base = mother();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #2 M.O.T.H.E.R — SPEC §8.14 row 2 (R101, R65, R78, R831009, R1748000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const BODY: &str = "stand-in (1) 1/1"; // a Tribute
    const CARD: &str = "stand-in (9) 1/1"; // a hand card priced above every reduction
    const FILLER: &str = "core-005"; // Stockpile: a card kept in p2's hand

    /// M.O.T.H.E.R first in p1's hand with `others` copies of `CARD` after it, two Units to tribute.
    fn board(radiant: bool, others: usize) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend((0..others).map(|_| json!(CARD)));
        scenario(json!({
            "seed": "mother",
            "p1": { "hand": hand, "field": [BODY, BODY], "library": crate::filler(3), "mana": 10 },
            "p2": { "hand": [FILLER], "library": crate::filler(3) },
        }))
    }

    /// Plays M.O.T.H.E.R, tributing both Units, by instance id.
    fn play(s: &mut Scenario) {
        let tributes = [s.unit_at(P1, 1).id, s.unit_at(P1, 2).id];
        s.play(ID, json!({ "tributes": tributes }));
    }

    /// Plays it and answers its prompt with the hand card at `at`; returns each hand card's `costMod`.
    fn reduced(radiant: bool, others: usize, at: usize) -> Vec<i32> {
        let mut s = board(radiant, others);
        play(&mut s);
        let pending = must(s.state().pending.clone(), "the hand prompt");
        assert_eq!(pending.player_id, P1);
        assert_eq!(pending.options.len(), others, "every card of the hand, M.O.T.H.E.R out of it");
        let chosen = s.hand(P1)[at].id.clone();
        s.answer(json!(chosen));
        assert!(s.state().pending.is_none());
        s.hand(P1).iter().map(|card| card.cost_mod).collect()
    }

    #[test]
    fn r101_it_asks_a_tribute_of_two() {
        let mut s = board(false, 1);
        let one = s.unit_at(P1, 1).id;
        s.expect_refused(|s| s.play(ID, json!({ "tributes": [one] })));
        play(&mut s);
        s.expect_in_zone(ID, "field");
    }

    #[test]
    fn r831009_an_empty_hand_asks_nothing_and_does_nothing() {
        for radiant in [false, true] {
            let mut s = board(radiant, 0);
            play(&mut s);
            assert!(s.state().pending.is_none(), "radiant {radiant}");
            s.expect_in_zone(ID, "field");
        }
    }

    mod base {
        use super::*;

        #[test]
        fn the_chosen_card_costs_3_less_and_the_cards_outward_2_then_1() {
            assert_eq!(reduced(false, 7, 3), vec![0, -1, -2, -3, -2, -1, 0]);
        }

        #[test]
        fn at_the_hands_edge_the_steps_run_one_way_only() {
            assert_eq!(reduced(false, 4, 0), vec![-3, -2, -1, 0]);
        }

        #[test]
        fn r65_the_cost_is_reduced_as_r65_reads_it() {
            let mut s = board(false, 3);
            play(&mut s);
            let chosen = s.hand(P1)[1].id.clone();
            s.answer(json!(chosen));
            let costs: Vec<i32> =
                s.hand(P1).iter().map(|card| effective_cost(s.state(), card, Default::default())).collect();
            assert_eq!(costs, vec![7, 6, 7]);
        }

        #[test]
        fn r1748000_the_discount_reads_through_param() {
            let mut s = board(false, 5);
            let discount = crate::upgrade_number(&mut s, ID, "discount");
            assert_eq!(discount, 4);
            play(&mut s);
            let chosen = s.hand(P1)[2].id.clone();
            s.answer(json!(chosen));
            let mods: Vec<i32> = s.hand(P1).iter().map(|card| card.cost_mod).collect();
            assert_eq!(mods, vec![-2, -3, -4, -3, -2]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_chosen_card_costs_5_less_and_the_cards_outward_4_3_2_1() {
            assert_eq!(reduced(true, 9, 4), vec![-1, -2, -3, -4, -5, -4, -3, -2, -1]);
        }

        #[test]
        fn at_the_hands_edge_the_steps_run_one_way_only() {
            assert_eq!(reduced(true, 6, 5), vec![0, -1, -2, -3, -4, -5]);
        }
    }
}
