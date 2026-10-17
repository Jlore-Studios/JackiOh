//! Candidate moves and the search's contract (docs/polish/3-ai.md B14, B15).
//!
//! B14: `candidateActions` is `legalActions` minus the four action types the AI never searches
//! (R84's concede, offerDraw and answerDraw, and the mulligan), with plays that differ only in their
//! lane collapsed to the lowest lane of each placement, at most `AI_SEARCH.zone_variants` of them
//! (#1014), and endTurn last whenever it is legal.
//!
//! B15: `decide` is a pure function of (state, seat, rng seed, budget): the same inputs give a
//! deep-equal Decision, it never spends more nodes than the budget, its action is always legal, and
//! a wall clock that says stop at the first poll still yields a legal action instead of a throw.
//!
//! Port of `packages/ai/test/search.test.ts`. TS's per-test `{ timeout }` has no `cargo test` twin.

use std::cell::Cell;

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, act, dealt_game, is_legal};

const SKIPPED: &[ActionType] = &[
    ActionType::Concede,
    ActionType::OfferDraw,
    ActionType::AnswerDraw,
    ActionType::Mulligan,
];

/// TS's `{ rng: createRng(seed), budget }`, with no clock.
fn ai_options(seed: &str, budget: SearchBudget) -> AiOptions<'static> {
    AiOptions {
        rng: create_rng(seed, 0),
        budget,
        should_stop: None,
    }
}

/// A value as its JSON: `SearchStats.stoppedBy` is compared by its literal, whatever its Rust name.
fn js<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

/// An action body from TS's object literal.
fn body(literal: Value) -> ActionBody {
    json_as(literal)
}

/// A mid-game p1 turn with plays in several lanes, a targeted spell, attacks and hidden p2 cards.
fn mid_game() -> GameState {
    jackioh_cards::register_all();
    scenario(json!({
        "seed": "search-mid",
        "p1": { "hand": ["core-008", "core-035"], "field": ["core-011"], "library": ["core-020", "core-053"] },
        "p2": {
            "hand": ["core-005", "core-016"],
            "field": ["core-008"],
            "backrow": [{ "def": "core-041", "faceUp": false }],
            "library": ["core-019", "core-047"],
            "health": 20,
        },
    }))
    .state()
    .clone()
}

/// A play with its zone removed: the "otherwise identical" key B14 groups by.
fn without_zone(action: &ActionBody) -> String {
    match action {
        ActionBody::Play { .. } => {
            let mut rest = action.clone();
            if let ActionBody::Play { zone, .. } = &mut rest {
                *zone = None;
            }
            action_key(&rest)
        }
        _ => action_key(action),
    }
}

fn lane_of(action: &ActionBody) -> Option<i32> {
    match action {
        ActionBody::Play { zone: Some(zone), .. } => Some(zone.lane),
        _ => None,
    }
}

/// One zone as B14's placement reads it: `|` past the board's edge, else the id of the card acting
/// there (`-` for none), the Unit a carrier holds there, how many cards lie beneath and `L` when it is
/// Locked.
fn zone_reading(state: &GameState, slot: ZoneSlot) -> String {
    if slot.lane < 1 || slot.lane > row_size(slot.row) {
        return "|".to_string();
    }
    format!(
        "{}{}:{}{}",
        card_at(state, slot).map_or("-", |card| card.id.as_str()),
        carried_at(state, slot).map_or(String::new(), |unit| format!("+{}", unit.id)),
        beneath_at(state, slot).len(),
        if is_locked(state, slot) { "L" } else { "" },
    )
}

/// B14's placement of a play into `zone` (#1014): the seat's zones beside it in its row, left then
/// right, or the board's edge, then the lane's four zones.
fn placement(state: &GameState, seat: PlayerId, zone: &ZoneChoice) -> String {
    let other_row = if zone.row == Row::Units {
        Row::Backrow
    } else {
        Row::Units
    };
    let enemy = seat.opponent();
    [
        (seat, zone.row, zone.lane - 1),
        (seat, zone.row, zone.lane + 1),
        (seat, zone.row, zone.lane),
        (seat, other_row, zone.lane),
        (enemy, Row::Units, zone.lane),
        (enemy, Row::Backrow, zone.lane),
    ]
    .into_iter()
    .map(|(player, row, lane)| zone_reading(state, ZoneSlot { player, row, lane }))
    .collect::<Vec<_>>()
    .join("/")
}

/// The lanes `candidate_actions` offers the AI's hand card of `def_id`, lowest first.
fn offered_lanes(state: &GameState, def_id: &str) -> Vec<i32> {
    let id = hand_id(state, def_id);
    let mut lanes: Vec<i32> = candidate_actions(state, AI)
        .iter()
        .filter_map(|action| match action {
            ActionBody::Play {
                instance_id,
                zone: Some(zone),
                ..
            } if *instance_id == id => Some(zone.lane),
            _ => None,
        })
        .collect();
    lanes.sort();
    lanes
}

