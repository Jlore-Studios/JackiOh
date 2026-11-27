//! R4788000 (#4788): a wide decision's lethal. `decide` scales a wide state's allowance down
//! (AI_SEARCH.fat_*, R2575000), the lethal solver's share with it, and a lethal of four to six actions
//! then needs more nodes than the solver is given. Once its scaled allowance is spent, the best-first walk
//! goes on only from a position whose ready attacks already reach lethal (`ready_gap` at or under 0), and
//! finishes the proof of a line that won on determinization 0, up to the allowance its caller gave.
//!
//! The pinned position is turn 8 of `audit:hard-vs-easy:2`, p2 to move with 32 root candidates: the
//! solver proves its six-action lethal in 213 nodes, and the scaled allowance was 140.
//!
//! The audit's (#4787) long lethals on boards of ordinary width are pinned beside it, each proved within
//! `AI_BUDGET.lethal_nodes` since R5119000 kept the walk's attackers ready and set a long flat dive aside
//! (`r5119000_the_audits_long_lethals_are_proved_within_the_lethal_allowance`).
//!
//! Its worlds sample the hidden cards from the catalog's pools, so a card joining one deals other worlds
//! (R380, #3117): the R4788000 tests search its stream for a cursor that meets their premise
//! (`premise_cursor`).

use jackioh_ai::{
    AI_BUDGET, AI_SEARCH, AiOptions, DecisionReason, DeterminizeOptions, NodeCounter, Prices, SearchBudget,
    candidate_actions, create_node_counter, decide, det_ground, determinize_on, find_lethal,
    find_lethal_reaching, probe_world, ready_gap, redact,
};
use jackioh_engine::testkit::{GameState, PlayerId, Value, create_rng, cursor_where, json, with_pool_shift};

use super::support::{AI, act, decision_prices_at, register_cards, scenario};

const WIDE_LETHAL: &str = include_str!("../fixtures/pinned-wide-lethal.json");
/// Turn 12 of the mirror game `audit:ai-vs-greedy:26`, p2 to move: a play, three Activates of it on the
/// Taunt that walls p2's one attacker, and that attacker's hit.
const AUDIT_MIRROR_26_T12: &str = include_str!("../fixtures/pinned-audit-mirror-26-t12.json");
/// Turn 12 of `audit:hard-vs-easy:8`, p2 to move: ten actions, the gap at 1 until the last.
const AUDIT_HVE_8_T12: &str = include_str!("../fixtures/pinned-audit-hve-8-t12.json");
/// Turn 21 of `audit:hard-vs-easy:12`, p1 to move: nine actions, the middle ones raising the gap before a
/// prompt's answer drops it to 0.
const AUDIT_HVE_12_T21: &str = include_str!("../fixtures/pinned-audit-hve-12-t21.json");

/// The pinned position: the true state, the deciding seat and its stream.
struct Pinned {
    state: GameState,
    seat: PlayerId,
    rng_seed: String,
    rng_cursor: u32,
}

fn pinned(text: &str) -> Pinned {
    register_cards();
    let fixture: Value = serde_json::from_str(text).expect("the fixture is JSON");
    Pinned {
        state: serde_json::from_value(fixture["state"].clone()).expect("the fixture's state"),
        seat: serde_json::from_value(fixture["seat"].clone()).expect("the fixture's seat"),
        rng_seed: fixture["rngSeed"]
            .as_str()
            .expect("the fixture's rngSeed")
            .to_string(),
        rng_cursor: fixture["rngCursor"]
            .as_u64()
            .and_then(|cursor| u32::try_from(cursor).ok())
            .expect("the fixture's rngCursor"),
    }
}

/// The worlds `decide` samples for the position, in its order, on its stream from `cursor`.
fn worlds(at: &Pinned, cursor: u32) -> Vec<GameState> {
    let public = redact(&at.state, at.seat);
    let ground = det_ground(&public, at.seat);
    let mut rng = create_rng(&at.rng_seed, cursor);
    (0..AI_BUDGET.determinizations)
        .map(|_| determinize_on(&public, at.seat, &mut rng, DeterminizeOptions::default(), &ground))
        .collect()
}

/// The position's root candidates, as `decide` lists them.
fn width(at: &Pinned) -> usize {
    let public = redact(&at.state, at.seat);
    let probe = probe_world(&public, at.seat, &det_ground(&public, at.seat));
    candidate_actions(&probe, at.seat, Prices::NONE).len()
}

