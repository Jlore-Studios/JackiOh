//! C+ #65.5 Mythic Grape (SPEC §8.7 row 65.5). (0) Spell, Fruit, Token (printed Mythic).
//!   Base:    "Replace your hand with random Mythic cards. They cost (0)."
//!   Radiant: "Replace your hand with random Radiant Mythic cards. They cost (0)."
//!   Engine:  "Each other card in your hand goes to your graveyard (a unit-token card ceases to exist,
//!            R11) and is replaced one for one by a random non-token Mythic of every set (R380), #76 Field
//!            of Dreams' reading; repeats allowed (R60); `costOverride` 0. An empty hand gets nothing.
//!            Tunes: none."
//!
//! The Grape is resolving, not in the hand, so "each other card" is the whole hand as it resolves.
//! `replaceHandWithRandom` (effects/fruit.ts) MOVES each card to the graveyard — it is not a discard,
//! so nothing that answers a discard sees it (BUILD M9) — and then makes as many random Mythics, each a
//! fresh card through Add to hand. The pool is the non-token cards of rarity Mythic, every set: a token's
//! printed rarity never feeds a pool (§5), so no Grape is ever one of them, this one included (R387).
//! An empty hand moves nothing and draws nothing from the rng (R129).

use jackioh_engine::effects::replace_hand_with_random;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-065-5";

