//! C+ #42.1 KY's Gift (SPEC §8.7 row 42.1, §7, R16, R62, R380). (4) Field Spell, KY, Token (printed
//! Legendary); C+ #42 KY's Test's Hard reward.
//!   Start of turn: Gain {mana} mana. Your opponent discards {discards|card|cards}. Heal your hero {heal}. Add a
//!   random Book, a random KY card, a random Legendary card and a random (4) Cost card to your hand.
//!   They cost (0). Radiant: 2 mana, 2 discards, heal 10, and the four cards are Radiant.
//!
//! Its controller's start of turn (R62), in the order written. The mana is temporary (§2.3). The discard
//! is random from the opponent's hand (R682): fewer cards than asked taking what they have and none
//! taking nothing. The four cards come from non-token pools of every set (R380), each at `costOverride`
//! 0; the hand cap burns. The numbers are the declared `mana`, `discards` and `heal` (R386); one script
//! runs both faces.

use jackioh_engine::effects::{add_random_from_catalog, discard_random, gain_mana, heal};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-042-1";

/// A Book, a KY card, a Legendary card and a (4) Cost card, in the order the text names them.
fn pools() -> [Value; 4] {
    [json!({ "tags": ["Book"] }), json!({ "tags": ["KY"] }), json!({ "rarity": "Legendary" }), json!({ "cost": 4 })]
}

