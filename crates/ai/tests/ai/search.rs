//! Candidate moves and the search's contract (docs/polish/3-ai.md B14, B15).
//!
//! B14: `candidateActions` is `legalActions` minus the four action types the AI never searches
//! (R84's concede, offerDraw and answerDraw, and the mulligan), with plays that differ only in their
//! lane collapsed to the lowest lane of each placement (its midlane lane when it has one, R2203001), at most `AI_SEARCH.zone_variants` of them
//! (#1014; the search's root keeps every placement, #1635), and endTurn last whenever it is legal.
//!
//! B15: `decide` is a pure function of (state, seat, rng seed, budget): the same inputs give a
//! deep-equal Decision, it never spends more nodes than the budget, its action is always legal, and
//! a wall clock that says stop at the first poll still yields a legal action instead of a throw.
//!
//! Port of `packages/ai/test/search.test.ts`. TS's per-test `{ timeout }` has no `cargo test` twin.

use std::cell::Cell;

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, ai_options, dealt_game, is_legal, js, trace};

const SKIPPED: &[ActionType] = &[
    ActionType::Concede,
    ActionType::OfferDraw,
    ActionType::AnswerDraw,
    ActionType::Mulligan,
];

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
    lanes_of(state, def_id, &candidate_actions(state, AI, Prices::NONE))
}

/// The lanes the search's root offers it (`root_candidate_actions`, every placement, #1635).
fn root_offered_lanes(state: &GameState, def_id: &str) -> Vec<i32> {
    lanes_of(state, def_id, &root_candidate_actions(state, AI, Prices::NONE))
}