/// The id of the AI's hand card of `def_id`.
fn hand_id(state: &GameState, def_id: &str) -> String {
    state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == def_id)
        .map(|card| card.id.clone())
        .expect("the card is in the AI's hand")
}

fn is_play(action: &ActionBody) -> bool {
    matches!(action, ActionBody::Play { .. })
}

// ---------------------------------------------------------------------------------------------
// B14
// ---------------------------------------------------------------------------------------------

mod candidate_actions_b14 {
    use super::*;

    /// B14: actionKey is canonical: key order does not matter, and different actions differ
    #[test]
    fn b14_action_key_is_canonical_key_order_does_not_matter_and_different_actions_differ() {
        assert_eq!(
            action_key(&body(
                json!({ "type": "attack", "attackerId": "c1", "targetId": "hero-p2" })
            )),
            action_key(&body(
                json!({ "targetId": "hero-p2", "attackerId": "c1", "type": "attack" })
            )),
        );
        assert_ne!(
            action_key(&body(
                json!({ "type": "attack", "attackerId": "c1", "targetId": "hero-p2" })
            )),
            action_key(&body(
                json!({ "type": "attack", "attackerId": "c1", "targetId": "c9" })
            )),
        );
        assert_ne!(
            action_key(&body(
                json!({ "type": "play", "instanceId": "c1", "zone": { "row": "units", "lane": 2 } })
            )),
            action_key(&body(
                json!({ "type": "play", "instanceId": "c1", "zone": { "row": "units", "lane": 3 } })
            )),
        );
    }

    /// B14: is legalActions minus concede, offerDraw, answerDraw and mulligan, zones collapsed, endTurn last
    #[test]
    fn b14_is_legal_actions_minus_concede_offer_draw_answer_draw_and_mulligan_zones_collapsed_end_turn_last()
    {
        let state = mid_game();
        let legal = legal_actions(&state, AI);
        let candidates = candidate_actions(&state, AI);
        let legal_keys: IndexSet<String> = legal.iter().map(action_key).collect();

        // Nothing that is not legal, and none of the four skipped types.
        for action in &candidates {
            assert!(legal_keys.contains(&action_key(action)), "{}", action_key(action));
            assert!(!SKIPPED.contains(&action.action_type()));
        }
        // No duplicates.
        let candidate_keys: IndexSet<String> = candidates.iter().map(action_key).collect();
        assert_eq!(candidate_keys.len(), candidates.len());

        // Every legal non-play action of a kept type is a candidate.
        for action in &legal {
            if is_play(action) || SKIPPED.contains(&action.action_type()) {
                continue;
            }
            assert!(
                candidate_keys.contains(&action_key(action)),
                "{}",
                action_key(action)
            );
        }

        // Plays: per otherwise-identical play, the lowest lane of each placement, in legal order, up to
        // AI_SEARCH.zone_variants of them, and no other lane (#1014).
        let cap = AI_SEARCH.zone_variants as usize;
        let mut groups: IndexMap<String, Vec<ActionBody>> = IndexMap::new();
        for action in &legal {
            if !is_play(action) {
                continue;
            }
            groups
                .entry(without_zone(action))
                .or_default()
                .push(action.clone());
        }
        assert!(
            groups.values().any(|group| group.len() > 2),
            "some play is offered in 3+ lanes"
        );
        for (key, group) in &groups {
            let offered: Vec<&ActionBody> = candidates
                .iter()
                .filter(|action| is_play(action) && without_zone(action) == *key)
                .collect();
            if group.iter().all(|action| lane_of(action).is_none()) {
                let offered_keys: Vec<String> = offered.iter().map(|action| action_key(action)).collect();
                let group_keys: Vec<String> = group.iter().map(action_key).collect();
                assert_eq!(offered_keys, group_keys, "{key}");
                continue;
            }
            let mut placements: IndexSet<String> = IndexSet::new();
            let mut expected: Vec<Option<i32>> = Vec::new();
            for action in group {
                let ActionBody::Play { zone: Some(zone), .. } = action else {
                    continue;
                };
                if placements.len() < cap && placements.insert(placement(&state, AI, zone)) {
                    expected.push(Some(zone.lane));
                }
            }
            expected.sort_by_key(|lane| lane.unwrap_or(0));
            let mut offered_lanes: Vec<Option<i32>> = offered.iter().map(|action| lane_of(action)).collect();
            offered_lanes.sort_by_key(|lane| lane.unwrap_or(0));
            assert_eq!(offered_lanes, expected, "{key}");
        }

        // endTurn is last, once.
        assert_eq!(
            candidates
                .iter()
                .filter(|action| **action == ActionBody::EndTurn)
                .count(),
            1
        );
        assert_eq!(candidates.last(), Some(&ActionBody::EndTurn));
    }

