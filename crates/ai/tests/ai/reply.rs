//! The opponent's reply (reply.ts, docs/polish/3-ai.md "The opponent's reply", SPEC §9.9): after a
//! line hands the opponent the turn, the opponent swings by a fixed trading rule and ends its turn,
//! every step a real `reduce` that costs one node, and the line is scored at the seat's next decision.
//!
//! Every board here is a `scenario()` with the AI as p1 on turn 9. The AI ends its turn through the
//! reducer, which hands p2 its turn 10; `simulateReply` then plays p2's reply.
//!
//! Port of `packages/ai/test/reply.test.ts`. TS's defaulted `hidden` argument (`hiddenCardIds` of the
//! state handed in) is passed explicitly wherever TS left it out.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, in_graveyard, on_field};

const TURN: i32 = 9;
const LIBRARY: &[&str] = &["core-008", "core-011", "core-008", "core-011"];

/// TS's `{ ...base, ...over }` on two JSON objects (an absent `over` changes nothing).
fn spread(base: Value, over: Option<&Value>) -> Value {
    let mut out = base;
    if let (Some(target), Some(Value::Object(extra))) = (out.as_object_mut(), over) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }
    out
}

/// An action body from TS's object literal.
fn body(literal: Value) -> ActionBody {
    json_as(literal)
}

/// p1's turn 9 with vanilla libraries on both sides, ended through the reducer: p2's turn 10.
fn handed_over(setup: Value) -> GameState {
    jackioh_cards::register_all();
    let mut options = spread(json!({ "seed": "reply", "active": "p1", "turn": TURN }), Some(&setup));
    options["p1"] = spread(json!({ "library": LIBRARY }), setup.get("p1"));
    options["p2"] = spread(json!({ "library": LIBRARY }), setup.get("p2"));
    let s = scenario(options);
    let ended = act(s.state(), AI, &ActionBody::EndTurn);
    assert_eq!(ended.active, HUMAN);
    assert_eq!(ended.turn, TURN + 1);
    ended
}

mod simulate_reply {
    use super::*;

    /// B41 swings an unblocked unit at the open face, ends the turn, and stops at the seat's next main phase
    #[test]
    fn b41_swings_an_unblocked_unit_at_the_open_face_ends_the_turn_and_stops_at_the_seats_next_main_phase() {
        let state = handed_over(json!({ "p2": { "field": ["core-008"] } }));
        let counter = create_node_counter(20, None);
        let after = simulate_reply(&state, AI, &counter, &hidden_card_ids(&state, AI));
        assert!(after.is_some());
        let reply = after.expect("a reply");
        assert!(reply.result.is_none());
        assert_eq!(reply.active, AI);
        assert_eq!(reply.turn, TURN + 2);
        assert_eq!(reply.phase, Phase::Main);
        assert!(reply.pending.is_none());
        // Mr. Vanilla's 4 to the face; the attack and the endTurn are one node each.
        assert_eq!(reply.players[AI].hero.health, 26);
        assert_eq!(counter.used(), 2);
    }

    /// B41 takes lethal when the face is in reach
    #[test]
    fn b41_takes_lethal_when_the_face_is_in_reach() {
        let state = handed_over(json!({ "p1": { "health": 4 }, "p2": { "field": ["core-008"] } }));
        let reply = simulate_reply(&state, AI, &create_node_counter(20, None), &hidden_card_ids(&state, AI)).expect("a reply");
        assert_eq!(reply.result.map(|result| result.winner), Some(Winner::from(HUMAN)));
    }

    /// B41 does not throw a unit into a Defense-Position wall it cannot hurt, and ends its turn instead
    #[test]
    fn b41_does_not_throw_a_unit_into_a_defense_position_wall_it_cannot_hurt_and_ends_its_turn_instead() {
        // The 7/7 in Defense Position has Taunt and 8 Armor: Mr. Vanilla's 4 does nothing and it dies.
        let state = handed_over(json!({
            "p1": { "field": [{ "def": "core-025", "position": "DEF" }] },
            "p2": { "field": ["core-008"] },
        }));
        let counter = create_node_counter(20, None);
        let reply = simulate_reply(&state, AI, &counter, &hidden_card_ids(&state, AI)).expect("a reply");
        assert!(on_field(&reply, HUMAN, "core-008"));
        assert_eq!(reply.players[AI].hero.health, 30);
        assert_eq!(reply.active, AI);
        assert_eq!(counter.used(), 1);
    }

    /// B41 trades into a unit worth more than the face damage it gives up
    #[test]
    fn b41_trades_into_a_unit_worth_more_than_the_face_damage_it_gives_up() {
        // Midrange Menace survives Pointmaster's First Strike 7 and kills it: an 11-point unit for free
        // beats 9 to the face.
        let state = handed_over(json!({ "p1": { "field": ["core-020"] }, "p2": { "field": ["core-019"] } }));
        let reply = simulate_reply(&state, AI, &create_node_counter(20, None), &hidden_card_ids(&state, AI)).expect("a reply");
        assert!(in_graveyard(&reply, AI, "core-020"));
        assert_eq!(reply.players[AI].hero.health, 30);
    }

    /// stops at the seat's own start-of-turn prompt, which is where its next decision begins
    #[test]
    fn stops_at_the_seats_own_start_of_turn_prompt_which_is_where_its_next_decision_begins() {
        // Masochism Mask asks p1 at the start of each of its turns.
        let state = handed_over(json!({ "p1": { "backrow": ["core-065"] } }));
        let reply = simulate_reply(&state, AI, &create_node_counter(20, None), &hidden_card_ids(&state, AI)).expect("a reply");
        assert_eq!(reply.active, AI);
        assert_eq!(reply.turn, TURN + 2);
        assert_eq!(reply.pending.as_ref().map(|pending| pending.player_id), Some(AI));
    }