/// The lanes `actions` play the AI's hand card of `def_id` into, lowest first.
fn lanes_of(state: &GameState, def_id: &str, actions: &[ActionBody]) -> Vec<i32> {
    let id = hand_id(state, def_id);
    let mut lanes: Vec<i32> = actions
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

/// A unit opposite every lane: each of the AI's lanes is its own placement.
fn crowded_row() -> GameState {
    jackioh_cards::register_all();
    scenario(json!({
        "seed": "search-crowded-row",
        "p1": { "hand": ["core-008"] },
        "p2": { "field": ["core-011", "core-011", "core-011", "core-011", "core-011"] },
    }))
    .state()
    .clone()
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
        let candidates = candidate_actions(&state, AI, Prices::NONE);
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

        // Plays: per otherwise-identical play, the lowest lane of each placement (its midlane lane when it has one, R2203001), in legal order, up to
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
        let one_lanes: Vec<Option<i32>> = candidate_actions(&one, AI, Prices::NONE)
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
        let mut two_lanes: Vec<Option<i32>> = candidate_actions(&two, AI, Prices::NONE)
            .iter()
            .filter(|action| is_play(action))
            .map(lane_of)
            .collect();
        two_lanes.sort_by_key(|lane| lane.unwrap_or(0));
        assert_eq!(two_lanes, vec![Some(4), Some(5)]);
    }

    /// B14: a play keeps one lane of each placement (the lowest, or its midlane lane, R2203001): every one at the search's root (#1635), none
    /// past AI_SEARCH.zone_variants deeper (#1014)
    #[test]
    fn b14_a_play_keeps_one_lane_of_each_placement_every_one_at_the_root_none_past_the_cap_deeper() {
        jackioh_cards::register_all();
        let cap = AI_SEARCH.zone_variants as usize;
        // An empty row: lane 1 has the edge on its left, lane 5 on its right, and lanes 2 to 4 are one
        // placement, played in the midlane (R2203001).
        let empty = scenario(json!({
            "seed": "search-empty-row",
            "p1": { "hand": ["core-008"] },
            "p2": { "hand": ["core-005"] },
        }))
        .state()
        .clone();
        let mut expected: Vec<i32> = [1, 3, 5].into_iter().take(cap).collect();
        expected.sort();
        assert_eq!(offered_lanes(&empty, "core-008"), expected);
        assert_eq!(root_offered_lanes(&empty, "core-008"), vec![1, 3, 5]);

        // A unit opposite every lane: each lane is its own placement, so the root keeps all five and a
        // deeper level the lowest ones up to the cap.
        let crowded = crowded_row();
        let deeper = (1..=5).take(cap).collect::<Vec<i32>>();
        assert_eq!(offered_lanes(&crowded, "core-008"), deeper);
        assert_eq!(root_offered_lanes(&crowded, "core-008"), vec![1, 2, 3, 4, 5]);
        let root = root_expansion(
            &crowded,
            AI,
            AI_BUDGET.root_branching,
            MoveOrder::Heuristic,
            &POLICY_PRIOR,
            Prices::NONE,
        );
        assert_eq!(lanes_of(&crowded, "core-008", &root), vec![1, 2, 3, 4, 5]);
        let deeper_expansion = expansion(
            &crowded,
            AI,
            AI_BUDGET.branching,
            MoveOrder::Heuristic,
            &POLICY_PRIOR,
            Prices::NONE,
        );
        assert_eq!(lanes_of(&crowded, "core-008", &deeper_expansion), deeper);
    }

    /// #1635: only the search's root keeps every placement. The lethal solver (lethal.rs
    /// `lethal_moves`, which reads `candidate_actions`), the opponent's reply (reply.rs, which reads
    /// `candidate_actions_in`) and a line's later actions (`candidate_set`) keep the lowest
    /// AI_SEARCH.zone_variants.
    #[test]
    fn the_lethal_solver_and_the_reply_keep_zone_variants_lanes_and_only_the_root_keeps_every_one() {
        let crowded = crowded_row();
        let capped = (1..=5)
            .take(AI_SEARCH.zone_variants as usize)
            .collect::<Vec<i32>>();
        assert_eq!(
            lanes_of(
                &crowded,
                "core-008",
                &candidate_actions(&crowded, AI, Prices::NONE)
            ),
            capped
        );
        assert_eq!(
            lanes_of(
                &crowded,
                "core-008",
                &candidate_actions_in(&crowded, AI, legal_actions(&crowded, AI), Prices::NONE)
            ),
            capped
        );
        assert_eq!(
            lanes_of(&crowded, "core-008", &candidate_set(&crowded, AI)),
            capped
        );
        assert_eq!(
            lanes_of(
                &crowded,
                "core-008",
                &root_candidate_actions(&crowded, AI, Prices::NONE)
            ),
            vec![1, 2, 3, 4, 5]
        );
        assert_eq!(
            lanes_of(&crowded, "core-008", &root_candidate_set(&crowded, AI)),
            vec![1, 2, 3, 4, 5]
        );
    }

    /// B14: an attack on the enemy hero leads, plays come before position switches, and endTurn trails
    #[test]
    fn b14_an_attack_on_the_enemy_hero_leads_plays_come_before_position_switches_and_end_turn_trails() {
        let state = mid_game();
        let candidates = candidate_actions(&state, AI, Prices::NONE);
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

    /// R1759000: three units, each with a hero attack, a trade and a stand-down, and three cards to play
    /// push every position switch past a level's `branching` moves. The cut keeps the first
    /// `AI_SEARCH.position_switches` of the stand-downs beside endTurn, and keeps none twice where the
    /// width holds them all.
    #[test]
    fn r1759000_position_switches_survive_truncation_up_to_named_constant() {
        jackioh_cards::register_all();
        let state = scenario(json!({
            "seed": "search-switches-survive",
            "p1": { "field": ["core-008", "core-008", "core-008"], "hand": ["core-003", "core-004", "core-011"], "mana": 5 },
            "p2": { "field": ["core-030"] },
        }))
        .state()
        .clone();
        let candidates = candidate_actions(&state, AI, Prices::NONE);
        let width = AI_BUDGET.branching;
        let stand_downs: Vec<ActionBody> = candidates
            .iter()
            .filter(|action| stands_down(&state, action))
            .cloned()
            .collect();
        assert_eq!(stand_downs.len(), 3);
        assert!(
            candidates
                .iter()
                .position(|action| stands_down(&state, action))
                .is_some_and(|first| first > width)
        );
        let kept = AI_SEARCH.position_switches as usize;
        assert!(0 < kept && kept < stand_downs.len());

        for order in [MoveOrder::Heuristic, MoveOrder::Learned] {
            let expanded = expansion(&state, AI, width, order, &POLICY_PRIOR, Prices::NONE);
            assert_eq!(expanded.len(), width + kept + 1, "{expanded:?}");
            assert_eq!(expanded[..width], candidates[..width]);
            assert_eq!(expanded[width..width + kept], stand_downs[..kept]);
            assert_eq!(expanded.last(), Some(&ActionBody::EndTurn));
        }

        // Wide enough to hold every candidate, nothing is cut and nothing is added twice.
        assert_eq!(
            expansion(
                &state,
                AI,
                candidates.len(),
                MoveOrder::Heuristic,
                &POLICY_PRIOR,
                Prices::NONE
            ),
            candidates
        );
    }

    /// R1759000: the stand-downs kept are the first the cut left out, not the first overall. With one
    /// stand-down inside the width, the next `AI_SEARCH.position_switches` after it are added.
    #[test]
    fn r1759000_the_kept_stand_downs_are_the_first_the_cut_left_out() {
        jackioh_cards::register_all();
        let state = scenario(json!({
            "seed": "search-switches-left-out",
            "p1": { "field": ["core-008", "core-008", "core-008"], "hand": ["core-003", "core-004", "core-011"], "mana": 5 },
            "p2": { "field": ["core-030"] },
        }))
        .state()
        .clone();
        let candidates = candidate_actions(&state, AI, Prices::NONE);
        let stand_downs: Vec<ActionBody> = candidates
            .iter()
            .filter(|action| stands_down(&state, action))
            .cloned()
            .collect();
        let first = candidates
            .iter()
            .position(|action| stands_down(&state, action))
            .expect("a stand-down");
        let width = first + 1;
        let kept = AI_SEARCH.position_switches as usize;
        assert!(stand_downs.len() > kept);
        let expanded = expansion(
            &state,
            AI,
            width,
            MoveOrder::Heuristic,
            &POLICY_PRIOR,
            Prices::NONE,
        );
        assert_eq!(expanded[..width], candidates[..width]);
        assert_eq!(expanded[width..width + kept], stand_downs[1..1 + kept]);
        assert_eq!(expanded.len(), width + kept + 1, "{expanded:?}");
    }

    /// R1759000: a unit no enemy unit would beat is not stood down: the same board against an equal
    /// 4/4 has three switches to Defense and none of them is a stand-down, so the cut drops them all.
    #[test]
    fn r1759000_a_unit_no_enemy_would_beat_is_not_stood_down() {
        jackioh_cards::register_all();
        let state = scenario(json!({
            "seed": "search-switch-unthreatened",
            "p1": { "field": ["core-008", "core-008", "core-008"], "hand": ["core-003", "core-004", "core-011"], "mana": 5 },
            "p2": { "field": ["core-008"] },
        }))
        .state()
        .clone();
        let candidates = candidate_actions(&state, AI, Prices::NONE);
        let width = AI_BUDGET.branching;
        assert!(
            candidates
                .iter()
                .any(|action| matches!(action, ActionBody::SwitchPosition { .. }))
        );
        assert!(!candidates.iter().any(|action| stands_down(&state, action)));
        let expanded = expansion(
            &state,
            AI,
            width,
            MoveOrder::Heuristic,
            &POLICY_PRIOR,
            Prices::NONE,
        );
        assert_eq!(expanded.len(), width + 1, "{expanded:?}");
    }

    /// R1759000: only a stand-down is kept past the cut. A unit already in Defense Position keeps its
    /// switch back to Attack where the cut leaves it, as before.
    #[test]
    fn r1759000_a_switch_back_to_attack_is_cut_with_the_rest() {
        jackioh_cards::register_all();
        let defending = json!({ "def": "core-008", "position": "DEF" });
        let state = scenario(json!({
            "seed": "search-switch-back-cut",
            "p1": { "field": [defending, defending, defending], "hand": ["core-003", "core-004", "core-011", "core-015"], "mana": 5 },
            "p2": { "field": ["core-030"] },
        }))
        .state()
        .clone();
        let candidates = candidate_actions(&state, AI, Prices::NONE);
        let width = AI_BUDGET.branching;
        let switches = |actions: &[ActionBody]| {
            actions
                .iter()
                .filter(|action| matches!(action, ActionBody::SwitchPosition { .. }))
                .count()
        };
        assert_eq!(switches(&candidates), 3);
        assert!(!candidates.iter().any(|action| stands_down(&state, action)));
        assert!(
            candidates
                .iter()
                .position(|action| matches!(action, ActionBody::SwitchPosition { .. }))
                .is_some_and(|first| first > width)
        );
        let expanded = expansion(
            &state,
            AI,
            width,
            MoveOrder::Heuristic,
            &POLICY_PRIOR,
            Prices::NONE,
        );
        assert_eq!(switches(&expanded), 0, "{expanded:?}");
        assert_eq!(expanded.len(), width + 1);
    }

    /// R2021000: whether switching p1's Mr. Vanilla (core-008, a 4/4 in Attack Position) to Defense is a
    /// stand-down facing p2's `archivist`, an Archivist (core-030, a 4/5 that kills the Vanilla and
    /// survives), once `change` has changed the board (given the Vanilla's id and the Archivist's).
    fn stands_down_facing(archivist: Value, change: impl FnOnce(&mut GameState, &str, &str)) -> bool {
        jackioh_cards::register_all();
        let mut state = scenario(json!({
            "seed": "search-stand-down-ready",
            "p1": { "field": ["core-008"] },
            "p2": { "field": [archivist] },
        }))
        .state()
        .clone();
        let vanilla = active_units_of(&state, AI)[0].id.clone();
        let enemy = active_units_of(&state, HUMAN)[0].id.clone();
        change(&mut state, &vanilla, &enemy);
        stands_down(&state, &ActionBody::SwitchPosition { instance_id: vanilla })
    }

    /// The state's copy of the card `id`, to change in place.
    fn live<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
        find_instance_mut(state, id).expect("the unit is on the field")
    }

    /// R2021000: an Archivist in Attack Position, ready next turn, beats the Vanilla: a stand-down, as
    /// under R1759000.
    #[test]
    fn r2021000_a_ready_enemy_that_beats_the_unit_stands_it_down() {
        assert!(stands_down_facing(json!("core-030"), |_, _, _| {}));
    }

    /// R2021000: in Defense Position without Deft it spends its next turn's exertion switching back.
    #[test]
    fn r2021000_an_enemy_in_defense_position_does_not_stand_it_down() {
        let defending = json!({ "def": "core-030", "position": "DEF" });
        assert!(!stands_down_facing(defending, |_, _, _| {}));
    }

    /// R2021000, R49: with Deft it switches to Attack and still attacks in the same turn.
    #[test]
    fn r2021000_an_enemy_with_deft_in_defense_position_stands_it_down() {
        let defending = json!({ "def": "core-030", "position": "DEF" });
        assert!(stands_down_facing(defending, |state, _, enemy| {
            live(state, enemy).granted_keywords.push(Keyword::Deft);
        }));
    }

    /// R2021000, R1174000: a Frozen unit of the player not on turn is Frozen through its next turn.
    #[test]
    fn r2021000_an_enemy_frozen_through_its_next_turn_does_not_stand_it_down() {
        assert!(!stands_down_facing(json!("core-030"), |state, _, enemy| {
            let turn = state.turn;
            live(state, enemy).frozen = Some(turn);
        }));
    }

    /// R2021000: an enemy granted "Can't attack" never attacks.
    #[test]
    fn r2021000_an_enemy_that_cannot_attack_does_not_stand_it_down() {
        assert!(!stands_down_facing(json!("core-030"), |state, _, enemy| {
            live(state, enemy).granted_keywords.push(Keyword::CantAttack);
        }));
    }

    /// R2021000, R1839000: a Dormant enemy neither attacks nor is attacked.
    #[test]
    fn r2021000_a_dormant_enemy_does_not_stand_it_down() {
        assert!(!stands_down_facing(json!("core-030"), |state, _, enemy| {
            live(state, enemy).slumber = Some(Slumber { turns: Some(2) });
        }));
    }

    /// R2021000, R783002: a Flying unit in Attack Position is attacked only by a unit with Flying or
    /// Ranged, so a ground Archivist never reaches it and one with either keyword does.
    #[test]
    fn r2021000_a_flying_unit_is_stood_down_only_against_a_flying_or_ranged_enemy() {
        let fly =
            |state: &mut GameState, unit: &str| live(state, unit).granted_keywords.push(Keyword::Flying);
        assert!(!stands_down_facing(json!("core-030"), |state, unit, _| fly(
            state, unit
        )));
        for reach in [Keyword::Flying, Keyword::Ranged] {
            assert!(
                stands_down_facing(json!("core-030"), |state, unit, enemy| {
                    fly(state, unit);
                    live(state, enemy).granted_keywords.push(reach.clone());
                }),
                "{reach:?}"
            );
        }
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

        let candidates = candidate_actions(&state, AI, Prices::NONE);
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
        assert_eq!(
            candidate_actions(&state, AI, Prices::NONE),
            Vec::<ActionBody>::new()
        );
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
        assert_eq!(
            candidate_actions(&offered, AI, Prices::NONE),
            Vec::<ActionBody>::new()
        );
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
        assert_eq!(
            candidate_actions(s.state(), AI, Prices::NONE),
            Vec::<ActionBody>::new()
        );
        assert_eq!(
            candidate_actions(s.state(), PlayerId::P2, Prices::NONE),
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
        assert_eq!(
            candidate_actions(&state, AI, Prices::NONE),
            Vec::<ActionBody>::new()
        );
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
        let candidates = candidate_actions(state, AI, Prices::NONE);

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
        let candidates = candidate_actions(state, AI, Prices::NONE);

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

    /// The enemy's 9/9 Taunt opposite lane 4 of an otherwise empty board.
    fn lane_eater_board() -> GameState {
        jackioh_cards::register_all();
        scenario(json!({
            "seed": "search-lane-eater-4",
            "p1": { "hand": ["classic-071"], "mana": 3, "library": LIBRARY },
            "p2": { "field": [{ "def": "core-019", "lane": 4 }], "library": LIBRARY },
        }))
        .state()
        .clone()
    }

    /// #1635: C #71 Lane Eater with the enemy's 9/9 Taunt opposite lane 4 and nothing else on the
    /// board. Lanes 1, 3, 4 and 5 are its placements (2 and 3 are one, played in the midlane, R2203001),
    /// so the capped candidates stop short of lane 4; the search's root offers it, and `decide` plays it there.
    #[test]
    fn lane_eater_is_tried_opposite_a_taunt_past_the_first_two_placements_and_decide_plays_it_there() {
        let state = lane_eater_board();
        let placements = vec![1, 3, 4, 5];
        let cap = AI_SEARCH.zone_variants as usize;
        assert_eq!(
            offered_lanes(&state, "classic-071"),
            placements[..cap.min(placements.len())]
        );
        assert_eq!(root_offered_lanes(&state, "classic-071"), placements);

        let eater = hand_id(&state, "classic-071");
        let decision =
            decide(&state, AI, &mut ai_options("search-lane-eater-4", AI_BUDGET)).expect("a decision");
        assert!(
            matches!(
                &decision.action,
                ActionBody::Play { instance_id, zone: Some(zone), .. }
                    if *instance_id == eater && zone.lane == 4
            ),
            "{}",
            action_key(&decision.action)
        );
    }

    /// #1635: a finalist that opens with a play only the root offers is replayed on another world, not
    /// cut at its first action: `score_line` checks a line's first action against the root's set.
    #[test]
    fn score_line_replays_a_first_action_only_the_root_offers() {
        let state = lane_eater_board();
        let play = root_candidate_actions(&state, AI, Prices::NONE)
            .into_iter()
            .find(|action| lane_of(action) == Some(4))
            .expect("the root offers lane 4");
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        let played = score_line(&state, AI, &[play], &counter, false, None, Prices::NONE).expect("in budget");
        let passed = score_line(&state, AI, &[], &counter, false, None, Prices::NONE).expect("in budget");
        assert!(played > passed, "{played} > {passed}");
    }

    /// #1635: C+ #20 Mushroom Power with its own units in lanes 3 and 5: lane 4, which buffs both, is
    /// past the capped candidates' first two placements, and the search's root offers it.
    #[test]
    fn mushroom_power_is_tried_between_its_units_in_lanes_3_and_5_at_the_root() {
        jackioh_cards::register_all();
        let state = scenario(json!({
            "seed": "search-mushroom-4",
            "p1": {
                "hand": ["classicplus-020"],
                "field": [{ "def": "core-008", "lane": 3 }, { "def": "core-008", "lane": 5 }],
                "library": LIBRARY,
            },
            "p2": { "library": LIBRARY },
        }))
        .state()
        .clone();
        assert!(!offered_lanes(&state, "classicplus-020").contains(&4));
        assert_eq!(root_offered_lanes(&state, "classicplus-020"), vec![1, 2, 4]);
    }

    /// Mr. Vanilla, 4/4 with no text.
    const VANILLA: &str = "core-008";

    /// One of the AI's Mr. Vanillas in lane 1, another in its hand, and the enemy's Radiant Nurse
    /// Cleaver (classic-073, 6/12 Cleave) opposite lane 1. The vanilla's placements are lanes 2, 3 and 5
    /// (3 and 4 are one); only lane 2 is beside the unit, where the Cleaver's reply would kill both.
    fn cleaver_board() -> GameState {
        jackioh_cards::register_all();
        scenario(json!({
            "seed": "search-cleaver-lanes",
            "p1": { "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 1 }], "library": LIBRARY },
            "p2": { "field": [{ "def": "classic-073", "radiant": true, "lane": 1 }], "library": LIBRARY },
        }))
        .state()
        .clone()
    }

    /// #1797: two orders of the same two plays into the same lanes are one position, for the lethal
    /// solver (`search_signature`) and the beam (`unit_lanes` beside it); the second play into another
    /// lane is the lethal solver's same position, and the beam's another.
    #[test]
    fn two_orders_of_two_plays_into_the_same_lanes_are_one_position_and_another_lane_is_another() {
        jackioh_cards::register_all();
        let state = scenario(json!({
            "seed": "search-two-orders",
            "p1": { "hand": [VANILLA, VANILLA], "library": LIBRARY },
            "p2": { "library": LIBRARY },
        }))
        .state()
        .clone();
        let first = state.players[AI].hand[0].id.clone();
        let second = state.players[AI].hand[1].id.clone();
        let play = |id: &str, lane: i32| {
            body(json!({ "type": "play", "instanceId": id, "zone": { "row": "units", "lane": lane } }))
        };
        let (a1, b2, b3) = (play(&first, 1), play(&second, 2), play(&second, 3));

        let one = act(&act(&state, AI, &a1), AI, &b2);
        let other = act(&act(&state, AI, &b2), AI, &a1);
        let apart = act(&act(&state, AI, &a1), AI, &b3);
        assert_eq!(search_signature(&one, AI), search_signature(&other, AI));
        assert_eq!(unit_lanes(&one), unit_lanes(&other));
        assert_eq!(search_signature(&one, AI), search_signature(&apart, AI));
        assert_ne!(unit_lanes(&one), unit_lanes(&apart));
    }

    /// #1797: beside an enemy Cleaver, the beam keeps a line that plays the vanilla away from its
    /// neighbour (lane 3), which the duplicate check once merged into lane 2's, and opens with at most
    /// `AI_SEARCH.zone_variants` of the vanilla's placements.
    #[test]
    fn the_beam_plays_a_vanilla_away_from_its_neighbour_and_opens_with_at_most_zone_variants_of_its_lanes() {
        let state = cleaver_board();
        let placements = vec![2, 3, 5];
        assert_eq!(root_offered_lanes(&state, VANILLA), placements);

        let lines = beam_search(
            &state,
            AI,
            &create_node_counter(AI_BUDGET.nodes, None),
            AI_BUDGET,
            Prices::NONE,
        );
        let keys = lines
            .iter()
            .map(|line| line.actions.iter().map(action_key).collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(
            lines.iter().any(|line| lanes_of(&state, VANILLA, &line.actions)
                .iter()
                .any(|lane| *lane >= 3)),
            "{keys}"
        );
        let opening: Vec<i32> = lines
            .iter()
            .flat_map(|line| lanes_of(&state, VANILLA, line.actions.get(..1).unwrap_or_default()))
            .collect();
        for lane in &placements[AI_SEARCH.zone_variants as usize..] {
            assert!(!opening.contains(lane), "lane {lane}: {keys}");
        }
    }

    /// #1797: a lane variant is closed only beside a line the beam keeps, and a tie goes to that line.
    /// At 3 mana with Midrange Menace (core-019, 3) and a vanilla (1) in hand and a beam one line wide,
    /// the Menace in lane 1 is kept and the vanilla in lane 1 dropped: the Menace's lane 3 (lanes 2 to 4
    /// are one placement, played in the midlane, R2203001) is a line,
    /// below lane 1's, and no line plays the vanilla at all. The same one depth deep, where lane 1's
    /// line is still open when the loop ends and is closed after it.
    #[test]
    fn a_lane_variant_is_closed_only_beside_a_line_the_beam_keeps_and_ranks_below_it() {
        jackioh_cards::register_all();
        let state = scenario(json!({
            "seed": "search-lane-variants-kept",
            "p1": { "hand": ["core-019", VANILLA], "mana": 3, "library": LIBRARY },
            "p2": { "library": LIBRARY },
        }))
        .state()
        .clone();
        let narrow = SearchBudget {
            beam_width: 1,
            ..AI_BUDGET
        };
        for budget in [
            narrow,
            SearchBudget {
                max_depth: 1,
                ..narrow
            },
        ] {
            let lines = beam_search(
                &state,
                AI,
                &create_node_counter(budget.nodes, None),
                budget,
                Prices::NONE,
            );
            let keys = lines
                .iter()
                .map(|line| line.actions.iter().map(action_key).collect::<Vec<_>>().join(" "))
                .collect::<Vec<_>>()
                .join(" | ");
            let menace_openings: Vec<i32> = lines
                .iter()
                .flat_map(|line| lanes_of(&state, "core-019", line.actions.get(..1).unwrap_or_default()))
                .collect();
            assert_eq!(menace_openings.first(), Some(&1), "{keys}");
            assert!(menace_openings.contains(&3), "{keys}");
            assert!(
                lines
                    .iter()
                    .all(|line| lanes_of(&state, VANILLA, &line.actions).is_empty()),
                "{keys}"
            );
        }
    }

    /// #1797: when the enemy Cleaver's reply would splash the vanilla's neighbour, the AI's turn plays
    /// the vanilla away from it.
    #[test]
    fn decide_plays_a_vanilla_where_the_enemy_cleaver_cannot_splash_it_from_its_neighbour() {
        let state = cleaver_board();
        let vanilla = hand_id(&state, VANILLA);
        let neighbour = active_units_of(&state, AI)[0].id.clone();

        let turn = play_ai_turn(&state, AI, &mut ai_options("search-cleaver-lanes", AI_BUDGET));
        let lane = |id: &str| {
            find_instance(&turn.state, id)
                .and_then(|card| slot_of(&turn.state, card))
                .map(|slot| slot.lane)
        };
        assert_eq!(lane(&neighbour), Some(1), "{}", trace(&turn));
        assert!(lane(&vanilla).is_some_and(|lane| lane >= 3), "{}", trace(&turn));
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
        let candidates = candidate_actions(&state, AI, Prices::NONE);
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
        assert_eq!(
            plays_of(&candidate_actions(&state, AI, Prices::NONE), &burn),
            expected
        );
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
        let candidates = candidate_actions(&state, AI, Prices::NONE);
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

// ---------------------------------------------------------------------------------------------
// #1112: a child waiting on the seat's own prompt is ranked where its answers lead
// ---------------------------------------------------------------------------------------------

mod own_prompt_lookahead {
    use super::*;

    /// Craft a Card (#99): 4 mana, Discover a Unit, then another, the fused result to hand at 0.
    const CRAFT: &str = "core-099";

    /// `evaluate` as the beam scores a child.
    fn eval(state: &GameState) -> f64 {
        evaluate(state, AI, NextSwing::Enemy, &AI_EVAL)
    }

    /// Craft a Card beside four 1-cost Spells that do nothing on this board (Book of Plague, Ancient
    /// Acquisition, Divine Favor, Rewind: no permanent, an empty graveyard, a hand bigger than the
    /// opponent's), at the default 4 crystals. The cast and its 0-cost fused Unit is the turn's best
    /// use, but at its first Discover the cast reads as a spent 4-cost card, worse than any Spell.
    fn craft_turn() -> GameState {
        jackioh_cards::register_all();
        scenario(json!({
            "seed": "search-own-prompt",
            "p1": {
                "hand": [CRAFT, "classic-070", "classic-034", "classic-046", "classic-054"],
                "library": ["core-053", "core-011", "core-008"],
            },
            "p2": { "hand": ["core-005"], "library": ["core-053", "core-011"] },
        }))
        .state()
        .clone()
    }

    fn craft_play(state: &GameState) -> ActionBody {
        let craft = hand_id(state, CRAFT);
        candidate_actions(state, AI, Prices::NONE)
            .into_iter()
            .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == craft))
            .unwrap_or_else(|| panic!("Craft a Card is not a candidate"))
    }

    /// Right after the cast: the first Discover is open, and it is the AI's.
    fn at_first_discover() -> GameState {
        let start = craft_turn();
        let state = act(&start, AI, craft_play(&start));
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(AI));
        state
    }

    /// #1112: beside AI_BUDGET.beam_width other plays, the best line casts Craft a Card, answers both
    /// Discovers and plays the fused Unit: a line the beam cut at its first Discover before.
    #[test]
    fn craft_a_cards_best_line_casts_it_answers_both_discovers_and_plays_the_fused_unit() {
        let state = craft_turn();
        let craft = hand_id(&state, CRAFT);
        let others: IndexSet<String> = candidate_actions(&state, AI, Prices::NONE)
            .iter()
            .filter_map(|action| match action {
                ActionBody::Play { instance_id, .. } if *instance_id != craft => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        assert!(others.len() >= AI_BUDGET.beam_width, "{others:?}");

        let counter = create_node_counter(AI_BUDGET.nodes, None);
        let lines = beam_search(&state, AI, &counter, AI_BUDGET, Prices::NONE);
        let best = lines.first().unwrap_or_else(|| panic!("the beam found no line"));
        let keys: Vec<String> = best.actions.iter().map(action_key).collect();
        assert!(best.actions.len() >= 4, "{keys:?}");
        assert!(
            matches!(&best.actions[0], ActionBody::Play { instance_id, .. } if *instance_id == craft),
            "{keys:?}"
        );
        assert!(matches!(best.actions[1], ActionBody::Answer { .. }), "{keys:?}");
        assert!(matches!(best.actions[2], ActionBody::Answer { .. }), "{keys:?}");
        let ActionBody::Play {
            instance_id: fused, ..
        } = &best.actions[3]
        else {
            panic!("the fused Unit is not played: {keys:?}");
        };
        assert!(
            state.players[AI].hand.iter().all(|card| card.id != *fused),
            "{keys:?}"
        );
        assert!(
            active_units_of(&best.end, AI)
                .iter()
                .any(|unit| unit.id == *fused && best.end.transient_defs.contains_key(&unit.def_id)),
            "{keys:?}"
        );
    }

    /// #1112: at its first Discover, the lookahead scores the cast at least as well as the best state
    /// its first answers reach, above the prompt state itself, one node per answer tried.
    #[test]
    fn a_child_at_its_own_prompt_scores_at_least_the_best_state_its_first_answers_reach() {
        let state = at_first_discover();
        let answers: Vec<ActionBody> = legal_actions(&state, AI)
            .into_iter()
            .filter(|answer| *answer != ActionBody::Concede)
            .take(AI_SEARCH.max_auto_answer_options as usize)
            .collect();
        assert!(answers.len() > 1, "{answers:?}");
        let best_first = answers
            .iter()
            .map(|answer| eval(&act(&state, AI, answer)))
            .fold(f64::NEG_INFINITY, f64::max);

        let counter = create_node_counter(AI_BUDGET.nodes, None);
        let score = own_prompt_score(&state, AI, &counter, Prices::NONE)
            .unwrap_or_else(|| panic!("no lookahead score"));
        assert!(score >= best_first, "{score} < {best_first}");
        assert!(score > eval(&state), "{score} <= {}", eval(&state));
        // Two Discovers of three Units each, every answer one node.
        assert_eq!(counter.used(), 2 * answers.len());
    }

    /// #1112: with no prompt of the seat's own open there is no lookahead, so the child keeps exactly
    /// its `evaluate`, and no node is spent.
    #[test]
    fn a_child_with_no_prompt_of_the_seats_own_gets_no_lookahead_and_spends_no_node() {
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        assert_eq!(own_prompt_score(&craft_turn(), AI, &counter, Prices::NONE), None);
        assert_eq!(
            own_prompt_score(&at_first_discover(), AI.opponent(), &counter, Prices::NONE),
            None
        );
        assert_eq!(counter.used(), 0);
    }

    /// #1112: a lookahead the counter refuses gives no score, so the child keeps its plain one.
    #[test]
    fn a_lookahead_the_counter_refuses_gives_no_score() {
        let counter = create_node_counter(1, None);
        assert_eq!(
            own_prompt_score(&at_first_discover(), AI, &counter, Prices::NONE),
            None
        );
        assert_eq!(counter.used(), 1);
    }
}

// ---------------------------------------------------------------------------------------------
// #1143: a hand Unit is priced by the face it will enter with
// ---------------------------------------------------------------------------------------------

mod make_radiant_targets_1143 {
    use super::*;

    /// Glowy Jelly Bean (#26): 3 mana, make a card in your hand Radiant, the card declared at play (R81).
    const BEAN: &str = "core-026";
    /// Brother Lar: (1) 1/1, Radiant 2/2.
    const LAR: &str = "classicplus-076";
    /// (4) 7/7 Armor 7, Radiant 14/14 Armor 7, Reborn.
    const BIG: &str = "core-025";

    /// #1143: at 3 crystals the 4-cost 7/7 cannot be played, so the turn is the Bean or Brother Lar.
    /// Before the face was read every Bean target scored the flat `radiant_in_hand`, below the Bean's
    /// own hand worth, and Brother Lar was played instead; now the Bean makes the 7/7 a 14/14 Reborn.
    /// Brother Lar comes first in hand, so a tie between the Bean's targets would pick him.
    #[test]
    fn glowy_jelly_bean_is_cast_on_the_hand_unit_whose_radiant_face_adds_most() {
        jackioh_cards::register_all();
        let library = ["core-008", "core-011", "core-008", "core-011"];
        let state = scenario(json!({
            "seed": "search-bean-uplift",
            "p1": { "hand": [BEAN, LAR, BIG], "mana": 3, "library": library },
            "p2": { "library": library },
        }))
        .state()
        .clone();
        let bean = hand_id(&state, BEAN);
        let big = hand_id(&state, BIG);

        let decision =
            decide(&state, AI, &mut ai_options("search-bean-uplift", AI_BUDGET)).expect("a decision");
        assert!(
            matches!(
                &decision.action,
                ActionBody::Play { instance_id, targets: Some(targets), .. }
                    if *instance_id == bean
                        && *targets == vec![Selection::Instance { instance_id: big.clone() }]
            ),
            "{}",
            action_key(&decision.action)
        );
    }
}

// ---------------------------------------------------------------------------------------------
// #1636: a finalist re-scored on another world answers that world's own Discover
// ---------------------------------------------------------------------------------------------

mod rescored_own_prompts_1636 {
    use super::*;

    /// Jewelosco Scarab (#7): (1) 1/1, "Cry: Discover a 2-cost card".
    const SCARAB: &str = "core-007";
    /// (4) 7/7 Armor 7: more than the turn's one crystal pays for.
    const BIG: &str = "core-025";

    /// The Scarab's turn at `mana` crystals on the world of `seed`: the same cards on every world,
    /// but each world's seed draws its own Discover offer.
    fn scarab_turn(seed: &str, mana: i32) -> GameState {
        jackioh_cards::register_all();
        let library = ["core-008", "core-011", "core-008", "core-011"];
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [SCARAB, BIG], "mana": mana, "library": library },
            "p2": { "hand": ["core-005"], "library": library },
        }))
        .state()
        .clone()
    }

    /// The candidate that plays the AI's hand card of `def_id`.
    fn play_of(state: &GameState, def_id: &str) -> ActionBody {
        let id = hand_id(state, def_id);
        candidate_actions(state, AI, Prices::NONE)
            .into_iter()
            .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == id))
            .unwrap_or_else(|| panic!("{def_id} is not a candidate"))
    }

    /// The answers of the AI's open prompt, Concede aside.
    fn answers(state: &GameState) -> Vec<ActionBody> {
        legal_actions(state, AI)
            .into_iter()
            .filter(|action| *action != ActionBody::Concede)
            .collect()
    }

    /// `line` scored on `world` after the opponent's reply, as `decide` re-scores a finalist.
    fn rescore(world: &GameState, line: &[ActionBody], counter: &dyn NodeCounter) -> Option<f64> {
        score_line(world, AI, line, counter, true, None, Prices::NONE)
    }

    /// #1636: the line [Scarab, its answer] found on one world, scored on a world whose offer lacks
    /// that answer, answers that world's own Discover, ends its turn and is scored after the
    /// opponent's reply, as a line that answers that world's offer is. Before, it stopped at the
    /// prompt with the prompt's static score while every other finalist was replied to.
    #[test]
    fn a_discover_answer_another_world_does_not_offer_is_answered_there_and_the_line_replied_to() {
        let found_on = scarab_turn("search-rescore-a", 1);
        let play = play_of(&found_on, SCARAB);
        let answer = answers(&act(&found_on, AI, &play))
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("the Discover is not open"));
        let line = [play.clone(), answer.clone()];

        let world = scarab_turn("search-rescore-b", 1);
        let Some(Ok(at_prompt)) = simulate(
            &world,
            AI,
            &play,
            &create_node_counter(AI_BUDGET.nodes, None),
            Prices::NONE,
        ) else {
            panic!("the Scarab was refused");
        };
        assert_eq!(
            at_prompt.pending.as_ref().map(|pending| pending.player_id),
            Some(AI)
        );
        let offered = answers(&at_prompt);
        assert!(
            !offered.contains(&answer),
            "{answer:?} is offered on this world too"
        );

        let counter = create_node_counter(AI_BUDGET.nodes, None);
        let score = rescore(&world, &line, &counter).unwrap_or_else(|| panic!("the counter ran out"));
        assert_ne!(
            score,
            finalist_score(&at_prompt, AI, NextSwing::Enemy, Prices::NONE)
        );
        // The Scarab, each answer tried, the closing endTurn, and the reply's own steps past them.
        assert!(counter.used() > 2 + offered.len(), "{} nodes", counter.used());
        let answered: Vec<f64> = offered
            .iter()
            .filter_map(|own| {
                rescore(
                    &world,
                    &[play.clone(), own.clone()],
                    &create_node_counter(AI_BUDGET.nodes, None),
                )
            })
            .collect();
        assert!(answered.contains(&score), "{score} is none of {answered:?}");
    }

    /// #1636: a line cut at a refused play, no prompt open, is scored as before: where it stopped,
    /// its turn ended and the opponent's reply played, the rest of the line dropped.
    #[test]
    fn a_line_cut_at_a_refused_play_with_no_prompt_open_scores_as_before() {
        let world = scarab_turn("search-rescore-b", 1);
        let big = play_of(&scarab_turn("search-rescore-b", 4), BIG);
        assert!(
            !is_legal(&world, AI, &big),
            "the 4-cost play is legal at one crystal"
        );
        assert_eq!(world.pending, None);

        let cut_counter = create_node_counter(AI_BUDGET.nodes, None);
        let cut = rescore(&world, &[big, play_of(&world, SCARAB)], &cut_counter);
        let stopped_counter = create_node_counter(AI_BUDGET.nodes, None);
        let stopped = rescore(&world, &[], &stopped_counter);
        assert!(cut.is_some());
        assert_eq!(cut, stopped);
        assert_eq!(cut_counter.used(), stopped_counter.used());
    }
}

