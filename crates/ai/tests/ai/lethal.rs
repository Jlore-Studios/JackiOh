//! The lethal solver's two walks (lethal.rs, SPEC §9.9): a depth-first walk in move order for
//! AI_SEARCH.lethalQuickNodes nodes, then a best-first walk by `ready_gap` on the rest of the allowance.
//! The board below is one the depth-first walk alone cannot solve in 150 nodes: the cheap targeted
//! spells beside its Lava Golem put hundreds of lines ahead of the Golem's tributes in move order. The
//! best-first walk ranks those tributes by what they leave the attackers, and finds the lethal.

use indexmap::IndexSet;
use jackioh_ai::{
    AI_BUDGET, AI_EVAL, AI_GATE, AI_GATE_BUDGET, AI_SEARCH, DecisionReason, DeterminizeOptions, LineStatus,
    MatchHooks, Matchup, NextSwing, NodeCounter, Prices, action_key, candidate_actions, create_node_counter,
    damage_past_taunts, determinize, evaluate, face_threat, find_lethal, find_lethal_with_quick_nodes,
    game_config, line_status, play_match, ready_gap, redact, search_signature, simulate,
};
use jackioh_engine::testkit::{
    ActionBody, ActionType, AttackTarget, CardInstance, DuosLife, GameState, IndexMap, PLAYER_IDS, PlayerId,
    Row, Value, Winner, ZoneChoice, attacks_per_turn, create_rng, find_instance, find_instance_mut, json,
    json_as, legal_actions, opponent_of, seat_to_act, unit_view, why_cannot_attack,
};

use super::support::{
    AI, HUMAN, every_card, random_policy_states, register_cards, run_puzzle, scenario, trace,
};

/// p1 (the AI), 4 crystals, holds a Radiant Lava Golem and two cheap spells; p2 is at 10 behind two Taunts.
/// The lethal: the Golem tributes both enemy Taunts and one of p1's two smaller units, then Pointmaster
/// and the other hit the face for 10 or more. The Golem is Radiant because the base face paid with
/// opposing units is summoned for the opponent (R360), where its Taunt would wall the face again.
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

/// The engine's refusal of an attack that only its Taunt wall stops (§4.2 step 3, the last test
/// `why_cannot_attack` makes).
const TAUNT_WALL: &str = "a Taunt unit must be attacked first";

/// The reference ready_gap must agree with: the attackers are the units `legal_actions` lists an
/// attack for. Whether one may hit the hero is asked of the engine's whole test, since `legal_actions`
/// lists no hero attack behind a Taunt: the hero is refused for nothing but the Taunt wall.
/// `hero_ineligible_attacks` pins the numbers by hand (#1823).
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
    let opp = opponent_of(seat);
    let hero = AttackTarget::Hero { player: opp };
    let mut attacks: Vec<(&CardInstance, i32, bool)> = Vec::new();
    for id in &ready {
        let Some(unit) = find_instance(state, id) else {
            continue;
        };
        let attack = unit_view(state, unit).attack;
        if attack > 0 {
            let may_hit_hero = match why_cannot_attack(state, unit, &hero) {
                Ok(()) => true,
                Err(refused) => refused.message == TAUNT_WALL,
            };
            // R636: once per attack the unit has left this turn.
            let spent = unit.exertion.attacks.unwrap_or(i32::from(unit.exertion.attacked));
            for _ in spent..attacks_per_turn(state, unit) {
                attacks.push((unit, attack, may_hit_hero));
            }
        }
    }
    f64::from(state.players[opp].hero.health - damage_past_taunts(state, opp, &attacks))
}

