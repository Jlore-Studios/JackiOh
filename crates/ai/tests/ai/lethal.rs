//! The lethal solver's two walks (lethal.rs, SPEC §9.9): a depth-first walk in move order for
//! AI_SEARCH.lethalQuickNodes nodes, then a best-first walk by `ready_gap` on the rest of the allowance.
//! The board below is one the depth-first walk alone cannot solve in 150 nodes (a test holds that, so
//! the board keeps exercising the second walk): the lethal starts with a Lava Golem that tributes both
//! enemy Taunts, and the cheap targeted spells beside it put hundreds of lines ahead of that play in
//! move order. The best-first walk ranks the Golem's tributes by what they leave the attackers, and
//! finds it.
//!
//! Port of `packages/ai/test/lethal.test.ts`. TS's "the depth-first walk alone" case set
//! `AI_SEARCH.lethalQuickNodes` at run time; `AI_SEARCH` is a `const` here, so the case asks the
//! solver with the depth-first share given (`find_lethal_with_quick_nodes`, see the notes' GAPS).

use indexmap::IndexSet;
use jackioh_ai::{
    AI_BUDGET, AI_EVAL, AI_GATE, AI_GATE_BUDGET, AI_SEARCH, DecisionReason, DeterminizeOptions, LineStatus,
    MatchHooks, Matchup, NextSwing, NodeCounter, action_key, candidate_actions, create_node_counter,
    damage_past_taunts, determinize, evaluate, find_lethal, find_lethal_with_quick_nodes, game_config,
    line_status, play_match, ready_gap, redact, search_signature, simulate,
};
use jackioh_engine::testkit::{
    ActionBody, ActionType, CardInstance, GameState, IndexMap, PLAYER_IDS, PlayerId, Row, Value, Winner,
    ZoneChoice, attacks_per_turn, create_rng, find_instance, json, json_as, legal_actions, opponent_of,
    seat_to_act, unit_view,
};

use super::support::{
    AI, HUMAN, every_card, random_policy_states, register_cards, run_puzzle, scenario, trace,
};

/// p1 (the AI) on turn 9 with 4 crystals: Pointmaster 7/1, Mr. Vanilla 4/4 and Tempo Timmy 3/3
/// ready on the field, a Radiant Lava Golem, Lunar Eclipse and KY's Math Equation in hand. p2 stands
/// behind two Taunts, Midrange Menace 9/9 and a Lava Golem 10/5, at 10 health. The lethal: the Golem
/// tributes both enemy Taunts and one of p1's two smaller units, then Pointmaster and the other hit
/// the face for 10 or more. The Golem is Radiant because the base face paid with opposing units is summoned for the
/// opponent (R360), where its Taunt would wall the face again.
fn wide() -> Value {
    json!({
        "p1": { "hand": [{ "def": "core-055", "radiant": true }, "core-035", "core-031"], "mana": 4, "field": ["core-020", "core-008", "core-011"] },
        "p2": { "field": ["core-019", "core-055"], "health": 10 },
    })
}

fn wide_board() -> GameState {
    register_cards();
    let mut options = json!({ "seed": "lethal-wide", "active": AI, "turn": 9 });
    if let (Some(base), Value::Object(extra)) = (options.as_object_mut(), wide()) {
        for (key, value) in extra {
            base.insert(key, value);
        }
    }
    scenario(options).state().clone()
}

/// The one instance of `defId` that `player` controls, in hand or on the field.
fn mine(state: &GameState, player: PlayerId, def_id: &str) -> CardInstance {
    let found: Vec<&CardInstance> = every_card(state)
        .into_iter()
        .filter(|card| card.def_id == def_id && card.controller == player)
        .collect();
    assert!(found.len() == 1, "{player} controls {} {def_id}", found.len());
    found[0].clone()
}

fn worlds(state: &GameState, seed: &str, count: usize) -> Vec<GameState> {
    let mut rng = create_rng(seed, 0);
    let public = redact(state, AI);
    (0..count)
        .map(|_| determinize(&public, AI, &mut rng, DeterminizeOptions::default()))
        .collect()
}