mod ways_of_being_played {
    use super::*;

    /// Acceptance criterion 1: On a hand wide enough to fill the root (10 cards),
    /// root_expansion at AI_BUDGET.root_branching holds Buff Billy (classicplus-069)
    /// at x == mana, an Embiggened play, and a Cloaked card's face-down set (#1778).
    #[test]
    fn buff_billy_embiggen_and_cloaked_set_survive_root_expansion() {
        jackioh_cards::register_all();
        let state = scenario(json!({
            "seed": "ways-root-wide",
            "p1": {
                "hand": [
                    "classicplus-069", // Buff Billy (X cost)
                    "core-084",        // Going Long (Embiggen 4 vs base 2)
                    "classic-006",     // Cloaked Toe Cracker (Cloaked unit, cost 2)
                    "core-008",
                    "core-011",
                    "core-017",
                    "core-005",
                    "core-020",
                    "core-025",
                    "core-035"
                ],
                "mana": 6,
                "library": ["core-008", "core-011"],
            },
            "p2": {
                "hand": ["core-005"],
                "library": ["core-008", "core-011"],
            }
        }))
        .state()
        .clone();

        let expanded = root_expansion(
            &state,
            AI,
            AI_BUDGET.root_branching,
            MoveOrder::Heuristic,
            &POLICY_PRIOR,
            Prices::NONE,
        );

        let billy_id = hand_id(&state, "classicplus-069");
        let going_long_id = hand_id(&state, "core-084");
        let toe_cracker_id = hand_id(&state, "classic-006");

        // Holds Buff Billy at x == mana (x == 6)
        let has_billy_max_x = expanded.iter().any(|action| {
            matches!(action, ActionBody::Play { instance_id, x: Some(6), .. } if *instance_id == billy_id)
        });
        assert!(
            has_billy_max_x,
            "root_expansion must hold Buff Billy at x == mana (6)"
        );

        // Holds an Embiggened play of Going Long
        let has_embiggened = expanded.iter().any(|action| {
            matches!(action, ActionBody::Play { instance_id, embiggen: Some(true), .. } if *instance_id == going_long_id)
        });
        assert!(
            has_embiggened,
            "root_expansion must hold an Embiggened play of Going Long"
        );

        // Holds Cloaked Toe Cracker's face-down set
        let has_face_down = expanded.iter().any(|action| {
            matches!(action, ActionBody::Play { instance_id, face_down: Some(_), .. } if *instance_id == toe_cracker_id)
        });
        assert!(
            has_face_down,
            "root_expansion must hold a Cloaked card's face-down set"
        );
    }

