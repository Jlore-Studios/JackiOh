//! The opponent's reply (reply.ts, docs/polish/3-ai.md "The opponent's reply", SPEC §9.9): after a
//! line hands the opponent the turn, the opponent swings by a fixed trading rule and ends its turn,
//! every step a real `reduce` that costs one node, and the line is scored at the seat's next decision.
//!
//! Every board here is a `scenario()` with the AI as p1 on turn 9. The AI ends its turn through the
//! reducer, which hands p2 its turn 10; `simulateReply` then plays p2's reply.
//!
//! Port of `packages/ai/test/reply.test.ts`. TS's defaulted `hidden` argument (`hiddenCardIds` of the
//! state handed in) is passed explicitly wherever TS left it out. #1036's tests also show the opponent
//! using the Activates it can see and never a face-down one.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, in_graveyard, is_legal, on_field};

const TURN: i32 = 9;
const LIBRARY: &[&str] = &["core-008", "core-011", "core-008", "core-011"];
/// C #20 The Power to Punish: "Activate: Choose one: Deal {damage} damage; …", once per turn.
const PUNISH: &str = "classic-020";
/// #98 Heroic Power, whose power is an `activate` (R752).
const HEROIC: &str = "core-098";
/// C #6 Cloaked Toe Cracker, the card that prints Cloaked.
const CRACKER: &str = "classic-006";

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
    let mut options = spread(
        json!({ "seed": "reply", "active": "p1", "turn": TURN }),
        Some(&setup),
    );
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
        let reply = simulate_reply(
            &state,
            AI,
            &create_node_counter(20, None),
            &hidden_card_ids(&state, AI),
        )
        .expect("a reply");
        assert_eq!(
            reply.result.map(|result| result.winner),
            Some(Winner::from(HUMAN))
        );
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

    /// p2's #45 Deft Duelist (4/3, Charge, Deft).
    fn duelist_of(state: &GameState) -> CardInstance {
        active_units_of(state, HUMAN)
            .into_iter()
            .find(|unit| unit.def_id == "core-045")
            .cloned()
            .expect("Deft Duelist is on p2's field")
    }

    /// switches a Deft unit out of Defense Position and swings it at the open face
    #[test]
    fn switches_a_deft_unit_out_of_defense_position_and_swings_it_at_the_open_face() {
        // R49: Deft Duelist's switch to Attack spends only its switch, so it still attacks.
        let state = handed_over(json!({ "p2": { "field": [{ "def": "core-045", "position": "DEF" }] } }));
        let counter = create_node_counter(20, None);
        let reply = simulate_reply(&state, AI, &counter, &hidden_card_ids(&state, AI)).expect("a reply");
        assert_eq!(reply.players[AI].hero.health, 26);
        assert_eq!(unit_view(&reply, &duelist_of(&reply)).position, Position::Atk);
        assert_eq!(reply.active, AI);
        // The switch, the attack and the endTurn, one node each.
        assert_eq!(counter.used(), 3);
    }

    /// leaves a unit without Deft in Defense Position and ends its turn
    #[test]
    fn leaves_a_unit_without_deft_in_defense_position_and_ends_its_turn() {
        let state = handed_over(json!({ "p2": { "field": [{ "def": "core-008", "position": "DEF" }] } }));
        let counter = create_node_counter(20, None);
        let reply = simulate_reply(&state, AI, &counter, &hidden_card_ids(&state, AI)).expect("a reply");
        assert_eq!(reply.players[AI].hero.health, 30);
        assert_eq!(reply.active, AI);
        assert_eq!(counter.used(), 1);
    }

    /// keeps a Deft unit in Defense Position when no attack after the switch is its own
    #[test]
    fn keeps_a_deft_unit_in_defense_position_when_no_attack_after_the_switch_is_its_own() {
        // The 7/7 in Defense Position has Taunt and 8 Armor: the Duelist's 4 does nothing and it dies, so
        // the trial switch is dropped.
        let state = handed_over(json!({
            "p1": { "field": [{ "def": "core-025", "position": "DEF" }] },
            "p2": { "field": [{ "def": "core-045", "position": "DEF" }] },
        }));
        let counter = create_node_counter(20, None);
        let reply = simulate_reply(&state, AI, &counter, &hidden_card_ids(&state, AI)).expect("a reply");
        assert_eq!(unit_view(&reply, &duelist_of(&reply)).position, Position::Def);
        assert_eq!(reply.players[AI].hero.health, 30);
        // The trial switch, then the endTurn.
        assert_eq!(counter.used(), 2);
    }

    /// B41 trades into a unit worth more than the face damage it gives up
    #[test]
    fn b41_trades_into_a_unit_worth_more_than_the_face_damage_it_gives_up() {
        // Midrange Menace survives Pointmaster's First Strike 7 and kills it: an 11-point unit for free
        // beats 9 to the face.
        let state = handed_over(json!({ "p1": { "field": ["core-020"] }, "p2": { "field": ["core-019"] } }));
        let reply = simulate_reply(
            &state,
            AI,
            &create_node_counter(20, None),
            &hidden_card_ids(&state, AI),
        )
        .expect("a reply");
        assert!(in_graveyard(&reply, AI, "core-020"));
        assert_eq!(reply.players[AI].hero.health, 30);
    }

    /// stops at the seat's own start-of-turn prompt, which is where its next decision begins
    #[test]
    fn stops_at_the_seats_own_start_of_turn_prompt_which_is_where_its_next_decision_begins() {
        // Masochism Mask asks p1 at the start of each of its turns.
        let state = handed_over(json!({ "p1": { "backrow": ["core-065"] } }));
        let reply = simulate_reply(
            &state,
            AI,
            &create_node_counter(20, None),
            &hidden_card_ids(&state, AI),
        )
        .expect("a reply");
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
        let ended = act(
            s.state(),
            AI,
            body(json!({ "type": "play", "instanceId": flood })),
        );
        assert_eq!(ended.active, HUMAN);
        assert_eq!(ended.turn, TURN + 1);
        assert!(ended.players[HUMAN].hand.iter().any(|card| card.id == menace));

        let reply = simulate_reply(&ended, AI, &create_node_counter(40, None), &hidden).expect("a reply");
        // Midrange Menace came back; the Mr. Vanilla p2 held from the start stayed in its hand.
        assert!(on_field(&reply, HUMAN, "core-019"));
        assert!(!on_field(&reply, HUMAN, "core-008"));

        // Without that history every card in p2's hand is unseen, and the reply plays none of them.
        let blind = simulate_reply(
            &ended,
            AI,
            &create_node_counter(40, None),
            &hidden_card_ids(&ended, AI),
        )
        .expect("a reply");
        assert!(!on_field(&blind, HUMAN, "core-019"));
        assert!(!on_field(&blind, HUMAN, "core-008"));
    }

    /// R82: the seat's next turn with nothing but ending it left to do ends itself inside the
    /// opponent's EndTurn reduce. The reply stops where the opponent's second turn begins —
    /// the seat's next decision never came — instead of playing the opponent's turns out to a
    /// phantom death.
    #[test]
    fn r82_stops_where_the_opponents_second_turn_begins_when_the_seats_next_turn_is_empty() {
        // p1's hand and library are empty: its turn 11 has only End turn to offer.
        let state = handed_over(json!({
            "p1": { "hand": [], "library": [] },
            "p2": { "field": ["core-008"] },
        }));
        let counter = create_node_counter(20, None);
        let reply = simulate_reply(&state, AI, &counter, &hidden_card_ids(&state, AI)).expect("a reply");
        // One hit for 4 and one fatigue draw ride inside p2's EndTurn; a second p2 turn does not run.
        assert!(reply.result.is_none());
        assert_eq!(reply.active, HUMAN);
        assert_eq!(reply.turn, TURN + 3);
        assert_eq!(reply.players[AI].hero.health, 25);
        assert_eq!(counter.used(), 2);
    }

    /// returns null without a node to spend
    #[test]
    fn returns_null_without_a_node_to_spend() {
        let state = handed_over(json!({ "p2": { "field": ["core-008"] } }));
        assert!(
            simulate_reply(
                &state,
                AI,
                &create_node_counter(0, None),
                &hidden_card_ids(&state, AI)
            )
            .is_none()
        );
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

        let after = simulate_reply(
            &ended,
            AI,
            &create_node_counter(20, None),
            &hidden_card_ids(&ended, AI),
        )
        .expect("a reply");
        let counter = create_node_counter(20, None);
        let score = reply_score(&ended, AI, TURN, &counter, &hidden_card_ids(&ended, AI));
        assert_eq!(
            score,
            Some(evaluate(&after, AI, NextSwing::Seat, &AI_EVAL) - AI_EVAL.unspent_mana * 2.0)
        );
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
            &ActionBody::Attack {
                attacker_id: attacker,
                target_id: format!("hero-{HUMAN}"),
            },
        );
        assert_eq!(
            won.result.as_ref().map(|result| result.winner),
            Some(Winner::from(AI))
        );
        let counter = create_node_counter(20, None);
        assert_eq!(
            reply_score(&won, AI, TURN, &counter, &hidden_card_ids(&won, AI)),
            Some(static_score(&won, AI, TURN))
        );
        assert_eq!(counter.used(), 0);
    }

    /// is null when the counter runs out during the reply
    #[test]
    fn is_null_when_the_counter_runs_out_during_the_reply() {
        let state = handed_over(json!({ "p2": { "field": ["core-008"] } }));
        assert!(
            reply_score(
                &state,
                AI,
                TURN,
                &create_node_counter(1, None),
                &hidden_card_ids(&state, AI)
            )
            .is_none()
        );
    }
}