    /// B14: a play with one free lane keeps that lane, and one with two free lanes keeps both
    #[test]
    fn b14_a_play_with_one_free_lane_keeps_that_lane_and_one_with_two_free_lanes_keeps_both() {
        jackioh_cards::register_all();
        let one = scenario(json!({
            "seed": "search-one-lane",
            "p1": { "hand": ["core-008"], "field": ["core-011", "core-020", "core-019", "core-025"] },
            "p2": { "hand": ["core-005"] },
        }))
        .state()
        .clone();
        let one_lanes: Vec<Option<i32>> = candidate_actions(&one, AI)
            .iter()
            .filter(|action| is_play(action))
            .map(lane_of)
            .collect();
        assert_eq!(one_lanes, vec![Some(5)]);

        let two = scenario(json!({
            "seed": "search-two-lanes",
            "p1": { "hand": ["core-008"], "field": ["core-011", "core-020", "core-019"] },
            "p2": { "hand": ["core-005"] },
        }))
        .state()
        .clone();
        let mut two_lanes: Vec<Option<i32>> = candidate_actions(&two, AI)
            .iter()
            .filter(|action| is_play(action))
            .map(lane_of)
            .collect();
        two_lanes.sort_by_key(|lane| lane.unwrap_or(0));
        assert_eq!(two_lanes, vec![Some(4), Some(5)]);
    }

    /// B14: a play keeps the lowest lane of each placement and none past AI_SEARCH.zone_variants (#1014)
    #[test]
    fn b14_a_play_keeps_the_lowest_lane_of_each_placement_and_none_past_the_cap() {
        jackioh_cards::register_all();
        let cap = AI_SEARCH.zone_variants as usize;
        // An empty row: lane 1 has the edge on its left, lane 5 on its right, and lanes 2 to 4 are one
        // placement.
        let empty = scenario(json!({
            "seed": "search-empty-row",
            "p1": { "hand": ["core-008"] },
            "p2": { "hand": ["core-005"] },
        }))
        .state()
        .clone();
        let mut expected: Vec<i32> = [1, 2, 5].into_iter().take(cap).collect();
        expected.sort();
        assert_eq!(offered_lanes(&empty, "core-008"), expected);

        // A unit opposite every lane: each lane is its own placement, so the lowest ones up to the cap.
        let crowded = scenario(json!({
            "seed": "search-crowded-row",
            "p1": { "hand": ["core-008"] },
            "p2": { "field": ["core-011", "core-011", "core-011", "core-011", "core-011"] },
        }))
        .state()
        .clone();
        assert_eq!(
            offered_lanes(&crowded, "core-008"),
            (1..=5).take(cap).collect::<Vec<i32>>()
        );
    }

    /// B14: an attack on the enemy hero leads, plays come before position switches, and endTurn trails
    #[test]
    fn b14_an_attack_on_the_enemy_hero_leads_plays_come_before_position_switches_and_end_turn_trails() {
        let state = mid_game();
        let candidates = candidate_actions(&state, AI);
        let first = candidates.first();
        assert_eq!(first.map(|action| action.action_type()), Some(ActionType::Attack));
        let target = match first {
            Some(ActionBody::Attack { target_id, .. }) => Some(target_id.as_str()),
            _ => None,
        };
        assert_eq!(target, Some("hero-p2"));

        let last_play = candidates
            .iter()
            .enumerate()
            .filter(|(_, action)| is_play(action))
            .map(|(at, _)| at)
            .max();
        let first_switch = candidates
            .iter()
            .position(|action| matches!(action, ActionBody::SwitchPosition { .. }));
        if let Some(first_switch) = first_switch {
            // TS's `Math.max(...[])` is -Infinity, so with no play any switch comes after.
            assert!(last_play.is_none_or(|last_play| first_switch > last_play));
        }
        assert_eq!(
            candidates
                .iter()
                .position(|action| *action == ActionBody::EndTurn),
            Some(candidates.len() - 1)
        );
    }

