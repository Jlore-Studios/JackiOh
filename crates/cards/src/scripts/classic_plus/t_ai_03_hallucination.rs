//! T-AI-3 Hallucination (SPEC §8.7 row T-AI-3, R57, R385; BUILD M9 row T-AI-3). (0) Spell, AI, Token.
//!   Base:    "Add a copy of a random card in your opponent's deck to your hand. Give it Brittle 2."
//!   Radiant: "Add copies of 2 different random cards in your opponent's deck to your hand. Give them Brittle 2."
//!
//! `add_library_copies` (engine): new cards you own carrying the picked cards' definition, radiant flag,
//! `statsOverride` and `tuning` (R57), the originals staying in their deck; only you learn what they
//! copied (R97, R177). Brittle 2 is given as each lands (R385), so a copy made on your turn t ticks to 1
//! at the start of t + 2 and crumbles at the start of t + 4. An empty deck gives nothing (R129); the hand
//! cap burns extras (§2.4). AI cards declare no numbers (B8), so the counts are this file's.

use jackioh_engine::effects::add_library_copies;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-t-ai-03";

const COPIES: i32 = 1;
const RADIANT_COPIES: i32 = 2;
const BRITTLE: i32 = 2;

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| {
                vec![add_library_copies(json_as(json!({ "of": "enemy", "count": COPIES, "brittle": BRITTLE })))]
            })),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| {
                vec![add_library_copies(json_as(json!({
                    "of": "enemy",
                    "count": RADIANT_COPIES,
                    "brittle": BRITTLE,
                })))]
            })),
            ..Script::default()
        },
    }
}