mod duos_life_r964000 {
    use super::*;

    /// The AI's seat as a Duos seat with a full 20 pool, `spent` or not.
    fn with_pool(state: &GameState, spent: bool) -> GameState {
        let mut out = state.clone();
        out.players[AI].duos = Some(DuosLife {
            pool: 20,
            pool_cap: 20,
            spent,
        });
        out
    }

    #[test]
    fn r964000_a_20_damage_face_hit_is_no_win_against_an_unspent_duos_seat() {
        let state = handed_over(json!({ "p1": { "health": 20 } }));
        assert_eq!(reply_face_value(&state, AI, 20), AI_EVAL.win, "a 1v1 hero at 20");

        let unspent = with_pool(&state, false);
        let chip = reply_face_value(&unspent, AI, 20);
        assert_eq!(chip, AI_REPLY.face_per_damage * 20.0, "20 own HP and a 20 pool");
        assert!(chip < AI_EVAL.win);
        assert_eq!(reply_face_value(&unspent, AI, 40), AI_EVAL.win, "all 40 of it");

        // Spent: the hero's health is the pool, and 20 empties it.
        assert_eq!(reply_face_value(&with_pool(&state, true), AI, 20), AI_EVAL.win);
    }
}

mod visible_activations {
    use super::*;

    /// What `reply_score` gives a passed line that the opponent's reply wins: its loss, less the
    /// crystals it left unspent, exactly as `reply_score` sums it.
    fn loss_score(state: &GameState) -> Option<f64> {
        let unspent = state.players[AI].turn_log.unspent_at_end.unwrap_or(0);
        Some(-AI_EVAL.win + f64::from(TURN + 1) - AI_EVAL.unspent_mana * f64::from(unspent))
    }

