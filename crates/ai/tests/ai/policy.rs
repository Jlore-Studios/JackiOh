//! The policy prior (#1171, `src/policy.rs`): its keys read only what the seat may know and name no
//! instance (R1171000), and it re-sorts the moves the beam expands without adding, dropping or
//! changing one, while the switch off or the null table leave today's order (R1171001).

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, clone, every_card, is_legal, random_policy_states};

/// A table with no keys: the null one `src/policy/weights.json` holds until a fit replaces it.
const NULL: PolicyPrior<'static> = PolicyPrior {
    keys_version: POLICY_KEYS_VERSION,
    catalog: "test",
    keys: &[],
    weights: &[],
};

/// A table that moves plays of core-008 up, attacks down and the heuristic's first move a little down.
const TEST: PolicyPrior<'static> = PolicyPrior {
    keys_version: POLICY_KEYS_VERSION,
    catalog: "test",
    keys: &["card:play:core-008", "kind:attack", "rank:0"],
    weights: &[2.0, -1.0, -0.5],
};

/// Mid-game states of a random-policy game in a main phase, each with the seat to act there.
fn states() -> Vec<(GameState, PlayerId)> {
    jackioh_cards::register_all();
    random_policy_states("policy-1171", 5, 200)
        .into_iter()
        .filter(|state| state.phase == Phase::Main && state.result.is_none())
        .filter_map(|state| seat_to_act(&state).map(|seat| (state, seat)))
        .collect()
}

/// Today's expansion, written out: the first `width` candidates that are not End Turn, then End Turn
/// when it is one.
fn todays(state: &GameState, seat: PlayerId, width: usize) -> Vec<ActionBody> {
    let candidates = candidate_actions(state, seat);
    let mut moves: Vec<ActionBody> = candidates
        .iter()
        .filter(|action| **action != ActionBody::EndTurn)
        .take(width)
        .cloned()
        .collect();
    if candidates.contains(&ActionBody::EndTurn) {
        moves.push(ActionBody::EndTurn);
    }
    moves
}

/// `features.rs`'s pair (B9): p2's hand, face-down trap and library differ.
fn p2_a() -> Value {
    json!({
        "hand": ["core-002", "core-011"],
        "field": ["core-019"],
        "backrow": [{ "def": "core-041", "faceUp": false }],
        "library": ["core-005", "core-016", "core-047"],
        "health": 20,
    })
}

fn p2_b() -> Value {
    json!({
        "hand": ["core-053", "core-020"],
        "field": ["core-019"],
        "backrow": [{ "def": "core-060", "faceUp": false }],
        "library": ["core-063", "core-044", "core-072"],
        "health": 20,
    })
}

fn p1_side() -> Value {
    json!({
        "hand": ["core-008", "core-036"],
        "field": ["core-011"],
        "mana": 3,
        "library": ["core-020", "core-053", "core-025"],
    })
}

fn build(seed: &str, p2: Value) -> GameState {
    jackioh_cards::register_all();
    scenario(json!({ "seed": seed, "active": "p1", "turn": 9, "p1": p1_side(), "p2": p2 }))
        .state()
        .clone()
}

/// The fixture `r1171001_a_move_past_the_prefix_…` searches: core-086 first in the heuristic order,
/// core-008 second, mana for one of them.
fn prefix_fixture() -> GameState {
    jackioh_cards::register_all();
    scenario(json!({
        "seed": "policy-1171-prefix",
        "active": "p1",
        "turn": 9,
        "p1": { "hand": ["core-086", "core-008"], "mana": 1, "library": ["core-008", "core-011"] },
        "p2": { "library": ["core-008", "core-011"], "health": 20 },
    }))
    .state()
    .clone()
}

/// The def id the action plays, if it is a play.
fn played_def<'a>(state: &'a GameState, action: &ActionBody) -> Option<&'a str> {
    match action {
        ActionBody::Play { instance_id, .. } => {
            find_instance(state, instance_id).map(|card| card.def_id.as_str())
        }
        _ => None,
    }
}