/// The lethal allowance `decide` scales a state of `fat` width down to at AI_BUDGET (decide.rs
/// `scaled_budget`).
fn scaled_allowance(fat: usize) -> usize {
    let reference = AI_SEARCH.fat_reference as usize;
    let nodes = AI_BUDGET.nodes.min(AI_SEARCH.fat_budget_nodes as usize) * reference / fat;
    AI_BUDGET.lethal_nodes * nodes / AI_BUDGET.nodes
}

/// R4788000's premise on the pinned wide position: the first cursor of its stream whose worlds the solver
/// proves lethal within the caller's allowance (`AI_BUDGET.lethal_nodes`), past the `scaled` one, and not
/// within it. Searched (#848), not the audit's own cursor, which pinned where the pools' rolls land.
fn premise_cursor(at: &Pinned, scaled: usize, prices: &Prices) -> u32 {
    cursor_where(&at.rng_seed, |cursor| {
        let worlds = worlds(at, cursor);
        let short = create_node_counter(scaled, None);
        let whole = create_node_counter(AI_BUDGET.lethal_nodes, None);
        find_lethal(&worlds, at.seat, &short, scaled, prices).is_none()
            && find_lethal(&worlds, at.seat, &whole, AI_BUDGET.lethal_nodes, prices).is_some()
            && whole.used() > scaled
    })
}

#[test]
fn r4788000_a_wide_decision_plays_the_lethal_its_scaled_allowance_cut_short() {
    let at = pinned(WIDE_LETHAL);
    let fat = width(&at);
    assert!(
        fat > AI_SEARCH.fat_reference as usize,
        "{fat} candidates is a wide state"
    );
    let scaled = scaled_allowance(fat);
    assert!(
        scaled < AI_BUDGET.lethal_nodes,
        "the allowance is scaled down to {scaled}"
    );

    // The premise: on the worlds `decide` samples from this cursor, the solver proves the lethal within
    // the caller's allowance and not within the scaled one.
    let prices = decision_prices_at(&at.state, at.seat);
    let cursor = premise_cursor(&at, scaled, &prices);

    let mut options = AiOptions::with_budget(create_rng(&at.rng_seed, cursor), AI_BUDGET);
    let decision = decide(&at.state, at.seat, &mut options).expect("the seat has a decision to make");
    assert_eq!(decision.reason, DecisionReason::Lethal, "{:?}", decision.stats);
    assert!(decision.stats.nodes <= AI_BUDGET.nodes, "{:?}", decision.stats);
}

#[test]
fn r4788000_past_its_allowance_the_walk_goes_on_from_a_position_that_reads_as_lethal() {
    let at = pinned(WIDE_LETHAL);
    let scaled = scaled_allowance(width(&at));
    let prices = decision_prices_at(&at.state, at.seat);
    let worlds = worlds(&at, premise_cursor(&at, scaled, &prices));
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let reach = AI_BUDGET.lethal_nodes;
    let proved = find_lethal_reaching(&worlds, at.seat, &counter, scaled, &counter, reach, &prices);
    assert!(
        proved.is_some(),
        "the walk reaches it in {} nodes",
        counter.used()
    );
    assert!(
        counter.used() > scaled,
        "{} nodes, past the {scaled} it was given",
        counter.used()
    );
    assert!(
        counter.used() <= reach,
        "{} nodes, within its reach of {reach}",
        counter.used()
    );
}

#[test]
fn r4788000_the_pinned_wide_decision_keeps_to_a_callers_own_figures() {
    let at = pinned(WIDE_LETHAL);
    // R4802000's edges: a solver given nothing, and one given more than the whole budget.
    let none = SearchBudget {
        lethal_nodes: 0,
        ..AI_BUDGET
    };
    let past = SearchBudget {
        nodes: 200,
        lethal_nodes: 400,
        ..AI_BUDGET
    };
    for budget in [none, past] {
        let mut options = AiOptions::with_budget(create_rng(&at.rng_seed, at.rng_cursor), budget);
        let decision = decide(&at.state, at.seat, &mut options).expect("the seat has a decision to make");
        assert!(
            decision.stats.nodes <= budget.nodes,
            "{budget:?}: {:?}",
            decision.stats
        );
        if budget.lethal_nodes == 0 {
            assert_ne!(
                decision.reason,
                DecisionReason::Lethal,
                "a solver given nothing has no reach"
            );
        }
    }
}

/// The lethal tests' wide board (lethal.rs) with the enemy hero at full health: a tree the solver cannot
/// search out, and no position of it reads as lethal.
fn far_from_lethal() -> GameState {
    register_cards();
    scenario(json!({
        "seed": "wide-lethal-far",
        "active": AI,
        "turn": 9,
        "p1": {
            "hand": [{ "def": "core-055", "radiant": true }, "core-035", "core-031"],
            "mana": 4,
            "field": ["core-025", "core-008", "core-011"],
        },
        "p2": { "field": ["core-019", "core-055"], "health": 30 },
    }))
    .state()
    .clone()
}