/// ready_gap as it was first written: the attackers are the units `legal_actions` lists an attack for.
/// lethal.rs now asks the engine unit by unit instead, which is cheaper on a wide hand; this is the
/// reference it must agree with.
fn ready_gap_by_legal_actions(state: &GameState, seat: PlayerId) -> f64 {
    if let Some(result) = state.result {
        return if result.winner == Winner::from(seat) {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    let mut ready: IndexSet<String> = IndexSet::new();
    for action in legal_actions(state, seat) {
        if let ActionBody::Attack { attacker_id, .. } = action {
            ready.insert(attacker_id);
        }
    }
    let mut attacks: Vec<(&CardInstance, i32)> = Vec::new();
    for id in &ready {
        let Some(unit) = find_instance(state, id) else {
            continue;
        };
        let attack = unit_view(state, unit).attack;
        if attack > 0 {
            // R636: once per attack the unit has left this turn.
            let spent = unit.exertion.attacks.unwrap_or(i32::from(unit.exertion.attacked));
            for _ in spent..attacks_per_turn(state, unit) {
                attacks.push((unit, attack));
            }
        }
    }
    let opp = opponent_of(seat);
    f64::from(state.players[opp].hero.health - damage_past_taunts(state, opp, &attacks))
}

/// p1 (the AI) holds Silence (0 mana, "Vanilla a Unit.") beside Pointmaster 7/1 and Mr. Vanilla 4/4,
/// ready; p2 stands behind Midrange Menace, a 9/9 Taunt, at 10 health. Silence on the Taunt and both
/// attacks to the face win; Silence on an own unit reaches a position that differs only in which unit
/// is Vanilla. Both sides hold a library, so ending the turn decks nobody out.
fn silence_board() -> Value {
    json!({
        "p1": { "hand": ["classicplus-009"], "field": ["core-020", "core-008"], "library": ["core-008", "core-011"] },
        "p2": { "field": ["core-019"], "health": 10, "library": ["core-008", "core-011"] },
    })
}

/// `legal_actions` by `action_key`, each play's lane blanked: `search_signature` leaves a unit's lane
/// out, so the two lanes `candidate_actions` keeps for one play stay one position.
fn moves_but_lanes(state: &GameState, seat: PlayerId) -> Vec<String> {
    legal_actions(state, seat)
        .into_iter()
        .map(|mut action| {
            if let ActionBody::Play { zone: Some(zone), .. } = &mut action {
                zone.lane = -1;
            }
            action_key(&action)
        })
        .collect()
}

/// Plays `line` on `state` through `simulate`; panics on a refusal.
fn play_out(state: &GameState, line: &[ActionBody]) -> GameState {
    let per_action = 1 + AI_SEARCH.max_auto_answers as usize * AI_SEARCH.max_auto_answer_options as usize;
    let counter = create_node_counter(line.len() * per_action, None);
    let mut current = state.clone();
    for action in line {
        match simulate(&current, AI, action, &counter) {
            Some(Ok(next)) => current = next,
            _ => panic!(
                "the line was refused at {}",
                serde_json::to_string(action).unwrap()
            ),
        }
    }
    current
}

mod the_lethal_solvers_walks {
    use super::*;

    #[test]
    fn ready_gap_is_the_enemy_heros_health_less_what_the_attacks_left_deal_it_past_its_taunts() {
        let state = wide_board();
        // The two Taunts soak the three attackers (7, 4, 3) and nothing reaches the hero.
        assert_eq!(ready_gap(&state, AI), 10.0);

        let golem = mine(&state, AI, "core-055");
        let menace = mine(&state, HUMAN, "core-019");
        let enemy_golem = mine(&state, HUMAN, "core-055");
        let vanilla = mine(&state, AI, "core-008");
        let cleared = play_out(
            &state,
            &[ActionBody::Play {
                instance_id: golem.id.clone(),
                zone: Some(ZoneChoice {
                    row: Row::Units,
                    lane: 4,
                }),
                x: None,
                embiggen: None,
                equip: None,
                tributes: Some(vec![
                    vanilla.id.clone(),
                    menace.id.clone(),
                    enemy_golem.id.clone(),
                ]),
                targets: None,
                modes: None,
                plague: None,
                face_down: None,
            }],
        );
        // No Taunt left, and Pointmaster (7) and Tempo Timmy (3) are still to attack: exactly lethal.
        // The new Golem has not been on the field a turn, so it adds nothing.
        assert_eq!(ready_gap(&cleared, AI), 0.0);
    }

    #[test]
    fn ready_gap_counts_exactly_the_units_legal_actions_lists_an_attack_for_from_either_seat() {
        let mut states: Vec<GameState> = vec![wide_board()];
        states.extend(random_policy_states("lethal-ready-gap", 3, 600));
        states.extend(random_policy_states("lethal-ready-gap-2", 5, 600));
        // Patch v0.1.1's rules moved these seeded games off the boards they used to reach, so a third
        // series keeps the count of positions with attackers ready above the floor below.
        states.extend(random_policy_states("lethal-ready-gap-3", 3, 600));
        // The unban lane's dealt decks play shorter games than the port's, so a fourth series and a
        // second played match keep the coverage floor honest.
        states.extend(random_policy_states("lethal-ready-gap-4", 3, 600));
        for game_index in [1, 2] {
            let mut config = game_config(
                Matchup::AiVsGreedy,
                game_index,
                AI_GATE_BUDGET,
                AI_GATE.seed_series,
            );
            config.max_actions = Some(150);
            let mut played: Vec<GameState> = Vec::new();
            {
                let mut hooks = MatchHooks {
                    after_action: Some(Box::new(
                        |before: &GameState, _after: &GameState, _seat: PlayerId, _action: &ActionBody| {
                            played.push(before.clone());
                        },
                    )),
                    ..MatchHooks::default()
                };
                play_match(&config, &mut hooks);
            }
            states.extend(played);
        }

        let mut with_attackers = 0;
        for state in &states {
            for seat in PLAYER_IDS {
                let expected = ready_gap_by_legal_actions(state, seat);
                assert_eq!(ready_gap(state, seat), expected, "turn {}, {seat}", state.turn);
                let opp = opponent_of(seat);
                if state.result.is_none() && expected < f64::from(state.players[opp].hero.health) {
                    with_attackers += 1;
                }
            }
        }
        // The comparison covered positions with attackers ready, not only empty boards.
        assert!(with_attackers > 20);
    }

    #[test]
    fn finds_the_golems_tribute_lethal_on_a_wide_hand_within_the_allowance_and_it_wins_on_every_world() {
        let dets = worlds(&wide_board(), "lethal-wide", 2);
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        let line = find_lethal(&dets, AI, &counter, AI_BUDGET.lethal_nodes);
        assert!(line.is_some());
        assert!(counter.used() <= AI_BUDGET.lethal_nodes);
        // The depth-first walk's share ran out, so the line came from the best-first walk.
        assert!(counter.used() > AI_SEARCH.lethal_quick_nodes as usize);

        let line = line.unwrap_or_default();
        let first = line.first();
        assert_eq!(first.map(ActionBody::action_type), Some(ActionType::Play));
        let state = wide_board();
        let enemy_taunts = [
            mine(&state, HUMAN, "core-019").id,
            mine(&state, HUMAN, "core-055").id,
        ];
        let tributes: Vec<String> = match first {
            Some(ActionBody::Play { tributes, .. }) => tributes.clone().unwrap_or_default(),
            _ => vec![],
        };
        for taunt in &enemy_taunts {
            assert!(tributes.contains(taunt), "{tributes:?} lacks {taunt}");
        }
        for det in &dets {
            assert_eq!(
                play_out(det, &line)
                    .result
                    .and_then(|result| result.winner.player()),
                Some(AI)
            );
        }
    }

    #[test]
    fn the_depth_first_walk_alone_does_not_find_it_within_the_same_allowance() {
        // lethalQuickNodes at the whole allowance is the solver before the best-first walk existed. If a
        // change to move order let it find this line, the board would no longer test the second walk.
        let dets = worlds(&wide_board(), "lethal-wide", 2);
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        assert!(
            find_lethal_with_quick_nodes(
                &dets,
                AI,
                &counter,
                AI_BUDGET.lethal_nodes,
                AI_BUDGET.lethal_nodes
            )
            .is_none()
        );
        assert_eq!(counter.used(), AI_BUDGET.lethal_nodes);
    }

    #[test]
    fn stops_after_the_depth_first_walk_when_that_walk_searched_the_whole_tree() {
        register_cards();
        // One attacker into a 20-health hero, a 1-drop in hand: a handful of lines, none lethal.
        let s = scenario(json!({
            "seed": "lethal-small",
            "active": AI,
            "turn": 9,
            "p1": { "hand": ["core-008"], "field": ["core-011"], "mana": 3, "library": ["core-008", "core-011"] },
            "p2": { "field": ["core-008"], "library": ["core-008", "core-011"], "health": 20 },
        }));
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        assert!(
            find_lethal(
                &worlds(s.state(), "lethal-small", 2),
                AI,
                &counter,
                AI_BUDGET.lethal_nodes
            )
            .is_none()
        );
        assert!(counter.used() > 0);
        assert!(counter.used() < AI_SEARCH.lethal_quick_nodes as usize);
    }

    #[test]
    fn a_whole_ai_turn_on_the_wide_board_kills_the_hero_lethal_from_the_first_decision() {
        let run = run_puzzle("lethal-wide", wide());
        assert_eq!(
            run.end.result.and_then(|result| result.winner.player()),
            Some(AI),
            "{}",
            trace(&run.turn)
        );
        assert_eq!(
            run.turn.decisions.first().map(|decision| decision.reason),
            Some(DecisionReason::Lethal),
            "{}",
            trace(&run.turn)
        );
    }
}

mod the_positions_the_search_merges {
    use super::*;

    #[test]
    fn silencing_the_enemy_taunt_is_not_the_position_silencing_an_own_unit_reaches_and_the_solver_finds_its_lethal()
     {
        register_cards();
        let mut options = json!({ "seed": "lethal-silence", "active": AI, "turn": 9 });
        if let (Some(base), Value::Object(extra)) = (options.as_object_mut(), silence_board()) {
            for (key, value) in extra {
                base.insert(key, value);
            }
        }
        let state = scenario(options).state().clone();

        let silence = mine(&state, AI, "classicplus-009");
        let silencing = |target: &CardInstance| -> ActionBody {
            json_as::<ActionBody>(json!({
                "type": "play",
                "instanceId": silence.id,
                "targets": [{ "pick": "instance", "instanceId": target.id }],
            }))
        };
        let on_taunt = silencing(&mine(&state, HUMAN, "core-019"));
        let on_own = silencing(&mine(&state, AI, "core-020"));
        // Two positions that play differently: the Taunt walls the face in one and not in the other.
        assert_ne!(
            search_signature(&play_out(&state, std::slice::from_ref(&on_taunt)), AI),
            search_signature(&play_out(&state, &[on_own]), AI)
        );

        let dets = worlds(&state, "lethal-silence", 2);
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        let line = find_lethal(&dets, AI, &counter, AI_BUDGET.lethal_nodes).unwrap_or_default();
        assert_eq!(line.first().map(action_key), Some(action_key(&on_taunt)));
        for det in &dets {
            assert_eq!(
                play_out(det, &line)
                    .result
                    .and_then(|result| result.winner.player()),
                Some(AI)
            );
        }
    }

    #[test]
    fn two_one_step_children_with_one_signature_score_the_same_and_offer_the_same_moves() {
        register_cards();
        let mut merged = 0;
        for k in 1..=8 {
            for state in random_policy_states(&format!("signature-{k}"), 2, 600) {
                let Some(seat) = seat_to_act(&state) else {
                    continue;
                };
                let counter = create_node_counter(1_000_000, None);
                let mut groups: IndexMap<String, Vec<GameState>> = IndexMap::new();
                for action in candidate_actions(&state, seat) {
                    let Some(Ok(child)) = simulate(&state, seat, &action, &counter) else {
                        continue;
                    };
                    if line_status(&child, seat, state.turn) != LineStatus::Open {
                        continue;
                    }
                    groups
                        .entry(search_signature(&child, seat))
                        .or_default()
                        .push(child);
                }
                for children in groups.values() {
                    let first = &children[0];
                    for other in &children[1..] {
                        merged += 1;
                        assert_eq!(
                            evaluate(first, seat, NextSwing::Enemy, &AI_EVAL),
                            evaluate(other, seat, NextSwing::Enemy, &AI_EVAL),
                            "turn {}, {seat}",
                            state.turn
                        );
                        assert_eq!(
                            moves_but_lanes(first, seat),
                            moves_but_lanes(other, seat),
                            "turn {}, {seat}",
                            state.turn
                        );
                    }
                }
            }
        }
        assert!(merged > 200, "{merged} merges");
    }

    #[test]
    fn ready_gap_counts_a_windfury_unit_once_per_attack_it_has_left() {
        register_cards();
        // p1 has no library and no hand, so the Prime's attack trigger summons nothing.
        let fresh = scenario(json!({
            "seed": "lethal-windfury",
            "active": AI,
            "turn": 9,
            "p1": { "field": ["meditative-091-1"] },
            "p2": { "health": 20, "library": ["core-008", "core-011"] },
        }))
        .state()
        .clone();
        // Windfurious Prime is 5 attack: two attacks left, 10 off 20 health.
        assert_eq!(ready_gap(&fresh, AI), 10.0);

        let prime = mine(&fresh, AI, "meditative-091-1");
        let swing = ActionBody::Attack {
            attacker_id: prime.id.clone(),
            target_id: format!("hero-{HUMAN}"),
        };
        let once = play_out(&fresh, std::slice::from_ref(&swing));
        assert_eq!(once.players[HUMAN].hero.health, 15);
        // One swing is left: 5 off 15 health.
        assert_eq!(ready_gap(&once, AI), 10.0);
        assert_eq!(ready_gap(&once, AI), ready_gap_by_legal_actions(&once, AI));

        // Both swings in one line: `simulate` numbers its actions from its counter, so a second call
        // from `once` would repeat the first swing's action id.
        let twice = play_out(&fresh, &[swing.clone(), swing]);
        assert_eq!(twice.players[HUMAN].hero.health, 10);
        // Both swings are spent: nothing left to count, the gap is the hero's health.
        assert_eq!(ready_gap(&twice, AI), 10.0);
    }
}