/// R185, R1171000: two states that differ only in p2's hidden hand, face-down trap and library, p1's
/// library order and the seed give p1 the same candidates, and each the same label, and the enemy's
/// face-down trap is aimed at by its place, never named.
#[test]
fn r1171000_r185_two_states_differing_only_in_hidden_cards_label_alike() {
    let a = build("policy-a", p2_a());
    let mut b = clone(&build("policy-b", p2_b()));
    b.players[PlayerId::P1].library.reverse();
    assert_ne!(hash_state(&a), hash_state(&b));

    let moves = candidate_actions(&a, AI);
    assert_eq!(moves, candidate_actions(&b, AI));
    assert_eq!(moves.len(), 6, "{moves:?}");
    for action in &moves {
        let label = policy_label(&a, AI, action);
        assert_eq!(label, policy_label(&b, AI, action), "{action:?}");
        assert!(label.chosen.is_some(), "{action:?} is one of decide's candidates");
        for keys in &label.candidates {
            assert!(
                !keys
                    .iter()
                    .any(|key| key.contains("core-041") || key.contains("core-060")),
                "the face-down trap is not named: {keys:?}"
            );
        }
    }

    // Magic Jammed (`c2`) aims at the enemy's face-down trap (`c10`): by its place alone.
    let jammed = moves
        .iter()
        .find(|action| played_def(&a, action) == Some("core-036"))
        .unwrap_or_else(|| panic!("Magic Jammed is a candidate: {moves:?}"));
    assert!(
        matches!(jammed, ActionBody::Play { instance_id, targets: Some(targets), .. }
            if instance_id == "c2"
                && targets == &[Selection::Instance { instance_id: "c10".to_string() }]),
        "{jammed:?}"
    );
    let keys = policy_keys(&redact(&a, AI), AI, jammed, Some(0));
    assert!(keys.contains(&"aim:play:enemy.backrow".to_string()), "{keys:?}");
    assert!(keys.contains(&"card:play:core-036".to_string()), "{keys:?}");
    assert!(!keys.iter().any(|key| key.starts_with("aimcard:")), "{keys:?}");
}

/// R1171000: over a real game, every candidate's keys are sorted, each once, the same on every call,
/// and no part of one is an instance id, a hero's target id or the open prompt's id.
#[test]
fn r1171000_no_key_names_an_instance_a_hero_or_a_prompt() {
    let states = states();
    assert!(states.len() > 10, "{} states", states.len());
    for (state, seat) in &states {
        let view = redact(state, *seat);
        let mut ids: Vec<String> = every_card(state).iter().map(|card| card.id.clone()).collect();
        ids.extend(["hero-p1".to_string(), "hero-p2".to_string()]);
        if let Some(pending) = &state.pending {
            ids.push(pending.id.clone());
        }
        for (rank, action) in candidate_actions(state, *seat).iter().enumerate() {
            let keys = policy_keys(&view, *seat, action, Some(rank));
            assert!(keys.windows(2).all(|pair| pair[0] < pair[1]), "{keys:?}");
            assert_eq!(keys, policy_keys(&view, *seat, action, Some(rank)));
            for key in &keys {
                assert!(
                    !key.split([':', '.']).any(|part| ids.iter().any(|id| id == part)),
                    "{key} names an id ({action:?})"
                );
            }
        }
    }
}

