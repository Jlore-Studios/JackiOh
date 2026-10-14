//! T-AI-1 Helpful Assistant (SPEC §8.7 row T-AI-1, §10.8, R60, R177, R218). (1) Unit, AI, Token; 1/3
//! Taunt → 2/6 Taunt, Divine Shield. Made by C+ #43 AI Slop and C+ #78 Claude's Datacenter.
//!   Cry: Discover a card from your deck. Radiant: it costs (1) less.
//!
//! A Discover whose pool is your own deck, as Core #51 reveals library cards: up to 3 different cards,
//! shown to you only (§10.8); the chosen card moves from deck to hand, which is not a draw (no Cast on
//! draw, no fatigue, no draw limit), and the hand cap burns it; an empty deck offers nothing. The
//! Radiant discount is a `costMod`, so it stacks with every other (R65). AI cards declare no params.

use jackioh_engine::effects::{add_to_hand, discover_from_library};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-t-ai-01";

/// Radiant: "It costs (1) less."
const DISCOUNT: i32 = 1;

fn assistant(cost_mod: i32) -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![discover_from_library(json_as(json!({
                "step": "picked",
                "prompt": "Discover a card from your deck",
            })))]
        })),
        resume: IndexMap::from([(
            "picked",
            hook(move |_ctx| {
                let mut args = json!({ "instance": { "of": "chosen" } });
                if cost_mod != 0 {
                    args["costMod"] = json!(cost_mod);
                }
                vec![add_to_hand(json_as(args))]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: assistant(0),
        radiant: assistant(-DISCOUNT),
    }
}

// T-AI-1 Helpful Assistant — SPEC §8.7 row T-AI-1, §10.8, R60, R97, R177, BUILD M9 row T-AI-1.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const ASSISTANT: &str = "classicplus-t-ai-01";
    const FILLER: &str = "core-005";
    const HINDER: &str = "core-021"; // Cast on draw
    const MENACE: &str = "core-019";
    const TIMMY: &str = "core-011";
    const RENO: &str = "core-053";
    const DECK: [&str; 5] = [MENACE, TIMMY, RENO, HINDER, FILLER];

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    use crate::js;

    fn assistant(radiant: bool, library: Option<&[&str]>, hand: Option<&[&str]>) -> Scenario {
        crate::register_all();
        let mut cards = vec![json!({ "def": ASSISTANT, "radiant": radiant })];
        cards.extend(hand.unwrap_or(&[FILLER]).iter().map(|id| json!(id)));
        scenario(json!({
            "p1": { "hand": cards, "library": library.unwrap_or(&DECK) },
            "p2": { "hand": [FILLER] },
        }))
    }

    /// The instance a prompt option's selection names, or "" (TS `pick === "instance" ? instanceId : ""`).
    fn instance_of(selection: &Value) -> String {
        if selection["pick"] == "instance" {
            selection["instanceId"].as_str().unwrap_or_default().to_string()
        } else {
            String::new()
        }
    }

    /// The library cards the open Discover offers, by def id.
    fn offered(s: &Scenario) -> Vec<String> {
        s.state()
            .pending
            .as_ref()
            .map(|pending| {
                pending
                    .options
                    .iter()
                    .map(|option| {
                        let id = instance_of(&js(&option.selection));
                        if id.is_empty() { String::new() } else { s.card(id.as_str()).def_id.clone() }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The instance the open prompt's option `at` names, or "".
    fn option_instance(s: &Scenario, at: usize) -> String {
        s.state()
            .pending
            .as_ref()
            .and_then(|pending| pending.options.get(at))
            .map(|option| instance_of(&js(&option.selection)))
            .unwrap_or_default()
    }

    fn kinds(keywords: &[Keyword]) -> Vec<String> {
        keywords
            .iter()
            .map(|keyword| js(keyword)["kind"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    mod t_ai_1_helpful_assistant {
        use super::*;

        #[test]
        fn is_a_1_3_taunt_radiant_2_6_taunt_and_divine_shield() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(def.id, ASSISTANT);
            let mut s = scenario(json!({ "p1": { "field": [ASSISTANT, { "def": ASSISTANT, "radiant": true }] } }));
            let first = s.unit(P1, 1).map(|unit| unit.id).unwrap_or_else(|| ASSISTANT.to_string());
            let second = s.unit(P1, 2).map(|unit| unit.id).unwrap_or_else(|| ASSISTANT.to_string());
            s.expect_stats(first.as_str(), json!({ "attack": 1, "health": 3 }));
            assert_eq!(kinds(&s.stats(first.as_str()).keywords), vec!["Taunt"]);
            s.expect_stats(second.as_str(), json!({ "attack": 2, "health": 6 }));
            assert_eq!(kinds(&s.stats(second.as_str()).keywords), vec!["Taunt", "Divine Shield"]);
        }

        mod base {
            use super::*;

            #[test]
            fn r60_cry_a_discover_of_3_different_cards_of_your_own_deck_shown_to_you_the_chosen_one_moves_to_your_hand() {
                let mut s = assistant(false, None, None);
                s.play(ASSISTANT, json!({}));
                assert_eq!(s.state().pending.as_ref().map(|pending| js(&pending.kind)), Some(json!("discover")));
                assert_eq!(s.state().pending.as_ref().map(|pending| pending.player_id), Some(P1));
                assert_eq!(offered(&s).into_iter().collect::<IndexSet<_>>().len(), 3);
                for id in offered(&s) {
                    assert!(DECK.contains(&id.as_str()));
                }
                let chosen = option_instance(&s, 0);
                let rest: Vec<String> = s
                    .pile(P1, "library")
                    .iter()
                    .filter(|card| card.id != chosen)
                    .map(|card| card.id.clone())
                    .collect();
                s.answer(json!(chosen));
                s.expect_in_zone(chosen.as_str(), "hand");
                // The rest stay put, in the order they lay.
                let library: Vec<String> = s.pile(P1, "library").iter().map(|card| card.id.clone()).collect();
                assert_eq!(library, rest);
                assert_eq!(s.card(chosen.as_str()).cost_mod, 0);
            }

            #[test]
            fn all_of_a_deck_shorter_than_three_are_offered_an_empty_deck_asks_nothing() {
                let mut short = assistant(false, Some(&[MENACE, TIMMY]), None);
                short.play(ASSISTANT, json!({}));
                let mut got = offered(&short);
                got.sort();
                let mut want = vec![MENACE.to_string(), TIMMY.to_string()];
                want.sort();
                assert_eq!(got, want);
                let mut empty = assistant(false, Some(&[]), None);
                empty.play(ASSISTANT, json!({}));
                assert!(empty.state().pending.is_none());
                assert!(!empty.events().iter().any(|event| event.event_type() == GameEventType::Fatigue));
            }

            #[test]
            fn r177_the_options_come_in_a_shuffled_order_never_the_decks() {
                crate::register_all();
                let mut orders: IndexSet<String> = IndexSet::new();
                for n in 0..12 {
                    let seed = format!("assistant-order-{n}");
                    let mut s = scenario(json!({
                        "seed": seed,
                        "p1": { "hand": [ASSISTANT], "library": [MENACE, TIMMY, RENO] },
                    }));
                    s.play(ASSISTANT, json!({}));
                    orders.insert(offered(&s).join(","));
                }
                assert!(orders.len() > 1);
            }

            #[test]
            fn s2_4_it_is_not_a_draw_a_cast_on_draw_card_reaches_the_hand_uncast_and_no_draw_is_counted() {
                let mut s = assistant(false, Some(&[HINDER]), None);
                s.play(ASSISTANT, json!({}));
                let draws = draws_this_turn(s.state(), P1);
                let hinder = s.card(HINDER).id.clone();
                s.answer(json!(hinder));
                assert_eq!(draws_this_turn(s.state(), P1), draws);
                s.expect_in_zone(HINDER, "hand");
                assert!(!s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::CardPlayed { def_id, .. } if def_id == HINDER
                )));
            }

            #[test]
            fn s2_4_a_full_hand_burns_the_chosen_card() {
                let full = [FILLER; 10];
                let mut s = assistant(false, Some(&[MENACE]), Some(&full));
                s.play(ASSISTANT, json!({}));
                let menace = s.card(MENACE).id.clone();
                s.answer(json!(menace));
                s.expect_in_zone(MENACE, "graveyard");
                s.expect_events(json!(["burned"]));
            }

            #[test]
            fn s10_8_r97_r177_the_opponent_sees_only_that_a_prompt_is_open_then_a_card_reaching_your_hand_under_the_sentinel() {
                let mut s = assistant(false, None, None);
                s.play(ASSISTANT, json!({}));
                assert_eq!(js(&s.view(P2))["pending"], json!({ "forYou": false, "pendingFor": "p1" }));
                let seen = serde_json::to_string(&s.view(P2)).expect("serialises");
                for id in [MENACE, TIMMY, RENO, HINDER] {
                    assert!(!seen.contains(id));
                }
                let pick = option_instance(&s, 0);
                s.answer(json!(pick));
                let added: Vec<Value> = js(&s.view(P2))["events"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|event| event["type"] == "addedToHand" && event["player"] == "p1")
                    .collect();
                assert_eq!(added.len(), 1);
                assert_eq!(added[0]["defId"], json!("hidden"));
            }

            #[test]
            fn s9_3_paused_on_the_discover_the_state_survives_json_and_answers_to_the_same_hash() {
                let mut s = assistant(false, None, None);
                s.play(ASSISTANT, json!({}));
                let thawed: GameState =
                    serde_json::from_str(&serde_json::to_string(s.state()).expect("serialises")).expect("parses");
                let pending = s.state().pending.clone();
                let pick = pending.as_ref().and_then(|pending| pending.options.get(1)).map(|option| js(&option.selection));
                let action: Action = json_as(json!({
                    "type": "answer",
                    "playerId": "p1",
                    "choiceId": pending.as_ref().map(|pending| pending.id.clone()).unwrap_or_default(),
                    "selection": [pick],
                    "nonce": "assistant-json",
                }));
                let live = reduce(s.state(), &action);
                let frozen = reduce(&thawed, &action);
                assert!(live.error.is_none());
                assert_eq!(hash_state(&frozen.state), hash_state(&live.state));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_chosen_card_costs_1_less_a_cost_mod_that_stacks() {
                let mut s = assistant(true, Some(&[MENACE]), None);
                s.play(ASSISTANT, json!({}));
                let before = effective_cost(s.state(), s.card(MENACE), Default::default());
                let menace = s.card(MENACE).id.clone();
                s.answer(json!(menace));
                assert_eq!(s.card(MENACE).cost_mod, -1);
                assert_eq!(effective_cost(s.state(), s.card(MENACE), Default::default()), before - 1);
            }

            #[test]
            fn an_empty_deck_asks_nothing() {
                let mut s = assistant(true, Some(&[]), None);
                s.play(ASSISTANT, json!({}));
                assert!(s.state().pending.is_none());
            }
        }
    }
}
