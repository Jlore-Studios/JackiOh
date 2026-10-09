//! Meditative #42 CN Flea Market (SPEC §8.8 row 42, ME-MARKET, R1000–R1002). (0) Spell, CN, Legendary.
//!
//!   Base:    "Open a night market. You have {yuan} yuan to buy its cards."
//!   Radiant: "Open a night market. You have {yuan} yuan to buy its cards, and you may barter cards
//!             from your hand."
//!
//! The whole card is the engine's night market (`subsystems/night_market.rs`): the stall rolled once
//! as the Spell resolves, the yuan prices, the `market` prompt that reopens after each deal until the
//! caster leaves, and the barter the Radiant face opens (it reads the face that cast it). This file
//! hands it the shelves: three different random CN cards (never this one, R387), two Auspicious Rocks
//! (M #39.1) and one random AI generated card (the ten C+ #78 names, read off their AI tag). The yuan
//! is the declared `yuan` (50, Radiant 80), so a Nerf or Buff moves it (R386).
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
// offers only the lots within the yuan left, none while the hand is full, and Leave; each buy adds the
// card on its base face at its printed cost and reopens the prompt (R113); … unspent yuan is gone on
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
            assert_eq!(subsystems::yuan_price(&def, false), price, "{ai} prints no rarity: Common");
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
            assert!(!pending(&s).options.iter().any(|option| option.key.starts_with("instance:")));
        }

        #[test]
        fn r97_the_opponent_sees_only_that_a_prompt_is_open() {
            let mut s = opened("market-hidden", false, &[]);
            let stall = stall(&s);
            let theirs = s.view(P2);
            assert!(matches!(theirs.pending, Some(PendingView::Elsewhere(_))));
            let text = serde_json::to_string(&theirs).expect("a view serialises");
            assert!(!stall.iter().any(|id| text.contains(id.as_str())), "no lot reaches p2");
            s.answer(json!([format!("mode:{ROCK}")]));
            let text = serde_json::to_string(&s.view(P2)).expect("a view serialises");
            assert!(!text.contains(ROCK), "the bought Rock arrives hidden");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1001_the_radiant_face_barters_a_hand_card_for_its_price_doubled_if_radiant() {
            let mut s = opened("market-barter", true, &[json!({ "def": FILLER, "radiant": true })]);
            let traded = s.hand(P1)[0].clone();
            let price = subsystems::yuan_price(&crate::card_def(FILLER), true);
            let key = format!("instance:{}", traded.id);
            assert_eq!(cost_of(&s, &key), Some(-price), "a barter's price is written negative");
            s.answer(json!([key]));
            assert!(s.state().players[P1].exile.iter().any(|card| card.id == traded.id));
            assert_eq!(pending(&s).budget, Some(80 + price));
        }

        #[test]
        fn r1000_the_radiant_market_still_sells_base_face_cards() {
            let mut s = opened("market-radiant-buy", true, &[]);
            s.answer(json!([format!("mode:{ROCK}")]));
            let rock = s
                .hand(P1)
                .into_iter()
                .find(|card| card.def_id == ROCK)
                .expect("the Rock arrived");
            assert!(!rock.radiant, "a bought card is on its base face on either face");
            assert_eq!(pending(&s).budget, Some(70));
        }
    }
}
