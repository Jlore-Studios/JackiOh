//! The opponent's reply (docs/polish/3-ai.md "The opponent's reply", SPEC §9.9): after a
//! line hands the opponent the turn, the opponent swings by a fixed trading rule and ends its turn,
//! every step a real `reduce` that costs one node, and the line is scored at the seat's next decision.
//!
//! Every board here is a `scenario()` with the AI as p1 on turn 9. The AI ends its turn through the
//! reducer, which hands p2 its turn 10; `simulateReply` then plays p2's reply.
//! #1036's tests also show the opponent using the Activates it can see and never a face-down one.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, all_out_attack, in_graveyard, is_legal, on_field, run_puzzle, trace};

const TURN: i32 = 9;
const LIBRARY: &[&str] = &["core-008", "core-011", "core-008", "core-011"];
/// C #20 The Power to Punish: "Activate: Choose one: Deal {damage} damage; …", once per turn.
const PUNISH: &str = "classic-020";
/// #98 Heroic Power, whose power is an `activate` (R752).
const HEROIC: &str = "core-098";
/// C #6 Cloaked Toe Cracker, the card that prints Cloaked.
const CRACKER: &str = "classic-006";
/// C+ #13 Mommy Barker, whose Radiant face is a 4/4 Reborn.
const BARKER: &str = "classicplus-013";

