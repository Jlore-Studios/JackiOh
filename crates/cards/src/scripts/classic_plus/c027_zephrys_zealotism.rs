//! C+ #27 Zephrys Zealotism (SPEC §8.7 row 27; R29, R364, R387, R416). (4) Spell, Mythic.
//!   Base:    "Replace your hand with the perfect hand of Classic and Classic+ cards. Refresh your mana."
//!   Radiant: "Replace your hand with the perfect Radiant hand of Classic and Classic+ cards. Refresh
//!            your mana."
//!
//! THE HAND IS THE SUBSYSTEM'S. B5 E34, the perfect-hand scorer (`engine/src/subsystems/perfectHand.ts`):
//! R29's Zephyrs scorer ranks every non-token Classic and Classic+ card but this one (R387) for the
//! state as this resolves, each on the face it would arrive with, ties by card id; each other card in
//! the caster's hand goes to their graveyard (not a discard; a unit-token card ceases to exist, R11) and
//! the top N distinct cards arrive in rank order, N being how many there were (R416). This card only
//! names the verb and the face, so no weight, pool or tie-break is restated here.
//!
//! "Refresh your mana" is §6.3 Refresh (R364): current mana rises toward max and never past it, so a
//! Refresh of every crystal gives back what was spent — this card's 4 included when max is 4 — and a
//! player at or above max gains nothing. It follows the hand, as the text orders them.

use jackioh_engine::effects::{refresh_mana, replace_hand_with_perfect};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-027";

/// "Refresh your mana": every spent crystal, which R364 caps at max mana (a Refresh never takes
/// current mana above it), so this is a full refill, whatever the player's max is (§9.9's handicaps).
/// TS wrote `Number.POSITIVE_INFINITY`; the largest `i32` is the same "every crystal" for any max.
const ALL_MANA: i32 = i32::MAX;

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

// C+ #27 Zephrys Zealotism — SPEC §8.7 row 27, B5 E34, R29, R364, R387, R416. BUILD M9 Classic+ row
// C+ 27: "Each other card in your hand goes to your graveyard (not a discard) and as many cards arrive:
// the scorer's top distinct picks (R29's scorer) among the non-token Classic and Classic+ cards but this
// one (R387), ranked for the current state, never a Core card on either face (R416); a card that enables
// lethal ranks first when lethal exists; the same state always gives the same hand; with only this card
// in hand nothing arrives; then a Refresh (R364) gives back up to max mana, its own 4 included, never
// past max; the replaced cards are public in the graveyard and the new ones hidden from the opponent
// (R97); the state survives JSON and replays to the same hash; radiant the new cards are Radiant".
//
// The ranking itself is pinned against a fixed pool in the engine (`packages/engine/test/perfectHand.test.ts`).
// Here it runs over the real catalog, so each case compares the hand that arrives with the subsystem's
// own ranking of the state the card resolves in: the same board and hand without this card, at the mana
// left once its (4) is paid (`resolvingState`).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const ZEALOTISM: &str = "classicplus-027";
    /// Plain hand cards to be replaced: #5 Stockpile, #19 Midrange Menace, #11 Tempo Timmy.
    const STOCKPILE: &str = "core-005";
    const MENACE: &str = "core-019";
    const TIMMY: &str = "core-011";
    /// A unit-token card, which ceases to exist rather than reach a graveyard (R11).
    const RUSH_TOKEN: &str = "core-t-rush";
    const COST: i32 = 4;
    const HIDDEN: &str = "hidden";

    /// TS `type Setup = { p1?: SideSetup; p2?: SideSetup; seed?: string }`, as the literal's JSON.
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

    /// TS `expect(xs).toEqual(expect.arrayContaining(want))`.
    fn contains_all(xs: &[String], want: &[&str]) -> bool {
        want.iter().all(|w| xs.iter().any(|x| x == w))
    }

    mod c_n27_zephrys_zealotism {
        use super::*;

        #[test]
        fn is_the_card_it_says_and_the_faces_differ_only_in_the_face_they_rank_and_hand_over() {
            crate::register_all();
            assert_eq!(ID, ZEALOTISM);
            assert_eq!(crate::card_def(ID).id, ZEALOTISM);
            let scripts = script();
            let (base_cry, radiant_cry) = (scripts.base.cry.unwrap(), scripts.radiant.cry.unwrap());
            // TS `expect(base).not.toBe(radiant)`: each face is its own script, built by its own call.
            assert!(!std::sync::Arc::ptr_eq(&base_cry, &radiant_cry));
        }

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
            fn r387_r416_never_a_core_card_never_a_token_never_zephrys_zealotism_itself() {
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
                    assert!(entry.def.set == SetName::Classic || entry.def.set == SetName::ClassicPlus);
                    assert!(!entry.def.token);
                    assert_ne!(card.def_id, ZEALOTISM);
                }
                let pool = ranked(&json!({}), false);
                assert!(pool.iter().all(|scored| {
                    (scored.def.set == SetName::Classic || scored.def.set == SetName::ClassicPlus) && !scored.def.token
                }));
                assert!(!pool.iter().any(|scored| scored.def.id == ZEALOTISM));
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
            fn r416_the_radiant_face_draws_on_classic_and_classic_plus_only_too() {
                crate::register_all();
                let s = play(&json!({ "p1": { "hand": [STOCKPILE, MENACE] } }), true);
                for card in s.hand(P1) {
                    assert!(card.def_id.starts_with("classic"));
                }
            }
        }
    }
}
