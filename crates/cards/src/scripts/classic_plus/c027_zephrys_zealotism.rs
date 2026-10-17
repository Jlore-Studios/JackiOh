//! C+ #27 Zephrys Zealotism (SPEC §8.7 row 27; R29, R364, R387, R416, R1446). (4) Spell, Mythic.
//!
//! THE HAND IS THE SUBSYSTEM'S. B5 E34, the perfect-hand scorer: R29's Zephyrs scorer ranks
//! every non-token card of every set that ships (R1446) but this one (R387) for the state as this
//! resolves, each on the face it would arrive with, ties by card id; each other card in the caster's
//! hand goes to their graveyard (not a discard; a unit-token card ceases to exist, R11) and the top N
//! distinct cards arrive in rank order, N being how many there were (R416).
//!
//! "Refresh your mana" is §6.3 Refresh (R364): current mana rises toward max and never past it,
//! so a Refresh of every crystal gives back what was spent, and a player at or above max gains nothing.

use jackioh_engine::effects::{refresh_mana, replace_hand_with_perfect};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-027";

/// "Refresh your mana": every spent crystal, capped at max mana (R364), whatever the player's max is
/// (§9.9's handicaps).
/// An `i32` has no infinity; the Refresh adds this to current mana before capping
/// (`mana::refresh_some_mana`), so `i32::MAX` would overflow: half of it is still
/// more than any max mana, and the sum stays in range.
const ALL_MANA: i32 = i32::MAX / 2;