/// §8.7: "They cost (0)": the declared number `setCost` (R386), less being better.
fn mythic_grape(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut args = json!({ "query": { "rarity": "Mythic" }, "costOverride": param(&*ctx, "setCost") });
            if radiant {
                args["radiant"] = json!(true);
            }
            vec![replace_hand_with_random(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = mythic_grape(false);

    let radiant = mythic_grape(true);

    CardScripts { base, radiant }
}

// C+ #65.5 Mythic Grape — SPEC §8.7 row 65.5, BUILD M9 Classic+ row C+ 65.5: "Each other card in your
// hand goes to your graveyard (not a discard) and is replaced by a random non-token Mythic card of any
// set that costs (0), never a Grape; an empty hand does nothing and draws nothing (R129); the old cards
// are public in the graveyard, the new ones hidden (R97); radiant the Mythics are Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MYTHIC_GRAPE: &str = "classicplus-065-5";
    const FILLER: &str = "core-005";
    const TIMMY: &str = "core-011";
    const RUSH: &str = "core-t-rush"; // a unit-token card

    use crate::js;

    fn has_event(events: &[GameEvent], kind: &str) -> bool {
        events.iter().any(|event| event.event_type().as_str() == kind)
    }

    #[test]
    fn is_a_0_fruit_spell_token_with_the_printed_rarity_mythic_whose_own_rarity_is_token() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.id, MYTHIC_GRAPE);
        assert_eq!(js(&def.cost), json!(0));
        assert_eq!(js(&def.rarity), json!("Token"));
        assert_eq!(js(&def.printed_rarity), json!("Mythic"));
        let CardScripts { base, radiant } = script();
        assert!(base.cry.is_some());
        assert!(radiant.cry.is_some());
    }

    #[test]
    fn r386_a_degrade_makes_the_mythics_cost_1_and_an_upgrade_finds_the_cost_at_its_floor_of_0() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [MYTHIC_GRAPE, FILLER, TIMMY] }, "p2": { "hand": [FILLER] } }));
        assert!(!crate::can_upgrade_number(&s, MYTHIC_GRAPE, "setCost"));
        assert_eq!(crate::degrade_number(&mut s, MYTHIC_GRAPE, "setCost"), 1);
        s.play(MYTHIC_GRAPE, json!({}));
        let hand = s.hand(P1);
        assert_eq!(hand.len(), 2);
        assert!(hand.iter().all(|card| card.cost_override == Some(1)));
    }

    mod base {
        use super::*;

        #[test]
        fn s6_3_each_other_hand_card_goes_to_your_graveyard_and_is_replaced_one_for_one_by_a_mythic_costing_0() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [MYTHIC_GRAPE, FILLER, TIMMY, FILLER] }, "p2": { "hand": [FILLER] } }));
            let old: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| card.def_id != MYTHIC_GRAPE).collect();
            s.play(MYTHIC_GRAPE, json!({}));

            assert!(old.iter().all(|card| s.card(&card.id).zone.z() == ZoneName::Graveyard));
            let hand = s.hand(P1);
            assert_eq!(hand.len(), 3);
            for card in &hand {
                let mythic = def_of(Some(s.state()), &card.def_id);
                assert_eq!(mythic.rarity, Rarity::Mythic);
                assert!(!mythic.token);
                assert_eq!(card.cost_override, Some(0));
                assert!(!card.radiant);
            }
            let view = js(&s.view(P1).you.hand);
            assert!(view.as_array().unwrap().iter().any(|card| card["cost"] == json!(0)));
        }

        #[test]
        fn build_it_is_not_a_discard_no_discarded_event_each_card_reported_by_enteredgraveyard() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [MYTHIC_GRAPE, FILLER, TIMMY] }, "p2": { "hand": [FILLER] } }));
            let old: Vec<String> = s
                .hand(P1)
                .into_iter()
                .filter(|card| card.def_id != MYTHIC_GRAPE)
                .map(|card| card.id)
                .collect();
            s.play(MYTHIC_GRAPE, json!({}));
            assert!(!has_event(s.last_events(), "discarded"));
            let landed: Vec<String> = s
                .last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::EnteredGraveyard { instance_id, .. } => Some(instance_id.clone()),
                    _ => None,
                })
                .collect();
            assert!(old.iter().all(|id| landed.contains(id)));
        }

        #[test]
        fn r380_r387_over_many_seeds_the_new_cards_are_non_token_mythics_of_every_set_never_a_grape() {
            crate::register_all();
            let mut seen: IndexSet<String> = IndexSet::new();
            for i in 0..40 {
                let mut s = scenario(json!({
                    "seed": format!("mythic-{i}"),
                    "p1": { "hand": [MYTHIC_GRAPE, FILLER, FILLER, FILLER] },
                    "p2": { "hand": [FILLER] }
                }));
                s.play(MYTHIC_GRAPE, json!({}));
                for card in s.hand(P1) {
                    seen.insert(card.def_id);
                }
            }
            assert!(!seen.iter().any(|id| id.starts_with("classicplus-065")));
            let sets: IndexSet<&str> = seen.iter().map(|id| id.split('-').next().unwrap_or("")).collect();
            assert!(sets.len() > 1);
        }

        #[test]
        fn r11_a_unit_token_card_in_the_hand_ceases_to_exist_and_is_still_replaced() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [MYTHIC_GRAPE, RUSH, FILLER] }, "p2": { "hand": [FILLER] } }));
            let token = s.card(RUSH).id.clone();
            s.play(MYTHIC_GRAPE, json!({}));
            assert!(!s.pile(P1, "graveyard").iter().any(|card| card.id == token));
            assert_eq!(s.hand(P1).len(), 2);
        }

        #[test]
        fn r129_an_empty_hand_does_nothing_and_draws_nothing_from_the_rng() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [MYTHIC_GRAPE] }, "p2": { "hand": [FILLER] } }));
            let cursor = s.state().rng_cursor;
            s.play(MYTHIC_GRAPE, json!({}));
            assert!(s.hand(P1).is_empty());
            assert_eq!(s.state().rng_cursor, cursor);
            assert!(!has_event(s.last_events(), "addedToHand"));
        }

        #[test]
        fn r97_the_old_cards_are_public_in_the_graveyard_the_new_ones_reach_the_opponent_under_the_sentinel() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [MYTHIC_GRAPE, TIMMY] }, "p2": { "hand": [FILLER] } }));
            let old = s.card(TIMMY).id.clone();
            s.play(MYTHIC_GRAPE, json!({}));
            let theirs = s.view(P2);
            let graveyard = js(&theirs.opponent.graveyard);
            assert!(
                graveyard
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|card| card["instanceId"] == json!(old) && card["defId"] == json!(TIMMY))
            );
            let added: Vec<&GameEvent> = theirs
                .events
                .iter()
                .filter(|event| event.event_type().as_str() == "addedToHand")
                .collect();
            assert_eq!(added.len(), 1);
            assert!(
                added
                    .iter()
                    .all(|event| matches!(event, GameEvent::AddedToHand { def_id, .. } if def_id == "hidden"))
            );
            let mythic = s.hand(P1).first().map(|card| card.id.clone()).unwrap_or_else(|| "?".to_string());
            assert!(!serde_json::to_string(&theirs).unwrap().contains(&mythic));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r74_the_mythics_are_radiant_and_cost_0() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": MYTHIC_GRAPE, "radiant": true }, FILLER, TIMMY] },
                "p2": { "hand": [FILLER] }
            }));
            s.play(MYTHIC_GRAPE, json!({}));
            let hand = s.hand(P1);
            assert_eq!(hand.len(), 2);
            assert!(hand.iter().all(|card| card.radiant && card.cost_override == Some(0)));
            assert!(hand.iter().all(|card| def_of(Some(s.state()), &card.def_id).rarity == Rarity::Mythic));
        }

        #[test]
        fn r129_an_empty_hand_does_nothing_on_the_radiant_face_either() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": MYTHIC_GRAPE, "radiant": true }] }, "p2": { "hand": [FILLER] } }));
            let cursor = s.state().rng_cursor;
            s.play(MYTHIC_GRAPE, json!({}));
            assert!(s.hand(P1).is_empty());
            assert_eq!(s.state().rng_cursor, cursor);
        }
    }
}
