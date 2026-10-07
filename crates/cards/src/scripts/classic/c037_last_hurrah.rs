//! C #37 Last Hurrah (SPEC §8.6 row 37). Spell, cost 0, Epic.
//!   Base:    "Draw your deck. At the end of this turn, discard your hand."
//!   Radiant: "Draw your deck. At the end of your next turn, discard your hand."
//!
//! "Draw your deck" is R58's "draw your whole library": as many draws as the library holds when the
//! effect starts, each an ordinary §2.4 draw — a cast-on-draw card is cast (R58), a card past the hand
//! cap of 10 burns into the graveyard (R4, R317), and a draw a draw limit stops does not happen at all
//! (§2.4). An empty library means no draws, so no fatigue.
//!
//! Then a delayed effect (§10.1, `discardHandAtTurnEnd`) discards the whole hand, with no prompt (there
//! is nothing to choose), in the end-of-turn delayed-effect step (R62): at the end of this turn on the
//! base face, at the end of its controller's next turn on the Radiant face, taking the hand held then.
//! It is a discard (§6.3), so C #64 Malzahar's Recycler sees each card. By design a fatigue clock.

use jackioh_engine::prelude::*;
use jackioh_engine::effects::{discard_hand_at_turn_end, draw};

pub const ID: &str = "classic-037";

