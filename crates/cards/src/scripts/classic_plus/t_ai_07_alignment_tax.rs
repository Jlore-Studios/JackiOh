//! T-AI-7 Alignment Tax (SPEC §8.7 row T-AI-7, §7, B8). (1) Spell, AI, Token.
//!   Base:    "Your opponent's cards cost (1) more during their next turn."
//!   Radiant: "Your opponent's cards cost (2) more during their next turn."
//!   Engine:  "A player modifier on the opponent (Cost, §6.3, R65), #77 Professor Curvature's timing
//!            turned outward (R48, R363): it holds only during their next turn and expires at its
//!            cleanup; X-cost cards ignore it (R65). Tunes: none."
//!
//! E15's price rule on the opponent (`add_cost_rule`, R455) lasting "theirNextTurn": R48's `nextTurnOf`
//! expiry, live only once they are the active player on a later turn and gone at that turn's cleanup.
//! Two Taxes are two modifiers, and they add.

use jackioh_engine::effects::add_cost_rule;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-t-ai-07";

/// TS's `{ base, radiant } as const`: one number per face.
#[derive(Clone, Copy)]
struct ByFaceTax {
    base: i32,
    radiant: i32,
}

/// §8.7: "(1) more", Radiant "(2) more". An AI card declares no params (B8), so its numbers live here.
const TAX: ByFaceTax = ByFaceTax { base: 1, radiant: 2 };