    /// B14: a seat answering its own prompt gets exactly the prompt's answers and no endTurn
    #[test]
    fn b14_a_seat_answering_its_own_prompt_gets_exactly_the_prompts_answers_and_no_end_turn() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "search-prompt",
            "p1": { "hand": ["core-072"], "graveyard": ["core-044", "core-008", "core-005"], "library": ["core-011"] },
            "p2": { "hand": ["core-005"] },
        }));
        s.play("core-072", json!({}));
        let state = s.state().clone();
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(AI));

        let candidates = candidate_actions(&state, AI);
        // R211 offers concede beside the prompt's answers, and the AI never takes it (R84, R188).
        let answers: Vec<ActionBody> = legal_actions(&state, AI)
            .into_iter()
            .filter(|action| *action != ActionBody::Concede)
            .collect();
        let mut candidate_keys: Vec<String> = candidates.iter().map(action_key).collect();
        candidate_keys.sort();
        let mut answer_keys: Vec<String> = answers.iter().map(action_key).collect();
        answer_keys.sort();
        assert_eq!(candidate_keys, answer_keys);
        assert!(
            candidates
                .iter()
                .all(|action| matches!(action, ActionBody::Answer { .. }))
        );
        assert_eq!(candidates.len(), 3);
    }

    /// B14: during the mulligan there is no candidate, since the mulligan is not searched
    #[test]
    fn b14_during_the_mulligan_there_is_no_candidate_since_the_mulligan_is_not_searched() {
        let state = dealt_game("search-mulligan");
        assert_eq!(
            mulligan_prompt_for(&state, AI).map(|prompt| prompt.kind),
            Some(PromptKind::Mulligan)
        );
        assert!(!legal_actions(&state, AI).is_empty());
        assert_eq!(candidate_actions(&state, AI), Vec::<ActionBody>::new());
    }

    /// B14: a seat facing an opponent's draw offer on the opponent's turn has no candidate
    #[test]
    fn b14_a_seat_facing_an_opponents_draw_offer_on_the_opponents_turn_has_no_candidate() {
        jackioh_cards::register_all();
        let s = scenario(json!({
            "seed": "search-offer",
            "active": "p2",
            "turn": 10,
            "p1": { "hand": ["core-008"] },
            "p2": { "hand": ["core-011"] },
        }));
        let offered = act(s.state(), PlayerId::P2, &ActionBody::OfferDraw);
        assert!(
            legal_actions(&offered, AI)
                .iter()
                .any(|action| matches!(action, ActionBody::AnswerDraw { .. }))
        );
        assert_eq!(candidate_actions(&offered, AI), Vec::<ActionBody>::new());
    }

    /// B14: a finished game has no candidate for either seat
    #[test]
    fn b14_a_finished_game_has_no_candidate_for_either_seat() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "search-over",
            "p1": { "field": ["core-011"], "hand": ["core-008"] },
            "p2": { "health": 3, "hand": ["core-005"] },
        }));
        s.attack("core-011", "hero");
        assert!(s.state().result.is_some());
        assert_eq!(candidate_actions(s.state(), AI), Vec::<ActionBody>::new());
        assert_eq!(
            candidate_actions(s.state(), PlayerId::P2),
            Vec::<ActionBody>::new()
        );
    }

    /// B14: on the opponent's turn with nothing to answer there is no candidate
    #[test]
    fn b14_on_the_opponents_turn_with_nothing_to_answer_there_is_no_candidate() {
        jackioh_cards::register_all();
        let state = scenario(
            json!({ "seed": "search-idle", "active": "p2", "turn": 10, "p1": { "hand": ["core-008"] } }),
        )
        .state()
        .clone();
        assert_eq!(candidate_actions(&state, AI), Vec::<ActionBody>::new());
    }

    /// R1282: A wide tribute card's candidate plays are cut to AI_SEARCH.tribute_sets (120) and stay legal
    #[test]
    fn r1282_a_wide_tribute_s_plays_are_cut_to_the_tribute_sets_cap_and_stay_legal() {
        jackioh_cards::register_all();
        // 5 cheap units on each side = 10 cheap permanents.
        // Tribute 5 chooses 5 out of 10 = 252 tribute combinations.
        let s = scenario(json!({
            "preview": ["Meditative"],
            "p1": {
                "field": [
                    { "def": "core-011", "lane": 1 },
                    { "def": "core-011", "lane": 2 },
                    { "def": "core-011", "lane": 3 },
                    { "def": "core-011", "lane": 4 },
                    { "def": "core-011", "lane": 5 },
                ],
                "hand": ["meditative-097-4"],
                "mana": 10,
            },
            "p2": {
                "field": [
                    { "def": "core-011", "lane": 1 },
                    { "def": "core-011", "lane": 2 },
                    { "def": "core-011", "lane": 3 },
                    { "def": "core-011", "lane": 4 },
                    { "def": "core-011", "lane": 5 },
                ],
            }
        }));
        let state = s.state();
        let legal = legal_actions(state, AI);
        let candidates = candidate_actions(state, AI);

        // Extract distinct tribute sets for Mega Church from candidate actions
        let church_id = &state.players.p1.hand[0].id;
        let mut candidate_tribute_sets = std::collections::BTreeSet::new();
        for action in &candidates {
            if let ActionBody::Play {
                instance_id,
                tributes: Some(t),
                ..
            } = action
                && instance_id == church_id
            {
                let mut sorted = t.clone();
                sorted.sort();
                candidate_tribute_sets.insert(sorted);
            }
        }

        // Distinct tribute sets in candidates is exactly capped at 120 (AI_SEARCH.tribute_sets)
        assert_eq!(candidate_tribute_sets.len(), 120);

        // Every candidate action must be legal
        let legal_keys: IndexSet<String> = legal.iter().map(action_key).collect();
        for action in &candidates {
            assert!(legal_keys.contains(&action_key(action)), "{}", action_key(action));
        }
    }

    /// R1282: A shipped tribute card keeps every tribute set
    #[test]
    fn r1282_a_shipped_tribute_keeps_every_set() {
        jackioh_cards::register_all();
        // A tribute play with fewer sets than the cap (e.g. Boom Big Max Tribute 2 on 3 units -> 3 sets)
        let s = scenario(json!({
            "p1": {
                "field": [
                    { "def": "core-011", "lane": 1 },
                    { "def": "core-011", "lane": 2 },
                    { "def": "core-011", "lane": 3 },
                ],
                "hand": ["classic-080"],
                "mana": 10,
            },
            "p2": {}
        }));
        let state = s.state();
        let legal = legal_actions(state, AI);
        let candidates = candidate_actions(state, AI);

        let boom_id = &state.players.p1.hand[0].id;
        let mut legal_tribute_sets = std::collections::BTreeSet::new();
        for action in &legal {
            if let ActionBody::Play {
                instance_id,
                tributes: Some(t),
                ..
            } = action
                && instance_id == boom_id
            {
                let mut sorted = t.clone();
                sorted.sort();
                legal_tribute_sets.insert(sorted);
            }
        }

        let mut candidate_tribute_sets = std::collections::BTreeSet::new();
        for action in &candidates {
            if let ActionBody::Play {
                instance_id,
                tributes: Some(t),
                ..
            } = action
                && instance_id == boom_id
            {
                let mut sorted = t.clone();
                sorted.sort();
                candidate_tribute_sets.insert(sorted);
            }
        }

        assert_eq!(candidate_tribute_sets, legal_tribute_sets);
        assert!(!candidate_tribute_sets.is_empty());
    }
}