#[test]
fn r4788000_a_walk_that_never_reads_as_lethal_stops_at_its_allowance() {
    let state = far_from_lethal();
    assert!(ready_gap(&state, AI) > 0.0);
    let public = redact(&state, AI);
    let ground = det_ground(&public, AI);
    let mut rng = create_rng("wide-lethal-far:worlds", 0);
    let worlds: Vec<GameState> = (0..AI_BUDGET.determinizations)
        .map(|_| determinize_on(&public, AI, &mut rng, DeterminizeOptions::default(), &ground))
        .collect();
    let allowance = AI_SEARCH.fat_min_lethal as usize;

    let plain = create_node_counter(AI_BUDGET.nodes, None);
    assert_eq!(find_lethal(&worlds, AI, &plain, allowance, Prices::NONE), None);
    assert_eq!(plain.used(), allowance, "the walk is cut, not searched out");

    let reaching = create_node_counter(AI_BUDGET.nodes, None);
    let reach = AI_BUDGET.lethal_nodes;
    assert_eq!(
        find_lethal_reaching(&worlds, AI, &reaching, allowance, &reaching, reach, Prices::NONE),
        None
    );
    assert_eq!(reaching.used(), plain.used(), "no node past the allowance");
}

/// R5119000: the audit's (#4787) long lethals, each proved within `AI_BUDGET.lethal_nodes`. Before the
/// ruling the walk needed 510 nodes for mirror 26's turn 12 and 462 for hard-vs-easy 8's, a dive below an
/// attack on the Taunt and below a dead play, and proved hard-vs-easy 12's turn 21 in 192, which it still
/// does. Each is pinned as the audit dealt it, on its fixture's own cursor, so its worlds read the pools as
/// they are, not as the pool probe (#3117) rotates them: a rotation deals other hidden cards and moves the
/// walk's cost. A card joining a pool can move one the same way, and it is measured again.
#[test]
fn r5119000_the_audits_long_lethals_are_proved_within_the_lethal_allowance() {
    with_pool_shift(0, the_audits_long_lethals_within_the_allowance);
}

fn the_audits_long_lethals_within_the_allowance() {
    let room = AI_BUDGET.lethal_nodes;
    for (name, text) in [
        ("audit:ai-vs-greedy:26 turn 12", AUDIT_MIRROR_26_T12),
        ("audit:hard-vs-easy:8 turn 12", AUDIT_HVE_8_T12),
        ("audit:hard-vs-easy:12 turn 21", AUDIT_HVE_12_T21),
    ] {
        let at = pinned(text);
        let worlds = worlds(&at, at.rng_cursor);
        let prices = decision_prices_at(&at.state, at.seat);
        let counter = create_node_counter(room, None);
        let proved = find_lethal(&worlds, at.seat, &counter, room, &prices);
        assert!(
            proved.is_some(),
            "{name}: no lethal in {} of {room} nodes",
            counter.used()
        );
    }
}

/// R5119000: hard-vs-easy 8's turn-12 lethal, as the walk proves it within `AI_BUDGET.lethal_nodes`, played
/// through the engine on the fixture's own state and on each of the walk's worlds: the game is won, so the
/// proof is the engine's, not an artifact of the walk's judge. It is five actions (a play, a second play,
/// two Activates of that one on the Taunt, and the hit the Taunt walled), not the ten the audit logged.
#[test]
fn r5119000_hard_vs_easy_8s_lethal_wins_the_game_through_the_engine() {
    with_pool_shift(0, hard_vs_easy_8s_lethal_through_the_engine);
}

fn hard_vs_easy_8s_lethal_through_the_engine() {
    let at = pinned(AUDIT_HVE_8_T12);
    let worlds = worlds(&at, at.rng_cursor);
    let prices = decision_prices_at(&at.state, at.seat);
    let room = AI_BUDGET.lethal_nodes;
    let counter = create_node_counter(room, None);
    let line = find_lethal(&worlds, at.seat, &counter, room, &prices).expect("the walk proves the lethal");
    assert_eq!(line.len(), 5, "{line:?}");
    let starts =
        std::iter::once(("the fixture", &at.state)).chain(worlds.iter().map(|world| ("a world", world)));
    for (name, start) in starts {
        let end = line
            .iter()
            .fold(start.clone(), |state, action| act(&state, at.seat, action));
        assert_eq!(
            end.result.and_then(|result| result.winner.player()),
            Some(at.seat),
            "{name}: {line:?}"
        );
    }
}