    fn winner(state: &GameState) -> Option<Winner> {
        state.result.map(|result| result.winner)
    }

    /// p2 sets a Cloaked Toe Cracker face-down by its Cloaked on its turn 8, and both seats end their
    /// turns: p2's turn 10, its set card's Reveal on offer (R783006).
    fn cracker_set() -> GameState {
        jackioh_cards::register_all();
        let s = scenario(json!({
            "seed": "reply-unseen",
            "active": "p2",
            "turn": TURN - 1,
            "p1": { "hand": ["core-008"], "library": LIBRARY },
            "p2": { "hand": [CRACKER, "core-008"], "library": LIBRARY },
        }));
        let cracker = s
            .hand(HUMAN)
            .into_iter()
            .find(|card| card.def_id == CRACKER)
            .map(|card| card.id)
            .expect("the Toe Cracker");
        let set = act(
            s.state(),
            HUMAN,
            ActionBody::Play {
                instance_id: cracker,
                zone: Some(ZoneChoice {
                    row: Row::Backrow,
                    lane: 1,
                }),
                x: None,
                embiggen: None,
                equip: None,
                tributes: None,
                targets: None,
                modes: None,
                plague: None,
                face_down: Some(RevealAt::Cloaked),
            },
        );
        let handed = act(&set, HUMAN, &ActionBody::EndTurn);
        assert_eq!(handed.active, AI);
        let back = act(&handed, AI, &ActionBody::EndTurn);
        assert_eq!(back.active, HUMAN);
        assert_eq!(back.turn, TURN + 1);
        back
    }