pub fn script() -> CardScripts {
    let base = Script {
        start_of_turn: Some(hook(|ctx| {
            let mut effects = vec![
                gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") }))),
                discard_random(json_as(json!({ "count": param(&*ctx, "discards"), "player": "enemy" }))),
                heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": param(&*ctx, "heal") }))),
            ];
            let radiant = ctx.radiant;
            // "They cost (0)": the declared number `setCost` (R386).
            let cost = param(&*ctx, "setCost");
            effects.extend(pools().into_iter().map(|query| {
                add_random_from_catalog(json_as(json!({ "query": query, "costOverride": cost, "radiant": radiant })))
            }));
            effects
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 2, 2 and 10 are its declared numbers, and `ctx.radiant` makes the cards Radiant.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C+ #42.1 KY's Gift — SPEC §8.7 row 42.1, R682, R62, R97, R177, R380, R386, BUILD M9 row C+ 42.1.
// p2 is active in each scenario, so its `endTurn()` starts p1's turn, where the Gift fires.
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GIFT: &str = "classicplus-042-1";
    const FILLER: &str = "core-005";
    const MENACE: &str = "core-019";
    const TIMMY: &str = "core-011";

    use crate::scenario;

    use crate::merged;

    /// p1's Gift in the backrow, p2 to move; `end_turn()` hands p1 its turn.
    fn gift(radiant: bool, p1: Value, p2: Value) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": merged(
                json!({ "backrow": [{ "def": GIFT, "radiant": radiant }], "library": [FILLER, FILLER], "hand": [FILLER] }),
                p1,
            ),
            "p2": merged(json!({ "hand": [MENACE, TIMMY], "library": [FILLER, FILLER] }), p2),
        }))
    }

    /// The cards the Gift added: p1's hand cards carrying `costOverride` 0.
    fn added(s: &Scenario) -> Vec<(String, bool)> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.cost_override == Some(0))
            .map(|card| (card.def_id, card.radiant))
            .collect()
    }

    fn expect_the_four(cards: &[(String, bool)], radiant_face: bool) {
        assert_eq!(cards.len(), 4);
        let defs: Vec<CardDef> = cards.iter().map(|(def_id, _)| crate::card_def(def_id)).collect();
        assert!(defs[0].tags.contains(&Tag::Book));
        assert!(defs[1].tags.contains(&Tag::Ky));
        assert_eq!(defs[2].rarity, Rarity::Legendary);
        assert_eq!(query_cost(&defs[3]), 4);
        for (def_id, radiant) in cards {
            assert!(!crate::card_def(def_id).token);
            assert_eq!(*radiant, radiant_face);
        }
    }

    fn view(s: &Scenario, seat: PlayerId) -> Value {
        serde_json::to_value(s.view(seat)).expect("a view is JSON")
    }

    fn view_events(s: &Scenario, seat: PlayerId) -> Vec<Value> {
        view(s, seat)["events"].as_array().cloned().unwrap_or_default()
    }

    fn events_json(s: &Scenario) -> Vec<Value> {
        s.events().iter().map(|event| serde_json::to_value(event).expect("an event is JSON")).collect()
    }

    #[test]
    fn is_a_start_of_turn_script_on_both_faces_its_numbers_the_declared_mana_discards_and_heal() {
        let def = crate::card_def(GIFT);
        assert_eq!(def.id, GIFT);
        assert!(def.token);
        assert_eq!(def.printed_rarity, Some(PrintedRarity::Legendary));
        let scripts = super::script();
        let (base, radiant) = (
            scripts.base.start_of_turn.as_ref().expect("a start-of-turn script"),
            scripts.radiant.start_of_turn.as_ref().expect("a start-of-turn script"),
        );
        assert!(Arc::ptr_eq(base, radiant));
        let params: Vec<Value> =
            def.params.unwrap_or_default().iter().map(|entry| json!([entry.key, entry.base, entry.radiant])).collect();
        assert_eq!(
            params,
            vec![json!(["mana", 1, 2]), json!(["discards", 1, 2]), json!(["heal", 5, 10]), json!(["setCost", 0, 0])]
        );
    }

    #[test]
    fn r386_a_degrade_makes_the_four_cost_1_and_an_upgrade_finds_the_cost_at_its_floor_of_0() {
        let mut s = gift(false, json!({}), json!({}));
        assert!(!crate::can_upgrade_number(&s, GIFT, "setCost"));
        assert_eq!(crate::degrade_number(&mut s, GIFT, "setCost"), 1);
        s.end_turn();
        let made = s.hand(P1).into_iter().filter(|card| card.cost_override == Some(1)).count();
        assert_eq!(made, 4);
    }

    mod base {
        use super::*;

        #[test]
        fn r62_at_the_start_of_your_turn_1_mana_above_the_cap_then_the_random_discard_lands_with_no_prompt() {
            let mut s = gift(false, json!({}), json!({}));
            s.end_turn();
            assert_eq!(s.state().active, P1);
            s.expect_mana(P1, MAX_MANA + 1);
            // R682: no prompt opens — the random discard landed and the rest followed.
            assert!(s.state().pending.is_none());
            assert_eq!(s.hand(P2).len(), 1);
            assert_eq!(s.pile(P2, "graveyard").len(), 1);
            s.expect_health(P1, HERO_HEALTH + 5);
            expect_the_four(&added(&s), false);
        }

        #[test]
        fn r682_the_opponent_discards_1_at_random_then_your_hero_heals_5_past_30_and_the_four_cards_arrive_at_0() {
            let mut s = gift(false, json!({}), json!({}));
            s.end_turn();
            let grave: Vec<String> = s.pile(P2, "graveyard").into_iter().map(|card| card.def_id).collect();
            assert_eq!(grave.len(), 1);
            assert!([MENACE, TIMMY].contains(&grave[0].as_str()));
            assert_eq!(s.hand(P2).len(), 1);
            s.expect_health(P1, HERO_HEALTH + 5);
            expect_the_four(&added(&s), false);
            s.expect_events(json!(["discarded", "healed", "addedToHand"]));
        }

        #[test]
        fn r380_the_four_come_from_every_sets_non_token_cards_and_over_seeds_reach_beyond_core() {
            let mut sets: Vec<String> = Vec::new();
            for n in 0..12 {
                let mut s = scenario(json!({
                    "seed": format!("kys-gift-sets-{n}"),
                    "active": "p2",
                    "p1": { "backrow": [GIFT], "library": [FILLER], "hand": [FILLER] },
                    "p2": { "hand": [], "library": [FILLER] },
                }));
                s.end_turn();
                for (def_id, _) in added(&s) {
                    sets.push(crate::card_def(&def_id).set.to_string());
                }
            }
            assert!(sets.iter().any(|set| set == "Classic" || set == "Classic+"));
        }

        #[test]
        fn r16_an_empty_opponent_hand_asks_nothing_and_the_rest_still_happens() {
            let mut s = gift(false, json!({}), json!({ "hand": [], "library": [FILLER] }));
            s.end_turn();
            assert!(s.state().pending.is_none());
            s.expect_health(P1, HERO_HEALTH + 5);
            expect_the_four(&added(&s), false);
        }

        #[test]
        fn r62_it_does_nothing_at_the_opponents_start_of_turn() {
            let mut s = gift(false, json!({}), json!({}));
            s.end_turn();
            let health = s.state().players.p1.hero.health;
            s.end_turn();
            assert_eq!(s.state().active, P2);
            assert!(s.state().pending.is_none());
            s.expect_health(P1, health);
        }

        #[test]
        fn r177_r682_no_prompt_opens_the_opponents_remaining_hand_stays_hidden_the_discard_is_public_the_four_cards_hidden_r97() {
            let mut s = gift(false, json!({}), json!({}));
            s.end_turn();
            assert!(s.state().pending.is_none());
            assert!(view(&s, P1)["pending"].is_null());
            let mine = serde_json::to_string(&s.view(P1)).expect("a view is JSON");
            for card in s.hand(P2) {
                assert!(!mine.contains(&format!("\"{}\"", card.id)));
                assert!(!mine.contains(&card.def_id));
            }
            let p2_events = view_events(&s, P2);
            let discarded = p2_events.iter().find(|event| event["type"] == "discarded");
            assert!(discarded.is_some_and(|event| event["defId"] == MENACE || event["defId"] == TIMMY));
            let p1_events = view_events(&s, P1);
            let p1_discard = p1_events.iter().find(|event| event["type"] == "discarded");
            assert!(p1_discard.is_some_and(|event| event["defId"] == MENACE || event["defId"] == TIMMY));
            // The four, and the turn's draw: every card reaching p1's hand is the sentinel to p2.
            let adds: Vec<&Value> =
                p2_events.iter().filter(|event| event["type"] == "addedToHand" && event["player"] == "p1").collect();
            assert!(adds.len() >= 4);
            for event in adds {
                assert_eq!(event["defId"], "hidden");
            }
        }

        #[test]
        fn r682_the_random_discard_comes_from_the_match_rng_the_same_game_discards_the_same_card() {
            let mut first = gift(false, json!({}), json!({}));
            first.end_turn();
            let mut second = gift(false, json!({}), json!({}));
            second.end_turn();
            let ids = |s: &Scenario| -> Vec<String> {
                events_json(s)
                    .iter()
                    .filter(|event| event["type"] == "discarded")
                    .filter_map(|event| event["instanceId"].as_str().map(str::to_string))
                    .collect()
            };
            assert_eq!(ids(&first), ids(&second));
        }

        #[test]
        fn s2_4_a_full_hand_burns_what_does_not_fit() {
            let hand: Vec<&str> = (0..8).map(|_| FILLER).collect();
            let mut s = gift(false, json!({ "hand": hand, "library": [FILLER] }), json!({}));
            s.end_turn();
            // Start-of-turn triggers come before the draw (§2): two of the four fit beside the 8, two are
            // burned, and then the turn's draw is burned too.
            assert_eq!(s.hand(P1).len(), 10);
            assert_eq!(added(&s).len(), 2);
            assert_eq!(s.pile(P1, "library").len(), 0);
            assert_eq!(events_json(&s).iter().filter(|event| event["type"] == "burned").count(), 3);
        }

        #[test]
        fn r386_its_numbers_read_through_param_an_upgrade_of_each() {
            let mut s = gift(false, json!({}), json!({}));
            step_param(s.card_mut(GIFT), "mana", 1);
            step_param(s.card_mut(GIFT), "discards", 1);
            step_param(s.card_mut(GIFT), "heal", 1);
            s.end_turn();
            s.expect_mana(P1, MAX_MANA + 2);
            assert!(s.state().pending.is_none());
            assert_eq!(s.pile(P2, "graveyard").len(), 2);
            assert_eq!(s.hand(P2).len(), 0);
            s.expect_health(P1, HERO_HEALTH + 6);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn two_mana_two_random_discards_heal_10_and_the_four_cards_are_radiant() {
            let mut s = gift(true, json!({}), json!({}));
            s.end_turn();
            s.expect_mana(P1, MAX_MANA + 2);
            assert!(s.state().pending.is_none());
            assert_eq!(s.hand(P2).len(), 0);
            assert_eq!(s.pile(P2, "graveyard").len(), 2);
            s.expect_health(P1, HERO_HEALTH + 10);
            expect_the_four(&added(&s), true);
        }

        #[test]
        fn r682_fewer_cards_than_asked_the_opponent_discards_all_they_have() {
            let mut s = gift(true, json!({}), json!({ "hand": [TIMMY], "library": [FILLER] }));
            s.end_turn();
            assert!(s.state().pending.is_none());
            assert_eq!(s.hand(P2).len(), 0);
            let grave: Vec<String> = s.pile(P2, "graveyard").into_iter().map(|card| card.def_id).collect();
            assert_eq!(grave, vec![TIMMY]);
            s.expect_health(P1, HERO_HEALTH + 10);
        }

        #[test]
        fn r386_a_degrade_steps_the_radiant_heal_from_10_to_9() {
            let mut s = gift(true, json!({}), json!({ "hand": [], "library": [FILLER] }));
            step_param(s.card_mut(GIFT), "heal", -1);
            s.end_turn();
            s.expect_health(P1, HERO_HEALTH + 9);
        }
    }
}