fn spread(base: Value, over: Option<&Value>) -> Value {
    let mut out = base;
    if let (Some(target), Some(Value::Object(extra))) = (out.as_object_mut(), over) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }
    out
}

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

    #[test]
    fn b41_swings_an_unblocked_unit_at_the_open_face_ends_the_turn_and_stops_at_the_seats_next_main_phase() {
        let state = handed_over(json!({ "p2": { "field": ["core-008"] } }));
        let counter = create_node_counter(20, None);
        let after = simulate_reply(&state, AI, &counter, &hidden_card_ids(&state, AI), Prices::NONE);
        assert!(after.is_some());
        let reply = after.expect("a reply");
        assert!(reply.result.is_none());
        assert_eq!(reply.active, AI);
        assert_eq!(reply.turn, TURN + 2);
        assert_eq!(reply.phase, Phase::Main);
        assert!(reply.pending.is_none());
        assert_eq!(reply.players[AI].hero.health, 26);
        assert_eq!(counter.used(), 2);
    }

    #[test]
    fn b41_takes_lethal_when_the_face_is_in_reach() {
        let state = handed_over(json!({ "p1": { "health": 4 }, "p2": { "field": ["core-008"] } }));
        let reply = simulate_reply(
            &state,
            AI,
            &create_node_counter(20, None),
            &hidden_card_ids(&state, AI),
            Prices::NONE,
        )
        .expect("a reply");
        assert_eq!(
            reply.result.map(|result| result.winner),
            Some(Winner::from(HUMAN))
        );
    }

    #[test]
    fn b41_does_not_throw_a_unit_into_a_defense_position_wall_it_cannot_hurt_and_ends_its_turn_instead() {
        let state = handed_over(json!({
            "p1": { "field": [{ "def": "core-025", "position": "DEF" }] },
            "p2": { "field": ["core-008"] },
        }));
        let counter = create_node_counter(20, None);
        let reply = simulate_reply(&state, AI, &counter, &hidden_card_ids(&state, AI), Prices::NONE)
            .expect("a reply");
        assert!(on_field(&reply, HUMAN, "core-008"));
        assert_eq!(reply.players[AI].hero.health, 30);
        assert_eq!(reply.active, AI);
        assert_eq!(counter.used(), 2);
    }

    /// p2's #45 Deft Duelist (4/3, Charge, Deft).
    fn duelist_of(state: &GameState) -> CardInstance {
        active_units_of(state, HUMAN)
            .into_iter()
            .find(|unit| unit.def_id == "core-045")
            .cloned()
            .expect("Deft Duelist is on p2's field")
    }

    #[test]
    fn switches_a_deft_unit_out_of_defense_position_and_swings_it_at_the_open_face() {
        let state = handed_over(json!({ "p2": { "field": [{ "def": "core-045", "position": "DEF" }] } }));
        let counter = create_node_counter(20, None);
        let reply = simulate_reply(&state, AI, &counter, &hidden_card_ids(&state, AI), Prices::NONE)
            .expect("a reply");
        assert_eq!(reply.players[AI].hero.health, 26);
        assert_eq!(unit_view(&reply, &duelist_of(&reply)).position, Position::Atk);
        assert_eq!(reply.active, AI);
        assert_eq!(counter.used(), 3);
    }

    #[test]
    fn leaves_a_unit_without_deft_in_defense_position_and_ends_its_turn() {
        let state = handed_over(json!({ "p2": { "field": [{ "def": "core-008", "position": "DEF" }] } }));
        let counter = create_node_counter(20, None);
        let reply = simulate_reply(&state, AI, &counter, &hidden_card_ids(&state, AI), Prices::NONE)
            .expect("a reply");
        assert_eq!(reply.players[AI].hero.health, 30);
        assert_eq!(reply.active, AI);
        assert_eq!(counter.used(), 1);
    }

    #[test]
    fn keeps_a_deft_unit_in_defense_position_when_no_attack_after_the_switch_is_its_own() {
        let state = handed_over(json!({
            "p1": { "field": [{ "def": "core-025", "position": "DEF" }] },
            "p2": { "field": [{ "def": "core-045", "position": "DEF" }] },
        }));
        let counter = create_node_counter(20, None);
        let reply = simulate_reply(&state, AI, &counter, &hidden_card_ids(&state, AI), Prices::NONE)
            .expect("a reply");
        assert_eq!(unit_view(&reply, &duelist_of(&reply)).position, Position::Def);
        assert_eq!(reply.players[AI].hero.health, 30);
        assert_eq!(counter.used(), 2);
    }

    #[test]
    fn b41_trades_into_a_unit_worth_more_than_the_face_damage_it_gives_up() {
        let state = handed_over(json!({ "p1": { "field": ["core-020"] }, "p2": { "field": ["core-019"] } }));
        let reply = simulate_reply(
            &state,
            AI,
            &create_node_counter(20, None),
            &hidden_card_ids(&state, AI),
            Prices::NONE,
        )
        .expect("a reply");
        assert!(in_graveyard(&reply, AI, "core-020"));
        assert_eq!(reply.players[AI].hero.health, 30);
    }

    /// #1796: kills a vanilla 4/4 rather than a Reborn 5/5, which comes straight back
    #[test]
    fn kills_a_vanilla_4_4_rather_than_a_reborn_5_5_that_comes_straight_back() {
        // Professor Curvature's Radiant 6/6 kills either and survives. Mommy Barker's Radiant 4/4
        // Reborn, buffed to 5/5, comes back at its printed 4/4 on 1 health (§4.5 step 4), so its kill
        // removes less than Mr. Vanilla's does, and Mr. Vanilla's beats 6 to the face.
        let mut state = handed_over(json!({
            "p1": { "field": [{ "def": BARKER, "radiant": true }, "core-008"] },
            "p2": { "field": [{ "def": "core-077", "radiant": true }] },
        }));
        let barker = active_units_of(&state, AI)
            .into_iter()
            .find(|unit| unit.def_id == BARKER)
            .map(|unit| unit.id.clone())
            .expect("Mommy Barker is on p1's field");
        let unit = find_instance_mut(&mut state, &barker).expect("Mommy Barker");
        unit.buffs = AttackHealth { attack: 1, health: 1 };
        let view = unit_view(&state, find_instance(&state, &barker).expect("Mommy Barker"));
        assert_eq!((view.attack, view.health), (5, 5));

        let reply = simulate_reply(
            &state,
            AI,
            &create_node_counter(20, None),
            &hidden_card_ids(&state, AI),
            Prices::NONE,
        )
        .expect("a reply");
        assert!(in_graveyard(&reply, AI, "core-008"));
        let barker = find_instance(&reply, &barker).expect("Mommy Barker");
        assert!(
            unit_has(&reply, barker, KeywordKind::Reborn),
            "its Reborn is unspent"
        );
        assert_eq!(reply.players[AI].hero.health, 30);
    }

    #[test]
    fn stops_at_the_seats_own_start_of_turn_prompt_which_is_where_its_next_decision_begins() {
        let state = handed_over(json!({ "p1": { "backrow": ["core-065"] } }));
        let reply = simulate_reply(
            &state,
            AI,
            &create_node_counter(20, None),
            &hidden_card_ids(&state, AI),
            Prices::NONE,
        )
        .expect("a reply");
        assert_eq!(reply.active, AI);
        assert_eq!(reply.turn, TURN + 2);
        assert_eq!(reply.pending.as_ref().map(|pending| pending.player_id), Some(AI));
    }

    #[test]
    fn replays_the_units_the_seats_own_flood_bounced_into_the_opponents_hand_and_nothing_it_held_unseen() {
        jackioh_cards::register_all();
        let s = scenario(json!({
            "seed": "reply-flood",
            "active": "p1",
            "turn": TURN,
            "p1": { "hand": ["core-017"], "mana": 4, "library": LIBRARY },
            "p2": { "field": ["core-019"], "hand": ["core-008"], "library": LIBRARY },
        }));
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

        let reply = simulate_reply(&ended, AI, &create_node_counter(40, None), &hidden, Prices::NONE)
            .expect("a reply");
        // Midrange Menace came back; the Mr. Vanilla p2 held from the start stayed in its hand.
        assert!(on_field(&reply, HUMAN, "core-019"));
        assert!(!on_field(&reply, HUMAN, "core-008"));

        let blind = simulate_reply(
            &ended,
            AI,
            &create_node_counter(40, None),
            &hidden_card_ids(&ended, AI),
            Prices::NONE,
        )
        .expect("a reply");
        assert!(!on_field(&blind, HUMAN, "core-019"));
        assert!(!on_field(&blind, HUMAN, "core-008"));
    }

    #[test]
    fn returns_null_without_a_node_to_spend() {
        let state = handed_over(json!({ "p2": { "field": ["core-008"] } }));
        assert!(
            simulate_reply(
                &state,
                AI,
                &create_node_counter(0, None),
                &hidden_card_ids(&state, AI),
                Prices::NONE,
            )
            .is_none()
        );
    }
}

