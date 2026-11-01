//! #1779: the move order (`candidate_actions`) tries each attacker's best kill at an even trade or
//! better, by the opponent model's own static read (`reply.rs`'s `trade_value`: the target's worth less
//! the attacker's when it dies too, at least 0), just after the winning trades and ahead of the plays.
//! A losing trade, and every other even trade of the same attacker, stays behind the plays.
//! #1821: a Cleave attack whose splash kills a neighbour is a kill, though its target lives.
//!
//! The cards: Wrong-House Attacker 1/1 Rush, Lifesteal, Poisonous; Midrange Menace 9/9 Taunt; Hit Job
//! (3 mana: destroy a Unit); Mr. Vanilla 4/4; Reno 4/6; Tempo Timmy 3/3 Rush, First Strike; Mommy
//! Barker, Radiant 4/4 Reborn; Nurse Cleaver 3/6 Rush, Cleave, Lifesteal; and Zoomerbin Oomen 1/2.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, register_cards};

const WRONG_HOUSE: &str = "classicplus-006";
const MENACE: &str = "core-019";
const HIT_JOB: &str = "core-016";
const VANILLA: &str = "core-008";
const RENO: &str = "core-053";
const TIMMY: &str = "core-011";
const BARKER: &str = "classicplus-013";
const CLEAVER: &str = "classic-073";
const OOMEN: &str = "core-067";
const LIBRARY: &[&str] = &["core-008", "core-011", "core-008", "core-011"];

fn board(setup: Value) -> GameState {
    register_cards();
    scenario(setup).state().clone()
}

/// The id of `player`'s unit of `def_id` on the field.
fn unit_id(state: &GameState, player: PlayerId, def_id: &str) -> String {
    active_units_of(state, player)
        .into_iter()
        .find(|unit| unit.def_id == def_id)
        .map(|unit| unit.id.clone())
        .expect("the unit stands on the field")
}

fn hand_id(state: &GameState, def_id: &str) -> String {
    state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == def_id)
        .map(|card| card.id.clone())
        .expect("the card is in the AI's hand")
}

fn attack(attacker_id: &str, target_id: &str) -> ActionBody {
    ActionBody::Attack {
        attacker_id: attacker_id.to_string(),
        target_id: target_id.to_string(),
    }
}

fn place(actions: &[ActionBody], wanted: &ActionBody) -> usize {
    actions
        .iter()
        .position(|action| action == wanted)
        .unwrap_or_else(|| panic!("{} is a candidate", action_key(wanted)))
}

fn first_play(actions: &[ActionBody]) -> usize {
    actions
        .iter()
        .position(|action| matches!(action, ActionBody::Play { .. }))
        .expect("the hand card's play is a candidate")
}

fn keys(actions: &[ActionBody]) -> Vec<String> {
    actions.iter().map(action_key).collect()
}

/// The enemy's Midrange Menace 9/9 Taunt in lane 3.
fn menace_side() -> Value {
    json!({ "field": [{ "def": MENACE, "lane": 3 }], "library": LIBRARY })
}

#[test]
fn wrong_house_attackers_trade_into_the_taunt_is_within_branching_after_its_play() {
    let state = board(json!({
        "seed": "ai-even-trades-busy",
        "p1": {
            "hand": [WRONG_HOUSE, HIT_JOB, VANILLA, VANILLA, TIMMY, RENO],
            "mana": 10,
            "library": LIBRARY,
        },
        "p2": menace_side(),
    }));
    let attacker = hand_id(&state, WRONG_HOUSE);
    let play = legal_actions(&state, AI)
        .into_iter()
        .find(|action| {
            matches!(action, ActionBody::Play { instance_id, zone: Some(zone), .. }
                if *instance_id == attacker && zone.lane == 3)
        })
        .expect("Wrong-House Attacker may be played into lane 3");
    let after = act(&state, AI, &play);

    let trade = attack(&unit_id(&after, AI, WRONG_HOUSE), &unit_id(&after, HUMAN, MENACE));
    let expanded = expansion(
        &after,
        AI,
        AI_BUDGET.branching,
        MoveOrder::Heuristic,
        &POLICY_PRIOR,
        Prices::NONE,
    );
    assert!(
        expanded.contains(&trade),
        "the Attacker's trade into the Menace is within {}: {:?}",
        AI_BUDGET.branching,
        keys(&expanded)
    );
}