/// R1171000: the table compiled in was fitted on this build's key layout, its keys sorted and unique,
/// one finite weight per key.
#[test]
fn r1171000_the_compiled_prior_names_this_build_s_key_layout() {
    assert_eq!(POLICY_PRIOR.keys_version, POLICY_KEYS_VERSION);
    assert!(POLICY_PRIOR.keys.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(POLICY_PRIOR.weights.len(), POLICY_PRIOR.keys.len());
    assert!(POLICY_PRIOR.weights.iter().all(|weight| weight.is_finite()));
}

/// R1171001: with the switch on Heuristic, whatever the table, or with the null table, whatever the
/// switch, each line's moves and the beam's lines are today's.
#[test]
fn r1171001_off_or_null_the_order_is_today_s() {
    let states = states();
    for (state, seat) in &states {
        for width in [1, 6, 20] {
            let today = todays(state, *seat, width);
            assert_eq!(expansion(state, *seat, width, MoveOrder::Heuristic, &TEST), today);
            assert_eq!(expansion(state, *seat, width, MoveOrder::Learned, &NULL), today);
        }
    }

    let mut searched: Vec<(GameState, PlayerId)> = vec![(prefix_fixture(), AI)];
    searched.extend(
        states
            .into_iter()
            .filter(|(state, _)| state.pending.is_none())
            .take(2),
    );
    for (state, seat) in &searched {
        let budget = AI_BUDGET;
        let off = beam_search_with(
            state,
            *seat,
            &create_node_counter(budget.nodes, None),
            budget,
            MoveOrder::Heuristic,
            &TEST,
        );
        let null = beam_search_with(
            state,
            *seat,
            &create_node_counter(budget.nodes, None),
            budget,
            MoveOrder::Learned,
            &NULL,
        );
        assert!(!off.is_empty());
        assert_eq!(null, off);
    }
}

/// R1171001: the prior returns the moves it was given, every one legal, sorted by score best first
/// with ties in their own order; the expansion keeps End Turn last exactly when it is a candidate; and
/// in a Duos duel the order is the heuristic's.
#[test]
fn r1171001_the_prior_reorders_and_never_adds_drops_or_changes_a_move() {
    let mut reordered = 0;
    for (state, seat) in &states() {
        let moves: Vec<ActionBody> = candidate_actions(state, *seat)
            .into_iter()
            .filter(|action| *action != ActionBody::EndTurn)
            .collect();
        let sorted = prior_order(state, *seat, moves.clone(), &TEST);
        assert_eq!(sorted.len(), moves.len());
        let view = redact(state, *seat);
        // Each move's original rank, so its score is taken where the heuristic put it.
        let ranks: Vec<usize> = sorted
            .iter()
            .map(|action| {
                moves
                    .iter()
                    .position(|original| original == action)
                    .unwrap_or_else(|| panic!("{action:?} was not given"))
            })
            .collect();
        let mut seen = ranks.clone();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), moves.len(), "a permutation");
        let scores: Vec<f64> = ranks
            .iter()
            .map(|rank| TEST.score(&policy_keys(&view, *seat, &moves[*rank], Some(*rank))))
            .collect();
        for at in 1..sorted.len() {
            assert!(scores[at - 1] >= scores[at], "{scores:?}");
            if scores[at - 1] == scores[at] {
                assert!(ranks[at - 1] < ranks[at], "a tie keeps its order: {ranks:?}");
            }
        }
        for action in &sorted {
            assert!(is_legal(state, *seat, action), "{action:?}");
        }
        if sorted != moves {
            reordered += 1;
        }

        let expanded = expansion(state, *seat, 20, MoveOrder::Learned, &TEST);
        let ends = candidate_actions(state, *seat).contains(&ActionBody::EndTurn);
        assert_eq!(expanded.last() == Some(&ActionBody::EndTurn), ends);
        assert_eq!(
            expanded
                .iter()
                .filter(|action| **action == ActionBody::EndTurn)
                .count(),
            usize::from(ends)
        );

        let mut duos = clone(state);
        duos.players[*seat].duos = Some(DuosLife {
            pool: 10,
            pool_cap: 10,
            spent: false,
        });
        assert_eq!(prior_order(&duos, *seat, moves.clone(), &TEST), moves);
    }
    assert!(reordered > 0, "the test table reorders some decision");
}

/// R1171001: with a root branching of one, the heuristic's first move is the only one the beam opens
/// with; once the prior ranks core-008 first, the beam opens with it and the line it finds is better.
#[test]
fn r1171001_a_move_past_the_prefix_is_searched_once_the_prior_ranks_it_first() {
    let state = prefix_fixture();
    let moves = candidate_actions(&state, AI);
    assert_eq!(played_def(&state, &moves[0]), Some("core-086"), "{moves:?}");
    assert_eq!(played_def(&state, &moves[1]), Some("core-008"), "{moves:?}");

    let budget = SearchBudget {
        root_branching: 1,
        ..AI_GATE_BUDGET
    };
    let prior = PolicyPrior {
        keys_version: POLICY_KEYS_VERSION,
        catalog: "test",
        keys: &["card:play:core-008"],
        weights: &[1.0],
    };
    let opens_with_008 =
        |line: &Line| line.actions.first().and_then(|first| played_def(&state, first)) == Some("core-008");

    let heuristic = beam_search_with(
        &state,
        AI,
        &create_node_counter(budget.nodes, None),
        budget,
        MoveOrder::Heuristic,
        &prior,
    );
    assert!(!heuristic.is_empty());
    assert!(
        !heuristic.iter().any(opens_with_008),
        "core-008 is past the prefix"
    );

    let learned = beam_search_with(
        &state,
        AI,
        &create_node_counter(budget.nodes, None),
        budget,
        MoveOrder::Learned,
        &prior,
    );
    let best = learned
        .first()
        .unwrap_or_else(|| panic!("the beam found no line"));
    assert!(opens_with_008(best), "{:?}", best.actions);
    assert!(
        best.score > heuristic[0].score,
        "{} <= {}",
        best.score,
        heuristic[0].score
    );
}