// ---------------------------------------------------------------------------------------------
// #1014: the lanes that play differently
// ---------------------------------------------------------------------------------------------

mod lanes_that_play_differently {
    use super::*;

    /// Both sides' libraries, so that no simulated end of turn decks anyone out.
    const LIBRARY: &[&str] = &["core-008", "core-011", "core-008", "core-011"];

    /// C #71 Lane Eater destroys every other card in its lane: with lanes 1, 3 and 5 free and the
    /// enemy's 9/9 Taunt opposite lane 3, lane 3 is a candidate and `decide` plays it there.
    #[test]
    fn lane_eater_is_tried_opposite_the_enemy_taunt_and_decide_plays_it_there() {
        jackioh_cards::register_all();
        let state = scenario(json!({
            "seed": "search-lane-eater",
            "p1": {
                "hand": ["classic-071"],
                "field": [{ "def": "core-011", "lane": 2 }, { "def": "core-011", "lane": 4 }],
                "library": LIBRARY,
            },
            "p2": { "field": [{ "def": "core-019", "lane": 3 }], "library": LIBRARY },
        }))
        .state()
        .clone();
        assert!(offered_lanes(&state, "classic-071").contains(&3));

        let eater = hand_id(&state, "classic-071");
        let decision =
            decide(&state, AI, &mut ai_options("search-lane-eater", AI_BUDGET)).expect("a decision");
        assert!(
            matches!(
                &decision.action,
                ActionBody::Play { instance_id, zone: Some(zone), .. }
                    if *instance_id == eater && zone.lane == 3
            ),
            "{}",
            action_key(&decision.action)
        );
    }

    /// M #23 Golly Bob Howdy turns the Units beside it into Sheep: with its own units in lanes 1 and 5,
    /// the harmless lane 3 is a candidate, and the AI's turn sheeps neither unit.
    #[test]
    fn golly_bob_howdy_is_tried_between_its_own_units_and_the_turn_sheeps_neither() {
        jackioh_cards::register_all();
        let state = scenario(json!({
            "seed": "search-golly",
            "p1": {
                "hand": ["meditative-023"],
                "field": [{ "def": "core-008", "lane": 1 }, { "def": "core-008", "lane": 5 }],
                "library": LIBRARY,
            },
            "p2": { "library": LIBRARY },
        }))
        .state()
        .clone();
        assert!(offered_lanes(&state, "meditative-023").contains(&3));

        let turn = play_ai_turn(&state, AI, &mut ai_options("search-golly", AI_BUDGET));
        assert_eq!(
            active_units_of(&turn.state, AI)
                .iter()
                .filter(|unit| unit.def_id == "core-008")
                .count(),
            2
        );
    }

