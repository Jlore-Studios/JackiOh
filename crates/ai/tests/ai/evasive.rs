//! #4761: `AI_EVAL.keyword.evasive` prices Evasive as every other keyword is priced. SPEC §6: no attack
//! may target an Evasive unit (R3543002), nor a Spell or Field Spell while it is in play, its controller's
//! own included (R4353002). `unit_worth` adds the weight for a unit that has it through the layers,
//! printed or granted, so the evaluation counts an own Evasive unit up and an enemy one down, and the move
//! order stops trying an Evasive attacker's even trade ahead of the plays (#1779), since `trade_value`
//! prices its death at its whole worth. `GREEDY_EVAL` gives it nothing, so the §9.9 gates keep their
//! yardstick.
//!
//! The cards: Mr. Vanilla 4/4 and Classic+ #51 Jlockeed's J15 Fighter 7/2 Ranged, Flying, Rush, Evasive.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, close, register_cards, unit};

const VANILLA: &str = "core-008";
const J15: &str = "classicplus-051";

/// The weight at #4761's start, whatever AI_EVAL holds (its screen decides that), so these tests pin the
/// term, not its tuning.
const PRICED: EvalWeights = EvalWeights {
    keyword: KeywordWeights {
        evasive: 1.0,
        ..AI_EVAL.keyword
    },
    ..AI_EVAL
};

/// The same weights with the term off.
const UNPRICED: EvalWeights = EvalWeights {
    keyword: KeywordWeights {
        evasive: 0.0,
        ..AI_EVAL.keyword
    },
    ..AI_EVAL
};

fn board(setup: Value) -> GameState {
    register_cards();
    scenario(setup).state().clone()
}

/// A Mr. Vanilla on each side.
fn even() -> GameState {
    board(json!({ "p1": { "field": [VANILLA] }, "p2": { "field": [VANILLA] } }))
}

/// `state` with `player`'s Mr. Vanilla on the field granted Evasive.
fn evasive(state: &GameState, player: PlayerId) -> GameState {
    let mut out = state.clone();
    let id = unit(&out, player, VANILLA).id.clone();
    find_instance_mut(&mut out, &id)
        .expect("the unit")
        .granted_keywords
        .push(Keyword::Evasive);
    out
}

fn worth(state: &GameState, player: PlayerId, def_id: &str, w: &EvalWeights) -> f64 {
    unit_worth(state, unit(state, player, def_id), w, Prices::NONE)
}

fn scored(state: &GameState, w: &EvalWeights) -> f64 {
    evaluate(state, AI, NextSwing::Enemy, w)
}

mod weight {
    use super::*;

    #[test]
    fn the_table_names_evasive_and_the_frozen_baseline_gives_it_nothing() {
        assert_eq!(
            AI_EVAL.keyword.of(KeywordKind::Evasive),
            Some(AI_EVAL.keyword.evasive)
        );
        assert_eq!(PRICED.keyword.of(KeywordKind::Evasive), Some(1.0));
        assert_eq!(GREEDY_EVAL.keyword.of(KeywordKind::Evasive), Some(0.0));
    }

    #[test]
    fn a_granted_evasive_unit_is_worth_the_weight_more_than_the_plain_body() {
        let plain = even();
        let own = evasive(&plain, AI);
        close(
            worth(&own, AI, VANILLA, &PRICED) - worth(&plain, AI, VANILLA, &PRICED),
            PRICED.keyword.evasive,
        );
        close(
            worth(&own, AI, VANILLA, &UNPRICED),
            worth(&plain, AI, VANILLA, &UNPRICED),
        );
    }

    #[test]
    fn a_printed_evasive_unit_counts_the_weight() {
        let state = board(json!({ "p1": { "field": [J15] } }));
        close(
            worth(&state, AI, J15, &PRICED) - worth(&state, AI, J15, &UNPRICED),
            PRICED.keyword.evasive,
        );
    }

    #[test]
    fn the_evaluation_counts_an_own_evasive_unit_up_and_an_enemy_one_down_by_the_weight() {
        let state = even();
        let own = evasive(&state, AI);
        let enemy = evasive(&state, HUMAN);
        close(
            scored(&own, &PRICED) - scored(&own, &UNPRICED),
            PRICED.keyword.evasive,
        );
        close(
            scored(&enemy, &UNPRICED) - scored(&enemy, &PRICED),
            PRICED.keyword.evasive,
        );
    }

    #[test]
    fn greedy_eval_prices_no_evasive() {
        let state = even();
        let own = evasive(&state, AI);
        assert_eq!(
            worth(&own, AI, VANILLA, &GREEDY_EVAL),
            worth(&state, AI, VANILLA, &GREEDY_EVAL)
        );
    }
}

/// Mr. Vanilla for Mr. Vanilla beside a Mr. Vanilla in hand: an even trade, which the move order tries
/// ahead of the plays (#1779) until the AI's attacker is Evasive, whose death then costs the AI the
/// keyword's weight too. These read AI_EVAL, as `candidate_actions` does.
mod move_order {
    use super::*;

    fn mirror(attacker_evasive: bool) -> (GameState, ActionBody) {
        let state = board(json!({
            "seed": "ai-evasive-mirror",
            "p1": { "hand": [VANILLA], "field": [VANILLA] },
            "p2": { "field": [VANILLA] },
        }));
        let state = if attacker_evasive {
            evasive(&state, AI)
        } else {
            state
        };
        let trade = ActionBody::Attack {
            attacker_id: unit(&state, AI, VANILLA).id.clone(),
            target_id: unit(&state, HUMAN, VANILLA).id.clone(),
        };
        (state, trade)
    }

    /// Where `candidate_actions` puts `trade` and the first play.
    fn places(state: &GameState, trade: &ActionBody) -> (usize, usize) {
        let actions = candidate_actions(state, AI, Prices::NONE);
        let attack = actions
            .iter()
            .position(|action| action == trade)
            .unwrap_or_else(|| panic!("{} is a candidate", action_key(trade)));
        let play = actions
            .iter()
            .position(|action| matches!(action, ActionBody::Play { .. }))
            .expect("the hand card's play is a candidate");
        (attack, play)
    }

    #[test]
    fn an_evasive_attackers_even_trade_waits_behind_the_plays() {
        let (state, trade) = mirror(true);
        let (attack, play) = places(&state, &trade);
        assert!(attack > play, "the trade at {attack}, the first play at {play}");
    }

    #[test]
    fn the_same_trade_by_a_plain_attacker_goes_ahead_of_the_plays() {
        let (state, trade) = mirror(false);
        let (attack, play) = places(&state, &trade);
        assert!(attack < play, "the trade at {attack}, the first play at {play}");
    }
}
