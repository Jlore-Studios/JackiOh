//! TG #19 Extra Rations (SPEC §8.11 row 19). (1) Field Spell, Rare. "Start of turn: Draw {draw|card|cards}."
//! (Radiant: "Draw {draw|card|cards}. They cost ({discount}) less.")
//! Core #6 Mana Well's `start_of_turn` hook, before the turn's draw (§2.2, R62, R68). The Radiant face
//! makes one M #80 Aluneth `draw_priced { costMod }` per card, so a burned card or one cast on draw is not
//! discounted (§2.4, R58); the `costMod` floors at (0) (R65) and lasts until the card leaves the hand (R766).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-019";

fn draws(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    vec![draw(json_as(json!({ "count": param(ctx, "draw") })))]
}

fn draws_discounted(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let discount = -param(ctx, "discount");
    (0..param(ctx, "draw"))
        .map(|_| draw_priced(json_as(json!({ "costMod": discount }))))
        .collect()
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            start_of_turn: Some(hook(draws)),
            ..Script::default()
        },
        radiant: Script {
            start_of_turn: Some(hook(draws_discounted)),
            ..Script::default()
        },
    }
}

// TG #19 Extra Rations — SPEC §8.11 row 19, BUILD M13 row TG 19. `start_turn()` includes the turn's draw.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::upgrade_number;

    const RATIONS: &str = super::ID;
    const VANILLA: &str = "core-008"; // (1)
    const BIGOT: &str = "core-002"; // (2)
    const RENO: &str = "core-053"; // (3)
    const FILLER: &str = "core-010";

    fn board(radiant: bool, p1: Value) -> Scenario {
        crate::register_all();
        let mut p1 = p1;
        p1["backrow"] = json!([{ "def": RATIONS, "radiant": radiant, "faceUp": true, "lane": 1 }]);
        scenario(json!({
            "seed": "extra-rations",
            "p1": p1,
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    fn drawn(s: &Scenario) -> usize {
        s.events().iter().filter(|event| matches!(event, GameEvent::Drawn { .. })).count()
    }

    mod base {
        use super::*;

        #[test]
        fn r62_at_your_start_of_turn_you_draw_1_beside_the_turns_draw() {
            let mut s = board(false, json!({ "hand": [FILLER], "library": [VANILLA, BIGOT, RENO] }));
            s.start_turn();

            assert_eq!(drawn(&s), 2);
            assert_eq!(s.hand(P1).len(), 3);
        }

        #[test]
        fn r68_nothing_at_the_opponents_start_of_turn() {
            let mut s = board(false, json!({ "hand": [FILLER], "library": [VANILLA, BIGOT] }));
            s.state_mut().active = P2;
            s.start_turn();

            assert_eq!(s.hand(P1).len(), 1);
        }

        #[test]
        fn r62_nothing_on_the_turn_it_is_played() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [RATIONS, FILLER], "library": [VANILLA, BIGOT], "mana": 10 },
            }));
            s.play(RATIONS, json!({}));

            assert_eq!(drawn(&s), 0);
            assert_eq!(s.hand(P1).len(), 1);
        }

        #[test]
        fn r4_a_full_hand_burns_the_card_it_draws() {
            let hand = vec![json!(FILLER); HAND_CAP as usize];
            let mut s = board(false, json!({ "hand": hand, "library": [VANILLA, BIGOT] }));
            s.start_turn();

            assert!(s.events().iter().any(|event| matches!(event, GameEvent::Burned { .. })));
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        }

        #[test]
        fn r3_an_empty_deck_takes_fatigue() {
            let mut s = board(false, json!({ "hand": [FILLER], "library": [] }));
            s.start_turn();

            assert!(s.events().iter().any(|event| matches!(event, GameEvent::Fatigue { .. })));
        }

        #[test]
        fn r386_draw_reads_through_param() {
            let mut s = board(false, json!({ "hand": [FILLER], "library": [VANILLA, BIGOT, RENO, FILLER] }));
            assert_eq!(upgrade_number(&mut s, RATIONS, "draw"), 2);
            s.start_turn();

            assert_eq!(drawn(&s), 3);
        }
    }

    mod radiant {
        use super::*;

        fn cost_in_hand(s: &Scenario, def: &str) -> i32 {
            let card = s.hand(P1).into_iter().find(|card| card.def_id == def).unwrap();
            cost_now(s.state(), &card)
        }

        #[test]
        fn r62_it_draws_2_before_the_turns_draw() {
            let mut s = board(true, json!({ "hand": [], "library": [BIGOT, RENO, VANILLA] }));
            s.start_turn();

            assert_eq!(drawn(&s), 3);
            // The first two in the deck were its draws, discounted; the third is the turn's own.
            assert_eq!(cost_in_hand(&s, BIGOT), 1);
            assert_eq!(cost_in_hand(&s, RENO), 2);
            assert_eq!(cost_in_hand(&s, VANILLA), 1);
        }

        #[test]
        fn r65_each_drawn_card_costs_1_less() {
            let mut s = board(true, json!({ "hand": [], "library": [BIGOT, RENO, FILLER] }));
            s.start_turn();

            let discounted: Vec<i32> = s
                .hand(P1)
                .iter()
                .filter(|card| card.def_id != FILLER)
                .map(|card| card.cost_mod)
                .collect();
            assert_eq!(discounted, vec![-1, -1]);
        }

        #[test]
        fn r65_a_0_card_stays_0_and_the_rest_of_the_hand_is_unchanged() {
            let mut s = board(true, json!({ "hand": [RENO], "library": [{ "def": VANILLA, "costOverride": 0 }, BIGOT, FILLER] }));
            s.start_turn();

            assert_eq!(cost_in_hand(&s, VANILLA), 0);
            assert_eq!(cost_in_hand(&s, RENO), 3);
        }

        #[test]
        fn r386_discount_reads_through_param() {
            let mut s = board(true, json!({ "hand": [], "library": [RENO, BIGOT, FILLER] }));
            assert_eq!(upgrade_number(&mut s, RATIONS, "discount"), 2);
            s.start_turn();

            assert_eq!(cost_in_hand(&s, RENO), 1);
        }
    }
}