/// `turn`: TS `"this" | "next"`.
fn last_hurrah(turn: &'static str) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![
                draw(json_as(json!({ "count": zone_count(&ctx.state, ctx.controller, OffFieldZone::Library) }))),
                discard_hand_at_turn_end(json_as(json!({ "turn": turn }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = last_hurrah("this");

    let radiant = last_hurrah("next");

    CardScripts { base, radiant }
}

// C #37 Last Hurrah — SPEC §8.6 row 37, BUILD M9 Classic row C 37: "Draws your whole deck, as many
// draws as its size when the effect starts (R58), the cards past 10 burning (§2.4, R317) and a draw
// limit stopping the rest; an empty deck draws nothing and takes no fatigue; then at the end of this
// turn, in the end-of-turn delayed-effect step (R62), your whole hand is discarded with no prompt, a
// discard (C #64 sees it); the drawn cards are never named in the opponent's view; radiant: the discard
// comes at the end of your next turn instead, taking the hand you hold then, and nothing is discarded
// at this turn's end; no tuned numbers".
//
// The draw-limit case uses C #4 Palantir's aura ("your opponent can't draw more than 1 card each
// turn"), whose own test file proves the limit in full.
#[cfg(test)]
mod tests {
    use super::{script, ID};
    use jackioh_engine::testkit::*;

    const HURRAH: &str = "classic-037";
    const PALANTIR: &str = "classic-004";
    const FILLER: &str = "core-005"; // (1) Spell.
    const VANILLA: &str = "core-008";
    const TIMMY: &str = "core-011";
    const MENACE: &str = "core-019";
    const POINTMASTER: &str = "core-020";
    const FELINORS: &str = "core-012";

    /// Five distinct cards, top first, so each draw is visible by def.
    const DECK: [&str; 5] = [VANILLA, TIMMY, MENACE, POINTMASTER, FELINORS];

    use crate::js;

    /// `{ ...base, ...over }`: the TS object spread, the override's keys winning.
    fn spread(mut base: Value, over: Value) -> Value {
        if let (Some(into), Some(from)) = (base.as_object_mut(), over.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        base
    }

    /// `p1`, `p2`: TS `SideSetup` overrides (`json!({})` for none); `radiant_face` defaults to false in TS.
    fn setup(p1: Value, p2: Value, radiant_face: bool) -> Scenario {
        scenario(json!({
            "p1": spread(json!({ "hand": [{ "def": HURRAH, "radiant": radiant_face }, "core-010"], "library": DECK }), p1),
            "p2": spread(json!({ "hand": [FILLER, FILLER], "library": [FILLER, FILLER, FILLER, FILLER] }), p2),
        }))
    }

    fn count(events: &[GameEvent], type_: &str) -> usize {
        events.iter().map(js).filter(|event| event["type"] == type_).count()
    }

    /// TS `expect(have).toEqual(expect.arrayContaining(want))`.
    fn contains_all(have: &[String], want: &[String]) -> bool {
        want.iter().all(|item| have.contains(item))
    }

    /// TS `Array.prototype.indexOf`: the first position, or -1.
    fn index_of(types: &[String], type_: &str) -> i64 {
        types.iter().position(|t| t == type_).map_or(-1, |at| at as i64)
    }

    /// TS `Array.prototype.lastIndexOf`: the last position, or -1.
    fn last_index_of(types: &[String], type_: &str) -> i64 {
        types.iter().rposition(|t| t == type_).map_or(-1, |at| at as i64)
    }

    mod c_37_last_hurrah {
        use super::*;

        #[test]
        fn has_no_declared_numbers() {
            crate::register_all();
            let def = js(&registered_catalog()[ID]);
            assert_eq!(def["id"], HURRAH);
            assert!(def["params"].is_null());
            let scripts = script();
            assert!(scripts.base.cry.is_some());
            assert!(scripts.radiant.cry.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn r58_draws_your_whole_deck_as_many_draws_as_it_holds_when_the_effect_starts() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);

                s.play(HURRAH, json!({}));

                assert_eq!(s.pile(PlayerId::P1, "library").len(), 0);
                let hand: Vec<String> = s.hand(PlayerId::P1).iter().map(|card| card.def_id.clone()).collect();
                let mut want = vec!["core-010"];
                want.extend(DECK);
                assert_eq!(hand, want);
                assert_eq!(count(&s.last_events(), "drawn"), DECK.len());
                assert_eq!(count(&s.last_events(), "fatigue"), 0);
            }

            #[test]
            fn r4_r317_the_cards_past_the_hand_cap_of_10_burn_into_the_graveyard_both_players_reading_which() {
                crate::register_all();
                let eight = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];
                let mut hand = vec![HURRAH, "core-010"];
                hand.extend(eight);
                let mut s = setup(json!({ "hand": hand }), json!({}), false);
                assert_eq!(s.hand(PlayerId::P1).len(), 10);

                s.play(HURRAH, json!({}));

                assert_eq!(s.hand(PlayerId::P1).len(), HAND_CAP as usize);
                assert_eq!(s.pile(PlayerId::P1, "library").len(), 0);
                assert_eq!(count(&s.last_events(), "burned"), DECK.len() - 1);
                let grave: Vec<String> =
                    s.pile(PlayerId::P1, "graveyard").iter().map(|card| card.def_id.clone()).collect();
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
                assert!(!serde_json::to_string(&s.view(PlayerId::P2)).expect("a view serialises").contains(VANILLA));
            }

            #[test]
            fn s2_4_a_draw_limit_stops_the_rest_those_draws_do_not_happen_at_all() {
                crate::register_all();
                let mut s = setup(json!({}), json!({ "backrow": [{ "def": PALANTIR, "faceUp": true }] }), false);

                s.play(HURRAH, json!({}));

                assert_eq!(count(&s.last_events(), "drawn"), 1);
                assert_eq!(s.pile(PlayerId::P1, "library").len(), DECK.len() - 1);
                assert_eq!(count(&s.last_events(), "fatigue"), 0);
                assert!(count(&s.last_events(), "drawLimited") > 0);
            }

            #[test]
            fn an_empty_deck_draws_nothing_and_takes_no_fatigue() {
                crate::register_all();
                let mut s = setup(json!({ "library": [] }), json!({}), false);

                s.play(HURRAH, json!({}));

                assert_eq!(count(&s.last_events(), "drawn"), 0);
                assert_eq!(count(&s.last_events(), "fatigue"), 0);
                s.expect_health(PlayerId::P1, 30);
            }

            #[test]
            fn r97_the_drawn_cards_are_never_named_in_the_opponents_view() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);

                s.play(HURRAH, json!({}));

                let seen = serde_json::to_string(&s.view(PlayerId::P2)).expect("a view serialises");
                for def_id in [TIMMY, MENACE, POINTMASTER, FELINORS] {
                    assert!(!seen.contains(def_id));
                }
            }

            #[test]
            fn r62_at_the_end_of_this_turn_the_whole_hand_is_discarded_with_no_prompt_before_the_next_turn_starts() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                s.play(HURRAH, json!({}));
                let held: Vec<String> = s.hand(PlayerId::P1).iter().map(|card| card.id.clone()).collect();

                s.end_turn();

                assert_eq!(s.hand(PlayerId::P1).len(), 0);
                let grave: Vec<String> = s.pile(PlayerId::P1, "graveyard").iter().map(|card| card.id.clone()).collect();
                assert!(contains_all(&grave, &held));
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
                assert_eq!(count(&s.last_events(), "promptOpened"), 0);
            }

            #[test]
            fn s6_3_each_card_goes_as_a_discard_with_its_own_discarded_event() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                s.play(HURRAH, json!({}));
                let held = s.hand(PlayerId::P1).len();

                s.end_turn();

                assert_eq!(
                    s.last_events()
                        .iter()
                        .map(js)
                        .filter(|event| event["type"] == "discarded" && event["owner"] == "p1")
                        .count(),
                    held,
                );
            }

            #[test]
            fn it_discards_only_its_controllers_hand() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                s.play(HURRAH, json!({}));

                s.end_turn();

                // p2 has drawn for their turn and still holds everything.
                assert!(s.hand(PlayerId::P2).len() >= 2);
            }

            #[test]
            fn the_discard_takes_the_hand_as_it_is_then_cards_played_in_between_are_not_discarded() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), false);
                s.play(HURRAH, json!({}));
                s.play(TIMMY, json!({ "zone": 1 }));

                s.end_turn();

                s.expect_in_zone(TIMMY, "field");
                assert_eq!(s.hand(PlayerId::P1).len(), 0);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r58_draws_your_whole_deck() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), true);

                s.play(HURRAH, json!({}));

                assert_eq!(s.pile(PlayerId::P1, "library").len(), 0);
                assert_eq!(s.hand(PlayerId::P1).len(), DECK.len() + 1);
            }

            #[test]
            fn nothing_is_discarded_at_this_turns_end() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), true);
                s.play(HURRAH, json!({}));
                let held = s.hand(PlayerId::P1).len();

                s.end_turn();

                assert_eq!(s.hand(PlayerId::P1).len(), held);
                assert!(!s
                    .last_events()
                    .iter()
                    .map(js)
                    .any(|event| event["type"] == "discarded" && event["owner"] == "p1"));
            }

            #[test]
            fn r62_at_the_end_of_your_next_turn_it_discards_the_hand_you_hold_then() {
                crate::register_all();
                let mut s = setup(json!({}), json!({}), true);
                s.play(HURRAH, json!({}));
                s.play(TIMMY, json!({ "zone": 1 }));
                s.end_turn();
                s.end_turn();
                assert_eq!(s.state().active, PlayerId::P1);
                // p1's start-of-turn draw found an empty deck: fatigue, no card.
                let held: Vec<String> = s.hand(PlayerId::P1).iter().map(|card| card.id.clone()).collect();
                assert!(!held.is_empty());

                s.end_turn();

                assert_eq!(s.hand(PlayerId::P1).len(), 0);
                let grave: Vec<String> = s.pile(PlayerId::P1, "graveyard").iter().map(|card| card.id.clone()).collect();
                assert!(contains_all(&grave, &held));
                s.expect_in_zone(TIMMY, "field");
            }
        }
    }
}