/// The faces differ only in which face the scorer ranks and the new cards arrive with.
fn zealotism(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![
                replace_hand_with_perfect(json_as(json!({ "radiant": radiant }))),
                refresh_mana(json_as(json!({ "amount": ALL_MANA }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: zealotism(false),
        radiant: zealotism(true),
    }
}

// C+ #27 Zephrys Zealotism (SPEC §8.7 row 27; B5 E34, R29, R364, R387, R416, R1446): replaces hand
// with the scorer's top distinct non-token picks of every set that ships for current state. Replaced cards
// go to graveyard and new ones are hidden (R97). Refreshes mana up to max. Radiant cards arrive radiant.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const ZEALOTISM: &str = "classicplus-027";
    const STOCKPILE: &str = "core-005";
    const MENACE: &str = "core-019";
    const TIMMY: &str = "core-011";
    /// A unit-token card, which ceases to exist rather than reach a graveyard (R11).
    const RUSH_TOKEN: &str = "core-t-rush";
    const COST: i32 = 4;
    const HIDDEN: &str = "hidden";

    fn side(setup: &Value, seat: &str) -> Value {
        setup.get(seat).cloned().unwrap_or_else(|| json!({}))
    }

    fn play(setup: &Value, radiant: bool) -> Scenario {
        let mut options = json!({});
        if let Some(seed) = setup.get("seed") {
            options["seed"] = seed.clone();
        }
        let mut p1 = side(setup, "p1");
        let mut hand = vec![json!({ "def": ZEALOTISM, "radiant": radiant })];
        if let Some(rest) = setup.get("p1").and_then(|p1| p1.get("hand")).and_then(Value::as_array) {
            hand.extend(rest.iter().cloned());
        }
        p1["hand"] = Value::Array(hand);
        options["p1"] = p1;
        if let Some(p2) = setup.get("p2") {
            options["p2"] = p2.clone();
        }
        let mut s = scenario(options);
        s.play(ZEALOTISM, json!({}));
        s
    }

    /// The state Zealotism resolves in: the same game with this card gone from hand and its (4) paid.
    fn resolving_state(setup: &Value) -> GameState {
        let mana = setup
            .get("p1")
            .and_then(|p1| p1.get("mana"))
            .and_then(Value::as_i64)
            .map(|mana| mana as i32)
            .unwrap_or(COST)
            - COST;
        let mut options = setup.clone();
        let mut p1 = side(setup, "p1");
        p1["mana"] = json!(mana);
        options["p1"] = p1;
        scenario(options).state().clone()
    }

    fn ranked(setup: &Value, radiant: bool) -> Vec<subsystems::Scored> {
        let mut options = json!({ "selfDefId": ZEALOTISM });
        if radiant {
            options["radiant"] = json!(true);
        }
        subsystems::rank_perfect_hand(&resolving_state(setup), P1, json_as(options))
    }

    fn hand_defs(s: &Scenario) -> Vec<String> {
        s.hand(P1).iter().map(|card| card.def_id.clone()).collect()
    }

    fn top_ids(scored: &[subsystems::Scored], n: usize) -> Vec<String> {
        scored.iter().take(n).map(|entry| entry.def.id.clone()).collect()
    }

    fn contains_all(xs: &[String], want: &[&str]) -> bool {
        want.iter().all(|w| xs.iter().any(|x| x == w))
    }

    fn is_token(def: &CardDef) -> bool {
        def.token || def.tags.contains(&Tag::Token)
    }

    mod c_n27_zephrys_zealotism {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r416_each_other_card_goes_to_your_graveyard_not_a_discard_and_as_many_cards_arrive() {
                crate::register_all();
                let setup = json!({ "p1": { "hand": [STOCKPILE, MENACE, TIMMY] } });
                let mut s = play(&setup, false);
                assert_eq!(s.hand(P1).len(), 3);
                for old in [STOCKPILE, MENACE, TIMMY] {
                    s.expect_in_zone(old, "graveyard");
                }
                assert!(!s.events().iter().any(|event| matches!(event, GameEvent::Discarded { .. })));
                let entered: Vec<String> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::EnteredGraveyard { def_id, .. } => Some(def_id.clone()),
                        _ => None,
                    })
                    .collect();
                assert!(contains_all(&entered, &[STOCKPILE, MENACE, TIMMY]));
                s.expect_in_zone(ZEALOTISM, "graveyard");
            }

            #[test]
            fn r416_the_new_hand_is_the_scorer_s_top_distinct_picks_in_rank_order_for_the_state_as_it_resolves() {
                crate::register_all();
                let setup = json!({ "p1": { "hand": [STOCKPILE, MENACE, TIMMY] } });
                let s = play(&setup, false);
                assert_eq!(hand_defs(&s), top_ids(&ranked(&setup, false), 3));
                let distinct: IndexSet<String> = hand_defs(&s).into_iter().collect();
                assert_eq!(distinct.len(), 3);
            }

            #[test]
            fn r387_r1446_every_card_from_a_set_that_ships_never_a_token_never_zephrys_zealotism_itself() {
                crate::register_all();
                let s = play(
                    &json!({ "p1": { "hand": [STOCKPILE, MENACE, TIMMY, STOCKPILE, MENACE, TIMMY, STOCKPILE, MENACE, TIMMY] } }),
                    false,
                );
                assert_eq!(s.hand(P1).len(), 9);
                for card in s.hand(P1) {
                    let ranking = subsystems::rank_perfect_hand(
                        s.state(),
                        P1,
                        json_as(json!({ "selfDefId": ZEALOTISM })),
                    );
                    let entry = ranking
                        .iter()
                        .find(|scored| scored.def.id == card.def_id)
                        .expect("the arrived card is in the ranking");
                    assert!(SHIPPED_SETS.contains(&entry.def.set));
                    assert!(!is_token(&entry.def));
                    assert_ne!(card.def_id, ZEALOTISM);
                }
                // R1446: the pool is every non-token card of every set that ships but this one (R387).
                let mut pool: Vec<String> =
                    ranked(&json!({}), false).iter().map(|scored| scored.def.id.clone()).collect();
                pool.sort();
                let mut expected: Vec<String> = crate::CATALOG
                    .values()
                    .filter(|def| !is_token(def) && SHIPPED_SETS.contains(&def.set) && def.id != ZEALOTISM)
                    .map(|def| def.id.clone())
                    .collect();
                expected.sort();
                assert_eq!(pool, expected);
                // Zealotism leaves out only its own id, so #97 Zephyrs is in its pool.
                assert!(pool.contains(&"core-097".to_string()));
                for set in SHIPPED_SETS {
                    assert!(
                        pool.iter().any(|id| crate::card_def(id).set == set),
                        "{set} is in the pool"
                    );
                }
            }

            #[test]
            fn r1446_every_candidate_scores_on_both_faces_without_a_panic() {
                crate::register_all();
                // Every set the catalog orders, shipped or previewed: the dry run plays each of them.
                let _preview = preview_every_set();
                let candidates = subsystems::perfect_pool(None, Some(ZEALOTISM)).len();
                let lethal = json!({ "p1": { "hand": [STOCKPILE, MENACE], "mana": 8 }, "p2": { "health": 4 } });
                let pressed = json!({ "p1": { "hand": [STOCKPILE], "health": 5 }, "p2": { "field": [TIMMY] } });
                for setup in [&lethal, &pressed] {
                    for radiant in [false, true] {
                        assert_eq!(ranked(setup, radiant).len(), candidates);
                    }
                }
            }

            #[test]
            fn r29_a_card_that_enables_lethal_ranks_first_when_lethal_exists() {
                crate::register_all();
                // p2 at 4: a 4-attack Charge body, played with the 4 mana left after this card's (4), is lethal.
                let setup = json!({ "p1": { "hand": [STOCKPILE, MENACE], "mana": 8 }, "p2": { "health": 4 } });
                let ranking = ranked(&setup, false);
                let top = ranking.first().expect("a ranking");
                assert!(matches!(top.priority, subsystems::ScorePriority::Lethal));
                let s = play(&setup, false);
                assert_eq!(hand_defs(&s)[0], top.def.id);
                // With no lethal on the board, nothing ranks for it.
                assert!(
                    ranked(&json!({ "p1": { "hand": [STOCKPILE, MENACE], "mana": 8 } }), false)
                        .iter()
                        .all(|scored| !matches!(scored.priority, subsystems::ScorePriority::Lethal))
                );
            }

            #[test]
            fn r1446_a_core_card_that_enables_lethal_is_dealt_45_deft_duelist() {
                crate::register_all();
                // p2 at 4 with the 4 mana left after this card's (4): #45 Deft Duelist, a Core 4/3 Charge
                // for 2, is lethal, and the pool reaches Core since R1446, so it is dealt.
                const DEFT_DUELIST: &str = "core-045";
                let setup = json!({ "p1": { "hand": [STOCKPILE, MENACE], "mana": 8 }, "p2": { "health": 4 } });
                let duelist = ranked(&setup, false)
                    .into_iter()
                    .find(|scored| scored.def.id == DEFT_DUELIST)
                    .expect("#45 is in the pool");
                assert_eq!(duelist.def.set, SetName::Core);
                assert!(matches!(duelist.priority, subsystems::ScorePriority::Lethal));
                assert!(hand_defs(&play(&setup, false)).contains(&DEFT_DUELIST.to_string()));
            }

            #[test]
            fn s10_7_the_same_state_always_gives_the_same_hand_whatever_the_seed_and_draws_nothing() {
                crate::register_all();
                let setup = json!({ "p1": { "hand": [STOCKPILE, MENACE, TIMMY], "field": [MENACE] }, "p2": { "field": [TIMMY] } });
                let mut seeded_a = setup.clone();
                seeded_a["seed"] = json!("zealotism-a");
                let mut seeded_b = setup.clone();
                seeded_b["seed"] = json!("zealotism-b");
                let a = play(&seeded_a, false);
                let b = play(&seeded_b, false);
                assert_eq!(hand_defs(&a), hand_defs(&b));
                let mut options = setup.clone();
                options["p1"]["hand"] = json!([ZEALOTISM, STOCKPILE, MENACE, TIMMY]);
                let mut before = scenario(options);
                let cursor = before.state().rng_cursor;
                before.play(ZEALOTISM, json!({}));
                assert_eq!(before.state().rng_cursor, cursor);
            }

            #[test]
            fn r416_with_only_this_card_in_hand_nothing_arrives_and_the_refresh_still_happens() {
                crate::register_all();
                let mut s = play(&json!({ "p1": { "hand": [] } }), false);
                assert_eq!(s.hand(P1).len(), 0);
                s.expect_mana(P1, 4);
            }

            #[test]
            fn r11_a_unit_token_card_in_the_replaced_hand_ceases_to_exist() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [ZEALOTISM, RUSH_TOKEN, STOCKPILE] } }));
                let token = s.card(RUSH_TOKEN).id.clone();
                s.play(ZEALOTISM, json!({}));
                assert_eq!(s.hand(P1).len(), 2);
                s.expect_in_zone(&token, "gone");
                let graveyard: Vec<String> = s.pile(P1, "graveyard").iter().map(|card| card.def_id.clone()).collect();
                assert!(!graveyard.iter().any(|id| id == RUSH_TOKEN));
                assert!(graveyard.iter().any(|id| id == STOCKPILE));
            }

            #[test]
            fn r364_the_refresh_gives_back_up_to_max_mana_this_card_s_4_included() {
                crate::register_all();
                play(&json!({ "p1": { "hand": [STOCKPILE] } }), false).expect_mana(P1, 4);
                // 6 current of 4 max: 2 left after paying, the Refresh gives back 2 more, up to max.
                play(&json!({ "p1": { "hand": [STOCKPILE], "mana": 6 } }), false).expect_mana(P1, 4);
            }

            #[test]
            fn r364_and_never_past_max_a_player_still_above_max_after_paying_gains_nothing() {
                crate::register_all();
                // A unit on the field keeps the turn alive (§2.5), whatever the perfect hand turns out to hold.
                let mut s = play(&json!({ "p1": { "hand": [STOCKPILE], "field": [MENACE], "mana": 9 } }), false);
                s.expect_mana(P1, 5);
                let changes = s
                    .events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::ManaChanged { .. }))
                    .count();
                assert_eq!(changes, 1);
            }

            #[test]
            fn r97_the_replaced_cards_are_public_in_the_graveyard_the_new_cards_are_hidden_from_the_opponent() {
                crate::register_all();
                let s = play(&json!({ "p1": { "hand": [STOCKPILE, MENACE] } }), false);
                let theirs = s.view(P2);
                let added: Vec<&GameEvent> = theirs
                    .events
                    .iter()
                    .filter(|event| matches!(event, GameEvent::AddedToHand { .. }))
                    .collect();
                assert_eq!(added.len(), 2);
                for event in added {
                    if let GameEvent::AddedToHand { def_id, instance_id, .. } = event {
                        assert_eq!(def_id, HIDDEN);
                        assert_eq!(instance_id, HIDDEN);
                    }
                }
                let graveyard: Vec<String> = theirs.opponent.graveyard.iter().map(|card| card.def_id.clone()).collect();
                assert!(contains_all(&graveyard, &[STOCKPILE, MENACE]));
                let text = serde_json::to_string(&theirs).unwrap();
                for card in s.hand(P1) {
                    assert!(!text.contains(&card.id));
                    assert!(!text.contains(&card.def_id));
                }
            }

            #[test]
            fn s9_3_the_state_survives_json_and_the_play_replays_to_the_same_hash() {
                crate::register_all();
                let s = scenario(json!({ "p1": { "hand": [ZEALOTISM, STOCKPILE, MENACE] }, "p2": { "field": [TIMMY] } }));
                let thawed: GameState =
                    serde_json::from_str(&serde_json::to_string(s.state()).unwrap()).unwrap();
                let action: Action = json_as(json!({
                    "type": "play",
                    "instanceId": s.card(ZEALOTISM).id,
                    "playerId": "p1",
                    "nonce": "zealotism-replay",
                }));
                let live = reduce(s.state(), &action);
                let again = reduce(&thawed, &action);
                assert!(live.error.is_none());
                assert_eq!(hash_state(&again.state), hash_state(&live.state));
                assert_eq!(again.events, live.events);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r416_the_perfect_radiant_hand_ranked_on_the_radiant_faces_and_the_new_cards_arrive_radiant() {
                crate::register_all();
                let setup = json!({ "p1": { "hand": [STOCKPILE, MENACE, TIMMY] } });
                let mut s = play(&setup, true);
                assert_eq!(hand_defs(&s), top_ids(&ranked(&setup, true), 3));
                assert!(s.hand(P1).iter().all(|card| card.radiant));
                s.expect_mana(P1, 4);
            }

            #[test]
            fn r1446_the_radiant_face_draws_on_every_set_that_ships_too() {
                crate::register_all();
                let s = play(&json!({ "p1": { "hand": [STOCKPILE, MENACE] } }), true);
                for card in s.hand(P1) {
                    assert!(card.radiant);
                    assert!(SHIPPED_SETS.contains(&crate::card_def(&card.def_id).set));
                }
                let pool = ranked(&json!({}), true);
                for set in SHIPPED_SETS {
                    assert!(pool.iter().any(|scored| scored.def.set == set), "{set} is in the Radiant pool");
                }
            }
        }
    }
}