/// p1 (the AI) holds Silence (0 mana, "Vanilla a Unit.") beside Pointmaster 7/1 and Mr. Vanilla 4/4; p2
/// stands behind Midrange Menace, a 9/9 Taunt, at 10 health. Silence on the Taunt and both attacks to the
/// face win; Silence on an own unit reaches a position that differs only in which unit is Vanilla. Both
/// sides hold a library, so ending the turn decks nobody out.
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
        match simulate(&current, AI, action, &counter, Prices::NONE) {
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
        assert_eq!(ready_gap(&cleared, AI), 0.0);
    }

    #[test]
    fn ready_gap_counts_exactly_the_units_legal_actions_lists_an_attack_for_from_either_seat() {
        let mut states: Vec<GameState> = vec![wide_board()];
        states.extend(random_policy_states("lethal-ready-gap", 3, 600));
        states.extend(random_policy_states("lethal-ready-gap-2", 5, 600));
        // The third and fourth series and the second played match keep the coverage floor below honest.
        states.extend(random_policy_states("lethal-ready-gap-3", 3, 600));
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
        assert!(with_attackers > 20);
    }

    #[test]
    fn finds_the_golems_tribute_lethal_on_a_wide_hand_within_the_allowance_and_it_wins_on_every_world() {
        let dets = worlds(&wide_board(), "lethal-wide", 2);
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        let line = find_lethal(&dets, AI, &counter, AI_BUDGET.lethal_nodes, Prices::NONE);
        assert!(line.is_some());
        assert!(counter.used() <= AI_BUDGET.lethal_nodes);
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
                AI_BUDGET.lethal_nodes,
                Prices::NONE,
            )
            .is_none()
        );
        assert_eq!(counter.used(), AI_BUDGET.lethal_nodes);
    }

    #[test]
    fn stops_after_the_depth_first_walk_when_that_walk_searched_the_whole_tree() {
        register_cards();
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
                AI_BUDGET.lethal_nodes,
                Prices::NONE,
            )
            .is_none()
        );
        assert!(counter.used() > 0);
        assert!(counter.used() < AI_SEARCH.lethal_quick_nodes as usize);
    }

    /// p1 (the AI) on turn 9 with `def_id` in Defense Position, on the field since before this turn;
    /// p2 at 4 health with no unit, so no Taunt. Both sides hold a library.
    fn in_defense_at_4(def_id: &str) -> GameState {
        register_cards();
        scenario(json!({
            "seed": "lethal-deft",
            "active": AI,
            "turn": 9,
            "p1": { "field": [{ "def": def_id, "position": "DEF" }], "library": ["core-008", "core-011"] },
            "p2": { "health": 4, "library": ["core-008", "core-011"] },
        }))
        .state()
        .clone()
    }

    #[test]
    fn switches_a_deft_unit_out_of_defense_position_and_attacks_the_hero_for_lethal() {
        // R49: Deft Duelist's switch to Attack spends only its switch, so its 4 still reaches the hero.
        let state = in_defense_at_4("core-045");
        let duelist = mine(&state, AI, "core-045");
        let dets = worlds(&state, "lethal-deft", 2);
        let line = find_lethal(
            &dets,
            AI,
            &create_node_counter(AI_BUDGET.nodes, None),
            AI_BUDGET.lethal_nodes,
            Prices::NONE,
        );
        let expected = vec![
            ActionBody::SwitchPosition {
                instance_id: duelist.id.clone(),
            },
            ActionBody::Attack {
                attacker_id: duelist.id.clone(),
                target_id: format!("hero-{HUMAN}"),
            },
        ];
        assert_eq!(line, Some(expected.clone()));
        for det in &dets {
            assert_eq!(
                play_out(det, &expected)
                    .result
                    .and_then(|result| result.winner.player()),
                Some(AI)
            );
        }
    }

    #[test]
    fn finds_no_lethal_for_a_unit_in_defense_position_without_deft() {
        let state = in_defense_at_4("core-008");
        let dets = worlds(&state, "lethal-deft", 2);
        assert!(
            find_lethal(
                &dets,
                AI,
                &create_node_counter(AI_BUDGET.nodes, None),
                AI_BUDGET.lethal_nodes,
                Prices::NONE,
            )
            .is_none()
        );
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
        assert_ne!(
            search_signature(&play_out(&state, std::slice::from_ref(&on_taunt)), AI),
            search_signature(&play_out(&state, &[on_own]), AI)
        );

        let dets = worlds(&state, "lethal-silence", 2);
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        let line = find_lethal(&dets, AI, &counter, AI_BUDGET.lethal_nodes, Prices::NONE).unwrap_or_default();
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
                for action in candidate_actions(&state, seat, Prices::NONE) {
                    let Some(Ok(child)) = simulate(&state, seat, &action, &counter, Prices::NONE) else {
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
        let fresh = scenario(json!({
            "seed": "lethal-windfury",
            "active": AI,
            "turn": 9,
            "p1": { "field": ["meditative-091-1"] },
            "p2": { "health": 20, "library": ["core-008", "core-011"] },
        }))
        .state()
        .clone();
        assert_eq!(ready_gap(&fresh, AI), 10.0);

        let prime = mine(&fresh, AI, "meditative-091-1");
        let swing = ActionBody::Attack {
            attacker_id: prime.id.clone(),
            target_id: format!("hero-{HUMAN}"),
        };
        let once = play_out(&fresh, std::slice::from_ref(&swing));
        assert_eq!(once.players[HUMAN].hero.health, 15);
        assert_eq!(ready_gap(&once, AI), 10.0);
        assert_eq!(ready_gap(&once, AI), ready_gap_by_legal_actions(&once, AI));

        // Both swings in one line: `simulate` numbers its actions from its counter, so a second call
        // from `once` would repeat the first swing's action id.
        let twice = play_out(&fresh, &[swing.clone(), swing]);
        assert_eq!(twice.players[HUMAN].hero.health, 10);
        assert_eq!(ready_gap(&twice, AI), 10.0);
    }
}

mod duos_life_r964000 {
    use super::*;

    /// `state` with p2 a Duos seat at its health and a full 20 pool, `spent` or not.
    fn with_pool(state: &GameState, spent: bool) -> GameState {
        let mut out = state.clone();
        out.players[HUMAN].duos = Some(DuosLife {
            pool: 20,
            pool_cap: 20,
            spent,
        });
        out
    }

    #[test]
    fn r964000_ready_gap_measures_against_own_hp_and_the_pool_in_a_duos_duel() {
        register_cards();
        let state = scenario(json!({
            "seed": "lethal-duos",
            "active": AI,
            "turn": 9,
            "p1": { "field": ["core-020", "core-008", "core-011"] },
            "p2": { "health": 20 },
        }))
        .state()
        .clone();
        assert_eq!(ready_gap(&state, AI), 6.0, "a 1v1 hero at 20");
        assert_eq!(
            ready_gap(&with_pool(&state, false), AI),
            26.0,
            "20 own HP and a 20 pool"
        );
        assert_eq!(ready_gap(&with_pool(&state, true), AI), 6.0);
    }
}

/// R1965000 (issue #1965): C #90 In Too Deep's reward L lets the AI play cards from its graveyard
/// (R454), so a (0) Spell there can be played again and again: C #37 Last Hurrah, M #62 Small Time
/// Recruits and The Coin. The best-first walk proves the board's lethal behind replays of the Spells,
/// which `decide`, planning again after each action, would replay without end; it plays the line
/// trimmed to the actions it needs (`trim_lethal`).
mod free_replays_r1965000 {
    use super::*;
    use std::cell::Cell;

    use jackioh_ai::{AiOptions, AiTurnResult, decide, play_ai_turn, trim_lethal};
    use jackioh_engine::testkit::{Action, Scenario, reduce, subsystems};

    const IN_TOO_DEEP: &str = "classic-090";
    const LAST_HURRAH: &str = "classic-037";
    const SMALL_TIME_RECRUITS: &str = "meditative-062";
    const COIN: &str = "core-t-coin";
    /// Meditative+ #1.1 Base Camp, a (0) Spell token: two Coins to its player's hand.
    const BASE_CAMP: &str = "meditativeplus-001-1";
    /// Meditative+ #17 Dogpile, (1): 2 damage to an enemy, and casts another Dogpile in hand at it.
    const DOGPILE: &str = "meditativeplus-017";
    const BIGOT: &str = "core-002";
    const SEVEN_SEVEN: &str = "core-025";
    const FELINOR_TOKEN: &str = "core-t-felinor";
    const STOCKPILE: &str = "core-005";

    /// In Too Deep's line as quest 9 leaves it, with reward L held.
    fn hold_reward_l(s: &mut Scenario) {
        let itd = s.card(IN_TOO_DEEP).id.clone();
        find_instance_mut(s.state_mut(), &itd)
            .expect("In Too Deep in the backrow")
            .memory
            .insert(
                subsystems::QUEST_MEMORY_KEY.to_string(),
                json!({ "active": [], "progress": {}, "done": ["9"], "auras": ["L"], "waiting": [] }),
            );
    }

    /// The board above, with `graveyard` in p1's graveyard.
    fn board(graveyard: &[&str]) -> GameState {
        register_cards();
        let mut s = scenario(json!({
            "seed": "r1965000",
            "active": AI,
            "turn": 9,
            "p1": {
                "field": [SEVEN_SEVEN, BIGOT],
                "backrow": [IN_TOO_DEEP],
                "graveyard": graveyard,
                "hand": [],
                "library": [],
            },
            "p2": {
                "health": 6,
                "field": [{ "def": FELINOR_TOKEN, "position": "DEF" }],
                "hand": [STOCKPILE],
                "library": [STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE],
            },
        }));
        hold_reward_l(&mut s);
        s.state().clone()
    }

    fn ai_turn(start: &GameState) -> AiTurnResult {
        let mut ai = AiOptions::with_budget(create_rng("r1965000", 0), AI_BUDGET);
        play_ai_turn(start, AI, &mut ai)
    }

    /// How many of the turn's actions play `id`.
    fn plays_of(turn: &AiTurnResult, id: &str) -> usize {
        turn.actions
            .iter()
            .filter(
                |action| matches!(&action.body, ActionBody::Play { instance_id, .. } if instance_id == id),
            )
            .count()
    }

    /// Who won when the turn stopped, if anyone did.
    fn winner(turn: &AiTurnResult) -> Option<PlayerId> {
        turn.state
            .result
            .as_ref()
            .and_then(|result| result.winner.player())
    }

    /// Three worlds of `state` for the solver, as `decide` makes them.
    fn worlds(state: &GameState, seed: &str) -> Vec<GameState> {
        let public = redact(state, AI);
        let mut rng = create_rng(seed, 0);
        (0..AI_BUDGET.determinizations)
            .map(|_| determinize(&public, AI, &mut rng, DeterminizeOptions::default()))
            .collect()
    }

    /// The AI's turn on `board(graveyard)`: it wins this turn, inside play_ai_turn's ceiling, and plays
    /// none of the graveyard's cards, which the lethal does not need, though each is offered.
    fn wins_without_replaying(graveyard: &[&str]) {
        let start = board(graveyard);
        let turn = ai_turn(&start);
        assert_eq!(winner(&turn), Some(AI), "{}", trace(&turn));
        for def in graveyard {
            let id = mine(&start, AI, def).id;
            assert!(
                legal_actions(&start, AI).iter().any(
                    |action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == id)
                ),
                "{def} is offered from the graveyard"
            );
            assert_eq!(plays_of(&turn, &id), 0, "{def}: {}", trace(&turn));
        }
    }

    #[test]
    fn r1965000_free_spells_that_change_nothing_are_not_played_before_the_lethal() {
        wins_without_replaying(&[LAST_HURRAH, SMALL_TIME_RECRUITS]);
    }

    #[test]
    fn r1965000_a_coin_the_lethal_does_not_need_is_not_played_before_it() {
        wins_without_replaying(&[COIN]);
    }

    #[test]
    fn r1965000_a_coin_and_the_two_spells_are_not_played_before_the_lethal() {
        wins_without_replaying(&[COIN, LAST_HURRAH, SMALL_TIME_RECRUITS]);
    }

    #[test]
    fn r1965000_trim_lethal_cuts_the_replays_the_walk_proved_the_lethal_behind() {
        let start = board(&[COIN]);
        let dets = worlds(&start, "r1965000-trim");
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        let found = find_lethal(&dets, AI, &counter, AI_BUDGET.lethal_nodes, Prices::NONE).expect("a lethal");
        let coin = mine(&start, AI, COIN).id;
        assert!(
            matches!(found.first(), Some(ActionBody::Play { instance_id, .. }) if *instance_id == coin),
            "the walk opens with the Coin: {found:?}"
        );
        let trimmed =
            trim_lethal(&dets, AI, found, &counter, Prices::NONE).expect("the trim fits the budget");
        let bigot = mine(&start, AI, BIGOT).id;
        let seven = mine(&start, AI, SEVEN_SEVEN).id;
        let token = mine(&start, HUMAN, FELINOR_TOKEN).id;
        assert_eq!(
            trimmed,
            vec![
                ActionBody::Attack {
                    attacker_id: bigot,
                    target_id: token
                },
                ActionBody::Attack {
                    attacker_id: seven,
                    target_id: format!("hero-{HUMAN}")
                },
            ]
        );
        assert!(counter.used() <= AI_BUDGET.nodes, "{} nodes", counter.used());
    }

    /// The decisions a clock-stopped turn below may take before it must have ended.
    const CLOCKED_DECISIONS: usize = 30;

    /// A clock that stops a decision inside the trim (a slow device under the practice clock, a ladder
    /// bot's acting clock) leaves no proved lethal: the decision ends the turn, never playing the untrimmed
    /// line's Coin, which a clock stopping each decision at the same point would play at every one. Every
    /// stop to the unhurried decision's last node is tried, so the trim's window is among them.
    #[test]
    fn r1965000_a_clock_that_stops_the_trim_plays_no_coin_and_the_turn_ends() {
        let start = board(&[COIN]);
        let coin = mine(&start, AI, COIN).id;
        let unhurried = decide(
            &start,
            AI,
            &mut AiOptions::with_budget(create_rng("r1965000", 0), AI_BUDGET),
        )
        .expect("the AI acts")
        .stats
        .nodes;
        for stop_after in 0..=unhurried {
            let mut state = start.clone();
            let mut rng = create_rng("r1965000", 0);
            for n in 0..CLOCKED_DECISIONS {
                let polls = Cell::new(0usize);
                let stop = || {
                    polls.set(polls.get() + 1);
                    polls.get() > stop_after
                };
                let mut ai = AiOptions {
                    rng,
                    budget: AI_BUDGET,
                    should_stop: Some(&stop),
                };
                let decision = decide(&state, AI, &mut ai).expect("the AI acts");
                rng = ai.rng;
                assert!(
                    !matches!(&decision.action, ActionBody::Play { instance_id, .. } if *instance_id == coin),
                    "stopped after {stop_after} polls, decision {n} plays the Coin"
                );
                let next = reduce(&state, &Action::new(decision.action, AI, format!("t{n}")));
                assert_eq!(next.error, None, "stopped after {stop_after} polls, decision {n}");
                state = next.state;
                if state.result.is_some() || state.active != AI {
                    break;
                }
            }
            assert!(
                state.result.is_some() || state.active != AI,
                "stopped after {stop_after} polls, the turn did not end"
            );
        }
    }

    /// A replay the lethal does need is kept: Base Camp, already played this turn and back in the
    /// graveyard, adds two Coins at no cost, and a Coin's mana pays for a Dogpile, which casts the other
    /// at the enemy hero for 4. Every step is needed, so the trim keeps them all.
    #[test]
    fn r1965000_a_replay_the_lethal_needs_is_found_and_kept() {
        register_cards();
        let mut s = scenario(json!({
            "seed": "r1965000-base-camp",
            "active": AI,
            "turn": 9,
            "p1": {
                "mana": 0,
                "backrow": [IN_TOO_DEEP],
                "graveyard": [BASE_CAMP],
                "hand": [DOGPILE, DOGPILE],
                "library": [],
            },
            "p2": {
                "health": 4,
                "hand": [STOCKPILE],
                "library": [STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE],
            },
        }));
        hold_reward_l(&mut s);
        let camp = s.card(BASE_CAMP).id.clone();
        let log = &mut s.state_mut().players[AI].turn_log;
        log.played_ids.push(camp.clone());
        log.cards_played += 1;
        let start = s.state().clone();

        let dets = worlds(&start, "r1965000-base-camp");
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        let found = find_lethal(&dets, AI, &counter, AI_BUDGET.lethal_nodes, Prices::NONE).expect("a lethal");
        assert!(
            matches!(found.first(), Some(ActionBody::Play { instance_id, .. }) if *instance_id == camp),
            "{found:?}"
        );
        assert_eq!(
            trim_lethal(&dets, AI, found.clone(), &counter, Prices::NONE),
            Some(found)
        );

        let turn = ai_turn(&start);
        assert_eq!(winner(&turn), Some(AI), "{}", trace(&turn));
        assert_eq!(plays_of(&turn, &camp), 1, "{}", trace(&turn));
    }
}

