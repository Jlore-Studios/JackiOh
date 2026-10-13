//! The skill mixture (R747000): the share of decisions that follow the best AI, versus a
//! random action. 100% plays exactly what the AI recommends; 0% is fully random.
//!
//! The roll draws on the caller's seeded stream, so seeded runs reproduce, and it knows nothing
//! about cards or sets, so it keeps working when the best AI is updated or a new set ships. At
//! exactly 1.0 it draws nothing, so the reference play is bit-for-bit what it was before skill.

use jackioh_engine::{ActionBody, GameState, PlayerId, Rng};

use crate::baselines::random_action;

/// A skill into [0, 1]. NaN behaves as 0.
pub fn clamp_skill(skill: f64) -> f64 {
    if skill.is_nan() {
        return 0.0;
    }
    skill.clamp(0.0, 1.0)
}

/// Whether this decision follows the best AI: true with chance `skill`. Draws one rng draw, except
/// at exactly 0.0 and 1.0, which draw nothing.
pub fn follows_best(skill: f64, rng: &mut Rng) -> bool {
    let skill = clamp_skill(skill);
    if skill >= 1.0 {
        return true;
    }
    if skill <= 0.0 {
        return false;
    }
    rng.chance(skill)
}

/// The skill-degraded action: the AI's answer when the roll passes, else SPEC §10.7's random
/// policy on the same stream. `None` only when the seat has no legal action at all.
pub fn degraded_action(
    best: Option<ActionBody>,
    state: &GameState,
    seat: PlayerId,
    skill: f64,
    rng: &mut Rng,
) -> Option<ActionBody> {
    if follows_best(skill, rng) {
        return best;
    }
    random_action(state, seat, rng)
}

/// Measured Elo below the reference (skill 1.0) per skill: PROVISIONAL, hand-set until R747000's
/// arena ladder measures the curve. Read by ascending skill; `skill_elo_offset` interpolates.
pub const SKILL_ELO_OFFSETS: &[(f64, f64)] = &[
    (0.0, 720.0),
    (0.25, 500.0),
    (0.5, 320.0),
    (0.75, 150.0),
    (0.9, 60.0),
    (1.0, 0.0),
];

/// The measured Elo a skill plays below the reference: piecewise-linear over
/// `SKILL_ELO_OFFSETS`, clamped past the ends.
pub fn skill_elo_offset(skill: f64) -> f64 {
    let skill = clamp_skill(skill);
    let mut prev: Option<(f64, f64)> = None;
    for &(at, off) in SKILL_ELO_OFFSETS {
        if skill <= at {
            return match prev {
                None => off,
                Some((lo_skill, lo_off)) => {
                    let span = at - lo_skill;
                    let into = if span <= 0.0 {
                        0.0
                    } else {
                        (skill - lo_skill) / span
                    };
                    lo_off + into * (off - lo_off)
                }
            };
        }
        prev = Some((at, off));
    }
    prev.map_or(0.0, |(_, off)| off)
}

/// The expected Elo of a skill against a reference rated `reference_elo`.
pub fn skill_expected_elo(skill: f64, reference_elo: f64) -> f64 {
    reference_elo - skill_elo_offset(skill)
}

/// A score's Elo delta over its opponent: draws count half in the caller's score. The score is
/// clamped away from 0 and 1, where the logistic has no finite answer; a non-finite score is 0.
pub fn elo_delta_from_score(score: f64) -> f64 {
    if !score.is_finite() {
        return 0.0;
    }
    let score = score.clamp(1e-6, 1.0 - 1e-6);
    400.0 * (score / (1.0 - score)).log10()
}