// T-AI-3 Hallucination — SPEC §8.7 row T-AI-3, BUILD M9 row T-AI-3: adds a copy of a random card of the
// opponent's deck to your hand, a new card you own (the original stays), carrying the original's radiant
// flag, `statsOverride` and `tuning` (R57), and gives it Brittle 2: held in your hand without ticking
// (R638), then on the field, entered on your turn t, it ticks to 1 at the start of your turn t + 2 and
// crumbles at the start of t + 4 (R385); an empty deck, nothing and no
// random draw (R129); only you learn what it copied — the opponent's view shows a card added under the
// sentinel and their library list does not change (R97, R310); radiant copies of 2 different random
// cards, both Brittle 2.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const HALLUCINATION: &str = "classicplus-t-ai-03";
    const UNIT: &str = "core-008"; // Mr. Vanilla 4/4, (1).
    const OTHERS: [&str; 5] = ["core-001", "core-002", "core-011", "core-019", "core-043"];
    const FILLER: &str = "core-005";

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    use crate::js;

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

    fn ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.id.clone()).collect()
    }

    fn cast(p2: Value, radiant: bool, seed: Option<&str>) -> Scenario {
        crate::register_all();
        let mut opts = json!({
            "p1": { "hand": [{ "def": HALLUCINATION, "radiant": radiant }, FILLER] },
            "p2": spread(json!({ "hand": [FILLER] }), &p2),
        });
        if let Some(seed) = seed {
            opts["seed"] = json!(seed);
        }
        scenario(opts)
    }

    fn copies(s: &Scenario, before: &[String]) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| !before.contains(&card.id)).collect()
    }

    fn played(s: &mut Scenario) -> Vec<CardInstance> {
        let before = ids(&s.hand(P1));
        s.play(HALLUCINATION, json!({}));
        copies(s, &before)
    }

    fn view_events(s: &Scenario, player: PlayerId) -> Vec<Value> {
        js(&s.view(player))["events"].as_array().cloned().unwrap_or_default()
    }

    fn has_event(s: &Scenario, kind: &str) -> bool {
        s.events().iter().any(|event| js(event)["type"] == kind)
    }

    mod t_ai_3_hallucination {
        use super::*;

        #[test]
        fn is_a_0_spell_ai_token() {
            crate::register_all();
            let def = js(&crate::card_def(super::super::ID));
            assert_eq!(def["cost"], json!(0));
            assert_eq!(def["type"], json!("Spell"));
            assert_eq!(def["tags"], json!(["AI", "Token"]));
            assert_eq!(def["token"], json!(true));
            let scripts = super::super::script();
            assert!(scripts.base.cry.is_some());
            assert!(scripts.radiant.cry.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn r57_adds_a_copy_of_a_card_of_the_opponents_deck_a_new_card_you_own_with_its_radiant_flag_stats_override_and_tuning_the_original_stays()
             {
                let mut s = cast(json!({ "library": [{ "def": UNIT, "radiant": true }] }), false, None);
                let original_id = s.pile(P2, "library").first().expect("the original").id.clone();
                {
                    let original = find_instance_mut(s.state_mut(), &original_id).expect("the original in the state");
                    original.tuning = Some(json_as(json!({ "attack": 2 })));
                    original.stats_override = Some(json_as(json!({ "attack": 1, "health": 5 })));
                    original.cost_mod = 1;
                }
                let copy = played(&mut s).into_iter().next().expect("a copy");
                assert_eq!(copy.def_id, UNIT);
                assert_eq!(copy.owner, P1);
                assert_eq!(copy.controller, P1);
                assert!(copy.radiant);
                assert_eq!(copy.cost_mod, 0);
                assert_ne!(copy.id, original_id);
                assert_eq!(js(&copy.tuning), json!({ "attack": 2 }));
                assert_eq!(js(&copy.stats_override), json!({ "attack": 1, "health": 5 }));
                assert_eq!(ids(&s.pile(P2, "library")), vec![original_id]);
            }

            #[test]
            fn r60_the_copy_is_of_a_random_card_of_their_deck_the_seeds() {
                let mut seen: IndexSet<String> = IndexSet::new();
                for n in 0..16 {
                    let seed = format!("hallucination-{n}");
                    let mut s = cast(json!({ "library": OTHERS }), false, Some(&seed));
                    let copy = played(&mut s).into_iter().next().expect("a copy");
                    assert!(OTHERS.contains(&copy.def_id.as_str()));
                    seen.insert(copy.def_id);
                }
                assert!(seen.len() > 2);
            }

            #[test]
            fn r385_r638_brittle_2_held_in_the_hand_without_ticking_on_the_field_it_ticks_to_1_at_t_plus_2_and_crumbles_at_t_plus_4_not_a_discard()
             {
                let mut s = cast(json!({ "library": [UNIT], "hand": [FILLER, FILLER, FILLER] }), false, None);
                let t = s.state().turn;
                let copy = played(&mut s).into_iter().next().expect("a copy");
                assert_eq!(js(&copy.brittle), json!({ "count": 2, "since": t }));
                // R638: two whole rounds in the hand and the count has not moved, nothing crumbled.
                s.end_turn().end_turn().end_turn().end_turn();
                assert_eq!(s.state().turn, t + 4);
                assert_eq!(js(&s.card(&copy).brittle), json!({ "count": 2, "since": t }));
                s.expect_in_zone(&copy, "hand");
                assert!(!has_event(&s, "crumbled"));

                // Entering the field starts its cycle: a count of 2 at turn t + 4 ticks at t + 6 and crumbles at t + 8.
                s.play(&copy, json!({}));
                assert_eq!(js(&s.card(&copy).brittle), json!({ "count": 2, "since": t + 4 }));
                s.end_turn().end_turn();
                assert_eq!(s.state().turn, t + 6);
                assert_eq!(s.card(&copy).brittle.as_ref().map(|brittle| brittle.count), Some(1));
                s.end_turn().end_turn();
                assert_eq!(s.state().turn, t + 8);
                s.expect_in_zone(&copy, "graveyard");
                assert!(s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::Crumbled { instance_id, .. } if *instance_id == copy.id
                )));
                assert!(!has_event(&s, "discarded"));
            }

            #[test]
            fn r129_an_empty_deck_gives_nothing_and_draws_nothing() {
                let mut s = cast(json!({ "library": [] }), false, None);
                let cursor = s.state().rng_cursor;
                assert!(played(&mut s).is_empty());
                assert_eq!(s.state().rng_cursor, cursor);
                s.expect_in_zone(HALLUCINATION, "graveyard");
            }

            #[test]
            fn r97_r310_only_you_learn_what_it_copied_their_view_shows_a_card_added_under_the_sentinel_their_library_list_unchanged() {
                let mut s = cast(json!({ "library": OTHERS }), false, None);
                let list_before = js(&s.view(P2))["you"]["ownLibrary"].clone();
                let copy = played(&mut s).into_iter().next().expect("a copy");
                let theirs: Vec<Value> =
                    view_events(&s, P2).into_iter().filter(|event| event["type"] == "addedToHand").collect();
                assert_eq!(theirs.len(), 1);
                assert_eq!(theirs[0]["player"], json!("p1"));
                assert_eq!(theirs[0]["instanceId"], json!(HIDDEN_ID));
                assert_eq!(theirs[0]["defId"], json!(HIDDEN_ID));
                assert!(!js(&s.view(P2))["events"].to_string().contains(&copy.id));
                assert_eq!(js(&s.view(P2))["you"]["ownLibrary"], list_before);
                let mine = view_events(&s, P1).into_iter().find(|event| event["type"] == "addedToHand");
                assert_eq!(
                    mine.map(|event| (event["instanceId"].clone(), event["defId"].clone())),
                    Some((json!(copy.id), json!(copy.def_id)))
                );
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r60_copies_of_2_different_random_cards_both_brittle_2() {
                for n in 0..8 {
                    let seed = format!("hallucination-r-{n}");
                    let mut s = cast(json!({ "library": OTHERS }), true, Some(&seed));
                    let made = played(&mut s);
                    assert_eq!(made.len(), 2);
                    assert_eq!(made.iter().map(|card| card.def_id.clone()).collect::<IndexSet<_>>().len(), 2);
                    assert!(made.iter().all(|card| card.brittle.as_ref().map(|brittle| brittle.count) == Some(2)));
                    assert_eq!(s.pile(P2, "library").len(), OTHERS.len());
                }
            }

            #[test]
            fn r586_s2_4_with_room_for_one_the_second_copy_is_burned_to_your_graveyard_and_takes_no_brittle() {
                crate::register_all();
                let mut hand = vec![json!({ "def": HALLUCINATION, "radiant": true })];
                hand.extend((0..HAND_CAP - 1).map(|_| json!(FILLER)));
                let mut s = scenario(json!({ "p1": { "hand": hand }, "p2": { "hand": [FILLER], "library": OTHERS } }));
                let before = ids(&s.hand(P1));
                s.play(HALLUCINATION, json!({}));
                let landed = copies(&s, &before);
                assert_eq!(landed.len(), 1);
                assert_eq!(landed[0].brittle.as_ref().map(|brittle| brittle.count), Some(2));
                assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
                let burned: Vec<CardInstance> = s
                    .pile(P1, "graveyard")
                    .into_iter()
                    .filter(|card| OTHERS.contains(&card.def_id.as_str()))
                    .collect();
                assert_eq!(burned.len(), 1);
                assert!(burned[0].brittle.is_none());
                s.expect_events(json!(["burned"]));
            }

            #[test]
            fn r129_a_deck_of_one_card_gives_one_copy_with_no_draw() {
                let mut s = cast(json!({ "library": [UNIT] }), true, None);
                let cursor = s.state().rng_cursor;
                let made: Vec<String> = played(&mut s).into_iter().map(|card| card.def_id).collect();
                assert_eq!(made, vec![UNIT]);
                assert_eq!(s.state().rng_cursor, cursor);
            }
        }
    }
}