mod hero_ineligible_attacks {
    use super::*;

    /// p1 (the AI) on turn 9 with `field` ready on the field, p2 at 10 health with `enemy` on its field.
    /// Both sides hold a library.
    fn board(seed: &str, field: Value, enemy: Value) -> GameState {
        register_cards();
        scenario(json!({
            "seed": seed,
            "active": AI,
            "turn": 9,
            "p1": { "field": field, "library": ["core-008", "core-011"] },
            "p2": { "field": enemy, "health": 10, "library": ["core-008", "core-011"] },
        }))
        .state()
        .clone()
    }

    /// `state` with the AI's `def_id` summoned this turn: `scenario` places no unit summoning sick.
    fn fresh(state: &GameState, def_id: &str) -> GameState {
        let mut out = state.clone();
        let id = mine(&out, AI, def_id).id;
        let turn = out.turn;
        find_instance_mut(&mut out, &id).expect("the unit").summoned_turn = Some(turn);
        out
    }

    #[test]
    fn a_fresh_rush_unit_with_a_unit_to_attack_leaves_the_gap_at_the_heros_health() {
        let ready = board("lethal-fresh-rush", json!(["classic-076"]), json!(["core-008"]));
        assert_eq!(ready_gap(&ready, AI), 6.0);
        // §6.1: on its summon turn Rush lets it attack units only, so nothing reaches the hero.
        let sick = fresh(&ready, "classic-076");
        assert_eq!(ready_gap(&sick, AI), 10.0);
        assert_eq!(ready_gap_by_legal_actions(&sick, AI), 10.0);
        assert_eq!(face_threat(&sick, AI), 4);
    }

    #[test]
    fn a_fresh_charge_unit_still_counts_at_the_hero() {
        let state = fresh(
            &board("lethal-fresh-charge", json!(["core-045"]), json!(["core-008"])),
            "core-045",
        );
        assert_eq!(ready_gap(&state, AI), 6.0);
    }

    #[test]
    fn a_fresh_rush_unit_still_binds_a_taunt_and_frees_a_ready_attacker_for_the_face() {
        let ready = board(
            "lethal-fresh-taunt",
            json!(["core-032", "core-011"]),
            json!(["classicplus-026"]),
        );
        let sick = fresh(&ready, "core-032");
        assert_eq!(ready_gap(&sick, AI), 7.0);
        assert_eq!(ready_gap_by_legal_actions(&sick, AI), 7.0);
        assert_eq!(face_threat(&sick, AI), 0);
    }
}