    /// C+ #20 Mushroom Power buffs the Units beside it: with its own units in lanes 2 and 4, lane 3,
    /// which buffs both, is a candidate.
    #[test]
    fn mushroom_power_is_tried_in_the_lane_between_the_two_units_it_buffs() {
        jackioh_cards::register_all();
        let state = scenario(json!({
            "seed": "search-mushroom",
            "p1": {
                "hand": ["classicplus-020"],
                "field": [{ "def": "core-008", "lane": 2 }, { "def": "core-008", "lane": 4 }],
                "library": LIBRARY,
            },
            "p2": { "library": LIBRARY },
        }))
        .state()
        .clone();
        assert!(offered_lanes(&state, "classicplus-020").contains(&3));
    }
}

// ---------------------------------------------------------------------------------------------
// B15
// ---------------------------------------------------------------------------------------------

mod decides_determinism_and_budget_b15 {
    use super::*;

    /// TS's loop over `[["AI_GATE_BUDGET", AI_GATE_BUDGET], ["AI_BUDGET", AI_BUDGET]]`: the one body both
    /// generated tests run.
    fn same_inputs_same_decision(budget: SearchBudget) {
        let state = mid_game();
        let before = hash_state(&state);
        let first = decide(&state, AI, &mut ai_options("search-b15", budget));
        let second = decide(&state, AI, &mut ai_options("search-b15", budget));

        assert!(first.is_some());
        assert_eq!(second, first);
        let decision = first.expect("a decision");
        assert!(decision.stats.nodes <= budget.nodes);
        assert!(decision.stats.nodes > 0);
        assert!(is_legal(&state, AI, &decision.action));
        assert!(!decision.line.is_empty());
        assert_eq!(action_key(&decision.line[0]), action_key(&decision.action));
        // decide read the state and changed nothing.
        assert_eq!(hash_state(&state), before);
    }

    /// B15: at AI_GATE_BUDGET the same state, seat, rng seed and budget give a deep-equal, legal, in-budget decision
    #[test]
    fn b15_at_ai_gate_budget_the_same_state_seat_rng_seed_and_budget_give_a_deep_equal_legal_in_budget_decision()
     {
        same_inputs_same_decision(AI_GATE_BUDGET);
    }

    /// B15: at AI_BUDGET the same state, seat, rng seed and budget give a deep-equal, legal, in-budget decision
    #[test]
    fn b15_at_ai_budget_the_same_state_seat_rng_seed_and_budget_give_a_deep_equal_legal_in_budget_decision() {
        same_inputs_same_decision(AI_BUDGET);
    }

    /// B15: omitting the budget is AI_BUDGET
    ///
    /// Rust has no omitted field: `AiOptions.budget` defaults to AI_BUDGET where the caller builds it
    /// (SURFACE §9), so the "implicit" decision is the one TS's `{ rng }` alone gave, built the way every
    /// caller without a budget builds it.
    #[test]
    fn b15_omitting_the_budget_is_ai_budget() {
        let state = mid_game();
        let implicit = decide(
            &state,
            AI,
            &mut AiOptions {
                rng: create_rng("search-default", 0),
                budget: AI_BUDGET,
                should_stop: None,
            },
        );
        let explicit = decide(&state, AI, &mut ai_options("search-default", AI_BUDGET));
        assert_eq!(implicit, explicit);
    }

    /// B15: a clock that says stop at the first poll still gives a legal action, stopped by the clock, with no node spent
    #[test]
    fn b15_a_clock_that_says_stop_at_the_first_poll_still_gives_a_legal_action_stopped_by_the_clock_with_no_node_spent()
     {
        let state = mid_game();
        let polls = Cell::new(0);
        let stop = || {
            polls.set(polls.get() + 1);
            true
        };
        let decision = decide(
            &state,
            AI,
            &mut AiOptions {
                rng: create_rng("search-clock", 0),
                budget: AI_GATE_BUDGET,
                should_stop: Some(&stop),
            },
        );
        assert!(decision.is_some());
        assert!(polls.get() > 0);
        let decision = decision.expect("a decision");
        assert_eq!(js(decision.stats.stopped_by), json!("clock"));
        assert_eq!(decision.stats.nodes, 0);
        assert!(is_legal(&state, AI, &decision.action));
    }

