//! C #37 Last Hurrah (SPEC §8.6 row 37). Spell, cost 0, Epic.
//!   Base:    "Draw your deck."
//!   Radiant: "Draw your deck. Give {flashback} random cards in your hand Flashback." (flashback 4)
//!   Through v0.6.6e: "Draw your deck. At the end of this turn (Radiant: of your next turn), discard
//!   {discard|card|cards}." (`script_v0_6_6e`, kept, R767000; discard 4, 6 until issue #3951).
//!   Through v0.5.11r: "… discard your hand." (`script_v0_5_11o`, kept, R767000).
//!
//! "Draw your deck" is R58's "draw your whole library": as many draws as the library holds when the
//! effect starts, each an ordinary §2.4 draw — a cast-on-draw card is cast (R58), a card past the hand
//! cap of 10 burns (R4, R317), and a draw a draw limit stops does not happen (§2.4). An empty library
//! means no draws, so no fatigue. Issue #4154 took the delayed discard and its `discard` off both faces
//! (R4154201). The Radiant face then gives `flashback` (R386, the Radiant face's alone) different random
//! cards of the hand, drawn as the clause is reached from the cards that lack Flashback (R60, R129),
//! Flashback for good (`gain_keyword_for_good`, R4154202): kept in their tuning, so a card played or
//! discarded from the hand still has it in the graveyard (R215), where it is played once (R783012,
//! R783013). The other player learns that cards of that hand gained it, never which (R177).
//! Through v0.6.6e a delayed effect (§10.1, `discardHandAtTurnEnd`) discarded `discard` random cards of
//! the hand (R682, R3134000) at the end of the turn it was played on (Radiant: its controller's next).

use jackioh_engine::effects::{discard_hand_at_turn_end, draw};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-037";

/// The declared number the faces through v0.6.6e printed: how many cards the delayed discard took
/// (R3134000).
const DISCARD: &str = "discard";
/// The declared number the Radiant face prints since issue #4154: how many hand cards gain Flashback.
const FLASHBACK: &str = "flashback";

/// R58: as many draws as the library holds as the effect starts.
fn draw_deck(ctx: &EffectContext<'_>) -> Effect {
    draw(json_as(json!({ "count": zone_count(ctx.state, ctx.controller, OffFieldZone::Library) })))
}

/// R4154202: `count` different cards of the controller's hand that lack Flashback, drawn as the clause
/// is reached; all of them, with no draw, when they are no more than `count` (R129).
fn flashback_picks(c: &mut EffectContext<'_>, count: usize) -> Vec<String> {
    let candidates: Vec<CardInstance> = zone_cards(&*c.state, c.controller, OffFieldZone::Hand)
        .into_iter()
        .filter(|card| card.flashback_spent != Some(true) && !unit_has(&*c.state, card, KeywordKind::Flashback))
        .collect();
    if candidates.len() <= count {
        return candidates.into_iter().map(|card| card.id).collect();
    }
    c.sink.rng.shuffle(&candidates).into_iter().take(count).map(|card| card.id).collect()
}

/// One face since issue #4154: draw the deck, then, with `flashback`, give `flashback` random cards of
/// the hand Flashback for good (R4154201, R4154202).
fn last_hurrah(flashback: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects = vec![draw_deck(ctx)];
            if flashback {
                let count = param(&*ctx, FLASHBACK).max(0) as usize;
                effects.push(for_each_card(ForEachCardArgs {
                    cards: Arc::new(move |c: &mut EffectContext<'_>| flashback_picks(c, count)),
                    each: Arc::new(|instance_id: &str| {
                        gain_keyword_for_good(json_as(json!({
                            "keyword": { "kind": "Flashback" },
                            "instanceId": instance_id,
                        })))
                    }),
                }));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: last_hurrah(false),
        radiant: last_hurrah(true),
    }
}

/// One face through v0.6.6e: draw the deck, then the delayed discard at `turn`'s end (R458); `counted`
/// is whether it takes `discard` cards (R3134000) rather than the whole hand, as v0.5.11r's did.
fn last_hurrah_discarding(turn: &'static str, counted: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let delayed = if counted {
                json!({ "turn": turn, "count": param(ctx, DISCARD) })
            } else {
                json!({ "turn": turn })
            };
            vec![draw_deck(ctx), discard_hand_at_turn_end(json_as(delayed))]
        })),
        ..Script::default()
    }
}

