//! C #61 Plague Bringer Goliath (SPEC §8.6 row 61). (3) Unit, Rare, 7/7 → 14/14.
//!   Base:    "Tribute 1, Rush, Trample
//!             Cry: Place {tokens|Plague Counter|Plague Counters}. Draw {draw}." — 3 tokens, draw 1
//!   Radiant: the same text — 3 tokens, draw 3
//!
//! Rush and Trample are printed on both catalog faces; §10.4 layer 1 reads them there. Tribute is the play's
//! price (§6.3, R101), paid at §10.5 step 2, so with no Unit to tribute it can't be played; on a full unit
//! row it may take the zone its own Tribute empties (R391, §3.2).
//!
//! "Place N Plague Counters" is N placements of 1 (`placePlagueTokens`), all on the one permanent a single
//! `target` prompt names, either side, face-down cards included (offered by id alone, R177) (R689). Each
//! placement is multiplied by the receiver (C #27) and is its own for C #53. The prompt parks the rest of the
//! Cry on `state.work` (R113), so the draw comes after. Both numbers are `tokens` and `draw` (R386).

use jackioh_engine::effects::{draw, place_plague_tokens};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-061";

/// §6.3 "Tribute 1": one of your Units, as #66 The Rock's.
const TRIBUTE_COST: i32 = 1;

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| {
            vec![
                place_plague_tokens(json_as(json!({ "count": param(ctx, "tokens") }))),
                draw(json_as(json!({ "count": param(ctx, "draw") }))),
            ]
        })),
        ..Script::default()
    };

    // The same script: the Radiant face's draw 3 is its declared `draw`, which `param` reads off the running
    // face; its stats are the catalog's.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C #61 Plague Bringer Goliath — SPEC §8.6 row 61, BUILD M9 Classic row C 61. Tribute: can't be played without
