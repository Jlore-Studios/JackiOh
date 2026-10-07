//! Damage in rounds until a Unit dies (SPEC §8.7 C+ #32.3 Blade Storm, §4.5, R59, R283).
//!
//! R59 checks state after a whole effect list; Blade Storm's is one of the two lists that check inside
//! themselves (R283's is the other): each round is its own list, a sweep and then the state check
//! (`after_state_check`), so the round's deaths and their Death hooks resolve before the next round is
//! built. The storm stops after a round in which a Unit died (a Reborn death counts; R55's destroyed
//! count says so), after `rounds` rounds, or when no Unit in scope is left.
//!
//! Each round is a part (`lazy_part`) holding the next, built when the list reaches it. A Death hook
//! that asks parks the rest on `state.work` (R113); the part's memo is the destroyed count it began
//! with, so a resumed part rebuilds exactly as it ran and the round after it decides afresh.
//!
//! Port of `packages/engine/src/effects/rounds.ts`.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::effects::after_check::after_state_check;
use crate::effects::cast::cast_new;
use crate::effects::damage::damage_all;
use crate::effects::targets::{BoardScope, cards_in_scope};
use crate::prelude::json_as;
use crate::resolve::lazy_part;
use crate::script::{Effect, EffectPart, Memo};

/// `BoardScope["side"]`: "any" | "self" | "enemy", relative to the running card's controller. Its own
/// type so this file names none of `BoardScope`'s; it reaches a scope through its JSON, which is the
/// same literal.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StormSide {
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "self")]
    SelfSide,
    #[serde(rename = "enemy")]
    Enemy,
}

/// `damageRoundsUntilDeath`'s arguments (TS `type Storm`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Storm {
    pub amount: i32,
    pub rounds: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<StormSide>,
}

/// TS `typeof memo === "number"`: the destroyed count a part began with, when one was recorded.
fn memo_count(memo: &Memo) -> Option<i32> {
    let value: &Value = memo.as_ref()?;
    if let Some(n) = value.as_i64() {
        return Some(n as i32);
    }
    value.as_f64().map(|n| n as i32)
}

/// The scope a storm's side names (`{ side }`, every other key at its default).
fn scope_of(side: Option<StormSide>) -> BoardScope {
    match side {
        Some(side) => json_as(json!({ "side": side })),
        None => json_as(json!({})),
    }
}

fn round(storm: Storm, at: i32, deaths_before: Option<i32>) -> Effect {
    lazy_part("damageRound", move |ctx, memo| {
        let deaths = ctx.state.counters.destroyed;
        let recorded = memo_count(memo);
        if recorded.is_none() {
            let died = deaths_before.is_some_and(|before| deaths > before);
            if died || at >= storm.rounds || cards_in_scope(ctx, &scope_of(storm.side)).is_empty() {
                return EffectPart {
                    effects: Vec::new(),
                    memo: None,
                };
            }
        }
        let start = recorded.unwrap_or(deaths);
        let mut sweep = json!({ "amount": storm.amount });
        if let Some(side) = storm.side {
            sweep["side"] = json!(side);
        }
        EffectPart {
            effects: vec![
                damage_all(json_as(sweep)),
                after_state_check(|_ctx| Vec::new()),
                round(storm.clone(), at + 1, Some(start)),
            ],
            memo: Some(json!(start)),
        }
    })
}

/// `amount` to every Unit in scope, round after round, each round with its own state check (R59).
pub fn damage_rounds_until_death(args: Storm) -> Effect {
    round(args, 0, None)
}

// ---------------------------------------------------------------------------
// Casts in rounds until a Unit dies (SPEC §8.7 C+ #32.3 Blade Storm's base face, R652)
// ---------------------------------------------------------------------------

/// `castRoundsUntilDeath`'s arguments (TS `type CastStorm`): the Spell to cast, and how many rounds.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CastStorm {
    pub def: String,
    pub rounds: i32,
}

fn cast_round(storm: CastStorm, at: i32, deaths_before: Option<i32>) -> Effect {
    lazy_part("castRound", move |ctx, memo| {
        let deaths = ctx.state.counters.destroyed;
        let recorded = memo_count(memo);
        if recorded.is_none() {
            let died = deaths_before.is_some_and(|before| deaths > before);
            if died || at >= storm.rounds || cards_in_scope(ctx, &scope_of(Some(StormSide::Any))).is_empty() {
                return EffectPart {
                    effects: Vec::new(),
                    memo: None,
                };
            }
        }
        let start = recorded.unwrap_or(deaths);
        EffectPart {
            effects: vec![
                cast_new(json_as(json!({ "def": storm.def }))),
                after_state_check(|_ctx| Vec::new()),
                cast_round(storm.clone(), at + 1, Some(start)),
            ],
            memo: Some(json!(start)),
        }
    })
}

/// R652: cast `def` round after round — each round a real Spell cast (R70) with its own state check
/// (R59) — until a round in which a Unit died, `rounds` rounds, or no Unit is left.
pub fn cast_rounds_until_death(args: CastStorm) -> Effect {
    cast_round(args, 0, None)
}
