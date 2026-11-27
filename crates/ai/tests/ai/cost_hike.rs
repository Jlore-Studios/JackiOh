//! R5122000 (issue #5122): the AI counts an unconditional price hike on the opponent's coming turn
//! in `coming_refreshes` as harm to that player's next turn.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, clone, close, register_cards, scenario};

fn board(opts: Value) -> GameState {
    register_cards();
    let mut options = json!({ "seed": "r5122000-cost-hike", "active": AI, "turn": 1 });
    if let (Some(base), Value::Object(extra)) = (options.as_object_mut(), opts) {
        for (key, value) in extra {
            base.insert(key, value);
        }
    }
    scenario(options).state().clone()
}

fn eval(state: &GameState, seat: PlayerId) -> f64 {
    evaluate(state, seat, NextSwing::Enemy, &AI_EVAL)
}

fn add_modifier(state: &GameState, target: PlayerId, modifier: PlayerModifier) -> GameState {
    let mut out = clone(state);
    out.players[target].mods.push(modifier);
    out
}

fn next_turn_cost_rule(target: PlayerId, from_turn: i32, rule: CostRule) -> PlayerModifier {
    PlayerModifier {
        id: "mod-r5122000".to_string(),
        expiry: ModifierExpiry::NextTurnOf {
            player: target,
            from_turn,
        },
        kind: ModifierKind::CostRule { rule },
    }
}

#[test]
fn r5122000_enemy_coming_cost_hike_evaluates_higher() {
    let plain = board(json!({
        "p1": { "hand": ["core-005"], "library": ["core-008", "core-011"] },
        "p2": { "hand": ["core-005"], "library": ["core-008", "core-011"] },
    }));
    for n in [1, 2, 3] {
        let hiked = add_modifier(
            &plain,
            HUMAN,
            next_turn_cost_rule(
                HUMAN,
                plain.turn,
                CostRule {
                    amount: Some(n),
                    ..Default::default()
                },
            ),
        );
        let expected_delta = AI_EVAL.cost_hike * AI_EVAL.next_turn_mana * f64::from(AI_TAXED_PLAYS * n);
        close(eval(&hiked, AI) - eval(&plain, AI), expected_delta);
        close(eval(&plain, HUMAN) - eval(&hiked, HUMAN), expected_delta);
        close(
            coming_mana(&hiked, AI, &AI_EVAL) - coming_mana(&plain, AI, &AI_EVAL),
            expected_delta,
        );
    }
}

#[test]
fn r5122000_own_coming_cost_hike_evaluates_lower() {
    let plain = board(json!({
        "p1": { "hand": ["core-005"], "library": ["core-008", "core-011"] },
        "p2": { "hand": ["core-005"], "library": ["core-008", "core-011"] },
    }));
    for n in [1, 2, 3] {
        let hiked = add_modifier(
            &plain,
            AI,
            next_turn_cost_rule(
                AI,
                plain.turn,
                CostRule {
                    amount: Some(n),
                    ..Default::default()
                },
            ),
        );
        let expected_delta = AI_EVAL.cost_hike * AI_EVAL.next_turn_mana * f64::from(AI_TAXED_PLAYS * n);
        close(eval(&plain, AI) - eval(&hiked, AI), expected_delta);
        close(eval(&hiked, HUMAN) - eval(&plain, HUMAN), expected_delta);
        close(
            coming_mana(&plain, AI, &AI_EVAL) - coming_mana(&hiked, AI, &AI_EVAL),
            expected_delta,
        );
    }
}