    /// B15: a clock that stops after N polls caps the nodes at N
    #[test]
    fn b15_a_clock_that_stops_after_n_polls_caps_the_nodes_at_n() {
        let state = mid_game();
        let limit = 25;
        let polls = Cell::new(0);
        let stop = || {
            polls.set(polls.get() + 1);
            polls.get() > limit
        };
        let decision = decide(
            &state,
            AI,
            &mut AiOptions {
                rng: create_rng("search-clock-n", 0),
                budget: AI_BUDGET,
                should_stop: Some(&stop),
            },
        )
        .expect("a decision");
        assert_eq!(js(decision.stats.stopped_by), json!("clock"));
        assert!(decision.stats.nodes <= limit);
        assert!(is_legal(&state, AI, &decision.action));
    }

    /// B15: a budget of zero nodes falls back to endTurn, stopped by the budget
    #[test]
    fn b15_a_budget_of_zero_nodes_falls_back_to_end_turn_stopped_by_the_budget() {
        let state = mid_game();
        let empty = SearchBudget {
            nodes: 0,
            lethal_nodes: 0,
            ..AI_GATE_BUDGET
        };
        let decision = decide(&state, AI, &mut ai_options("search-empty", empty)).expect("a decision");
        assert_eq!(decision.reason, DecisionReason::Fallback);
        assert_eq!(decision.action, ActionBody::EndTurn);
        assert_eq!(decision.stats.nodes, 0);
        assert_eq!(js(decision.stats.stopped_by), json!("budget"));
    }

    /// B15: the smallest search (one world, a beam of one, one step deep) still gives a legal, in-budget decision
    #[test]
    fn b15_the_smallest_search_one_world_a_beam_of_one_one_step_deep_still_gives_a_legal_in_budget_decision()
    {
        let state = mid_game();
        let tiny = SearchBudget {
            nodes: 12,
            lethal_nodes: 2,
            determinizations: 1,
            beam_width: 1,
            root_branching: 1,
            branching: 1,
            max_depth: 1,
            finalists: 1,
        };
        let decision = decide(&state, AI, &mut ai_options("search-tiny", tiny));
        assert!(decision.is_some());
        let decision = decision.expect("a decision");
        assert!(decision.stats.nodes <= tiny.nodes);
        assert!(is_legal(&state, AI, &decision.action));
    }

    /// B15: with no lethal budget the solver finds nothing, yet a lethal board still gets a legal decision
    #[test]
    fn b15_with_no_lethal_budget_the_solver_finds_nothing_yet_a_lethal_board_still_gets_a_legal_decision() {
        jackioh_cards::register_all();
        let state = scenario(json!({
            "seed": "search-no-lethal",
            "p1": { "field": ["core-011", "core-008"] },
            "p2": { "health": 6 },
        }))
        .state()
        .clone();
        let budget = SearchBudget {
            lethal_nodes: 0,
            ..AI_GATE_BUDGET
        };
        let decision = decide(&state, AI, &mut ai_options("search-no-lethal", budget)).expect("a decision");
        assert_ne!(decision.reason, DecisionReason::Lethal);
        assert!(decision.stats.nodes <= budget.nodes);
        assert!(is_legal(&state, AI, &decision.action));
    }

    /// B15: decide never throws on a state whose hidden cards it cannot know, across many rng seeds
    #[test]
    fn b15_decide_never_throws_on_a_state_whose_hidden_cards_it_cannot_know_across_many_rng_seeds() {
        let state = mid_game();
        for k in 0..6 {
            let decision = decide(
                &state,
                AI,
                &mut ai_options(&format!("search-many:{k}"), AI_GATE_BUDGET),
            );
            assert!(decision.is_some(), "seed {k}");
            let decision = decision.expect("a decision");
            assert!(is_legal(&state, AI, &decision.action), "seed {k}");
            assert!(decision.stats.nodes <= AI_GATE_BUDGET.nodes, "seed {k}");
        }
    }

    /// B15: the node counter grants exactly its limit, then reports the budget
    #[test]
    fn b15_the_node_counter_grants_exactly_its_limit_then_reports_the_budget() {
        let counter = create_node_counter(3, None);
        assert_eq!(
            [counter.take(), counter.take(), counter.take()],
            [true, true, true]
        );
        assert!(!counter.take());
        assert_eq!(counter.used(), 3);
        assert_eq!(counter.limit(), 3);
        assert_eq!(js(counter.stopped_by()), json!("budget"));

        let none = create_node_counter(0, None);
        assert!(!none.take());
        assert_eq!(none.used(), 0);
    }

    /// B15: the node counter polls the clock before each node and reports it
    #[test]
    fn b15_the_node_counter_polls_the_clock_before_each_node_and_reports_it() {
        let polls = Cell::new(0);
        let stop = || {
            polls.set(polls.get() + 1);
            polls.get() > 2
        };
        let counter = create_node_counter(10, Some(&stop));
        assert!(counter.take());
        assert!(counter.take());
        assert!(!counter.take());
        assert_eq!(counter.used(), 2);
        assert_eq!(js(counter.stopped_by()), json!("clock"));
    }
}