#[test]
fn decide_trades_into_the_taunt_rather_than_cast_hit_job_on_a_hand_that_fills_the_root() {
    let state = board(json!({
        "seed": "ai-even-trades-root",
        "p1": {
            "hand": [HIT_JOB, RENO, RENO, VANILLA, VANILLA, TIMMY],
            "mana": 4,
            "field": [{ "def": WRONG_HOUSE, "lane": 3 }],
            "library": LIBRARY,
        },
        "p2": menace_side(),
    }));
    let root = root_candidate_actions(&state, AI, Prices::NONE);
    assert!(
        root.len() > AI_BUDGET.root_branching + 1,
        "the hand fills the root: {:?}",
        keys(&root)
    );

    let decision = decide(
        &state,
        AI,
        &mut AiOptions {
            rng: create_rng("ai-even-trades-root", 0),
            budget: AI_BUDGET,
            should_stop: None,
        },
    )
    .expect("a decision");
    let trade = attack(&unit_id(&state, AI, WRONG_HOUSE), &unit_id(&state, HUMAN, MENACE));
    let hit_job = hand_id(&state, HIT_JOB);
    assert!(
        decision.line.contains(&trade),
        "the line trades the Attacker into the Menace: {:?}",
        keys(&decision.line)
    );
    assert!(
        !decision
            .line
            .iter()
            .any(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == hit_job)),
        "the line spends no Hit Job: {:?}",
        keys(&decision.line)
    );
}

#[test]
fn an_even_trade_mr_vanilla_for_mr_vanilla_is_tried_ahead_of_the_plays() {
    let state = board(json!({
        "seed": "ai-even-trades-mirror",
        "p1": { "hand": [VANILLA], "field": [VANILLA] },
        "p2": { "field": [VANILLA] },
    }));
    let actions = candidate_actions(&state, AI, Prices::NONE);
    let trade = place(
        &actions,
        &attack(&unit_id(&state, AI, VANILLA), &unit_id(&state, HUMAN, VANILLA)),
    );
    assert!(trade < first_play(&actions), "{:?}", keys(&actions));
}

#[test]
fn each_attackers_best_even_trade_goes_ahead_and_its_other_ones_stay_behind_the_plays() {
    let state = board(json!({
        "seed": "ai-even-trades-best",
        "p1": { "hand": [VANILLA], "field": [WRONG_HOUSE] },
        "p2": { "field": [VANILLA, RENO] },
    }));
    let actions = candidate_actions(&state, AI, Prices::NONE);
    let attacker = unit_id(&state, AI, WRONG_HOUSE);
    let on_reno = place(&actions, &attack(&attacker, &unit_id(&state, HUMAN, RENO)));
    let on_vanilla = place(&actions, &attack(&attacker, &unit_id(&state, HUMAN, VANILLA)));
    let play = first_play(&actions);
    assert!(
        on_reno < play && play < on_vanilla,
        "the trade up into Reno at {on_reno}, the first play at {play}, the one into Mr. Vanilla at \
         {on_vanilla}: {:?}",
        keys(&actions)
    );
}

#[test]
fn a_losing_trade_stays_behind_the_plays() {
    let state = board(json!({
        "seed": "ai-even-trades-losing",
        "p1": { "hand": [VANILLA], "field": [MENACE] },
        "p2": { "field": [WRONG_HOUSE] },
    }));
    let actions = candidate_actions(&state, AI, Prices::NONE);
    let trade = place(
        &actions,
        &attack(&unit_id(&state, AI, MENACE), &unit_id(&state, HUMAN, WRONG_HOUSE)),
    );
    assert!(trade > first_play(&actions), "{:?}", keys(&actions));
}

#[test]
fn of_two_targets_with_equal_stats_the_even_trade_goes_into_the_one_without_reborn() {
    // #1796: Mommy Barker's Radiant 4/4 Reborn comes back at 1 health, so the kill that trades Mr.
    // Vanilla for it is a losing one, and the trade into the other Mr. Vanilla is the even one.
    let state = board(json!({
        "seed": "ai-even-trades-reborn",
        "p1": { "hand": [VANILLA], "field": [VANILLA] },
        "p2": { "field": [{ "def": BARKER, "radiant": true }, VANILLA] },
    }));
    let actions = candidate_actions(&state, AI, Prices::NONE);
    let attacker = unit_id(&state, AI, VANILLA);
    let on_vanilla = place(&actions, &attack(&attacker, &unit_id(&state, HUMAN, VANILLA)));
    let on_barker = place(&actions, &attack(&attacker, &unit_id(&state, HUMAN, BARKER)));
    let play = first_play(&actions);
    assert!(
        on_vanilla < play && play < on_barker,
        "the trade into Mr. Vanilla at {on_vanilla}, the first play at {play}, the one into Mommy \
         Barker at {on_barker}: {:?}",
        keys(&actions)
    );
}

#[test]
fn a_cleave_attack_whose_only_kills_are_its_splash_is_tried_ahead_of_the_plays() {
    // #1821: Nurse Cleaver's 3 leaves Reno standing, but its splash kills the Oomen on each side.
    let state = board(json!({
        "seed": "ai-even-trades-cleave",
        "p1": { "hand": [VANILLA], "field": [CLEAVER] },
        "p2": { "field": [
            { "def": OOMEN, "lane": 1 },
            { "def": RENO, "lane": 2 },
            { "def": OOMEN, "lane": 3 },
        ] },
    }));
    let actions = candidate_actions(&state, AI, Prices::NONE);
    let trade = place(
        &actions,
        &attack(&unit_id(&state, AI, CLEAVER), &unit_id(&state, HUMAN, RENO)),
    );
    assert!(trade < first_play(&actions), "{:?}", keys(&actions));
}
