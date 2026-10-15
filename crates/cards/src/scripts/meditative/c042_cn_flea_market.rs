//! Meditative #42 CN Flea Market (SPEC §8.8 row 42, ME-MARKET, R1000–R1002, R831002). (1) Spell, CN,
//! Legendary ((0) before patch v0.3.9).
//!
//!   Base:    "Open a night market. You have {yuan} yuan to buy its cards."
//!   Radiant: "Open a night market. You have {yuan} yuan to buy its cards, and you may barter cards
//!             from your hand."
//!
//! The whole card is the engine's night market (`subsystems/night_market.rs`): the stall rolled once
//! as the Spell resolves, the yuan prices, the `market` prompt that reopens after each deal until the
//! caster leaves, and the barter the Radiant face opens (it reads the face that cast it). This file
//! hands it the shelves: three different random CN cards (never this one, R387), two Auspicious Rocks
//! (M #39.1) and one random AI generated card (the ten C+ #78 names, read off their AI tag). Each lot
//! is sold on its base face or, at its Radiant price, its Radiant face (patch v0.3.9, R831002). The
//! yuan is the declared `yuan` (50, Radiant 80), so a Nerf or Buff moves it (R386).
//!
//! Voice audio is owed: the `card-audio.json5` entry is written, and a person with macOS renders it
//! (`docs/ADDING_CARDS.md` §7).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-042";

/// The Auspicious Rock its stall always holds two of (M #39.1).
const ROCK: &str = "meditative-039-1";

fn market() -> Script {
    let made = subsystems::night_market_script(&[
        subsystems::MarketShelf::Pool {
            query: Box::new(json_as(json!({ "tags": ["CN"] }))),
            count: NIGHT_MARKET_CN_LOTS,
        },
        subsystems::MarketShelf::Named {
            def_id: ROCK.to_string(),
            count: NIGHT_MARKET_ROCK_LOTS,
        },
        subsystems::MarketShelf::Pool {
            query: Box::new(json_as(json!({ "tags": ["AI"], "token": true }))),
            count: NIGHT_MARKET_AI_LOTS,
        },
    ]);
    Script {
        cry: Some(made.cry),
        resume: made.resume,
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: market(),
        radiant: market(),
    }
}

