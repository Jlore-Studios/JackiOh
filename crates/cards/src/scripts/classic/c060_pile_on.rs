//! C #60 Pile On (SPEC §8.6 row 60; §6.2 Replacement, §6.3 Recruit, §2.3; R1, R11, R33, R80, R275). Spell,
//! cost 5, Epic.
//!   Base:    "Recruit every permanent in your deck.\nIf this would go to your graveyard, put it on the
//!            bottom of your deck instead."
//!   Radiant: "Recruit every permanent in your deck." (the clause dropped on purpose, R275)
//!   Engine:  `recruitAll`: the deck top down, each permanent into its row while that row has an open
//!            zone, Traps face-down (R33), no Cry (R1); Spells and unit-token cards stay (R11). The base
//!            clause is a replacement on this card's own move to its graveyard — resolving, discarded,
//!            burned — to the bottom of the deck; a full deck turns it away (R80).

use jackioh_engine::effects::recruit_all;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-060";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| vec![recruit_all(Default::default())])),
        replacements: vec![ReplacementDef {
            id: "pile-on".to_string(),
            on: ReplacementMoment::ToGraveyard,
            where_: Some(ReplacementWhere::SelfCard),
            when: None,
            instead: ReplacementInstead {
                to: Some(InsteadTo::BottomOfLibrary),
                ..ReplacementInstead::default()
            },
            then: None,
            by: None,
        }],
        ..Script::default()
    };

    // The Radiant face drops the return (SPEC §8.6 row 60): the same Recruit, and this Spell goes to the
    // graveyard as any Spell does.
    let radiant = Script {
        cry: base.cry.clone(),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// The deck holds Core cards with their own tests; Bigot's Cry would destroy an enemy Unit, which a
// Recruit never fires (R1); a Rush Token card stands for a unit-token card a deck may hold (R11).
#[cfg(test)]
mod tests {
    use super::{script, ID};
    use jackioh_engine::testkit::*;
    use std::sync::Arc;

    const PILE_ON: &str = "classic-060";
    const VANILLA: &str = "core-008"; // Unit 4/4
    const GARY: &str = "core-004"; // Unit 1/1, Cry: flip 5 coins
    const BIGOT: &str = "core-002"; // Unit 6/1, Cry: destroy target enemy non-Human Unit
    const MANA_WELL: &str = "core-006"; // Field Spell
    const SHEEPISH: &str = "core-041"; // Trap
    const BREAD: &str = "core-018"; // Field Trap
    const STOCKPILE: &str = "core-005"; // Spell: draw 2, heal 2
    const LUNAR: &str = "core-035"; // Spell
    const ZAO_GAO: &str = "core-080"; // Spell: discard 2 random cards; summon 2 Rush Tokens
    const RUSH_TOKEN: &str = "core-t-rush";
    const FILLER: &str = "core-005";

    use crate::js;

    fn library_ids(s: &Scenario) -> Vec<String> {
        s.pile(PlayerId::P1, "library").iter().map(|card| card.def_id.clone()).collect()
    }

    /// p1's unit zones, lane 1 to 5: each top card's def id, or `null`.
    fn unit_row(s: &Scenario) -> Value {
        Value::Array((1..=5).map(|lane| s.unit(PlayerId::P1, lane).map(|card| json!(card.def_id)).unwrap_or(Value::Null)).collect())
    }

    /// p1's backrow zones, lane 1 to 5: each card's def id, or `null`.
    fn back_row(s: &Scenario) -> Value {
        Value::Array((1..=5).map(|lane| s.backrow(PlayerId::P1, lane).map(|card| json!(card.def_id)).unwrap_or(Value::Null)).collect())
    }

    const MIXED_DECK: [&str; 8] = [STOCKPILE, VANILLA, MANA_WELL, LUNAR, SHEEPISH, GARY, BREAD, BIGOT];

    mod c_60_pile_on {
        use super::*;

        #[test]
        fn costs_5_and_declares_no_numbers_the_radiant_face_drops_the_replacement() {
            crate::register_all();
            let def = js(&registered_catalog()[ID]);
            assert_eq!(def["id"], PILE_ON);
            assert_eq!(def["cost"], 5);
            assert!(def["params"].is_null());
            let scripts = script();
            // A declaration holds a hook slot (`when`), so it is compared field by field.
            assert_eq!(scripts.base.replacements.len(), 1);
            let replacement = &scripts.base.replacements[0];
            assert_eq!(replacement.id, "pile-on");
            assert_eq!(replacement.on, ReplacementMoment::ToGraveyard);
            assert_eq!(replacement.where_, Some(ReplacementWhere::SelfCard));
            assert!(replacement.when.is_none());
            assert_eq!(js(&replacement.instead), json!({ "to": "bottomOfLibrary" }));
            assert!(replacement.then.is_none());
            assert!(replacement.by.is_none());
            assert!(scripts.radiant.replacements.is_empty());
            assert!(match (&scripts.radiant.cry, &scripts.base.cry) {
                (Some(radiant_cry), Some(base_cry)) => Arc::ptr_eq(radiant_cry, base_cry),
                _ => false,
            });
        }

        mod base {
            use super::*;

            #[test]
            fn s2_3_at_the_mana_cap_of_4_it_cannot_be_played_refused_and_legalactions_offers_no_play_of_it() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [PILE_ON, FILLER], "library": [VANILLA] }, "p2": { "hand": [FILLER] } }));
                let pile_on = s.card(PILE_ON).id.clone();
                assert!(
                    !legal_actions(s.state(), PlayerId::P1)
                        .iter()
                        .map(js)
                        .any(|action| action["type"] == "play" && action["instanceId"] == pile_on)
                );
                s.expect_refused(|s| s.play(PILE_ON, json!({})));
                s.expect_in_zone(&pile_on, "hand");
            }

            #[test]
            fn s2_3_with_5_mana_it_is_offered_and_plays() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [PILE_ON, FILLER], "library": [VANILLA], "mana": 5 }, "p2": { "hand": [FILLER] } }));
                let pile_on = s.card(PILE_ON).id.clone();
                assert!(
                    legal_actions(s.state(), PlayerId::P1)
                        .iter()
                        .map(js)
                        .any(|action| action["type"] == "play" && action["instanceId"] == pile_on)
                );
                s.play(PILE_ON, json!({}));
                s.expect_mana(PlayerId::P1, 0);
            }

            #[test]
            fn recruits_every_permanent_of_the_deck_top_to_bottom_each_into_its_row_spells_stay_in_order() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [PILE_ON, FILLER], "library": MIXED_DECK, "mana": 5 }, "p2": { "hand": [FILLER] } }));
                s.play(PILE_ON, json!({}));
                assert_eq!(unit_row(&s), json!([VANILLA, GARY, BIGOT, null, null]));
                assert_eq!(back_row(&s), json!([MANA_WELL, SHEEPISH, BREAD, null, null]));
                // The Spells keep their order, and Pile On itself goes under them (the base clause).
                assert_eq!(library_ids(&s), vec![STOCKPILE, LUNAR, PILE_ON]);
            }

            #[test]
            fn r1_no_recruited_cards_cry_fires_bigot_destroys_nothing_gary_flips_no_coins() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [PILE_ON, FILLER], "library": [BIGOT, GARY], "mana": 5 },
                    "p2": { "hand": [FILLER], "field": [VANILLA] },
                }));
                s.play(PILE_ON, json!({}));
                s.expect_in_zone(VANILLA, "field");
                s.expect_stats(GARY, json!({ "attack": 1, "health": 1 }));
                assert!(!s.events().iter().map(js).any(|event| event["type"] == "destroyed" || event["type"] == "buffed"));
            }

            #[test]
            fn r33_a_recruited_trap_or_field_trap_is_set_face_down_and_never_named_in_the_opponents_view_a_field_spell_is_face_up() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [PILE_ON, FILLER], "library": [SHEEPISH, MANA_WELL, BREAD], "mana": 5 }, "p2": { "hand": [FILLER] } }));
                s.play(PILE_ON, json!({}));
                let (trap, field, field_trap) = (s.backrow(PlayerId::P1, 1), s.backrow(PlayerId::P1, 2), s.backrow(PlayerId::P1, 3));
                assert_eq!(
                    [
                        trap.as_ref().map(|card| card.def_id.as_str()),
                        field.as_ref().map(|card| card.def_id.as_str()),
                        field_trap.as_ref().map(|card| card.def_id.as_str()),
                    ],
                    [Some(SHEEPISH), Some(MANA_WELL), Some(BREAD)],
                );
                assert_ne!(trap.as_ref().and_then(|card| card.face_up), Some(true));
                assert_ne!(field_trap.as_ref().and_then(|card| card.face_up), Some(true));
                assert_eq!(field.as_ref().and_then(|card| card.face_up), Some(true));
                let theirs = s.view(PlayerId::P2);
                assert_eq!(js(&theirs.opponent.backrow[0])["faceDown"], true);
                assert_eq!(js(&theirs.opponent.backrow[2])["faceDown"], true);
                assert!(!js(&theirs).to_string().contains(SHEEPISH));
                assert!(!js(&theirs).to_string().contains(BREAD));
                assert!(js(&theirs).to_string().contains(MANA_WELL));
            }

            #[test]
            fn stops_filling_a_row_once_it_is_full_the_permanents_that_no_longer_fit_stay_in_the_deck_in_order() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [PILE_ON, FILLER],
                        "field": [VANILLA, VANILLA, VANILLA, VANILLA],
                        "library": [GARY, LUNAR, BIGOT, VANILLA, MANA_WELL],
                        "mana": 5,
                    },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(PILE_ON, json!({}));
                assert_eq!(unit_row(&s), json!([VANILLA, VANILLA, VANILLA, VANILLA, GARY]));
                assert_eq!(back_row(&s), json!([MANA_WELL, null, null, null, null]));
                assert_eq!(library_ids(&s), vec![LUNAR, BIGOT, VANILLA, PILE_ON]);
            }

            #[test]
            fn r11_a_unit_token_card_in_the_deck_is_no_card_a_recruit_takes_it_stays() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [PILE_ON, FILLER], "library": [RUSH_TOKEN, VANILLA], "mana": 5 }, "p2": { "hand": [FILLER] } }));
                s.play(PILE_ON, json!({}));
                assert_eq!(unit_row(&s), json!([VANILLA, null, null, null, null]));
                assert_eq!(library_ids(&s), vec![RUSH_TOKEN, PILE_ON]);
            }

            #[test]
            fn with_an_empty_deck_it_recruits_nothing_and_still_goes_to_the_bottom_of_the_deck() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [PILE_ON, FILLER], "mana": 5 }, "p2": { "hand": [FILLER] } }));
                s.play(PILE_ON, json!({}));
                assert!(!s.events().iter().map(js).any(|event| event["type"] == "summoned"));
                assert_eq!(library_ids(&s), vec![PILE_ON]);
            }

            #[test]
            fn no_event_carries_a_deck_position_the_summons_name_only_the_zone_each_lands_in() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [PILE_ON, FILLER], "library": MIXED_DECK, "mana": 5 }, "p2": { "hand": [FILLER] } }));
                let pile_on = s.card(PILE_ON).id.clone();
                s.play(&pile_on, json!({}));
                assert_eq!(s.last_events().iter().map(js).filter(|event| event["type"] == "summoned").count(), 6);
                // The one event that names a deck slot is Pile On's own public move to the bottom of the deck.
                let positioned: Vec<Value> = s.last_events().iter().map(js).filter(|event| event.get("position").is_some()).collect();
                assert_eq!(positioned.len(), 1);
                assert_eq!(positioned[0]["type"], "shuffledIn");
                assert_eq!(positioned[0]["instanceId"], pile_on);
                let mut radiant_play = scenario(json!({
                    "p1": { "hand": [{ "def": PILE_ON, "radiant": true }, FILLER], "library": MIXED_DECK, "mana": 5 },
                    "p2": { "hand": [FILLER] },
                }));
                radiant_play.play(PILE_ON, json!({}));
                assert_eq!(
                    radiant_play.last_events().iter().map(js).filter(|event| event.get("position").is_some()).collect::<Vec<Value>>(),
                    Vec::<Value>::new(),
                );
            }

            #[test]
            fn its_return_is_a_replacement_after_it_resolves_it_goes_to_the_bottom_of_your_deck_never_the_graveyard() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [PILE_ON, FILLER], "library": [LUNAR, STOCKPILE], "mana": 5 }, "p2": { "hand": [FILLER] } }));
                let pile_on = s.card(PILE_ON).id.clone();
                s.play(&pile_on, json!({}));
                s.expect_in_zone(&pile_on, "library");
                assert_eq!(library_ids(&s), vec![LUNAR, STOCKPILE, PILE_ON]);
                assert!(s.pile(PlayerId::P1, "graveyard").is_empty());
                assert!(!s.events().iter().map(js).any(|event| event["type"] == "enteredGraveyard"));
            }

            #[test]
            fn discarded_it_goes_to_the_bottom_of_your_deck_instead() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [ZAO_GAO, PILE_ON, LUNAR], "library": [STOCKPILE] }, "p2": { "hand": [FILLER] } }));
                let pile_on = s.card(PILE_ON).id.clone();
                s.play(ZAO_GAO, json!({}));
                s.expect_in_zone(&pile_on, "library");
                s.expect_in_zone(LUNAR, "graveyard");
                assert_eq!(library_ids(&s), vec![STOCKPILE, PILE_ON]);
            }

            #[test]
            fn burned_at_the_hand_cap_it_goes_to_the_bottom_of_your_deck_instead() {
                crate::register_all();
                let nine: Vec<&str> = (0..9).map(|_| FILLER).collect();
                let mut hand = vec![STOCKPILE];
                hand.extend(nine);
                let mut s = scenario(json!({ "p1": { "hand": hand, "library": [LUNAR, PILE_ON, GARY] }, "p2": { "hand": [FILLER] } }));
                s.play(STOCKPILE, json!({}));
                assert!(s.events().iter().map(js).any(|event| event["type"] == "burned"));
                assert_eq!(library_ids(&s), vec![GARY, PILE_ON]);
                assert_eq!(
                    s.pile(PlayerId::P1, "graveyard").iter().map(|card| card.def_id.clone()).collect::<Vec<String>>(),
                    vec![STOCKPILE],
                );
            }

            #[test]
            fn r80_a_full_deck_turns_it_away_so_it_reaches_the_graveyard_after_all() {
                crate::register_all();
                let full: Vec<&str> = (0..LIBRARY_CAP).map(|_| FILLER).collect();
                let mut s = scenario(json!({ "p1": { "hand": [PILE_ON, FILLER], "library": full, "mana": 5 }, "p2": { "hand": [FILLER] } }));
                let pile_on = s.card(PILE_ON).id.clone();
                s.play(&pile_on, json!({}));
                s.expect_in_zone(&pile_on, "graveyard");
                assert_eq!(s.pile(PlayerId::P1, "library").len(), LIBRARY_CAP as usize);
            }

            #[test]
            fn counts_as_a_played_spell_and_the_recruited_cards_are_summoned_summoning_sick() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [PILE_ON, FILLER], "library": [VANILLA], "mana": 5 }, "p2": { "hand": [FILLER] } }));
                s.play(PILE_ON, json!({}));
                s.expect_events(json!(["cardPlayed", "summoned", "cardResolved"]));
                s.expect_refused(|s| s.attack(VANILLA, "hero"));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn recruits_every_permanent_of_the_deck_the_same_way() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": PILE_ON, "radiant": true }, FILLER], "library": MIXED_DECK, "mana": 5 },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(PILE_ON, json!({}));
                assert_eq!(unit_row(&s), json!([VANILLA, GARY, BIGOT, null, null]));
                assert_eq!(back_row(&s), json!([MANA_WELL, SHEEPISH, BREAD, null, null]));
                assert_eq!(library_ids(&s), vec![STOCKPILE, LUNAR]);
            }

            #[test]
            fn has_no_return_clause_after_it_resolves_it_goes_to_the_graveyard_as_any_spell_does() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": PILE_ON, "radiant": true }, FILLER], "library": [VANILLA], "mana": 5 }, "p2": { "hand": [FILLER] } }));
                let pile_on = s.card(PILE_ON).id.clone();
                s.play(&pile_on, json!({}));
                s.expect_in_zone(&pile_on, "graveyard");
                assert_eq!(library_ids(&s), Vec::<String>::new());
            }

            #[test]
            fn discarded_it_goes_to_the_graveyard() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [ZAO_GAO, { "def": PILE_ON, "radiant": true }, LUNAR] }, "p2": { "hand": [FILLER] } }));
                s.play(ZAO_GAO, json!({}));
                s.expect_in_zone(PILE_ON, "graveyard");
            }

            #[test]
            fn r33_a_recruited_trap_is_face_down_and_hidden_from_the_opponent() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": PILE_ON, "radiant": true }, FILLER], "library": [SHEEPISH], "mana": 5 },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(PILE_ON, json!({}));
                assert!(!js(&s.view(PlayerId::P2)).to_string().contains(SHEEPISH));
            }

            #[test]
            fn s2_3_at_the_mana_cap_of_4_it_cannot_be_played() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": PILE_ON, "radiant": true }, FILLER] }, "p2": { "hand": [FILLER] } }));
                s.expect_refused(|s| s.play(PILE_ON, json!({})));
            }
        }
    }
}