    /// Whether p2 may Reveal its set card `id` (R783006).
    fn reveal_offered(state: &GameState, id: &str) -> bool {
        legal_actions(state, HUMAN).iter().any(|action| {
            matches!(action, ActionBody::Activate { instance_id, ability, .. }
                if instance_id.as_str() == id && ability.as_deref() == Some(alt_play::CLOAKED_REVEAL))
        })
    }

    /// The seat passes at 2 health into a face-up The Power to Punish it can see: its 2 damage ends the game
    #[test]
    fn a_face_up_power_to_punish_is_lethal_on_the_seat_at_2_health_and_the_line_scores_as_that_loss() {
        let state = handed_over(json!({ "p1": { "health": 2 }, "p2": { "backrow": [PUNISH] } }));
        let reply = simulate_reply(
            &state,
            AI,
            &create_node_counter(20, None),
            &hidden_card_ids(&state, AI),
        )
        .expect("a reply");
        assert_eq!(reply.turn, TURN + 1);
        assert_eq!(winner(&reply), Some(Winner::from(HUMAN)));

        assert_eq!(line_status(&state, AI, TURN), LineStatus::Passed);
        let score = reply_score(
            &state,
            AI,
            TURN,
            &create_node_counter(20, None),
            &hidden_card_ids(&state, AI),
        );
        assert_eq!(score, loss_score(&state));
    }

    /// A Heroic Power that rolled Ping finishes the seat at 1 health through the activate legal_actions lists
    #[test]
    fn a_heroic_power_that_rolled_ping_is_lethal_on_the_seat_at_1_health_through_its_listed_activate() {
        let mut state = handed_over(json!({ "p1": { "health": 1 }, "p2": { "backrow": [HEROIC] } }));
        let id = state.players[HUMAN]
            .backrow
            .iter()
            .flatten()
            .find(|card| card.def_id == HEROIC)
            .map(|card| card.id.clone())
            .expect("the Heroic Power");
        find_instance_mut(&mut state, &id)
            .expect("the Heroic Power")
            .memory
            .insert(subsystems::POWER_KEY.to_string(), json!("ping"));
        // Ping is an `activate` (R752), the action the engine offers for it.
        assert!(is_legal(
            &state,
            HUMAN,
            ActionBody::Activate {
                instance_id: id,
                ability: Some("ping".into()),
                targets: Some(vec![Selection::Hero { player: AI }]),
                modes: None,
                tributes: None,
            }
        ));

        let reply = simulate_reply(
            &state,
            AI,
            &create_node_counter(20, None),
            &hidden_card_ids(&state, AI),
        )
        .expect("a reply");
        assert_eq!(winner(&reply), Some(Winner::from(HUMAN)));
        let score = reply_score(
            &state,
            AI,
            TURN,
            &create_node_counter(20, None),
            &hidden_card_ids(&state, AI),
        );
        assert_eq!(score, loss_score(&state));
    }

    /// The activation aimed at the seat's hero is tried first, though legal_actions lists the units first
    #[test]
    fn tries_the_activation_aimed_at_the_seats_hero_first_so_a_busy_board_never_hides_the_lethal() {
        let state = handed_over(json!({
            "p1": { "health": 2, "field": ["core-008", "core-008", "core-008", "core-008"] },
            "p2": { "backrow": [PUNISH] },
        }));
        // The premise: in legal_actions order, the hit on p1's hero is past the cap.
        let at_hero = legal_actions(&state, HUMAN)
            .iter()
            .filter(|action| matches!(action, ActionBody::Activate { .. }))
            .position(|action| {
                matches!(action, ActionBody::Activate { targets: Some(targets), .. }
                    if targets.contains(&Selection::Hero { player: AI }))
            })
            .expect("Punish's hit on p1's hero");
        assert!(at_hero >= AI_REPLY.visible_activations as usize);

        let reply = simulate_reply(
            &state,
            AI,
            &create_node_counter(20, None),
            &hidden_card_ids(&state, AI),
        )
        .expect("a reply");
        assert_eq!(winner(&reply), Some(Winner::from(HUMAN)));
    }

