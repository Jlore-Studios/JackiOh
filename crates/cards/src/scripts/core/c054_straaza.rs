//! #54 Straaza (SPEC §8.3, §5.1, §6.3 Add to hand; R4, R60, R65, R78, R215, R275). Unit, cost 4,
//! 8/8 → 16/16.
//!   Base:    "Cry: add 2 random Units costing 3 or 4 to your hand; they cost 1"
//!   Radiant: "Cry: add 2 random Radiant Units costing 3 or 4 to your hand; they cost 0" — §8's
//!            cell "They are Radiant and cost 0" (R275's raise: the cards' face and their price).
//!            The cell restates only what the cards are and cost, so the count and the pool are
//!            the base clause's, unchanged (§8 Conventions).
//!
//! The Engine cell is "Non-token pool excluding #54; `costOverride`", which is §5.1's one query and
//! nothing else:
//!   * no tokens — automatic: "Random pools never include Token-tagged cards", so `{ type: "Unit" }`
//!     already leaves out the Sheep, Rush, Felinor and Spikey Pillow token units;
//!   * not #54 — `excludeDefId: ID`, §5.1's "never include the generating card's own
//!     definition", passed explicitly rather than trusted to the verb;
//!   * costing 3 or 4 — `costRange: { min: 3, max: 4 }`, read out of play per R65, so an X-cost card
//!     counts as 0 (never in this bracket) and an embiggen card at its base price (#59, base 2, also
//!     out). `query_cost` in engine/src/catalog.rs is the one number every bracket in the game reads.
//!
//! R60: "Cards generated from the catalog may repeat unless the card says 'different'". This row
//! does not say different, so both picks may land on the same def — two draws with replacement, not
//! a shuffle of the pool.
//! R65/R78: "they cost 1" (radiant: 0) is `cost_override` on each created instance, which is the first
//! term of the cost calculation and survives in every zone, so the discount is still there next turn.
//! §5.2: "Radiant" on the radiant face is the created instance's flag, set as it is made, so each
//! card arrives showing its Radiant face.
//! R4: the hand caps at 10 and an extra add is burned to the graveyard; the add-to-hand pipeline
//! owns that (engine/src/draw.rs), so this file never counts hand space. R215: the price is the
//! card's price in the hand, so the verb sets it only on a card that reaches one, and a card the
//! full hand burns reaches the graveyard at its printed price — Radiant still on the radiant face,
//! since that flag is set as the card is made (engine/src/effects/add_to_hand.rs).
//!
//! The verb is `add_random_from_catalog` (engine/src/effects/add_to_hand.rs): a hook may not roll the
//! dice itself — `ctx.rng.*` advances `rng_cursor`, which is state — so it picks `count` definitions
//! from `query(...)` with `ctx.rng` (repeats allowed, R60) and creates each one in the hand through the
//! same pipeline `add_to_hand` uses (§2.4, R4), with `cost_override` and `radiant` set on every card.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-054";

/// §8: two cards, both Units in the 3-4 bracket.
const COUNT: i32 = 2;

/// §5.1's pool: Units costing 3 or 4, no tokens (automatic), never Straaza herself. `ID` is
/// "core-054", the card's own catalog id, so the exclusion cannot drift from the card's own id (R387).
fn unit_pool() -> Value {
    json!({
        "type": "Unit",
        "costRange": { "min": 3, "max": 4 },
        "excludeDefId": ID,
    })
}

/// Base: "they cost 1".
const BASE_COST: i32 = 1;

/// Radiant: "they cost 0".
const RADIANT_COST: i32 = 0;

/// The two faces differ in what the generated cards cost and whether they are Radiant.
fn straaza(cost_override: i32, radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": unit_pool(),
                "count": COUNT,
                "costOverride": cost_override,
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: straaza(BASE_COST, false),
        radiant: straaza(RADIANT_COST, true),
    }
}