/// R767000: the scripts v0.6.6e and the versions after v0.5.11r ran, kept for their replays: `discard`
/// random cards at the end of this turn, on the Radiant face of the controller's next (issue #4154).
pub fn script_v0_6_6e() -> CardScripts {
    CardScripts { base: last_hurrah_discarding("this", true), radiant: last_hurrah_discarding("next", true) }
}

/// R767000: the scripts v0.5.11r and the versions before it ran, kept for their replays: the delayed
/// discard took the whole hand, both faces.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts { base: last_hurrah_discarding("this", false), radiant: last_hurrah_discarding("next", false) }
}

// C #37 Last Hurrah — SPEC §8.6 row 37, BUILD M9 Classic row C 37: draws the whole deck (R58), the
// cards past 10 burning (§2.4, R317) and a draw limit stopping the rest; an empty deck takes no
// fatigue; nothing is discarded since issue #4154 (R4154201); radiant: `flashback` random cards of the
// hand gain Flashback for good, kept into the graveyard (R4154202). The script kept for v0.6.6e discards
// `discard` random cards at the end of this turn (radiant: the controller's next, R62, R682, R3134000),
// and the one kept for v0.5.11r the whole hand (R767000).
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{script_v0_5_11o, script_v0_6_6e};
    use crate::def_ids;
    use crate::merged;
    use crate::scenario;
    use jackioh_engine::testkit::*;

    const HURRAH: &str = "classic-037";
    const PALANTIR: &str = "classic-004";
    const RECYCLER: &str = "classic-064"; // Malzahar's Recycler: whenever you discard cards, draw that many
    const FILLER: &str = "core-005"; // Stockpile: a Spell
    const VANILLA: &str = "core-008";
    const TIMMY: &str = "core-011";
    const MENACE: &str = "core-019";
    const POINTMASTER: &str = "core-020";
    const FELINORS: &str = "core-012";

    const DECK: [&str; 5] = [VANILLA, TIMMY, MENACE, POINTMASTER, FELINORS];

    use crate::js;

    fn setup(p1: Value, p2: Value, radiant_face: bool) -> Scenario {
        scenario(json!({
            "p1": merged(json!({ "hand": [{ "def": HURRAH, "radiant": radiant_face }, "core-010"], "library": DECK }), p1),
            "p2": merged(json!({ "hand": [FILLER, FILLER], "library": [FILLER, FILLER, FILLER, FILLER] }), p2),
        }))
    }

    /// Last Hurrah and Rapid Draw, then two more cards: with the five the deck draws, a hand of 8.
    fn eight_cards(radiant_face: bool) -> Scenario {
        setup(
            json!({ "hand": [{ "def": HURRAH, "radiant": radiant_face }, "core-010", FILLER, FILLER] }),
            json!({}),
            radiant_face,
        )
    }

    /// The instance ids p1 discarded, in order, over the events so far.
    fn discarded(s: &Scenario) -> Vec<String> {
        s.last_events()
            .iter()
            .map(js)
            .filter(|event| event["type"] == "discarded" && event["owner"] == "p1")
            .filter_map(|event| event["instanceId"].as_str().map(str::to_string))
            .collect()
    }

    fn hand_ids(s: &Scenario) -> Vec<String> {
        s.hand(PlayerId::P1).iter().map(|card| card.id.clone()).collect()
    }

    fn grave_ids(s: &Scenario) -> Vec<String> {
        s.pile(PlayerId::P1, "graveyard").iter().map(|card| card.id.clone()).collect()
    }

    fn contains_all(have: &[String], want: &[String]) -> bool {
        want.iter().all(|item| have.contains(item))
    }

    fn index_of(types: &[String], type_: &str) -> i64 {
        types.iter().position(|t| t == type_).map_or(-1, |at| at as i64)
    }

    fn last_index_of(types: &[String], type_: &str) -> i64 {
        types.iter().rposition(|t| t == type_).map_or(-1, |at| at as i64)
    }

    /// R767000: `run` under the scripts v0.6.6e ran and its declared `discard`, 4 on both faces (issue
    /// #4154 took both off).
    fn discarding<T>(run: impl FnOnce() -> T) -> T {
        crate::under_kept_as(
            HURRAH,
            script_v0_6_6e(),
            |def| {
                def.params = Some(vec![json_as(json!({
                    "key": "discard", "base": 4, "radiant": 4, "better": "down", "step": 1, "min": 1,
                }))]);
            },
            run,
        )
    }

    /// p1's hand cards that have Flashback now, read through the layers.
    fn with_flashback(s: &Scenario) -> Vec<String> {
        s.hand(PlayerId::P1)
            .iter()
            .filter(|card| unit_has(s.state(), card, KeywordKind::Flashback))
            .map(|card| card.id.clone())
            .collect()
    }

    mod c_37_last_hurrah {
        use super::*;

        #[test]
        fn r4154201_declares_flashback_4_on_the_radiant_face_alone_and_no_discard() {
            crate::register_all();
            let def = js(&crate::card_def(HURRAH));
            assert_eq!(
                def["params"],
                json!([{ "key": "flashback", "base": 4, "radiant": 4, "better": "up", "step": 1, "min": 1, "tunedOn": "radiant" }]),
            );
            assert_eq!(def["base"]["text"], json!("Draw your deck."));
            for face in ["base", "radiant"] {
                assert!(!def[face]["text"].as_str().unwrap_or_default().contains("discard"));
            }
        }

        mod base {
            use super::*;

            #[test]
            fn r58_draws_your_whole_deck_as_many_draws_as_it_holds_when_the_effect_starts() {
                let mut s = setup(json!({}), json!({}), false);

                s.play(HURRAH, json!({}));

                assert_eq!(s.pile(PlayerId::P1, "library").len(), 0);
                let hand = def_ids(&s.hand(PlayerId::P1));
                let mut want = vec!["core-010"];
                want.extend(DECK);
                assert_eq!(hand, want);
                assert_eq!(s.last_count("drawn"), DECK.len());
                assert_eq!(s.last_count("fatigue"), 0);
            }

            #[test]
            fn r4_r317_the_cards_past_the_hand_cap_of_10_burn_into_the_graveyard_both_players_reading_which() {
                let eight = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];
                let mut hand = vec![HURRAH, "core-010"];
                hand.extend(eight);
                let mut s = setup(json!({ "hand": hand }), json!({}), false);
                assert_eq!(s.hand(PlayerId::P1).len(), 10);

                s.play(HURRAH, json!({}));

                assert_eq!(s.hand(PlayerId::P1).len(), HAND_CAP as usize);
                assert_eq!(s.pile(PlayerId::P1, "library").len(), 0);
                assert_eq!(s.last_count("burned"), DECK.len() - 1);
                let grave = def_ids(&s.pile(PlayerId::P1, "graveyard"));
                let burnt_ones: Vec<String> = [MENACE, POINTMASTER, FELINORS].iter().map(|id| id.to_string()).collect();
                assert!(contains_all(&grave, &burnt_ones));
                // R317: both players read which cards burned; the one kept in hand stays unread by p2.
                let burned = |player: PlayerId| -> Vec<String> {
                    s.view(player)
                        .events
                        .iter()
                        .map(js)
                        .filter(|event| event["type"] == "burned")
                        .map(|event| event["defId"].as_str().unwrap_or_default().to_string())
                        .collect()
                };
                assert_eq!(burned(PlayerId::P2), burned(PlayerId::P1));
                assert!(contains_all(&burned(PlayerId::P2), &burnt_ones));
                assert!(!serde_json::to_string(&s.view(PlayerId::P2)).expect("a view serializes").contains(VANILLA));
            }

            #[test]
            fn s2_4_a_draw_limit_stops_the_rest_those_draws_do_not_happen_at_all() {
                let mut s = setup(
                    json!({ "hand": [{ "def": HURRAH, "costMod": 1 }, "core-010"] }),
                    json!({ "backrow": [{ "def": PALANTIR, "faceUp": true }] }),
                    false,
                );

                s.play(HURRAH, json!({}));

                assert_eq!(s.last_count("drawn"), 1);
                assert_eq!(s.pile(PlayerId::P1, "library").len(), DECK.len() - 1);
                assert_eq!(s.last_count("fatigue"), 0);
                assert!(s.last_count("drawLimited") > 0);
            }

            #[test]
            fn an_empty_deck_draws_nothing_and_takes_no_fatigue() {
                let mut s = setup(json!({ "library": [] }), json!({}), false);

                s.play(HURRAH, json!({}));

                assert_eq!(s.last_count("drawn"), 0);
                assert_eq!(s.last_count("fatigue"), 0);
                s.expect_health(PlayerId::P1, 30);
            }

            #[test]
            fn r97_the_drawn_cards_are_never_named_in_the_opponents_view() {
                let mut s = setup(json!({}), json!({}), false);

                s.play(HURRAH, json!({}));

                let seen = serde_json::to_string(&s.view(PlayerId::P2)).expect("a view serializes");
                for def_id in [TIMMY, MENACE, POINTMASTER, FELINORS] {
                    assert!(!seen.contains(def_id));
                }
            }

            #[test]
            fn r4154201_nothing_is_discarded_at_the_end_of_this_turn_or_the_next() {
                let mut s = eight_cards(false);
                s.play(HURRAH, json!({}));
                let held = s.hand(PlayerId::P1).len();
                assert_eq!(held, 8);
                assert!(with_flashback(&s).is_empty(), "the base face gives no Flashback");

                s.end_turn();
                s.end_turn();
                assert_eq!(s.state().active, PlayerId::P1);
                s.end_turn();

                assert!(discarded(&s).is_empty());
                assert_eq!(s.count("discarded"), 0);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r58_draws_your_whole_deck() {
                let mut s = setup(json!({}), json!({}), true);

                s.play(HURRAH, json!({}));

                assert_eq!(s.pile(PlayerId::P1, "library").len(), 0);
                assert_eq!(s.hand(PlayerId::P1).len(), DECK.len() + 1);
            }

            #[test]
            fn nothing_is_discarded_at_this_turns_end() {
                let mut s = setup(json!({}), json!({}), true);
                s.play(HURRAH, json!({}));
                let held = s.hand(PlayerId::P1).len();

                s.end_turn();

                assert_eq!(s.hand(PlayerId::P1).len(), held);
                assert!(discarded(&s).is_empty());
            }

            #[test]
            fn r4154202_four_random_cards_of_the_hand_gain_flashback_and_the_rest_do_not() {
                let mut s = setup(json!({}), json!({}), true);
                s.play(HURRAH, json!({}));
                let hand = hand_ids(&s);
                assert_eq!(hand.len(), DECK.len() + 1);
                let given = with_flashback(&s);
                assert_eq!(given.len(), 4);
                assert!(contains_all(&hand, &given));
                // R177: the other player reads that cards of that hand gained it, never which.
                let granted = |player: PlayerId| -> Vec<String> {
                    s.view(player)
                        .events
                        .iter()
                        .map(js)
                        .filter(|event| event["type"] == "keywordGranted")
                        .map(|event| event["instanceId"].as_str().unwrap_or_default().to_string())
                        .collect()
                };
                let mut mine = granted(PlayerId::P1);
                mine.sort();
                let mut want = given.clone();
                want.sort();
                assert_eq!(mine, want);
                assert_eq!(granted(PlayerId::P2), vec!["hidden".to_string(); 4]);
            }

            #[test]
            fn r4154202_a_hand_of_four_or_fewer_all_gain_it_and_keep_it_into_the_graveyard() {
                let mut s = setup(
                    json!({ "hand": [{ "def": HURRAH, "radiant": true }, FILLER, VANILLA], "library": [], "mana": 10 }),
                    json!({}),
                    true,
                );
                s.play(HURRAH, json!({}));
                let hand = hand_ids(&s);
                assert_eq!(hand.len(), 2);
                let mut given = with_flashback(&s);
                given.sort();
                let mut want = hand.clone();
                want.sort();
                assert_eq!(given, want, "both cards, with no draw to pick them");

                // R215 keeps tuning: played from the hand, the Spell reaches the graveyard with it.
                let stockpile = s.hand(PlayerId::P1).into_iter().find(|card| card.def_id == FILLER).expect("the Stockpile").id;
                s.play(&stockpile, json!({}));
                s.expect_in_zone(&stockpile, "graveyard");
                assert!(s.can_play(&stockpile), "its Flashback offers it from the graveyard (R783012)");
            }

            #[test]
            fn r4154202_a_card_that_has_flashback_is_passed_over() {
                let mut s = setup(
                    json!({ "hand": [{ "def": HURRAH, "radiant": true }, FILLER, VANILLA], "library": [] }),
                    json!({}),
                    true,
                );
                let vanilla = s.hand(PlayerId::P1).into_iter().find(|card| card.def_id == VANILLA).expect("the Vanilla").id;
                s.card_mut(&vanilla).granted_keywords.push(Keyword::Flashback);
                s.play(HURRAH, json!({}));
                assert_eq!(s.count("keywordGranted"), 1, "only the Stockpile gains it");
            }

            #[test]
            fn r386_flashback_reads_through_param() {
                let mut s = setup(json!({}), json!({}), true);
                assert_eq!(crate::degrade_number(&mut s, HURRAH, "flashback"), 3);
                s.play(HURRAH, json!({}));
                assert_eq!(with_flashback(&s).len(), 3);
            }
        }

        /// R767000: the scripts kept for v0.6.6e, which discarded `discard` cards (issue #4154).
        mod through_v0_6_6e {
            use super::*;

            mod base {
                use super::*;

                #[test]
                fn r3134000_r62_at_the_end_of_this_turn_4_of_8_cards_are_discarded_with_no_prompt_before_the_next_turn_starts() {
                    discarding(|| {
                        let mut s = eight_cards(false);
                        s.play(HURRAH, json!({}));
                        let held = hand_ids(&s);
                        assert_eq!(held.len(), 8);

                        s.end_turn();

                        assert_eq!(s.hand(PlayerId::P1).len(), 4);
                        let gone = discarded(&s);
                        assert_eq!(gone.len(), 4);
                        assert!(contains_all(&held, &gone));
                        assert!(contains_all(&grave_ids(&s), &gone));
                        // R62: `turnEnded` closes the end-of-turn triggers; the trap window and then the end-of-turn
                        // delayed effects follow it, all before the next turn starts.
                        let types: Vec<String> = s
                            .last_events()
                            .iter()
                            .map(|event| js(event)["type"].as_str().unwrap_or_default().to_string())
                            .collect();
                        let first_discard = index_of(&types, "discarded");
                        let ended = index_of(&types, "turnEnded");
                        let started = last_index_of(&types, "turnStarted");
                        assert!(ended >= 0);
                        assert!(first_discard > ended);
                        assert!(first_discard < started);
                        assert_eq!(s.last_count("promptOpened"), 0);
                    });
                }

                #[test]
                fn s6_3_each_card_goes_as_a_discard_with_its_own_discarded_event() {
                    discarding(|| {
                        let mut s = eight_cards(false);
                        s.play(HURRAH, json!({}));

                        s.end_turn();

                        assert_eq!(discarded(&s).len(), 4);
                        assert_eq!(s.last_count("discarded"), 4);
                    });
                }

                #[test]
                fn r458_c_64_malzahars_recycler_sees_each_card_and_draws_that_many() {
                    discarding(|| {
                        let mut s = setup(
                            json!({ "hand": [HURRAH, "core-010", FILLER, FILLER], "backrow": [RECYCLER] }),
                            json!({}),
                            false,
                        );
                        s.play(HURRAH, json!({}));
                        // The deck is drawn; Recycler (base) draws as many as the effect discarded, here from
                        // the empty deck: one fatigue hit each.
                        s.end_turn();

                        assert_eq!(discarded(&s).len(), 4);
                        assert_eq!(s.last_count("fatigue"), 4);
                    });
                }

                #[test]
                fn r682_which_4_go_is_random_each_card_of_the_hand_is_discarded_under_some_seed() {
                    discarding(|| {
                        let mut kept_sets: BTreeSet<Vec<String>> = BTreeSet::new();
                        let mut ever_gone: BTreeSet<String> = BTreeSet::new();
                        for n in 0..8 {
                            let mut s = scenario(json!({
                                "seed": format!("hurrah-random-{n}"),
                                "p1": { "hand": [HURRAH, "core-010", FILLER, FILLER], "library": DECK },
                                "p2": { "hand": [FILLER, FILLER], "library": [FILLER, FILLER, FILLER, FILLER] },
                            }));
                            s.play(HURRAH, json!({}));
                            s.end_turn();
                            kept_sets.insert(hand_ids(&s));
                            ever_gone.extend(discarded(&s));
                        }
                        assert!(kept_sets.len() > 1, "the same 4 cards stayed under every seed");
                        assert!(ever_gone.len() > 4, "the same 4 cards went under every seed");
                    });
                }

                #[test]
                fn r3134000_a_hand_of_exactly_4_loses_all_4() {
                    discarding(|| {
                        let mut s = setup(json!({ "library": [VANILLA, TIMMY, MENACE] }), json!({}), false);
                        s.play(HURRAH, json!({}));
                        assert_eq!(s.hand(PlayerId::P1).len(), 4);

                        s.end_turn();

                        assert_eq!(s.hand(PlayerId::P1).len(), 0);
                        assert_eq!(discarded(&s).len(), 4);
                    });
                }

                #[test]
                fn r682_a_hand_of_fewer_than_4_discards_all_there_are() {
                    discarding(|| {
                        let mut s = setup(json!({ "library": [VANILLA, TIMMY, MENACE] }), json!({}), false);
                        s.play(HURRAH, json!({}));
                        s.play(TIMMY, json!({ "zone": 1 }));
                        let held = hand_ids(&s);
                        assert_eq!(held.len(), 3);

                        s.end_turn();

                        assert_eq!(s.hand(PlayerId::P1).len(), 0);
                        let gone = discarded(&s);
                        assert_eq!(gone.len(), 3);
                        assert!(contains_all(&gone, &held));
                    });
                }

                #[test]
                fn it_discards_only_its_controllers_hand() {
                    discarding(|| {
                        let mut s = setup(json!({}), json!({}), false);
                        s.play(HURRAH, json!({}));

                        s.end_turn();

                        assert!(s.hand(PlayerId::P2).len() >= 2);
                        assert!(!s
                            .last_events()
                            .iter()
                            .map(js)
                            .any(|event| event["type"] == "discarded" && event["owner"] == "p2"));
                    });
                }

                #[test]
                fn the_discard_takes_the_hand_as_it_is_then_cards_played_in_between_are_not_counted() {
                    discarding(|| {
                        let mut s = eight_cards(false);
                        s.play(HURRAH, json!({}));
                        s.play(TIMMY, json!({ "zone": 1 }));
                        assert_eq!(s.hand(PlayerId::P1).len(), 7);

                        s.end_turn();

                        s.expect_in_zone(TIMMY, "field");
                        // 4 of the 7 cards still held, not 4 of 8: three stay.
                        assert_eq!(s.hand(PlayerId::P1).len(), 3);
                        assert_eq!(discarded(&s).len(), 4);
                    });
                }

                #[test]
                fn r386_discard_reads_through_param_an_upgrade_discards_3_and_a_degrade_5() {
                    discarding(|| {
                        let mut s = eight_cards(false);
                        assert_eq!(crate::upgrade_number(&mut s, HURRAH, "discard"), 3);
                        s.play(HURRAH, json!({}));
                        s.end_turn();
                        assert_eq!(discarded(&s).len(), 3);
                        assert_eq!(s.hand(PlayerId::P1).len(), 5);

                        let mut s = eight_cards(false);
                        assert_eq!(crate::degrade_number(&mut s, HURRAH, "discard"), 5);
                        s.play(HURRAH, json!({}));
                        s.end_turn();
                        assert_eq!(discarded(&s).len(), 5);
                        assert_eq!(s.hand(PlayerId::P1).len(), 3);
                    });
                }

                #[test]
                fn r386_discard_cannot_go_below_1() {
                    discarding(|| {
                        let mut s = eight_cards(false);
                        for step in (1..=3).rev() {
                            assert_eq!(crate::upgrade_number(&mut s, HURRAH, "discard"), step);
                        }
                        assert!(!crate::can_upgrade_number(&s, HURRAH, "discard"));
                        s.play(HURRAH, json!({}));
                        s.end_turn();
                        assert_eq!(discarded(&s).len(), 1);
                        assert_eq!(s.hand(PlayerId::P1).len(), 7);
                    });
                }
            }

            mod radiant {
                use super::*;

                #[test]
                fn r3134000_r62_at_the_end_of_your_next_turn_it_discards_4_of_the_hand_you_hold_then() {
                    discarding(|| {
                        let mut s = eight_cards(true);
                        s.play(HURRAH, json!({}));
                        s.play(TIMMY, json!({ "zone": 1 }));
                        s.end_turn();
                        s.end_turn();
                        assert_eq!(s.state().active, PlayerId::P1);
                        let held = hand_ids(&s);
                        assert_eq!(held.len(), 7);

                        s.end_turn();

                        assert_eq!(s.hand(PlayerId::P1).len(), 3);
                        let gone = discarded(&s);
                        assert_eq!(gone.len(), 4);
                        assert!(contains_all(&held, &gone));
                        assert!(contains_all(&grave_ids(&s), &gone));
                        s.expect_in_zone(TIMMY, "field");
                    });
                }

                #[test]
                fn r682_a_hand_of_fewer_than_4_then_loses_all_there_are() {
                    discarding(|| {
                        let mut s = setup(json!({ "library": [VANILLA, TIMMY, MENACE] }), json!({}), true);
                        s.play(HURRAH, json!({}));
                        s.play(TIMMY, json!({ "zone": 1 }));
                        s.end_turn();
                        s.end_turn();
                        let held = hand_ids(&s);
                        assert_eq!(held.len(), 3);

                        s.end_turn();

                        assert_eq!(s.hand(PlayerId::P1).len(), 0);
                        let gone = discarded(&s);
                        assert_eq!(gone.len(), 3);
                        assert!(contains_all(&gone, &held));
                    });
                }
            }
        }

        mod through_v0_5_11o {
            use super::*;

            #[test]
            fn r767000_r458_the_kept_script_discards_the_whole_hand_on_both_faces() {
                for radiant_face in [false, true] {
                    crate::under_kept(HURRAH, script_v0_5_11o(), || {
                        let mut s = eight_cards(radiant_face);
                        s.play(HURRAH, json!({}));
                        s.end_turn();
                        if radiant_face {
                            assert_eq!(s.hand(PlayerId::P1).len(), 8);
                            s.end_turn();
                            s.end_turn();
                        }
                        assert_eq!(s.hand(PlayerId::P1).len(), 0);
                        assert_eq!(discarded(&s).len(), 8);
                    });
                }
            }
        }
    }
}