    /// A card set face-down is one the seat cannot see, so the reply never uses its Reveal
    #[test]
    fn never_reveals_a_card_set_face_down_that_the_seat_cannot_see() {
        let state = cracker_set();
        let set = state.players[HUMAN].backrow[0]
            .as_ref()
            .expect("the set Toe Cracker");
        assert!(set.set_as.is_some());
        assert!(reveal_offered(&state, &set.id));

        let reply = simulate_reply(
            &state,
            AI,
            &create_node_counter(40, None),
            &hidden_card_ids(&state, AI),
        )
        .expect("a reply");
        let still = reply.players[HUMAN].backrow[0].as_ref().expect("still set");
        assert_eq!(still.id, set.id);
        assert!(still.set_as.is_some());
        assert!(!on_field(&reply, HUMAN, CRACKER));
        assert_eq!(reply.players[AI].hero.health, state.players[AI].hero.health);
    }

    /// Nor a card a determinization filled in face-down: it is a sample, whatever it was drawn as
    #[test]
    fn never_reveals_a_card_a_determinization_filled_in_face_down() {
        let state = cracker_set();
        let public = redact(&state, AI);
        let mut checked = 0;
        for n in 0..300 {
            let world = determinize(
                &public,
                AI,
                &mut create_rng(&format!("reply-unseen-{n}"), 0),
                DeterminizeOptions::default(),
            );
            let Some(set) = world.players[HUMAN].backrow[0].as_ref() else {
                continue;
            };
            if set.set_as.map(|set_as| set_as.reveal) != Some(RevealAt::Cloaked) {
                continue;
            }
            assert!(reveal_offered(&world, &set.id));
            let reply = simulate_reply(
                &world,
                AI,
                &create_node_counter(40, None),
                &hidden_card_ids(&world, AI),
            )
            .expect("a reply");
            let still = reply.players[HUMAN].backrow[0].as_ref().expect("still set");
            assert_eq!(still.id, set.id);
            assert!(still.set_as.is_some());
            assert_eq!(reply.players[AI].hero.health, world.players[AI].hero.health);
            checked += 1;
            if checked == 3 {
                break;
            }
        }
        assert!(checked > 0, "no world filled the set card in as a Cloaked set");
    }

    /// decide heals rather than pass at 2 health into a face-up The Power to Punish
    #[test]
    fn decide_heals_rather_than_pass_at_2_health_into_a_face_up_power_to_punish() {
        jackioh_cards::register_all();
        let s = scenario(json!({
            "seed": "reply-decide-heal",
            "active": "p1",
            "turn": TURN,
            "p1": { "health": 2, "mana": 1, "hand": ["core-005", "core-008"], "library": LIBRARY },
            "p2": { "backrow": [PUNISH], "hand": ["core-005"], "library": LIBRARY },
        }));
        let stockpile = s
            .hand(AI)
            .into_iter()
            .find(|card| card.def_id == "core-005")
            .map(|card| card.id)
            .expect("the Stockpile");
        let decision = decide(
            s.state(),
            AI,
            &mut AiOptions::new(create_rng("reply-decide-heal", 0)),
        )
        .expect("a decision");
        assert!(
            matches!(&decision.action, ActionBody::Play { instance_id, .. } if *instance_id == stockpile),
            "{:?}",
            decision.action
        );
    }

    /// decide exiles a face-up The Power to Punish rather than play a bigger body and pass into it
    #[test]
    fn decide_exiles_a_face_up_power_to_punish_rather_than_play_a_bigger_body_and_pass_into_it() {
        jackioh_cards::register_all();
        let s = scenario(json!({
            "seed": "reply-decide-exile",
            "active": "p1",
            "turn": TURN,
            "p1": { "health": 2, "hand": ["core-034", "core-025"], "library": LIBRARY },
            "p2": { "backrow": [PUNISH], "hand": ["core-005"], "library": LIBRARY },
        }));
        let collateral = s
            .hand(AI)
            .into_iter()
            .find(|card| card.def_id == "core-034")
            .map(|card| card.id)
            .expect("the Collateral Damage");
        let punish = s.backrow(HUMAN, 1).map(|card| card.id).expect("the Punish");
        let decision = decide(
            s.state(),
            AI,
            &mut AiOptions::new(create_rng("reply-decide-exile", 0)),
        )
        .expect("a decision");
        assert!(
            matches!(&decision.action, ActionBody::Play { instance_id, targets: Some(targets), .. }
                if *instance_id == collateral
                    && targets.contains(&Selection::Instance { instance_id: punish.clone() })),
            "{:?}",
            decision.action
        );
    }
}