mod reply_score {
    use super::*;

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
            Prices::NONE,
        )
        .expect("a reply");
        let counter = create_node_counter(20, None);
        let score = reply_score(
            &ended,
            AI,
            TURN,
            &counter,
            &hidden_card_ids(&ended, AI),
            Prices::NONE,
        );
        // A finalist's score (#1010): the hand eval while the switch is off.
        assert_eq!(
            score,
            Some(finalist_score(&after, AI, NextSwing::Seat, Prices::NONE) - AI_EVAL.unspent_mana * 2.0)
        );
        assert_eq!(counter.used(), 2);
        assert_eq!(after.players[AI].hero.health, 26);
    }

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
            reply_score(&won, AI, TURN, &counter, &hidden_card_ids(&won, AI), Prices::NONE),
            Some(static_score(&won, AI, TURN, Prices::NONE))
        );
        assert_eq!(counter.used(), 0);
    }

    #[test]
    fn is_null_when_the_counter_runs_out_during_the_reply() {
        let state = handed_over(json!({ "p2": { "field": ["core-008"] } }));
        assert!(
            reply_score(
                &state,
                AI,
                TURN,
                &create_node_counter(1, None),
                &hidden_card_ids(&state, AI),
                Prices::NONE,
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

    #[test]
    fn a_face_up_power_to_punish_is_lethal_on_the_seat_at_2_health_and_the_line_scores_as_that_loss() {
        let state = handed_over(json!({ "p1": { "health": 2 }, "p2": { "backrow": [PUNISH] } }));
        let reply = simulate_reply(
            &state,
            AI,
            &create_node_counter(20, None),
            &hidden_card_ids(&state, AI),
            Prices::NONE,
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
            Prices::NONE,
        );
        assert_eq!(score, loss_score(&state));
    }

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
            Prices::NONE,
        )
        .expect("a reply");
        assert_eq!(winner(&reply), Some(Winner::from(HUMAN)));
        let score = reply_score(
            &state,
            AI,
            TURN,
            &create_node_counter(20, None),
            &hidden_card_ids(&state, AI),
            Prices::NONE,
        );
        assert_eq!(score, loss_score(&state));
    }

    #[test]
    fn tries_the_activation_aimed_at_the_seats_hero_first_so_a_busy_board_never_hides_the_lethal() {
        let state = handed_over(json!({
            "p1": { "health": 2, "field": ["core-008", "core-008", "core-008", "core-008"] },
            "p2": { "backrow": [PUNISH] },
        }));
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
            Prices::NONE,
        )
        .expect("a reply");
        assert_eq!(winner(&reply), Some(Winner::from(HUMAN)));
    }

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
            Prices::NONE,
        )
        .expect("a reply");
        let still = reply.players[HUMAN].backrow[0].as_ref().expect("still set");
        assert_eq!(still.id, set.id);
        assert!(still.set_as.is_some());
        assert!(!on_field(&reply, HUMAN, CRACKER));
        assert_eq!(reply.players[AI].hero.health, state.players[AI].hero.health);
    }

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
                Prices::NONE,
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

