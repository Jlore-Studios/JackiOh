//! C #58 Common Resources (SPEC §8.6 row 58, BUILD M9 Classic row C 58). (2) Field Spell, Common.
//!   Base:    "Start of turn: Draw {cards|card|cards} from the bottom of your opponent's deck." (1)
//!   Radiant: "Start of turn and end of turn: Draw {cards|card|cards} from the bottom of your
//!            opponent's deck." (1)
//!   Engine:  "Cards between players' piles (§6.3 Steal, §3.2): a draw of yours taken from the bottom of
//!            the opponent's deck, at the start of your turn (Radiant: and at the end of it); it is your
//!            draw for your hand cap, cast on draw, your draw limit (§2.4) and your per-turn draw count
//!            (§10.1), and the card becomes yours (its owner changes, R12). An empty enemy deck gives
//!            nothing, and fatigue for no one. Tunes: cards 1 ↑."
//!
//! Each draw is B5 E16's `drawFromOpponent`: one draw of this card's controller's out of the bottom of
//! the other player's library (`ownership.drawFromLibraryOf`), the owner changing as it leaves (R12, a
//! `stolen` event hidden per zone, R97, B5 E16), then §2.4's draw finishing it as the drawer's own — the
//! draw counters and `drawn`, a cast on draw for the drawer (R58), the drawer's hand cap (a burn goes
//! to the drawer's graveyard, R317). "Draw N" is N draws (§2.4), so the declared count (`param`, R386)
//! is that many effects, each its own draw, and a cast on draw that asks pauses the list between them
//! (R113). The hooks are §6.2's start- and end-of-turn triggers: their controller's turn only, while the
//! card acts on the field (R153), the start one before the turn's own draw (§2.2, R62).

use jackioh_engine::prelude::*;
use jackioh_engine::effects::draw_from_opponent;

pub const ID: &str = "classic-058";

/// "Draw {cards} from the bottom of your opponent's deck": that many separate draws (§2.4).
fn draw_bottoms(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    (0..param(ctx, "cards")).map(|_| draw_from_opponent(json_as(json!({ "end": "bottom" })))).collect()
}