// ---------------------------------------------------------------------------------------------
// #1062: the enemy's targets beside the seat's own
// ---------------------------------------------------------------------------------------------

mod enemy_targets_first {
    use super::*;

    /// Burn (C #36: any unit or hero) in the AI's hand beside two Tempo Timmys and a Mr. Vanilla in
    /// Defense Position, against the 4-mana 7/7: two face attacks lead, and `legal_actions` offers
    /// Burn's own-side targets before the enemy's (`play_choices::sides_for`).
    fn burn_board() -> GameState {
        jackioh_cards::register_all();
        scenario(json!({
            "seed": "candidates-enemy-first",
            "p1": { "hand": ["classic-036"], "field": ["core-011", "core-011", { "def": "core-008", "position": "DEF" }] },
            "p2": { "field": ["core-025"] },
        }))
        .state()
        .clone()
    }

    /// Whether a play's first target is the enemy hero or a card the enemy controls.
    fn aimed_at_the_enemy(state: &GameState, action: &ActionBody) -> bool {
        let first = match action {
            ActionBody::Play { targets, .. } => targets.as_deref().and_then(|targets| targets.first()),
            _ => None,
        };
        match first {
            Some(Selection::Hero { player }) => *player != AI,
            Some(Selection::Instance { instance_id }) => {
                find_instance(state, instance_id).is_some_and(|card| card.controller != AI)
            }
            _ => false,
        }
    }

    /// The action keys of `actions`' plays of `instance_id`, in order.
    fn plays_of(actions: &[ActionBody], instance_id: &str) -> Vec<String> {
        actions
            .iter()
            .filter(|action| matches!(action, ActionBody::Play { instance_id: id, .. } if id == instance_id))
            .map(action_key)
            .collect()
    }

    /// #1062: with three units of its own and two face attacks, a Burn at the enemy is among the
    /// first AI_BUDGET.branching candidates, so every step of the beam tries one.
    #[test]
    fn a_burn_at_the_enemy_is_within_branching_beside_three_own_units_and_two_face_attacks() {
        let state = burn_board();
        let candidates = candidate_actions(&state, AI);
        let first_at_enemy = candidates
            .iter()
            .position(|action| aimed_at_the_enemy(&state, action));
        assert!(
            first_at_enemy.is_some_and(|at| at < AI_BUDGET.branching),
            "{first_at_enemy:?} in {:?}",
            candidates.iter().map(action_key).collect::<Vec<_>>()
        );
    }

    /// #1062: one source's variants at the enemy (the enemy hero's first) alternate with the rest,
    /// enemy first, each group in legal_actions order.
    #[test]
    fn a_sources_variants_alternate_enemy_first_with_the_enemy_hero_first() {
        let state = burn_board();
        let burn = hand_id(&state, "classic-036");
        let legal = plays_of(&legal_actions(&state, AI), &burn);
        // legal_actions: Tempo Timmy, Tempo Timmy, Mr. Vanilla, the AI's hero, the 7/7, the enemy hero.
        assert_eq!(legal.len(), 6, "{legal:?}");
        let expected: Vec<String> = [5, 0, 4, 1, 2, 3].iter().map(|&at| legal[at].clone()).collect();
        assert_eq!(plays_of(&candidate_actions(&state, AI), &burn), expected);
    }

    /// #1062: Golden Grape, which only aims at the AI's own field and hand, and Mr. Vanilla from hand,
    /// which aims at nothing, keep legal_actions order.
    #[test]
    fn a_source_aimed_only_at_its_own_side_and_one_aimed_at_nothing_keep_legal_actions_order() {
        jackioh_cards::register_all();
        let state = scenario(json!({
            "seed": "candidates-enemy-first",
            "p1": {
                "hand": ["classic-036", "classicplus-065-4", "core-008"],
                "mana": 2,
                "field": ["core-011", "core-011", { "def": "core-008", "position": "DEF" }],
            },
            "p2": { "field": ["core-025"] },
        }))
        .state()
        .clone();
        let legal = legal_actions(&state, AI);
        let candidates = candidate_actions(&state, AI);
        for def_id in ["classicplus-065-4", "core-008"] {
            let id = hand_id(&state, def_id);
            let kept = plays_of(&candidates, &id);
            let offered: Vec<String> = plays_of(&legal, &id)
                .into_iter()
                .filter(|key| kept.contains(key))
                .collect();
            assert!(offered.len() > 1, "{def_id}: {offered:?}");
            assert_eq!(kept, offered, "{def_id}");
        }
    }
}