    /// Acceptance criterion 2: decide with 6 mana, Buff Billy and three 2-drops plays Billy at X=6 (#1778).
    /// Billy at X=4 beside a 2-drop is among the root's moves too, so the search weighs the two.
    #[test]
    fn decide_with_six_mana_and_buff_billy_plays_billy_at_x_six() {
        jackioh_cards::register_all();
        let state = scenario(json!({
            "seed": "ways-billy-decide",
            "p1": {
                "hand": [
                    "classicplus-069", // Buff Billy
                    "core-001",        // Big D-fender, a 2-cost Unit
                    "core-020",        // Pointmaster, a 2-cost Unit
                    "core-030"         // Archivist, a 2-cost Unit
                ],
                "mana": 6,
                "library": ["core-008", "core-011"],
            },
            "p2": {
                "hand": ["core-005"],
                "library": ["core-008", "core-011"],
            }
        }))
        .state()
        .clone();

        let billy_id = hand_id(&state, "classicplus-069");
        let root = root_expansion(
            &state,
            AI,
            AI_BUDGET.root_branching,
            MoveOrder::Heuristic,
            &POLICY_PRIOR,
            Prices::NONE,
        );
        assert!(
            root.iter().any(|action| {
                matches!(action, ActionBody::Play { instance_id, x: Some(4), .. } if *instance_id == billy_id)
            }),
            "the root must also try Billy at X=4, which leaves 2 mana for a 2-drop"
        );

        let mut options = ai_options("billy-decide-seed", AI_BUDGET);
        let decision = decide(&state, AI, &mut options).expect("a decision is reached");

        assert!(
            matches!(&decision.action, ActionBody::Play { instance_id, x: Some(6), .. } if *instance_id == billy_id),
            "decision should be playing Buff Billy at X=6, got: {:?}",
            decision.action
        );
    }