// Meditative #42 CN Flea Market — SPEC §8.8 row 42, ME-MARKET, R97, R113, R387, R1000–R1002.
//
// BUILD M10 row M 42: "Rolls one stall as it resolves (3 different CN cards, 2 Auspicious Rocks and 1
// AI generated card; R1000), priced 10 yuan a printed mana plus 5 a rarity rank; a `market` prompt
// offers only the lots within the yuan left, none while the hand is full, and Leave; each lot is offered
// on its base face and on its Radiant face at twice the price (R831002); each buy adds the card on the
// face it was sold on at its printed cost and reopens the prompt (R113); … unspent yuan is gone on
// Leave; the opponent sees only an open prompt and hidden cards arriving; a timeout leaves (R1002);
// yuan reads through `param()`; radiant 80 yuan, and one barter per hand card that exiles it and adds
// its price, doubled if it is Radiant (R1001)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005";

    /// The market cast from p1's hand, its prompt open.
    fn opened(seed: &str, radiant: bool, hand: &[Value]) -> Scenario {
        crate::register_all();
        let mut cards = vec![json!({ "def": ID, "radiant": radiant })];
        cards.extend(hand.iter().cloned());
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": cards, "library": [FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }));
        s.play(ID, json!({}));
        s
    }

    fn pending(s: &Scenario) -> PendingChoice {
        s.state().pending.clone().expect("the market is open")
    }

    fn stall(s: &Scenario) -> Vec<String> {
        json_as(
            pending(s)
                .resume
                .data
                .get(subsystems::MARKET_STALL_KEY)
                .cloned()
                .expect("the stall"),
        )
    }

    fn cost_of(s: &Scenario, key: &str) -> Option<i32> {
        pending(s)
            .options
            .iter()
            .find(|option| option.key == key)
            .and_then(|option| option.cost)
    }

    mod base {
        use super::*;

        #[test]
        fn r1000_it_opens_a_market_of_50_yuan_or_80_on_the_radiant_face() {
            let s = opened("market-yuan", false, &[]);
            let open = pending(&s);
            assert_eq!(open.kind, PromptKind::Market);
            assert_eq!(open.player_id, P1);
            assert_eq!(open.budget, Some(50));
            let radiant = opened("market-yuan", true, &[]);
            assert_eq!(pending(&radiant).budget, Some(80));
        }

        #[test]
        fn r1000_the_stall_holds_three_cn_cards_two_auspicious_rocks_and_one_ai_generated_card() {
            for n in 0..10 {
                let s = opened(&format!("market-stall-{n}"), false, &[]);
                let stall = stall(&s);
                assert_eq!(stall.len(), 6, "{stall:?}");
                for id in &stall[..3] {
                    let def = crate::card_def(id);
                    assert!(def.tags.contains(&Tag::Cn) && !def.token, "{id}");
                    assert_ne!(id, ID, "never itself (R387)");
                }
                assert!(stall[0] != stall[1] && stall[1] != stall[2] && stall[0] != stall[2]);
                assert_eq!(stall[3..5], [ROCK.to_string(), ROCK.to_string()]);
                assert!(stall[5].starts_with("classicplus-t-ai-"), "{stall:?}");
            }
        }

        #[test]
        fn r1000_a_rock_costs_10_yuan_and_an_ai_card_prices_as_a_common() {
            let s = opened("market-prices", false, &[]);
            assert_eq!(cost_of(&s, &format!("mode:{ROCK}")), Some(10));
            let ai = stall(&s)[5].clone();
            let def = crate::card_def(&ai);
            let price = 10 * query_cost(&def) + 5;
            assert_eq!(
                subsystems::yuan_price(&def, false),
                price,
                "{ai} prints no rarity: Common"
            );
            if price <= 50 {
                assert_eq!(cost_of(&s, &format!("mode:{ai}")), Some(price));
            }
        }

        #[test]
        fn r1000_a_bought_rock_arrives_on_its_base_face_and_the_market_reopens() {
            let mut s = opened("market-buy", false, &[]);
            s.answer(json!([format!("mode:{ROCK}")]));
            let rock = s
                .hand(P1)
                .into_iter()
                .find(|card| card.def_id == ROCK)
                .expect("the Rock arrived");
            assert!(!rock.radiant, "on its base face");
            assert_eq!(pending(&s).budget, Some(40));
            assert_eq!(cost_of(&s, &format!("mode:{ROCK}")), Some(10), "the second Rock");
        }

        #[test]
        fn r1000_leave_loses_the_yuan_and_the_spell_is_played() {
            let mut s = opened("market-leave", false, &[]);
            s.answer(json!(["none"]));
            assert!(s.state().pending.is_none());
            s.expect_in_zone(ID, "graveyard");
            assert!(s.hand(P1).is_empty(), "nothing bought");
        }

        #[test]
        fn r1001_the_base_face_never_barters() {
            let s = opened("market-no-barter", false, &[json!(FILLER)]);
            assert!(
                !pending(&s)
                    .options
                    .iter()
                    .any(|option| option.key.starts_with("instance:"))
            );
        }

        #[test]
        fn r97_the_opponent_sees_only_that_a_prompt_is_open() {
            let mut s = opened("market-hidden", false, &[]);
            let stall = stall(&s);
            let theirs = s.view(P2);
            assert!(matches!(theirs.pending, Some(PendingView::Elsewhere(_))));
            let text = serde_json::to_string(&theirs).expect("a view serialises");
            assert!(
                !stall.iter().any(|id| text.contains(id.as_str())),
                "no lot reaches p2"
            );
            s.answer(json!([format!("mode:{ROCK}")]));
            let text = serde_json::to_string(&s.view(P2)).expect("a view serialises");
            assert!(!text.contains(ROCK), "the bought Rock arrives hidden");
        }

        #[test]
        fn r1000_every_lot_is_priced_10_yuan_a_printed_mana_plus_5_a_rarity_rank() {
            for n in 0..5 {
                let s = opened(&format!("market-price-sweep-{n}"), true, &[]);
                for lot in stall(&s) {
                    let def = crate::card_def(&lot);
                    let rarity = if def.token {
                        def.printed_rarity
                            .and_then(|printed| printed.as_str().parse::<Rarity>().ok())
                            .unwrap_or(Rarity::Common)
                    } else {
                        def.rarity
                    };
                    let rank = [
                        Rarity::Common,
                        Rarity::Rare,
                        Rarity::Epic,
                        Rarity::Legendary,
                        Rarity::Mythic,
                    ]
                    .iter()
                    .position(|listed| *listed == rarity)
                    .expect("a ranked rarity") as i32
                        + 1;
                    let price = 10 * query_cost(&def) + 5 * rank;
                    assert_eq!(subsystems::yuan_price(&def, false), price, "{lot}");
                    if price <= 80 {
                        assert_eq!(cost_of(&s, &format!("mode:{lot}")), Some(price), "{lot}");
                    }
                }
            }
        }

        #[test]
        fn r1000_only_the_lots_within_the_yuan_left_and_leave_are_offered() {
            for n in 0..10 {
                let s = opened(&format!("market-within-{n}"), false, &[]);
                let open = pending(&s);
                let budget = open.budget.expect("a budget");
                assert!(
                    open.options.iter().any(|option| option.key == "none"),
                    "Leave is offered"
                );
                for lot in stall(&s) {
                    let price = subsystems::yuan_price(&crate::card_def(&lot), false);
                    let offered = cost_of(&s, &format!("mode:{lot}")).is_some();
                    assert_eq!(offered, price <= budget, "{lot} at {price} with {budget} left");
                }
            }
        }

        #[test]
        fn r1000_none_while_the_hand_is_full() {
            // The Radiant face, so the market still opens on its barters: no lot is offered.
            let hand: Vec<Value> = vec![json!(FILLER); HAND_CAP as usize - 1];
            let mut s = opened("market-full", true, &hand);
            // The cast left HAND_CAP - 1 cards: a Rock fills the last slot.
            s.answer(json!([format!("mode:{ROCK}")]));
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            let keys: Vec<String> = pending(&s)
                .options
                .iter()
                .map(|option| option.key.clone())
                .collect();
            assert!(
                keys.iter()
                    .all(|key| key == "none" || key.starts_with("instance:")),
                "{keys:?}"
            );
            assert!(keys.contains(&"none".to_string()));
        }

        #[test]
        fn r831002_every_lot_is_also_sold_on_its_radiant_face_at_its_radiant_price_when_the_yuan_covers_it() {
            for n in 0..10 {
                let s = opened(&format!("market-radiant-lots-{n}"), false, &[]);
                let open = pending(&s);
                let budget = open.budget.expect("a budget");
                for lot in stall(&s) {
                    let price = subsystems::yuan_price(&crate::card_def(&lot), true);
                    assert_eq!(price, YUAN_RADIANT_MULTIPLIER * subsystems::yuan_price(&crate::card_def(&lot), false));
                    let key = format!("mode:{lot}{}", subsystems::MARKET_RADIANT_SUFFIX);
                    let option = open.options.iter().find(|option| option.key == key);
                    assert_eq!(option.is_some(), price <= budget, "{lot} at {price} with {budget} left");
                    if let Some(option) = option {
                        assert_eq!(option.cost, Some(price));
                        assert_eq!(option.radiant, Some(true));
                    }
                }
                // The base faces carry no Radiant mark.
                assert!(
                    open.options
                        .iter()
                        .filter(|option| option.key.starts_with("mode:") && !option.key.ends_with(":radiant"))
                        .all(|option| option.radiant.is_none())
                );
            }
        }

        #[test]
        fn r831002_a_rock_bought_radiant_arrives_radiant_for_20_yuan_and_the_second_is_still_sold_both_ways() {
            let mut s = opened("market-radiant-rock", false, &[]);
            assert_eq!(cost_of(&s, &format!("mode:{ROCK}:radiant")), Some(20));
            s.answer(json!([format!("mode:{ROCK}:radiant")]));
            let rock = s
                .hand(P1)
                .into_iter()
                .find(|card| card.def_id == ROCK)
                .expect("the Rock arrived");
            assert!(rock.radiant, "on its Radiant face");
            assert_eq!(rock.cost_override, None, "at its printed cost");
            assert_eq!(pending(&s).budget, Some(30));
            assert_eq!(cost_of(&s, &format!("mode:{ROCK}")), Some(10));
            assert_eq!(cost_of(&s, &format!("mode:{ROCK}:radiant")), Some(20));
        }

        #[test]
        fn r113_each_buy_adds_its_card_at_its_printed_cost_and_reopens_the_prompt() {
            let mut s = opened("market-buys", false, &[]);
            for budget in [40, 30] {
                s.answer(json!([format!("mode:{ROCK}")]));
                assert_eq!(pending(&s).kind, PromptKind::Market, "the prompt reopens");
                assert_eq!(pending(&s).budget, Some(budget));
            }
            let rocks: Vec<CardInstance> = s
                .hand(P1)
                .into_iter()
                .filter(|card| card.def_id == ROCK)
                .collect();
            assert_eq!(rocks.len(), 2);
            for rock in rocks {
                assert!(!rock.radiant);
                assert_eq!(rock.cost_override, None, "at its printed cost");
                assert_eq!(rock.cost_mod, 0);
            }
        }

        #[test]
        fn r1000_the_stall_never_rerolls_inside_one_market() {
            let mut s = opened("market-no-reroll", false, &[]);
            let first = stall(&s);
            s.answer(json!([format!("mode:{ROCK}")]));
            let after = stall(&s);
            // The Rock sold leaves the stall; every other lot is the one rolled at the start.
            let mut expected = first.clone();
            let sold = expected.iter().position(|id| id == ROCK).expect("a Rock");
            expected.remove(sold);
            assert_eq!(after, expected);
            // Every simulation of the same answer sees the same lots.
            let text = serde_json::to_string(s.state()).expect("the state serialises");
            let copy: GameState = serde_json::from_str(&text).expect("the state parses");
            assert_eq!(
                copy.pending
                    .expect("open")
                    .resume
                    .data
                    .get(subsystems::MARKET_STALL_KEY),
                pending(&s).resume.data.get(subsystems::MARKET_STALL_KEY)
            );
        }

        #[test]
        fn r1000_unspent_yuan_is_gone_on_leave() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "market-twice",
                "p1": { "hand": [ID, ID, FILLER], "library": [FILLER, FILLER] },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            }));
            let markets: Vec<String> = s
                .hand(P1)
                .iter()
                .filter(|card| card.def_id == ID)
                .map(|card| card.id.clone())
                .collect();
            s.play(&markets[0], json!({}));
            s.answer(json!([format!("mode:{ROCK}")]));
            assert_eq!(pending(&s).budget, Some(40));
            s.answer(json!(["none"]));
            assert!(s.state().pending.is_none());
            // The next market opens at its own 50, nothing carried over.
            s.play(&markets[1], json!({}));
            assert_eq!(pending(&s).budget, Some(50));
        }

        #[test]
        fn r1002_a_timeout_leaves() {
            let s = opened("market-timeout", false, &[]);
            let hand = s.hand(P1).len();
            let timed = reduce(
                s.state(),
                &json_as::<Action>(json!({ "type": "timeout", "playerId": "p1", "nonce": "market-t1" })),
            );
            assert_eq!(timed.error, None);
            assert!(timed.state.pending.is_none() || timed.state.active == P2);
            assert!(
                timed.state.players.p1.hand.len() <= hand + 1,
                "nothing was bought"
            );
            assert!(!timed.state.players.p1.hand.iter().any(|card| card.def_id == ROCK));
        }

        #[test]
        fn yuan_reads_through_param() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "market-param",
                "p1": { "hand": [ID, FILLER], "library": [FILLER, FILLER] },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            }));
            set_param(s.card_mut(ID), "yuan", 30);
            s.play(ID, json!({}));
            assert_eq!(pending(&s).budget, Some(30));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1001_the_radiant_face_barters_a_hand_card_for_its_price_doubled_if_radiant() {
            let mut s = opened(
                "market-barter",
                true,
                &[json!({ "def": FILLER, "radiant": true })],
            );
            let traded = s.hand(P1)[0].clone();
            let price = subsystems::yuan_price(&crate::card_def(FILLER), true);
            let key = format!("instance:{}", traded.id);
            assert_eq!(
                cost_of(&s, &key),
                Some(-price),
                "a barter's price is written negative"
            );
            s.answer(json!([key]));
            assert!(
                s.state().players[P1]
                    .exile
                    .iter()
                    .any(|card| card.id == traded.id)
            );
            assert_eq!(pending(&s).budget, Some(80 + price));
        }

        #[test]
        fn r1000_r831002_the_radiant_market_sells_both_faces_too() {
            let mut s = opened("market-radiant-buy", true, &[]);
            s.answer(json!([format!("mode:{ROCK}")]));
            assert_eq!(pending(&s).budget, Some(70));
            s.answer(json!([format!("mode:{ROCK}:radiant")]));
            assert_eq!(pending(&s).budget, Some(50));
            let rocks: Vec<bool> = s
                .hand(P1)
                .into_iter()
                .filter(|card| card.def_id == ROCK)
                .map(|card| card.radiant)
                .collect();
            assert_eq!(rocks, [false, true]);
        }

        #[test]
        fn r1001_one_barter_per_hand_card_a_base_card_at_its_plain_price() {
            let mut s = opened(
                "market-barters",
                true,
                &[json!(FILLER), json!(FILLER), json!("core-008")],
            );
            let hand = s.hand(P1);
            let barters: Vec<String> = pending(&s)
                .options
                .iter()
                .filter(|option| option.key.starts_with("instance:"))
                .map(|option| option.key.clone())
                .collect();
            assert_eq!(barters.len(), hand.len(), "one barter per hand card");
            for card in &hand {
                assert!(barters.contains(&format!("instance:{}", card.id)));
            }
            let traded = hand[2].clone();
            let price = subsystems::yuan_price(&crate::card_def(&traded.def_id), false);
            s.answer(json!([format!("instance:{}", traded.id)]));
            assert_eq!(pending(&s).budget, Some(80 + price), "a base card is not doubled");
            // The bartered card is gone: no second barter for it.
            assert!(
                !pending(&s)
                    .options
                    .iter()
                    .any(|option| option.key == format!("instance:{}", traded.id))
            );
        }
    }
}