mod open_lethal_1634 {
    use super::*;

    /// B41, #1634: two attackers whose hits only together reach the seat's hero take the face, not a
    /// kill worth more than one hit.
    #[test]
    fn b41_takes_a_lethal_that_needs_both_attacks_over_a_kill_worth_more_than_one_hit() {
        let state = handed_over(json!({
            "p1": { "health": 8, "field": ["core-008"] },
            "p2": { "field": ["core-025", "core-025"] },
        }));
        let hidden = hidden_card_ids(&state, AI);

        let counter = create_node_counter(20, None);
        let reply = simulate_reply_with(&state, AI, &counter, &hidden, true, Prices::NONE).expect("a reply");
        assert_eq!(
            reply.result.map(|result| result.winner),
            Some(Winner::from(HUMAN))
        );
        assert!(on_field(&reply, AI, "core-008"));
        assert_eq!(counter.used(), 2);

        // One hit at a time, the kill outranks the face: Mr. Vanilla dies and the seat lives on 1.
        let blind = simulate_reply_with(
            &state,
            AI,
            &create_node_counter(20, None),
            &hidden,
            false,
            Prices::NONE,
        )
        .expect("a reply");
        assert!(blind.result.is_none());
        assert!(in_graveyard(&blind, AI, "core-008"));
        assert_eq!(blind.players[AI].hero.health, 1);

        // `simulate_reply` plays by the shipped switch.
        let shipped = simulate_reply(&state, AI, &create_node_counter(20, None), &hidden, Prices::NONE)
            .expect("a reply");
        assert_eq!(shipped.result.is_some(), AI_REPLY.open_lethal);
    }

    /// #1634: at 8 health with Mr. Vanilla against two 4-mana 7/7s, lethal only together, the AI puts
    /// Mr. Vanilla in Defense Position, whose Taunt takes one swing, and survives the swing back. One
    /// hit at a time, the reply sees a kill and a 7, and the AI swings at the face and dies.
    #[test]
    fn decide_blocks_one_of_two_attackers_that_are_lethal_only_together() {
        let run = run_puzzle(
            "open-lethal-1634",
            json!({
                "p1": { "health": 8, "field": ["core-008"], "library": LIBRARY },
                "p2": { "health": 12, "field": ["core-025", "core-025"], "library": LIBRARY },
            }),
        );
        assert!(run.end.result.is_none(), "{}", trace(&run.turn));
        let after = all_out_attack(&run.end, HUMAN);
        let survives = after.result.as_ref().map(|result| result.winner) != Some(Winner::from(HUMAN));
        assert_eq!(survives, AI_REPLY.open_lethal, "{}", trace(&run.turn));
    }
}