#[test]
fn r5122000_disqualified_modifiers_add_nothing() {
    let plain = board(json!({
        "p1": { "hand": ["core-005"], "library": ["core-008", "core-011"] },
        "p2": { "hand": ["core-005"], "library": ["core-008", "core-011"] },
    }));

    // 1. Expired rule
    let expired = add_modifier(
        &plain,
        HUMAN,
        PlayerModifier {
            id: "mod-expired".to_string(),
            expiry: ModifierExpiry::ThisTurn { turn: plain.turn - 1 },
            kind: ModifierKind::CostRule {
                rule: CostRule {
                    amount: Some(2),
                    ..Default::default()
                },
            },
        },
    );
    assert_eq!(eval(&expired, AI), eval(&plain, AI));

    // 2. Live only this turn
    let this_turn = add_modifier(
        &plain,
        HUMAN,
        PlayerModifier {
            id: "mod-this-turn".to_string(),
            expiry: ModifierExpiry::ThisTurn { turn: plain.turn },
            kind: ModifierKind::CostRule {
                rule: CostRule {
                    amount: Some(2),
                    ..Default::default()
                },
            },
        },
    );
    assert_eq!(eval(&this_turn, AI), eval(&plain, AI));

    // 3. Discount (amount <= 0)
    let discount = add_modifier(
        &plain,
        HUMAN,
        next_turn_cost_rule(
            HUMAN,
            plain.turn,
            CostRule {
                amount: Some(-2),
                ..Default::default()
            },
        ),
    );
    assert_eq!(eval(&discount, AI), eval(&plain, AI));

    // 4. Rule with types (e.g. Spells only)
    let types_rule = add_modifier(
        &plain,
        HUMAN,
        next_turn_cost_rule(
            HUMAN,
            plain.turn,
            CostRule {
                amount: Some(2),
                types: Some(vec![CardType::Spell]),
                ..Default::default()
            },
        ),
    );
    assert_eq!(eval(&types_rule, AI), eval(&plain, AI));

    // 5. Rule with min_cost
    let min_cost_rule = add_modifier(
        &plain,
        HUMAN,
        next_turn_cost_rule(
            HUMAN,
            plain.turn,
            CostRule {
                amount: Some(2),
                min_cost: Some(4),
                ..Default::default()
            },
        ),
    );
    assert_eq!(eval(&min_cost_rule, AI), eval(&plain, AI));

    // 6. Created-only filter
    let created_rule = add_modifier(
        &plain,
        HUMAN,
        next_turn_cost_rule(
            HUMAN,
            plain.turn,
            CostRule {
                amount: Some(2),
                created: Some(true),
                ..Default::default()
            },
        ),
    );
    assert_eq!(eval(&created_rule, AI), eval(&plain, AI));

    // 7. From opponent's deck filter
    let opp_deck_rule = add_modifier(
        &plain,
        HUMAN,
        next_turn_cost_rule(
            HUMAN,
            plain.turn,
            CostRule {
                amount: Some(2),
                from_opponents_deck: Some(true),
                ..Default::default()
            },
        ),
    );
    assert_eq!(eval(&opp_deck_rule, AI), eval(&plain, AI));

    // 8. setTo instead of amount
    let set_to_rule = add_modifier(
        &plain,
        HUMAN,
        next_turn_cost_rule(
            HUMAN,
            plain.turn,
            CostRule {
                set_to: Some(5),
                ..Default::default()
            },
        ),
    );
    assert_eq!(eval(&set_to_rule, AI), eval(&plain, AI));

    // 9. Turn already active and cards played
    let mut active_turn_state = clone(&plain);
    active_turn_state.turn = 2;
    active_turn_state.active = HUMAN;
    active_turn_state.players[HUMAN].turn_log.cards_played = 1;
    let active_rule = add_modifier(
        &active_turn_state,
        HUMAN,
        next_turn_cost_rule(
            HUMAN,
            1,
            CostRule {
                amount: Some(2),
                ..Default::default()
            },
        ),
    );
    assert_eq!(eval(&active_rule, AI), eval(&active_turn_state, AI));

    // 10. Turn already past (state.turn > from_turn + 1)
    let mut past_turn_state = clone(&plain);
    past_turn_state.turn = 3;
    past_turn_state.active = AI;
    let past_rule = add_modifier(
        &past_turn_state,
        HUMAN,
        next_turn_cost_rule(
            HUMAN,
            1,
            CostRule {
                amount: Some(2),
                ..Default::default()
            },
        ),
    );
    assert_eq!(eval(&past_rule, AI), eval(&past_turn_state, AI));
}

#[test]
fn r5122000_greedy_eval_ignores_cost_hike() {
    let plain = board(json!({
        "p1": { "hand": ["core-005"], "library": ["core-008", "core-011"] },
        "p2": { "hand": ["core-005"], "library": ["core-008", "core-011"] },
    }));
    let hiked = add_modifier(
        &plain,
        HUMAN,
        next_turn_cost_rule(
            HUMAN,
            plain.turn,
            CostRule {
                amount: Some(2),
                ..Default::default()
            },
        ),
    );
    let greedy = |state: &GameState| evaluate(state, AI, NextSwing::Enemy, &GREEDY_EVAL);
    assert_eq!(greedy(&hiked), greedy(&plain));
    assert_eq!(coming_mana(&hiked, AI, &GREEDY_EVAL), 0.0);
}

#[test]
fn r5122000_passed_line_casting_cost_injection_preserves_tax_in_replied_score() {
    register_cards();
    let s = scenario(json!({
        "seed": "r5122000-passed-line",
        "active": AI,
        "turn": 9,
        "p1": { "hand": ["infiniteassault-123"], "field": ["core-008"], "mana": 1, "library": ["core-005", "core-008"] },
        "p2": { "hand": ["core-008"], "field": ["core-008"], "mana": 4, "library": ["core-005", "core-011"] },
    }));
    let mut state = s.state().clone();
    let p1_unit_id = active_units_of(&state, AI)[0].id.clone();
    let p1_unit = find_instance_mut(&mut state, &p1_unit_id).expect("p1 unit");
    p1_unit.exertion.attacked = true;
    p1_unit.exertion.switched = true;

    let card_id = s.card("infiniteassault-123").id.clone();
    let play_action = legal_actions(&state, AI)
        .into_iter()
        .find(|a| matches!(a, ActionBody::Play { instance_id, .. } if *instance_id == card_id))
        .expect("legal play action");
    let ended = act(&state, AI, play_action);
    assert_eq!(ended.active, HUMAN, "auto-ended to human after last action");

    let hidden = hidden_card_ids(&ended, AI);
    let after =
        simulate_reply(&ended, AI, &create_node_counter(20, None), &hidden, Prices::NONE).expect("a reply");

    let hike_amount = 1;
    let expected_tax = AI_EVAL.cost_hike * AI_EVAL.next_turn_mana * f64::from(AI_TAXED_PLAYS * hike_amount);
    close(
        coming_mana(&ended, AI, &AI_EVAL) - coming_mana(&after, AI, &AI_EVAL),
        expected_tax,
    );
}