// a Unit to tribute, may take its zone on a full board (R391). Cry: three placements on the one permanent a
// prompt names, face-down ones by id alone (R689, R177), then draw; Trample's excess hits the hero (R63);
// radiant draws 3; tuned numbers read through `param()` (R386).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;
    use std::collections::BTreeSet;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GOLIATH: &str = "classic-061";
    const CRAWLER: &str = "classic-053"; // (1) Unit: whenever Plague Counters are placed on this, draw 1.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const PAWN: &str = "core-096"; // (1) Trap: answers only an attack that would be lethal.
    const MANA_WELL: &str = "core-006"; // (3) Field Spell.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).
    const X: &str = "core-020"; // library filler.

    use crate::js;

    /// The state written out and read back field by field, in field order.
    fn round_trip(state: &GameState) -> GameState {
        let text = serde_json::to_string(state).expect("the state serialises");
        serde_json::from_str(&text).expect("the state parses back")
    }

    fn lib(n: usize) -> Vec<&'static str> {
        vec![X; n]
    }

    /// The value, or a failure naming what is missing.
    fn must<T>(value: Option<T>, what: &str) -> T {
        value.unwrap_or_else(|| panic!("missing: {what}"))
    }

    fn option_ids(s: &Scenario) -> Vec<String> {
        let pending = must(s.state().pending.clone(), "an open prompt");
        pending
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Instance { instance_id } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn draws_by(events: &[GameEvent], player: PlayerId) -> usize {
        events
            .iter()
            .map(js)
            .filter(|event| event["type"] == "drawn" && event["player"] == js(&player))
            .count()
    }

    fn pick(card: &CardInstance) -> Value {
        json!([{ "pick": "instance", "instanceId": card.id }])
    }

    /// Goliath in hand with a Vanilla to tribute, and a board to place on. `extra`: `p2Backrow` and
    /// `library` (defaults `[]` and 5).
    fn ready(radiant_face: bool, extra: Value) -> Scenario {
        let p2_backrow = if extra["p2Backrow"].is_null() { json!([]) } else { extra["p2Backrow"].clone() };
        let library = extra["library"].as_u64().map_or(5, |n| n as usize);
        scenario(json!({
            "p1": { "hand": [{ "def": GOLIATH, "radiant": radiant_face }, ANCHOR], "field": [VANILLA], "library": lib(library) },
            "p2": { "hand": [ANCHOR], "field": [MENACE], "backrow": p2_backrow },
        }))
    }

    fn play_goliath(s: &mut Scenario) -> CardInstance {
        let vanilla = s.card(VANILLA).id.clone();
        s.play(GOLIATH, json!({ "tributes": [vanilla] }));
        s.card(GOLIATH).clone()
    }

    mod c_61_plague_bringer_goliath {
        use super::*;

        #[test]
        fn declares_tribute_1_its_two_numbers_and_one_script_on_both_faces_rush_and_trample_are_printed() {
            crate::register_all();
            let def = js(&crate::card_def(ID));
            assert_eq!(def["id"], GOLIATH);
            let scripts = script();
            assert_eq!(js(&scripts.base.static_flags), json!({ "tribute": 1 }));
            assert_eq!(
                def["params"],
                json!([
                    { "key": "tokens", "base": 3, "radiant": 3, "better": "up", "step": 1, "min": 1 },
                    { "key": "draw", "base": 1, "radiant": 3, "better": "up", "step": 1, "min": 1 },
                ]),
            );
            assert_eq!(def["base"]["keywords"], json!([{ "kind": "Rush" }, { "kind": "Trample" }]));
            assert_eq!(def["radiant"]["keywords"], json!([{ "kind": "Rush" }, { "kind": "Trample" }]));
            // The Radiant face is the same declaration and the same hooks.
            assert_eq!(js(&scripts.radiant.static_flags), js(&scripts.base.static_flags));
            assert_eq!(scripts.radiant.cry.is_some(), scripts.base.cry.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn r101_cant_be_played_without_a_unit_to_tribute_legalactions_offers_no_play_and_the_play_is_refused() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [GOLIATH, ANCHOR] }, "p2": { "hand": [ANCHOR], "field": [MENACE] } }));
                let goliath = s.card(GOLIATH).clone();

                assert!(
                    !legal_actions(s.state(), P1)
                        .iter()
                        .map(js)
                        .any(|action| action["type"] == "play" && action["instanceId"] == goliath.id.as_str())
                );
                s.expect_refused_with(|s| s.play(&goliath, json!({})), "Tribute");
            }

            #[test]
            fn s6_3_the_tribute_is_paid_with_one_of_your_units_a_death_the_goliath_enters_a_7_7_with_rush_and_trample() {
                crate::register_all();
                let mut s = ready(false, json!({}));
                let vanilla = s.card(VANILLA).clone();

                let goliath = play_goliath(&mut s);

                s.expect_in_zone(&vanilla, "graveyard");
                s.expect_stats(&goliath, json!({ "attack": 7, "health": 7 }));
                let kinds: Vec<Value> = js(&s.stats(&goliath).keywords)
                    .as_array()
                    .map(|keywords| keywords.iter().map(|keyword| keyword["kind"].clone()).collect())
                    .unwrap_or_default();
                assert_eq!(kinds, vec![json!("Rush"), json!("Trample")]);
                s.expect_events(json!(["destroyed", "cardPlayed"]));
            }

            #[test]
            fn r391_on_a_full_unit_row_it_may_take_the_zone_its_own_tribute_empties() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [GOLIATH, ANCHOR], "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA], "library": lib(3) },
                    "p2": { "hand": [ANCHOR] },
                }));
                let third = must(s.unit(P1, 3), "lane 3");
                let goliath = s.card(GOLIATH).id.clone();
                let plays: Vec<Value> = legal_actions(s.state(), P1)
                    .iter()
                    .map(js)
                    .filter(|action| action["type"] == "play" && action["instanceId"] == goliath.as_str())
                    .collect();
                assert!(plays.iter().any(|play| play["zone"]["lane"] == 3
                    && play["tributes"].as_array().is_some_and(|tributes| tributes.contains(&json!(third.id)))));

                s.play(GOLIATH, json!({ "zone": 3, "tributes": [third.id] }));

                assert_eq!(s.unit(P1, 3).map(|unit| unit.def_id), Some(GOLIATH.to_string()));
            }

            #[test]
            fn cry_three_placements_on_the_one_permanent_a_single_prompt_names_over_every_permanent_on_either_side_itself_included() {
                crate::register_all();
                let mut s = ready(false, json!({ "p2Backrow": [MANA_WELL] }));
                let goliath = play_goliath(&mut s);
                let menace = s.card(MENACE).clone();
                let well = s.card(MANA_WELL).clone();

                let pending = js(&s.state().pending);
                assert_eq!(pending["playerId"], "p1");
                assert_eq!(pending["kind"], "target");
                assert_eq!(
                    option_ids(&s).into_iter().collect::<BTreeSet<String>>(),
                    BTreeSet::from([goliath.id.clone(), menace.id.clone(), well.id.clone()]),
                );
                // One answer puts all three on the pick: no second prompt opens.
                s.answer(json!(menace.id));

                assert!(s.state().pending.is_none());
                assert_eq!(s.card(&menace).counters.plague, Some(3));
                assert_eq!(s.card(&well).counters.plague.unwrap_or(0), 0);
                assert_eq!(s.card(&goliath).counters.plague.unwrap_or(0), 0);
                let placed = s
                    .events()
                    .iter()
                    .map(js)
                    .filter(|event| event["type"] == "counterChanged" && event.get("placed").is_some())
                    .count();
                assert_eq!(placed, 3);
            }

            #[test]
            fn r689_no_spreading_one_answer_puts_all_three_on_the_pick_three_placements_of_1() {
                crate::register_all();
                let mut s = ready(false, json!({}));
                play_goliath(&mut s);
                let menace = s.card(MENACE).clone();

                s.answer(json!(menace.id));

                assert!(s.state().pending.is_none());
                assert_eq!(s.card(&menace).counters.plague, Some(3));
                let placed: Vec<Value> = s
                    .events()
                    .iter()
                    .map(js)
                    .filter(|event| event["type"] == "counterChanged" && event.get("placed").is_some())
                    .map(|event| event["placed"].clone())
                    .collect();
                assert_eq!(placed, vec![json!(1), json!(1), json!(1)]);
            }

            #[test]
            fn r113_then_draw_1_the_draw_waits_for_the_one_answer() {
                crate::register_all();
                let mut s = ready(false, json!({}));
                play_goliath(&mut s);
                let menace = s.card(MENACE).clone();

                s.answer(json!(menace.id));

                assert!(s.state().pending.is_none());
                assert_eq!(draws_by(s.last_events(), P1), 1);
                let kinds: Vec<Value> = s
                    .last_events()
                    .iter()
                    .map(js)
                    .filter(|event| event["type"] == "counterChanged" || event["type"] == "drawn")
                    .map(|event| event["type"].clone())
                    .collect();
                assert_eq!(
                    kinds,
                    vec![json!("counterChanged"), json!("counterChanged"), json!("counterChanged"), json!("drawn")],
                );
            }

            #[test]
            fn r177_a_face_down_enemy_trap_is_an_option_by_its_id_alone_and_placing_on_it_never_names_it_to_you() {
                crate::register_all();
                let mut s = ready(false, json!({ "p2Backrow": [{ "def": PAWN, "faceUp": false }] }));
                play_goliath(&mut s);
                let trap = s.card(PAWN).clone();

                assert!(option_ids(&s).contains(&trap.id));
                let mine = must(Some(js(&s.view(P1).pending)).filter(|pending| !pending.is_null()), "p1's view of the prompt");
                assert!(mine["forYou"] == true, "the prompt is p1's");
                let option = must(
                    mine["options"].as_array().and_then(|options| {
                        options
                            .iter()
                            .find(|entry| {
                                entry["instanceId"] == trap.id.as_str()
                                    || entry["key"].as_str().is_some_and(|key| key.contains(&trap.id))
                            })
                            .cloned()
                    }),
                    "the trap's option",
                );
                assert!(option.get("defId").is_none());
                assert!(!js(&s.view(P1)).to_string().contains(PAWN));
                assert!(!js(&s.view(P1)).to_string().contains("My Pawn"));

                s.answer(json!(trap.id));

                assert!(s.state().pending.is_none());
                assert_eq!(s.card(&trap).counters.plague, Some(3));
                assert!(!js(&s.view(P1)).to_string().contains(PAWN));
            }

            #[test]
            fn s10_6_the_opponent_sees_only_that_a_prompt_is_open() {
                crate::register_all();
                let mut s = ready(false, json!({}));
                play_goliath(&mut s);

                assert_eq!(js(&s.view(P2).pending), json!({ "forYou": false, "pendingFor": "p1" }));
            }

            #[test]
            fn each_placement_is_its_own_a_c_53_plague_crawler_that_takes_all_three_draws_three_times() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [GOLIATH, ANCHOR], "field": [VANILLA, CRAWLER], "library": lib(6) },
                    "p2": { "hand": [ANCHOR] },
                }));
                play_goliath(&mut s);
                let crawler = s.card(CRAWLER).clone();

                s.answer(json!(crawler.id));

                // Three Crawler draws and the Goliath's own.
                assert_eq!(draws_by(s.events(), P1), 4);
            }

            #[test]
            fn s9_3_the_open_prompt_survives_a_json_round_trip_and_finishes_as_the_live_one_does() {
                crate::register_all();
                let mut s = ready(false, json!({}));
                play_goliath(&mut s);
                let menace = s.card(MENACE).clone();

                let revived = round_trip(s.state());
                assert_eq!(&revived, s.state());
                let choice = must(revived.pending.clone(), "the placement prompt");
                let answer: Action = json_as(json!({
                    "type": "answer",
                    "playerId": "p1",
                    "choiceId": choice.id,
                    "selection": pick(&menace),
                    "nonce": "goliath-json-0",
                }));
                let result = reduce(&revived, &answer);
                assert!(result.error.is_none());
                s.answer(json!(menace.id));

                assert!(result.state.pending.is_none());
                assert_eq!(hash_state(&result.state), hash_state(s.state()));
            }

            #[test]
            fn r63_trample_with_rush_it_attacks_a_unit_the_turn_it_enters_and_the_excess_hits_the_hero() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [GOLIATH, ANCHOR], "field": [VANILLA], "library": lib(3) },
                    "p2": { "hand": [ANCHOR], "field": [{ "def": VANILLA, "lane": 2 }], "health": 20 },
                }));
                let theirs = must(s.unit(P2, 2), "p2's Vanilla");
                let goliath = play_goliath(&mut s);
                s.answer(json!(goliath.id));

                s.attack(&goliath, &theirs);

                s.expect_in_zone(&theirs, "graveyard");
                s.expect_health(P2, 17);
            }

            #[test]
            fn s2_4_a_full_hand_burns_the_draw_an_empty_deck_makes_it_fatigue() {
                crate::register_all();
                let mut hand = vec![GOLIATH];
                hand.extend([ANCHOR; 9]);
                let mut full = scenario(json!({
                    "p1": { "hand": hand, "field": [VANILLA, CRAWLER], "library": lib(6) },
                    "p2": { "hand": [ANCHOR] },
                }));
                play_goliath(&mut full);
                let crawler = full.card(CRAWLER).clone();
                full.answer(json!(crawler.id));
                // 9 in hand after the play, and four draws (three of the Crawler's, one of its own): one fills it, three burn.
                assert_eq!(full.hand(P1).len(), 10);
                assert_eq!(full.events().iter().map(js).filter(|event| event["type"] == "burned").count(), 3);

                let mut empty = ready(false, json!({ "library": 0 }));
                let goliath = play_goliath(&mut empty);
                empty.answer(json!(goliath.id));
                assert_eq!(
                    empty
                        .last_events()
                        .iter()
                        .map(js)
                        .filter(|event| event["type"] == "fatigue" && event["player"] == "p1")
                        .count(),
                    1,
                );
            }

            #[test]
            fn r386_an_upgrade_of_its_tokens_places_four_of_its_draw_draws_2() {
                crate::register_all();
                let mut s = ready(false, json!({}));
                step_param(s.card_mut(GOLIATH), "tokens", 1);
                step_param(s.card_mut(GOLIATH), "draw", 1);
                play_goliath(&mut s);
                let menace = s.card(MENACE).clone();

                s.answer(json!(menace.id));

                assert!(s.state().pending.is_none());
                assert_eq!(s.card(&menace).counters.plague, Some(4));
                assert_eq!(draws_by(s.last_events(), P1), 2);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn is_a_14_14_with_rush_and_trample_the_same_tribute_three_placements_on_the_one_pick_then_draw_3() {
                crate::register_all();
                let mut s = ready(true, json!({}));
                let mut alone = scenario(json!({ "p1": { "hand": [{ "def": GOLIATH, "radiant": true }, ANCHOR] } }));
                alone.expect_refused_with(|s| s.play(GOLIATH, json!({})), "Tribute");
                let goliath = play_goliath(&mut s);
                let menace = s.card(MENACE).clone();

                s.expect_stats(&goliath, json!({ "attack": 14, "health": 14 }));
                s.answer(json!(menace.id));

                assert!(s.state().pending.is_none());
                assert_eq!(s.card(&menace).counters.plague, Some(3));
                assert_eq!(draws_by(s.last_events(), P1), 3);
            }

            #[test]
            fn r386_a_degrade_of_its_draw_draws_2_of_its_tokens_places_two() {
                crate::register_all();
                let mut s = ready(true, json!({}));
                step_param(s.card_mut(GOLIATH), "draw", -1);
                step_param(s.card_mut(GOLIATH), "tokens", -1);
                play_goliath(&mut s);
                let menace = s.card(MENACE).clone();

                s.answer(json!(menace.id));

                assert!(s.state().pending.is_none());
                assert_eq!(s.card(&menace).counters.plague, Some(2));
                assert_eq!(draws_by(s.last_events(), P1), 2);
            }
        }
    }
}
