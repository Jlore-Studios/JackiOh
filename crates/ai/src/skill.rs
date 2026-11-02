//! R747000 (SPEC §9.12, R1452): the skill knob. A seat at skill `s` follows the best AI at a decision
//! with chance `s` and plays the skill miss (`skill_miss_action`, R2214000) otherwise, so 1.0 decides
//! exactly as the AI always did and 0.0 is the miss at every decision. The roll draws on the caller's
//! own stream, seeded like every other, so a seeded run reproduces. The knob knows nothing about cards
//! or sets, which is why a weaker seat is this one number and not a model of its own: it keeps meaning
//! the same when the AI improves or a set ships.

use jackioh_engine::{ActionBody, GameState, PlayerId, Rng, mulligan_prompt_for};

use crate::candidates::candidate_set;
use crate::config::{
    AI_SEARCH, AI_SKILL_MAX, AI_SKILL_MIN, AI_SKILL_MISS_TEMPERATURE, GREEDY_EVAL, GREEDY_MULLIGAN,
};
use crate::decide::pick_finalist;
use crate::determinize::{DeterminizeOptions, determinize};
use crate::evaluate::{NextSwing, evaluate};
use crate::mulligan::mulligan_keep;
use crate::observe::{ai_to_act, redact, unanswered_draw_offer};
use crate::simulate::{create_node_counter, simulate_first_answers};

/// R747000: `skill` kept inside `AI_SKILL_MIN..=AI_SKILL_MAX`; a NaN is full skill.
pub fn clamp_skill(skill: f64) -> f64 {
    if skill.is_nan() {
        AI_SKILL_MAX
    } else {
        skill.clamp(AI_SKILL_MIN, AI_SKILL_MAX)
    }
}

/// R747000: whether this decision follows the AI (true) or is the skill miss (false, R2214000), by one
/// roll of `rng` at `skill` (clamped); either end of the range takes no draw.
pub fn plays_best(skill: f64, rng: &mut Rng) -> bool {
    let skill = clamp_skill(skill);
    if skill >= AI_SKILL_MAX {
        return true;
    }
    if skill <= AI_SKILL_MIN {
        return false;
    }
    rng.chance(skill)
}

/// R2214000: a decision the skill's roll does not give to the AI: the true state's candidates (each
/// legal), read one ply ahead on one determinization (`one_ply_scores`) and drawn by a softmax at
/// `AI_SKILL_MISS_TEMPERATURE`. A draw offer, a mulligan (the greedy baseline's keep) and a lone
/// candidate take no draw.
pub fn skill_miss_action(state: &GameState, seat: PlayerId, rng: &mut Rng) -> Option<ActionBody> {
    if !ai_to_act(state, seat) {
        return None;
    }
    let public = redact(state, seat);
    if unanswered_draw_offer(&public, seat) {
        return Some(ActionBody::AnswerDraw { accept: false });
    }
    if public.pending.is_none() && mulligan_prompt_for(&public, seat).is_some() {
        return Some(ActionBody::Mulligan {
            keep: mulligan_keep(&public, seat, GREEDY_MULLIGAN.keep_max_cost),
        });
    }
    let candidates = candidate_set(state, seat);
    if candidates.len() < 2 {
        return candidates.into_iter().next();
    }
    let det = determinize(
        &public,
        seat,
        rng,
        DeterminizeOptions {
            match_shown_cost: Some(false),
        },
    );
    let scores = one_ply_scores(&det, seat, &candidates);
    candidates
        .get(pick_finalist(&scores, AI_SKILL_MISS_TEMPERATURE, rng))
        .cloned()
}

/// R2214000: each candidate's greedy-baseline score on `det` one ply ahead (GREEDY_EVAL after
/// `simulate_first_answers`); endTurn and an unsimulated step score as standing still.
pub fn one_ply_scores(det: &GameState, seat: PlayerId, candidates: &[ActionBody]) -> Vec<f64> {
    let standing = evaluate(det, seat, NextSwing::Enemy, &GREEDY_EVAL);
    let counter = create_node_counter(candidates.len() * (1 + AI_SEARCH.max_auto_answers as usize), None);
    candidates
        .iter()
        .map(|action| {
            if matches!(action, ActionBody::EndTurn) {
                return standing;
            }
            match simulate_first_answers(det, seat, action, &counter) {
                Some(Ok(next)) => evaluate(&next, seat, NextSwing::Enemy, &GREEDY_EVAL),
                _ => standing,
            }
        })
        .collect()
}