    /// replays the units the seat's own Flood bounced into the opponent's hand, and nothing it held unseen
    #[test]
    fn replays_the_units_the_seats_own_flood_bounced_into_the_opponents_hand_and_nothing_it_held_unseen() {
        jackioh_cards::register_all();
        let s = scenario(json!({
            "seed": "reply-flood",
            "active": "p1",
            "turn": TURN,
            // Flood costs (4) since patch v0.2.0.
            "p1": { "hand": ["core-017"], "mana": 4, "library": LIBRARY },
            "p2": { "field": ["core-019"], "hand": ["core-008"], "library": LIBRARY },
        }));
        // The seat's view when the decision began: p2's hand and library are what it cannot see.
        let hidden = hidden_card_ids(s.state(), AI);
        let menace = s.unit(HUMAN, 1).map(|card| card.id.clone()).unwrap_or_default();
        assert!(!hidden.contains(&menace));

        // Flood bounces Midrange Menace. p1 has nothing left to do, so R82 ends its turn on the spot and
        // p2 is into its turn 10, having drawn.
        let flood = s.hand(AI).first().map(|card| card.id.clone()).unwrap_or_default();
        let ended = act(s.state(), AI, body(json!({ "type": "play", "instanceId": flood })));
        assert_eq!(ended.active, HUMAN);
        assert_eq!(ended.turn, TURN + 1);
        assert!(ended.players[HUMAN].hand.iter().any(|card| card.id == menace));

        let reply = simulate_reply(&ended, AI, &create_node_counter(40, None), &hidden).expect("a reply");
        // Midrange Menace came back; the Mr. Vanilla p2 held from the start stayed in its hand.
        assert!(on_field(&reply, HUMAN, "core-019"));
        assert!(!on_field(&reply, HUMAN, "core-008"));

        // Without that history every card in p2's hand is unseen, and the reply plays none of them.
        let blind = simulate_reply(&ended, AI, &create_node_counter(40, None), &hidden_card_ids(&ended, AI)).expect("a reply");
        assert!(!on_field(&blind, HUMAN, "core-019"));
        assert!(!on_field(&blind, HUMAN, "core-008"));
    }

    /// returns null without a node to spend
    #[test]
    fn returns_null_without_a_node_to_spend() {
        let state = handed_over(json!({ "p2": { "field": ["core-008"] } }));
        assert!(simulate_reply(&state, AI, &create_node_counter(0, None), &hidden_card_ids(&state, AI)).is_none());
    }
}

mod reply_score {
    use super::*;

    /// scores a passed line at the seat's next decision, where it swings first, less its unspent crystals
    #[test]
    fn scores_a_passed_line_at_the_seats_next_decision_where_it_swings_first_less_its_unspent_crystals() {
        jackioh_cards::register_all();
        let s = scenario(json!({
            "seed": "reply-score",
            "active": "p1",
            "turn": TURN,
            "p1": { "mana": 2, "library": LIBRARY },
            "p2": { "field": ["core-008"], "library": LIBRARY },
        }));
        let ended = act(s.state(), AI, &ActionBody::EndTurn);
        assert_eq!(line_status(&ended, AI, TURN), LineStatus::Passed);
        assert_eq!(ended.players[AI].turn_log.unspent_at_end, Some(2));

        let after = simulate_reply(&ended, AI, &create_node_counter(20, None), &hidden_card_ids(&ended, AI)).expect("a reply");
        let counter = create_node_counter(20, None);
        let score = reply_score(&ended, AI, TURN, &counter, &hidden_card_ids(&ended, AI));
        assert_eq!(score, Some(evaluate(&after, AI, NextSwing::Seat, &AI_EVAL) - AI_EVAL.unspent_mana * 2.0));
        assert_eq!(counter.used(), 2);
        // The state it was scored at carries the reply's 4 damage.
        assert_eq!(after.players[AI].hero.health, 26);
    }

    /// keeps the static score of a line that ended the game, with no node spent
    #[test]
    fn keeps_the_static_score_of_a_line_that_ended_the_game_with_no_node_spent() {
        jackioh_cards::register_all();
        let s = scenario(json!({
            "seed": "reply-over",
            "active": "p1",
            "turn": TURN,
            "p1": { "field": ["core-008"] },
            "p2": { "health": 3 },
        }));
        let attacker = s.unit(AI, 1).map(|card| card.id.clone()).unwrap_or_default();
        let won = act(
            s.state(),
            AI,
            &ActionBody::Attack { attacker_id: attacker, target_id: format!("hero-{HUMAN}") },
        );
        assert_eq!(won.result.as_ref().map(|result| result.winner), Some(Winner::from(AI)));
        let counter = create_node_counter(20, None);
        assert_eq!(reply_score(&won, AI, TURN, &counter, &hidden_card_ids(&won, AI)), Some(static_score(&won, AI, TURN)));
        assert_eq!(counter.used(), 0);
    }

    /// is null when the counter runs out during the reply
    #[test]
    fn is_null_when_the_counter_runs_out_during_the_reply() {
        let state = handed_over(json!({ "p2": { "field": ["core-008"] } }));
        assert!(reply_score(&state, AI, TURN, &create_node_counter(1, None), &hidden_card_ids(&state, AI)).is_none());
    }
}