mod defense_switch_1696 {
    use super::*;

    /// p2's unit `def_id` on its field.
    fn unit_of(state: &GameState, def_id: &str) -> CardInstance {
        active_units_of(state, HUMAN)
            .into_iter()
            .find(|unit| unit.def_id == def_id)
            .cloned()
            .expect("the unit is on p2's field")
    }

    /// What p2's own evaluation gains by switching its unit `def_id` to Defense Position, the seat
    /// swinging next: the comparison the reply's trial switch makes.
    fn switch_gain(state: &GameState, def_id: &str) -> f64 {
        let switch = ActionBody::SwitchPosition {
            instance_id: unit_of(state, def_id).id,
        };
        assert!(is_legal(state, HUMAN, switch.clone()));
        let switched = act(state, HUMAN, &switch);
        evaluate(&switched, HUMAN, NextSwing::Enemy, &AI_EVAL)
            - evaluate(state, HUMAN, NextSwing::Enemy, &AI_EVAL)
    }

    /// #1696: Mr. Vanilla's one attack is into the seat's Defense-Position wall, worth nothing, and the
    /// seat's 7/7 in Attack Position would finish p2 at 7. Defense Position's Taunt takes that swing, so
    /// p2 switches Mr. Vanilla to Defense and ends its turn.
    #[test]
    fn switches_a_unit_with_no_attack_worth_making_to_defense_when_its_evaluation_prefers_it() {
        let state = handed_over(json!({
            "p1": { "field": [{ "def": "core-025", "position": "DEF" }, "core-025"] },
            "p2": { "health": 7, "field": ["core-008"] },
        }));
        assert!(switch_gain(&state, "core-008") > 0.0);

        let counter = create_node_counter(20, None);
        let reply = simulate_reply(&state, AI, &counter, &hidden_card_ids(&state, AI), Prices::NONE)
            .expect("a reply");
        assert_eq!(
            unit_view(&reply, &unit_of(&reply, "core-008")).position,
            Position::Def
        );
        assert_eq!(reply.players[HUMAN].hero.health, 7);
        assert_eq!(reply.active, AI);
        assert_eq!(counter.used(), 2);
    }

    /// #1696: the seat's only unit is a Defense-Position wall, so there is no swing to stop and Defense
    /// Position would only cost Mr. Vanilla half its attack: the trial switch is dropped.
    #[test]
    fn keeps_a_unit_with_no_attack_worth_making_in_attack_when_its_evaluation_prefers_that() {
        let state = handed_over(json!({
            "p1": { "field": [{ "def": "core-025", "position": "DEF" }] },
            "p2": { "field": ["core-008"] },
        }));
        assert!(switch_gain(&state, "core-008") < 0.0);

        let counter = create_node_counter(20, None);
        let reply = simulate_reply(&state, AI, &counter, &hidden_card_ids(&state, AI), Prices::NONE)
            .expect("a reply");
        assert_eq!(
            unit_view(&reply, &unit_of(&reply, "core-008")).position,
            Position::Atk
        );
        assert_eq!(reply.active, AI);
        assert_eq!(counter.used(), 2);
    }

    /// #1696, R49: Deft Duelist hits the open face and, its switch still unspent, goes to Defense
    /// Position, where its Taunt takes the swing of the seat's 7/7 that would finish p2 at 7.
    #[test]
    fn a_deft_unit_that_attacked_switches_to_defense_with_the_switch_it_has_left() {
        let state = handed_over(json!({
            "p1": { "field": ["core-025"] },
            "p2": { "health": 7, "field": ["core-045"] },
        }));
        let counter = create_node_counter(20, None);
        let reply = simulate_reply(&state, AI, &counter, &hidden_card_ids(&state, AI), Prices::NONE)
            .expect("a reply");
        assert_eq!(reply.players[AI].hero.health, 26);
        assert_eq!(
            unit_view(&reply, &unit_of(&reply, "core-045")).position,
            Position::Def
        );
        assert_eq!(counter.used(), 3);
    }
}
