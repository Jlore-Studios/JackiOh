//! R747000 (SPEC §9.12, R1452): the skill knob. A seat at skill `s` follows the best AI at a decision
//! with chance `s` and plays a random legal action (`random_action`, §10.7's policy) otherwise, so
//! 1.0 decides exactly as the AI always did and 0.0 is the random policy. The roll draws on the
//! caller's own stream, seeded like every other, so a seeded run reproduces; at either end of the
//! range no draw is taken. The knob knows nothing about cards or sets, which is why a weaker seat is
//! this one number and not a model of its own: it keeps meaning the same when the AI improves or a set
//! ships.

use jackioh_engine::Rng;

use crate::config::{AI_SKILL_MAX, AI_SKILL_MIN};

/// R747000: `skill` kept inside `AI_SKILL_MIN..=AI_SKILL_MAX`; a NaN is full skill, the AI as it
/// was before the knob.
pub fn clamp_skill(skill: f64) -> f64 {
    if skill.is_nan() {
        AI_SKILL_MAX
    } else {
        skill.clamp(AI_SKILL_MIN, AI_SKILL_MAX)
    }
}

/// R747000: whether this decision follows the AI (true) or is a random legal action (false), by one
/// roll of `rng` at `skill` (clamped, `clamp_skill`). Full skill is true and zero skill false, each
/// with no draw, so a seat at either end takes exactly the draws it took before the knob.
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