    /// Tribute sets prefer cheaper sacrificed units (#1778).
    #[test]
    fn tribute_sets_prefer_cheaper_units() {
        jackioh_cards::register_all();
        // The Rock (core-066) requires Tribute 1.
        // Field has a 3-cost unit (core-013) in lane 1 and a 1-cost unit (core-003) in lane 2:
        // legal_actions lists the lane-1 unit's set first, so only the sort puts the cheap set first.
        let state = scenario(json!({
            "seed": "ways-tribute-prefer",
            "p1": {
                "hand": ["core-066"],
                "field": [
                    { "def": "core-013", "lane": 1 },
                    { "def": "core-003", "lane": 2 }
                ],
                "mana": 4,
                "library": ["core-008", "core-011"],
            },
            "p2": {
                "hand": ["core-005"],
                "library": ["core-008", "core-011"],
            }
        }))
        .state()
        .clone();

        let rock_id = hand_id(&state, "core-066");
        let unit_1cost_id = state.players[AI]
            .units
            .iter()
            .flatten()
            .flatten()
            .find(|c| c.def_id == "core-003")
            .map(|c| c.id.clone())
            .unwrap();
        let unit_3cost_id = state.players[AI]
            .units
            .iter()
            .flatten()
            .flatten()
            .find(|c| c.def_id == "core-013")
            .map(|c| c.id.clone())
            .unwrap();

        let first_tributing = |actions: &[ActionBody], unit_id: &String| {
            actions.iter().position(|a| {
                matches!(a, ActionBody::Play { instance_id, tributes: Some(t), .. }
                    if *instance_id == rock_id && t.contains(unit_id))
            })
        };
        let listed = legal_actions(&state, AI);
        let listed_dear = first_tributing(&listed, &unit_3cost_id).expect("a set tributing the 3-cost unit");
        let listed_cheap = first_tributing(&listed, &unit_1cost_id).expect("a set tributing the 1-cost unit");
        assert!(
            listed_dear < listed_cheap,
            "legal_actions lists the lane-1 (dearer) unit's set first, so the order below is the sort's"
        );

        let candidates = candidate_actions(&state, AI, Prices::NONE);
        let rock_plays: Vec<&ActionBody> = candidates
            .iter()
            .filter(|a| matches!(a, ActionBody::Play { instance_id, .. } if *instance_id == rock_id))
            .collect();

        assert!(!rock_plays.is_empty(), "Rock should have candidate plays");

        let first_tributes_cheap = rock_plays.iter().position(|a| match a {
            ActionBody::Play {
                tributes: Some(t), ..
            } => t.contains(&unit_1cost_id),
            _ => false,
        });
        let first_tributes_expensive = rock_plays.iter().position(|a| match a {
            ActionBody::Play {
                tributes: Some(t), ..
            } => t.contains(&unit_3cost_id),
            _ => false,
        });

        assert!(
            first_tributes_cheap.is_some() && first_tributes_expensive.is_some(),
            "both tribute combinations should be present in candidate plays"
        );
        assert!(
            first_tributes_cheap.unwrap() < first_tributes_expensive.unwrap(),
            "tributing cheaper unit (index {:?}) must come before tributing expensive unit (index {:?})",
            first_tributes_cheap,
            first_tributes_expensive
        );
    }
}