// #54 Straaza (SPEC §8.3, §5.1, §6.3 Add to hand; R4, R60, R65, R78, R215, R275).
// BUILD M4-T4 row 54: "2 random units of cost 3 or 4, no tokens, not #54, cost override 1; radiant
// Radiant units at 0".
//   Base:    "Cry: add 2 random Units costing 3 or 4 to your hand; they cost 1"
//   Radiant: "Cry: add 2 random Radiant Units costing 3 or 4 to your hand; they cost 0" — the same
//            pool, the cards Radiant as they are made (R275).
// R215: the price is the card's price in the hand, so a card the full hand burns reaches the
// graveyard without it — Radiant still, on the radiant face, since that flag is set as it is made.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    /// §5.1's pool for this card, which is what the script passes to the effect: the legal answers.
    fn straaza_pool() -> Vec<CardDef> {
        crate::query::pool("core-054", &json_as(json!({ "type": "Unit", "costRange": { "min": 3, "max": 4 } })))
            .into_iter()
            .map(owned_def)
            .collect()
    }

    /// A pool's definition as an owned copy, whether the pool lends it or hands it over.
    fn owned_def<D: std::borrow::Borrow<CardDef>>(def: D) -> CardDef {
        <D as std::borrow::Borrow<CardDef>>::borrow(&def).clone()
    }

    fn pool_ids() -> Vec<String> {
        straaza_pool().iter().map(|def| def.id.clone()).collect()
    }

    /// Nine cards to sit beside Straaza in a full hand (HAND_CAP 10), none of them Straaza.
    const FILLER: [&str; 9] = [
        "core-001", "core-002", "core-003", "core-004", "core-005", "core-006", "core-007", "core-008",
        "core-009",
    ];

    fn added(hand: Vec<CardInstance>) -> Vec<CardInstance> {
        hand.into_iter().filter(|card| card.def_id != "core-054").collect()
    }

    /// The one card a full hand burned (§2.4, R4), read back where it landed.
    fn burned_card(s: &mut Scenario) -> CardInstance {
        let burned: Vec<GameEvent> = s
            .events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Burned)
            .cloned()
            .collect();
        assert_eq!(burned.len(), 1);
        let Some(GameEvent::Burned { instance_id, .. }) = burned.first() else {
            panic!("expected a burned event");
        };
        let card = s.card(instance_id).clone();
        s.expect_in_zone(&card, "graveyard");
        let graveyard: Vec<String> = s.pile(P1, "graveyard").iter().map(|entry| entry.id.clone()).collect();
        assert_eq!(graveyard, vec![card.id.clone()]);
        card
    }

    /// A Radiant Straaza in hand, with nothing else to do (the §5.2 flag, seeded by the harness).
    fn radiant_straaza(seed: &str, hand: &[&str]) -> Scenario {
        let mut cards = vec![json!({ "def": "core-054", "radiant": true })];
        cards.extend(hand.iter().map(|id| json!(id)));
        scenario(json!({ "seed": seed, "p1": { "hand": cards } }))
    }

    mod n54_straaza_the_pool_itself_s5_1 {
        use super::*;

        #[test]
        fn s5_1_holds_only_units_costing_3_or_4_no_tokens_and_never_straaza_herself() {
            crate::register_all();
            let pool = straaza_pool();
            let ids = pool_ids();
            assert!(ids.len() > 2);
            assert!(!ids.contains(&"core-054".to_string()));
            for def in &pool {
                assert_eq!(def.type_, CardType::Unit);
                assert!(!def.token);
                assert!(!def.tags.contains(&Tag::Token));
                // R65: the cost is read out of play, so an X-cost card reads 0 and an embiggen card its base.
                assert!(query_cost(def) >= 3);
                assert!(query_cost(def) <= 4);
            }
            // The 1-cost unit tokens (Sheep, Rush, Felinor, Spikey Pillow) are out twice over.
            assert!(!ids.contains(&"core-t-rush".to_string()));
            assert!(!ids.contains(&"core-t-sheep".to_string()));
        }
    }

    mod n54_straaza_base {
        use super::*;

        #[test]
        fn adds_2_random_units_costing_3_or_4_to_your_hand() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": "straaza-base", "p1": { "hand": ["core-054"] } }));

            s.play("core-054", json!({}));

            let cards = added(s.hand(P1));
            assert_eq!(cards.len(), 2);
            for card in &cards {
                assert!(pool_ids().contains(&card.def_id));
                let def = crate::card_def(&card.def_id);
                assert_eq!(def.type_, CardType::Unit);
                assert!(query_cost(&def) >= 3);
                assert!(query_cost(&def) <= 4);
            }
            s.expect_in_zone("core-054", "field")
                .expect_events(json!(["cardPlayed", "addedToHand", "addedToHand"]));
        }

        #[test]
        fn r65_r78_they_cost_1_a_cost_override_on_each_created_instance_kept_in_every_zone() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": "straaza-base", "p1": { "hand": ["core-054"] } }));

            s.play("core-054", json!({}));

            for card in added(s.hand(P1)) {
                assert_eq!(card.cost_override, Some(1));
            }
        }

        #[test]
        fn s9_3_the_picks_come_from_the_seeded_rng_so_the_same_seed_gives_the_same_two_cards() {
            crate::register_all();
            let ids = |seed: &str| -> Vec<String> {
                let mut s = scenario(json!({ "seed": seed, "p1": { "hand": ["core-054"] } }));
                s.play("core-054", json!({}));
                added(s.hand(P1)).iter().map(|card| card.def_id.clone()).collect()
            };

            assert_eq!(ids("straaza-replay"), ids("straaza-replay"));
        }

        #[test]
        fn r60_the_two_picks_may_repeat_nothing_here_asks_for_different_cards() {
            crate::register_all();
            // Both cards come out of the same pool with replacement, so a pair of the same def is legal —
            // this test pins the reading, not a particular seed: whatever comes out, it is in the pool.
            let mut s = scenario(json!({ "seed": "straaza-repeat", "p1": { "hand": ["core-054"] } }));

            s.play("core-054", json!({}));

            let def_ids: Vec<String> = added(s.hand(P1)).iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(def_ids.len(), 2);
            let pool = pool_ids();
            assert!(def_ids.iter().all(|id| pool.contains(id)));
        }

        #[test]
        fn r4_r215_a_full_hand_burns_the_extra_nine_other_cards_leave_room_for_one() {
            crate::register_all();
            let mut hand = vec!["core-054"];
            hand.extend(FILLER);
            let mut s = scenario(json!({ "seed": "straaza-cap", "p1": { "hand": hand } }));
            let before: IndexSet<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();

            s.play("core-054", json!({}));

            // Straaza left the hand for the field, so nine cards had ten slots: one add lands, one burns.
            assert_eq!(s.hand(P1).len(), 10);
            assert_eq!(s.pile(P1, "graveyard").len(), 1);
            s.expect_events(json!(["addedToHand", "burned"]));

            // R215: "they cost 1" is the card's price in the hand, so the card that landed has it and the
            // burned one, which never reached the hand, lies in the graveyard without it.
            let kept: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| !before.contains(&card.id)).collect();
            assert_eq!(kept.len(), 1);
            assert_eq!(kept.first().and_then(|card| card.cost_override), Some(1));
            let burned = burned_card(&mut s);
            assert!(pool_ids().contains(&burned.def_id));
            assert!(!burned.radiant);
            assert!(burned.cost_override.is_none());
        }
    }

    mod n54_straaza_radiant {
        use super::*;

        #[test]
        fn r275_adds_2_radiant_units_costing_3_or_4_each_costing_0() {
            crate::register_all();
            let mut s = radiant_straaza("straaza-radiant", &[]);

            s.play("core-054", json!({}));

            let cards = added(s.hand(P1));
            assert_eq!(cards.len(), 2);
            for card in &cards {
                assert!(card.radiant);
                assert_eq!(card.cost_override, Some(0));
                assert!(pool_ids().contains(&card.def_id));
            }
            s.expect_events(json!(["cardPlayed", "addedToHand", "addedToHand"]));
        }

        #[test]
        fn s5_2_the_view_shows_each_one_s_radiant_face_at_0_mana() {
            crate::register_all();
            let mut s = radiant_straaza("straaza-radiant-view", &[]);

            s.play("core-054", json!({}));

            let HandView::Cards(hand) = s.view(P1).you.hand else {
                panic!("the viewer's own hand should be cards, not a count");
            };
            let shown: Vec<CardView> = hand.into_iter().filter(|card| card.def_id != "core-054").collect();
            assert_eq!(shown.len(), 2);
            for card in &shown {
                assert!(card.radiant);
                assert_eq!(card.cost, 0);
                // R243: a Unit in its owner's hand shows its Radiant face's printed stats.
                let face = crate::card_def(&card.def_id).radiant;
                assert_eq!(card.attack, face.attack);
            }
        }

        #[test]
        fn r275_the_pool_is_the_base_clause_s_still_2_still_units_costing_3_or_4_still_not_n54() {
            crate::register_all();
            let mut s = radiant_straaza("straaza-radiant", &[]);

            s.play("core-054", json!({}));

            let def_ids: Vec<String> = added(s.hand(P1)).iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(def_ids.len(), 2);
            assert!(!def_ids.contains(&"core-054".to_string()));
            for id in &def_ids {
                assert!(query_cost(&crate::card_def(id)) >= 3);
                assert!(query_cost(&crate::card_def(id)) <= 4);
            }
        }

        #[test]
        fn r65_r78_the_0_is_paid_a_radiant_card_it_made_is_played_for_nothing() {
            crate::register_all();
            let mut s = radiant_straaza("straaza-radiant-play", &[]);

            s.play("core-054", json!({}));
            let made = added(s.hand(P1))
                .into_iter()
                .next()
                .unwrap_or_else(|| panic!("Straaza should have added a card"));
            s.expect_mana(P1, 0);

            s.play(&made, json!({}));

            s.expect_mana(P1, 0).expect_in_zone(&made, "field");
            assert!(s.card(&made).radiant);
        }

        #[test]
        fn s5_2_the_radiant_body_is_a_16_16() {
            crate::register_all();
            let mut s = radiant_straaza("straaza-radiant", &[]);

            s.play("core-054", json!({}));

            s.expect_stats("core-054", json!({ "attack": 16, "health": 16, "maxHealth": 16 }));
        }

        #[test]
        fn r4_r215_a_full_hand_burns_the_extra_on_the_radiant_face_too_radiant_and_without_the_0() {
            crate::register_all();
            let mut s = radiant_straaza("straaza-radiant-cap", &FILLER);
            let before: IndexSet<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();

            s.play("core-054", json!({}));

            assert_eq!(s.hand(P1).len(), 10);
            assert_eq!(s.pile(P1, "graveyard").len(), 1);
            let kept: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| !before.contains(&card.id)).collect();
            assert_eq!(kept.len(), 1);
            assert_eq!(kept.first().map(|card| card.radiant), Some(true));
            assert_eq!(kept.first().and_then(|card| card.cost_override), Some(0));

            // R215: the 0 is the card's price in the hand, and the burned card never reached one, so it
            // lies in the graveyard at its printed price; the Radiant flag was set as it was made (§5.2,
            // R74) and R215 keeps it there.
            let burned = burned_card(&mut s);
            assert!(pool_ids().contains(&burned.def_id));
            assert!(burned.radiant);
            assert!(burned.cost_override.is_none());
        }
    }
}