fn tax(amount: i32) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![add_cost_rule(json_as(json!({
                "player": "enemy",
                "rule": { "amount": amount },
                "lasts": "theirNextTurn",
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: tax(TAX.base),
        radiant: tax(TAX.radiant),
    }
}

// T-AI-7 Alignment Tax — SPEC §8.7 row T-AI-7, BUILD M9 Classic+ row T-AI-7: "A player modifier on the
// opponent: their cards cost (1) more during their next turn only, inert for the rest of this turn,
// active through their next turn and gone at its cleanup (R48's timing turned outward, R363); X-cost
// cards are untouched (R65); two Taxes add; a price the tax lifts past their mana drops the play from
// `legalActions`; radiant (2) more".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const TAX: &str = "classicplus-t-ai-07";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt
    const FELINOR: &str = "core-043"; // (4) Spell
    const ADAPTIVE_UI: &str = "core-074"; // (X) Spell
    const STOCKPILE: &str = "core-005"; // (1) Spell

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialises")
    }

    /// TS's `{ ...base, ...extra }` on a side setup.
    fn spread(base: Value, extra: &Value) -> Value {
        let mut out = base;
        if let (Some(into), Some(from)) = (out.as_object_mut(), extra.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        out
    }

    /// p2's hand holds one card of each price; both decks are deep enough to cross several turns.
    fn setup(taxes: Vec<Value>, p2: Value) -> Scenario {
        crate::register_all();
        let mut hand = taxes;
        hand.push(json!(STOCKPILE));
        scenario(json!({
            "p1": { "hand": hand, "library": [VANILLA, VANILLA, VANILLA, VANILLA], "mana": 8 },
            "p2": spread(
                json!({ "hand": [VANILLA, MENACE, FELINOR, ADAPTIVE_UI], "library": [VANILLA, VANILLA, VANILLA, VANILLA] }),
                &p2,
            ),
        }))
    }

    fn one_tax() -> Scenario {
        setup(vec![json!(TAX)], json!({}))
    }

    /// What `player` reads as the price of their own hand card of `def_id` (the client prints this).
    fn price_of(s: &Scenario, player: PlayerId, def_id: &str) -> i64 {
        let hand = js(&s.view(player))["you"]["hand"].clone();
        let card = s.hand(player).into_iter().find(|card| card.def_id == def_id);
        let shown = card.and_then(|card| {
            hand.as_array()
                .and_then(|cards| cards.iter().find(|shown| shown["instanceId"] == card.id.as_str()).cloned())
        });
        match shown {
            Some(shown) => shown["cost"].as_i64().unwrap_or_default(),
            None => panic!("{player:?} holds no {def_id}"),
        }
    }

    fn playable(s: &Scenario, player: PlayerId, def_id: &str) -> bool {
        let ids: Vec<String> =
            s.hand(player).into_iter().filter(|card| card.def_id == def_id).map(|card| card.id).collect();
        legal_actions(s.state(), player).iter().any(|action| {
            let action = js(action);
            action["type"] == "play" && action["instanceId"].as_str().is_some_and(|id| ids.iter().any(|own| own == id))
        })
    }

    fn has_cost_rule(s: &Scenario) -> bool {
        s.state().players.p2.mods.iter().any(|modifier| js(modifier)["kind"] == "costRule")
    }

    mod t_ai_7_alignment_tax {
        use super::*;

        #[test]
        fn is_a_1_ai_spell_token_with_no_params() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(js(&def.cost), json!(1));
            assert_eq!(def.type_, CardType::Spell);
            assert_eq!(js(&def.tags), json!(["AI", "Token"]));
            assert!(def.params.is_none());
            let scripts = super::super::script();
            assert!(scripts.base.cry.is_some());
            assert!(scripts.radiant.cry.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn r48_inert_for_the_rest_of_this_turn_the_opponents_prices_dont_move_yet() {
                let mut s = one_tax();
                s.play(TAX, json!({}));
                assert_eq!(price_of(&s, P2, VANILLA), 1);
                assert_eq!(price_of(&s, P2, MENACE), 3);
                assert!(has_cost_rule(&s));
            }

            #[test]
            fn r48_r363_their_cards_cost_1_more_during_their_next_turn() {
                let mut s = one_tax();
                s.play(TAX, json!({})).end_turn();
                assert_eq!(s.state().active, P2);
                assert_eq!(price_of(&s, P2, VANILLA), 2);
                assert_eq!(price_of(&s, P2, MENACE), 4);
                assert_eq!(price_of(&s, P2, FELINOR), 5);
                s.play(VANILLA, json!({ "zone": 1 }));
                // 4 mana, the taxed Vanilla paid 2.
                s.expect_mana(P2, 2);
            }

            #[test]
            fn r48_gone_at_the_cleanup_of_their_next_turn_back_to_printed_prices_from_then_on() {
                let mut s = one_tax();
                s.play(TAX, json!({})).end_turn().end_turn();
                assert_eq!(s.state().active, P1);
                assert_eq!(price_of(&s, P2, VANILLA), 1);
                s.end_turn();
                assert_eq!(s.state().active, P2);
                assert_eq!(price_of(&s, P2, VANILLA), 1);
                assert!(!has_cost_rule(&s));
            }

            #[test]
            fn taxes_only_the_opponent_your_own_cards_keep_their_prices_on_both_turns() {
                let mut s = one_tax();
                s.play(TAX, json!({}));
                assert_eq!(price_of(&s, P1, STOCKPILE), 1);
                s.end_turn().end_turn();
                assert_eq!(price_of(&s, P1, STOCKPILE), 1);
            }

            #[test]
            fn r65_an_x_cost_card_is_untouched() {
                let mut s = one_tax();
                s.play(TAX, json!({})).end_turn();
                assert_eq!(price_of(&s, P2, ADAPTIVE_UI), 0);
                s.play(ADAPTIVE_UI, json!({ "x": 2, "targets": [{ "pick": "hero", "player": "p1" }] }));
                s.expect_mana(P2, 2);
            }

            #[test]
            fn two_taxes_add_2_more() {
                let mut s = setup(vec![json!(TAX), json!(TAX)], json!({}));
                s.play(TAX, json!({})).play(TAX, json!({})).end_turn();
                assert_eq!(price_of(&s, P2, VANILLA), 3);
                assert_eq!(price_of(&s, P2, MENACE), 5);
            }

            #[test]
            fn a_price_the_tax_lifts_past_their_mana_drops_the_play_from_legal_actions() {
                let mut s = one_tax();
                s.end_turn();
                assert!(playable(&s, P2, FELINOR));
                s.end_turn();
                s.play(TAX, json!({})).end_turn();
                assert_eq!(s.state().players.p2.mana.current, 4);
                assert!(!playable(&s, P2, FELINOR));
                s.expect_refused(|s| s.play(FELINOR, json!({})));
                assert!(playable(&s, P2, MENACE));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r48_their_cards_cost_2_more_during_their_next_turn_and_nothing_before_or_after() {
                let mut s = setup(vec![json!({ "def": TAX, "radiant": true })], json!({}));
                s.play(TAX, json!({}));
                assert_eq!(price_of(&s, P2, VANILLA), 1);
                s.end_turn();
                assert_eq!(price_of(&s, P2, VANILLA), 3);
                assert_eq!(price_of(&s, P2, MENACE), 5);
                assert_eq!(price_of(&s, P2, ADAPTIVE_UI), 0);
                assert!(!playable(&s, P2, MENACE));
                s.end_turn();
                assert_eq!(price_of(&s, P2, VANILLA), 1);
            }
        }
    }
}