/// R1829000: a card a Stack pile uncovers keeps its id and is summoning sick again, so the search
/// signature tells apart two positions that differ only in a unit's sickness (#1884): merged, a child
/// whose unit cannot attack this turn stood for one whose unit can, and the lethal solver missed it.
#[test]
fn r1829000_two_positions_that_differ_only_in_summoning_sickness_sign_apart() {
    super::support::register_cards();
    let state = scenario(json!({
        "seed": "ai-sick",
        "p1": { "hand": ["core-011"], "field": ["core-008"], "library": ["core-008", "core-011"] },
        "p2": { "hand": ["core-011"], "field": ["core-011"], "library": ["core-008", "core-011"] },
    }))
    .state()
    .clone();
    let unit = active_units_of(&state, AI)[0].id.clone();
    assert!(
        !is_sick(&state, active_units_of(&state, AI)[0]),
        "the board's unit is ready"
    );
    let mut sick = state.clone();
    let turn = sick.turn;
    find_instance_mut(&mut sick, &unit)
        .expect("the unit")
        .summoned_turn = Some(turn);
    assert!(is_sick(&sick, active_units_of(&sick, AI)[0]));
    assert_ne!(search_signature(&state, AI), search_signature(&sick, AI));
    // An earlier turn's entry is no sickness, and signs as the ready board does.
    let mut earlier = state.clone();
    find_instance_mut(&mut earlier, &unit)
        .expect("the unit")
        .summoned_turn = Some(turn - 1);
    assert_eq!(search_signature(&state, AI), search_signature(&earlier, AI));
}