pub fn script() -> CardScripts {
    let base = Script {
        start_of_turn: Some(hook(draw_bottoms)),
        ..Script::default()
    };

    let radiant = Script {
        start_of_turn: Some(hook(draw_bottoms)),
        end_of_turn: Some(hook(draw_bottoms)),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #58 Common Resources — SPEC §8.6 row 58, BUILD M9 Classic row C 58: "Start of your turn: a draw of
// yours taken from the bottom of the opponent's deck, the card becoming yours (its owner changes, R12),
// under your hand cap (a burn goes to your graveyard, R317), your cast on draw and your draw limit; an
// empty enemy deck gives nothing and nobody takes fatigue; the opponent's view never names the card
// (R97) and no event carries a deck position; radiant: at the start and at the end of your turn; its
// tuned number (cards) reads through `param()` (R386)".
//
// The draw limit is C #49 Anti-Greed Machine's (B5 E3); the start-of-turn trigger runs before the
// turn's own draw (§2.2, R62), so it is the turn's first draw and the turn's own draw is the one a
// limit of 1 stops.
#[cfg(test)]
mod tests {
    use super::{script, ID};
    use jackioh_engine::testkit::*;

    const RESOURCES: &str = "classic-058";
    const MACHINE: &str = "classic-049"; // Radiant: your opponent can't draw more than 1 card each turn.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const MENACE: &str = "core-019"; // (3) Unit 9/9
    const STOCKPILE: &str = "core-005"; // (1) Spell
    const CN_VIRUS: &str = "core-090-1"; // (1) Spell, Cast on draw: take 1 damage
    const FILLER: &str = "core-010"; // (0) Spell
    const HINDER: &str = "core-021"; // (0) Spell, Cast on draw: your opponent has 1 less mana next turn. Discard 1.
    const INCOME_TAX: &str = "classic-009"; // Trap: when the cards your opponent has drawn in a turn reach 2 …

    use crate::js;

    /// The `drawn` events of `player`'s, as JSON.
    fn drawn_by(events: &[GameEvent], player: PlayerId) -> Vec<Value> {
        let player = js(&player);
        events.iter().map(js).filter(|event| event["type"] == "drawn" && event["player"] == player).collect()
    }

    /// `opts[key]`, or `fallback` where the TS default (`??`) applies.
    fn or(opts: &Value, key: &str, fallback: Value) -> Value {
        if opts[key].is_null() { fallback } else { opts[key].clone() }
    }

    /// p1's Common Resources face-up in the backrow, on p2's turn; `endTurn()` starts p1's turn.
    /// `opts`: `radiant`, `p1Library`, `p2Library`, `p1Hand`, `p2Field`, as the TS helper's.
    fn waiting(opts: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": or(&opts, "p1Hand", json!([FILLER])),
                "backrow": [{ "def": RESOURCES, "radiant": opts["radiant"] == true, "faceUp": true }],
                "library": or(&opts, "p1Library", json!([STOCKPILE, STOCKPILE])),
            },
            "p2": {
                "hand": [FILLER],
                "library": or(&opts, "p2Library", json!([VANILLA, MENACE])),
                "field": or(&opts, "p2Field", json!([])),
            },
            "active": "p2",
        }))
    }

    mod c_58_common_resources {
        use super::*;

        #[test]
        fn is_a_2_field_spell_whose_count_of_cards_is_a_declared_number() {
            crate::register_all();
            let def = js(&registered_catalog()[ID]);
            assert_eq!(def["type"], "Field Spell");
            assert_eq!(def["cost"], 2);
            assert_eq!(def["params"], json!([{ "key": "cards", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1 }]));
            let scripts = script();
            assert!(scripts.base.start_of_turn.is_some());
            assert!(scripts.base.end_of_turn.is_none());
            assert!(scripts.radiant.start_of_turn.is_some());
            assert!(scripts.radiant.end_of_turn.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn r12_at_the_start_of_your_turn_you_draw_the_bottom_card_of_the_opponents_deck_and_it_becomes_yours() {
                crate::register_all();
                let mut s = waiting(json!({}));
                let bottom = s.card(MENACE).id.clone();
                s.end_turn();
                s.expect_in_zone(&bottom, "hand");
                assert_eq!(js(&s.card(&bottom).owner), "p1");
                assert!(s.hand(PlayerId::P1).iter().any(|card| card.id == bottom));
                assert_eq!(
                    s.pile(PlayerId::P2, "library").iter().map(|card| card.def_id.clone()).collect::<Vec<String>>(),
                    vec![VANILLA],
                );
                // Then the turn's own draw, from your own deck.
                assert_eq!(
                    drawn_by(s.last_events(), PlayerId::P1).iter().map(|event| event["defId"].clone()).collect::<Vec<Value>>(),
                    vec![json!(MENACE), json!(STOCKPILE)],
                );
            }

            #[test]
            fn s2_2_r62_it_is_a_start_of_turn_trigger_of_yours_only_the_opponents_turn_draws_nothing_from_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "backrow": [{ "def": RESOURCES, "faceUp": true }], "library": [STOCKPILE] },
                    "p2": { "hand": [FILLER], "library": [VANILLA, MENACE] },
                }));
                s.end_turn();
                assert_eq!(js(&s.state().active), "p2");
                assert_eq!(drawn_by(s.last_events(), PlayerId::P1).len(), 0);
                assert_eq!(s.pile(PlayerId::P2, "library").len(), 1);
            }

            #[test]
            fn it_is_your_draw_it_counts_toward_your_draws_this_turn() {
                crate::register_all();
                let mut s = waiting(json!({}));
                s.end_turn();
                assert_eq!(draws_this_turn(s.state(), PlayerId::P1), 2);
            }

            #[test]
            fn r317_under_your_hand_cap_at_a_full_hand_it_burns_into_your_graveyard_yours() {
                crate::register_all();
                let full_hand: Vec<&str> = (0..10).map(|_| FILLER).collect();
                let mut s = waiting(json!({ "p1Hand": full_hand }));
                let bottom = s.card(MENACE).id.clone();
                s.end_turn();
                s.expect_in_zone(&bottom, "graveyard");
                assert_eq!(js(&s.card(&bottom).owner), "p1");
                assert!(s.pile(PlayerId::P1, "graveyard").iter().any(|card| card.id == bottom));
                assert!(s.last_events().iter().map(js).any(|event| event["type"] == "burned"));
            }

            #[test]
            fn r58_your_cast_on_draw_a_cast_on_draw_card_at_the_bottom_of_their_deck_is_cast_for_you() {
                crate::register_all();
                let mut s = waiting(json!({ "p2Library": [VANILLA, CN_VIRUS] }));
                s.end_turn();
                let cast = s
                    .last_events()
                    .iter()
                    .map(js)
                    .find(|event| event["type"] == "cardPlayed" && event["defId"] == CN_VIRUS)
                    .expect("the cast's cardPlayed event");
                assert_eq!(cast["player"], "p1");
                assert_eq!(cast["costPaid"], 0);
                // "Take 1 damage" is its caster's: p1's hero.
                assert!(
                    s.last_events()
                        .iter()
                        .map(js)
                        .any(|event| event["type"] == "damage" && event["targetId"] == "hero-p1" && event["amount"] == 1)
                );
            }

            #[test]
            fn r58_r549_s9_3_a_cast_on_draw_is_yours_either_way_no_prompt_its_repeat_draws_from_your_own_deck_and_the_turn_finishes_after_a_json_round_trip() {
                crate::register_all();
                let mut s = waiting(json!({ "p1Hand": [FILLER, VANILLA], "p2Library": [VANILLA, MENACE, HINDER] }));
                s.end_turn();
                // R682: the cast's discard is random, so no prompt opens and the turn just finishes.
                assert!(s.state().pending.is_none());
                let round: GameState = serde_json::from_str(&serde_json::to_string(s.state()).expect("the state serialises"))
                    .expect("the state parses back");
                assert_eq!(&round, s.state());
                // R70: the cast resolved as p1's play without asking.
                let played = s
                    .events()
                    .iter()
                    .map(js)
                    .find(|event| event["type"] == "cardPlayed" && event["defId"] == HINDER)
                    .expect("Hinder's cardPlayed event");
                assert_eq!(played["player"], "p1");
                let p1 = &s.state().players.p1;
                let survivor: Option<String> = p1
                    .hand
                    .iter()
                    .find(|card| card.def_id == FILLER || card.def_id == VANILLA)
                    .map(|card| card.def_id.clone());
                let victim = if survivor.as_deref() == Some(FILLER) { VANILLA } else { FILLER };
                assert_eq!(
                    p1.graveyard.iter().map(|card| json!([card.def_id, js(&card.owner)])).collect::<Vec<Value>>(),
                    vec![json!([victim, "p1"]), json!([HINDER, "p1"])],
                );
                // §2.4's repeat of the draw and the turn's own draw are p1's own draws, from p1's deck: only the
                // one bottom card left p2's deck.
                assert_eq!(
                    s.state().players.p2.library.iter().map(|card| card.def_id.clone()).collect::<Vec<String>>(),
                    vec![VANILLA, MENACE],
                );
                assert_eq!(
                    p1.hand.iter().map(|card| json!(card.def_id)).collect::<Vec<Value>>(),
                    vec![json!(survivor), json!(STOCKPILE), json!(STOCKPILE)],
                );
                assert!(s.state().pending.is_none());
            }

            #[test]
            fn s10_1_it_is_your_draw_in_your_per_turn_count_with_the_turns_own_draw_it_sets_off_c_9_income_tax() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER, VANILLA], "backrow": [{ "def": RESOURCES, "faceUp": true }], "library": [STOCKPILE] },
                    "p2": { "hand": [FILLER], "backrow": [{ "def": INCOME_TAX, "faceUp": false }], "library": [VANILLA, MENACE] },
                    "active": "p2",
                }));
                let taken = s.card(MENACE).id.clone();
                s.end_turn();
                assert!(s.events().iter().map(js).any(|event| event["type"] == "trapFired"));
                let pending = js(&s.state().pending);
                assert_eq!(pending["playerId"], "p1");
                assert_eq!(pending["kind"], "hand");
                s.answer(json!([{ "pick": "instance", "instanceId": taken }]));
                assert_eq!(s.hand(PlayerId::P1).iter().map(|card| card.id.clone()).collect::<Vec<String>>(), vec![taken.clone()]);
                let mut theirs: Vec<String> = s.hand(PlayerId::P2).iter().map(|card| card.def_id.clone()).collect();
                theirs.sort();
                let mut expected = vec![FILLER, FILLER, STOCKPILE, VANILLA];
                expected.sort();
                assert_eq!(theirs, expected);
            }

            #[test]
            fn s2_4_your_draw_limit_under_the_opponents_radiant_anti_greed_machine_it_is_your_one_draw_and_your_turns_own_draw_is_stopped() {
                crate::register_all();
                let mut s = waiting(json!({ "p2Field": [{ "def": MACHINE, "radiant": true }] }));
                s.end_turn();
                assert_eq!(
                    drawn_by(s.last_events(), PlayerId::P1).iter().map(|event| event["defId"].clone()).collect::<Vec<Value>>(),
                    vec![json!(MENACE)],
                );
                assert_eq!(
                    s.last_events()
                        .iter()
                        .map(js)
                        .filter(|event| event["type"] == "drawLimited" && event["player"] == "p1")
                        .count(),
                    1,
                );
                assert_eq!(s.pile(PlayerId::P1, "library").len(), 2);
            }

            #[test]
            fn an_empty_enemy_deck_gives_nothing_and_nobody_takes_fatigue() {
                crate::register_all();
                let mut s = waiting(json!({ "p2Library": [] }));
                s.end_turn();
                assert_eq!(s.state().players.p1.fatigue_count, 0);
                assert_eq!(s.state().players.p2.fatigue_count, 0);
                assert_eq!(
                    drawn_by(s.last_events(), PlayerId::P1).iter().map(|event| event["defId"].clone()).collect::<Vec<Value>>(),
                    vec![json!(STOCKPILE)],
                );
            }

            #[test]
            fn r97_the_opponents_view_never_names_the_card_and_no_event_carries_a_deck_position() {
                crate::register_all();
                let mut s = waiting(json!({}));
                let bottom = s.card(MENACE).id.clone();
                s.end_turn();
                let theirs = s.view(PlayerId::P2);
                assert_eq!(js(&theirs.opponent.hand), json!({ "count": s.hand(PlayerId::P1).len() }));
                for event in theirs.events.iter().map(js).filter(|e| e["type"] == "drawn" || e["type"] == "stolen") {
                    assert!(!event.to_string().contains(&bottom));
                    assert!(!event.to_string().contains(MENACE));
                }
                for viewer in [PlayerId::P1, PlayerId::P2] {
                    for event in s.view(viewer).events.iter().map(js) {
                        // TS `not.toHaveProperty("position", expect.any(Number))`.
                        assert!(!event["position"].is_number());
                    }
                }
                // Its new owner reads it.
                assert!(js(&s.view(PlayerId::P1).you.hand).to_string().contains(&bottom));
            }

            #[test]
            fn r386_its_count_is_the_declared_number_an_upgrades_step_draws_2_from_the_bottom_bottom_first() {
                crate::register_all();
                let mut s = waiting(json!({ "p2Library": [STOCKPILE, VANILLA, MENACE] }));
                step_param(s.card_mut(RESOURCES), "cards", 1);
                s.end_turn();
                assert_eq!(
                    drawn_by(s.last_events(), PlayerId::P1).iter().map(|event| event["defId"].clone()).collect::<Vec<Value>>(),
                    vec![json!(MENACE), json!(VANILLA), json!(STOCKPILE)],
                );
                assert_eq!(
                    s.pile(PlayerId::P2, "library").iter().map(|card| card.def_id.clone()).collect::<Vec<String>>(),
                    vec![STOCKPILE],
                );
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn draws_from_the_bottom_of_the_opponents_deck_at_the_start_of_your_turn_and_at_its_end() {
                crate::register_all();
                let mut s = waiting(json!({ "radiant": true, "p2Library": [STOCKPILE, VANILLA, MENACE] }));
                s.end_turn();
                assert_eq!(
                    drawn_by(s.last_events(), PlayerId::P1).iter().map(|event| event["defId"].clone()).collect::<Vec<Value>>(),
                    vec![json!(MENACE), json!(STOCKPILE)],
                );
                s.end_turn();
                let end_draws: Vec<Value> =
                    drawn_by(s.last_events(), PlayerId::P1).iter().map(|event| event["defId"].clone()).collect();
                assert_eq!(end_draws, vec![json!(VANILLA)]);
                assert_eq!(js(&s.card(VANILLA).owner), "p1");
            }

            #[test]
            fn r386_its_declared_count_steps_for_both_an_upgrade_draws_2_at_each_end() {
                crate::register_all();
                let mut s = waiting(json!({ "radiant": true, "p2Library": [VANILLA, VANILLA, MENACE, MENACE] }));
                step_param(s.card_mut(RESOURCES), "cards", 1);
                s.end_turn();
                assert_eq!(drawn_by(s.last_events(), PlayerId::P1).iter().filter(|event| event["defId"] == MENACE).count(), 2);
                s.end_turn();
                assert_eq!(drawn_by(s.last_events(), PlayerId::P1).iter().filter(|event| event["defId"] == VANILLA).count(), 2);
            }

            #[test]
            fn an_empty_enemy_deck_at_the_end_of_the_turn_gives_nothing_and_no_fatigue() {
                crate::register_all();
                let mut s = waiting(json!({ "radiant": true, "p2Library": [MENACE] }));
                s.end_turn();
                assert_eq!(s.pile(PlayerId::P2, "library").len(), 0);
                s.end_turn();
                // Up to p2's own start of turn (whose own draw from its empty deck is fatigue of its own, §2.4),
                // p1's end of turn drew nothing and hit nobody.
                let events: Vec<Value> = s.last_events().iter().map(js).collect();
                let p2_starts = events.iter().position(|event| event["type"] == "turnStarted");
                let before = &events[..p2_starts.unwrap_or(events.len())];
                assert!(!before.iter().any(|event| event["type"] == "drawn" || event["type"] == "fatigue"));
                assert_eq!(s.state().players.p1.fatigue_count, 0);
            }
        }
    }
}
