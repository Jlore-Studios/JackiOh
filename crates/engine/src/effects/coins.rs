//! Coin flips that buff a unit or grant it a keyword (SPEC §8.1 #4 Gary the Gambler, §10.4
//! layer 4, §10.7, R32, R130, R1440).
//!
//! "Cry: flip 5 coins; +1 attack per heads, +1 max health per tails. Then flip a coin: heads
//! gains Divine Shield, tails gains Rush", radiant "7 coins; +2 per heads, +2 per tails" plus the
//! same rider verbatim. #4's Engine cell read "5 or 7 seeded rolls; permanent buff layer; then
//! one seeded coin granting Divine Shield on heads, Rush on tails; Lucky has no defined 'best'
//! here so does not apply". The first three clauses are decisions this file keeps; R1440 has
//! since given every coin flip a best, heads, which replaces the fourth:
//!
//!   * SEEDED ROLLS. Every flip is `ctx.rng.lucky_coin(lucky)`, the only source of randomness §10.7
//!     allows. OS randomness is out of reach of this crate (CLAUDE.md rule 4, SURFACE §3), and a card
//!     file may not roll for itself, which is the whole reason these verbs exist.
//!   * PERMANENT BUFF LAYER. The totals are applied through `buff` from `super::buff`, and the rider
//!     keyword through `grant_keyword` from the same module, so there is ONE implementation of
//!     §10.4's layer 4: nothing here writes `card.buffs` or `card.granted_keywords`, and the unit's
//!     totals stay something `unit_view` computes on every read rather than something stored.
//!   * LUCKY KEEPS HEADS. §6.1's Lucky X is "repeat a luck-based roll X extra times, keep the best",
//!     and R1440 makes a coin flip luck-based with heads its better side, though Gary pays on both
//!     faces (revising R32/R130). Each coin is its own roll: with the flipping card's Lucky X
//!     (`query::lucky_on`, printed plus given, R1438) it flips X more coins and lands heads if any
//!     does. With no Lucky every flip is the one draw it always was, so no shipped game changes.
//!   * ONE ROLL FOR THE RIDER. `flip_coin_keyword` is one coin, rolled the same way: one seeded draw
//!     with no Lucky, 1 + X with Lucky X.
//!
//! THE FIZZLE IS TOTAL, AND THAT IS A DETERMINISM RULE, NOT TIDINESS. §10.7 stores `rngCursor` in
//! state, so the cursor is part of the match: every later draw in the game depends on how many were
//! taken before it. If a flip happened only when a target was there, the whole rest of the match's
//! randomness would depend on the board at this moment — two replays of the same action list could
//! diverge, and §9.3 requires they never do. So the target is resolved FIRST and a missing one takes
//! no draws at all, leaving the cursor exactly where it was.
//!
//! Port of `packages/engine/src/effects/coins.ts`.

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::buff::{BuffAmount, buff, grant_keyword};
use super::targets::{TargetSpec, instance_of};
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext};
use crate::wire::Keyword;

/// R1438, R1440: the Lucky of the card flipping (the card running the script), which every one of its
/// flips takes; 0 when no card is running it.
fn flipper_lucky(ctx: &EffectContext<'_>) -> i32 {
    ctx.live_self()
        .map_or(0, |card| crate::query::lucky_on(&*ctx.sink.state, card))
}

/// TS `key: "attack" | "health"`: which stat `total_for` sums.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stat {
    Attack,
    Health,
}

/// Heads × per-heads plus tails × per-tails, in one stat.
fn total_for(heads: i32, tails: i32, per_heads: &BuffAmount, per_tails: &BuffAmount, key: Stat) -> i32 {
    let of = |amount: &BuffAmount| match key {
        Stat::Attack => amount.attack.unwrap_or(0),
        Stat::Health => amount.health.unwrap_or(0),
    };
    heads * of(per_heads) + tails * of(per_tails)
}

/// `flip_coins`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlipCoinsArgs {
    pub target: TargetSpec,
    pub coins: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_heads: Option<BuffAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_tails: Option<BuffAmount>,
}

/// §8.1 #4: flip `coins` coins and buff the target by `perHeads` per heads and `perTails` per tails.
/// Each coin is a `lucky_coin` at the flipping card's Lucky (R1440).
///
/// The two totals are applied as ONE layer-4 buff, so heads + tails always accounts for every flip
/// and there is a single `buffed` event carrying the whole result — which is what §10.10 animates
/// and what a test can read back. A buff that works out to +0/+0 (all five coins landing on a side
/// the card pays nothing for) is silently nothing, like every other no-op buff.
pub fn flip_coins(args: FlipCoinsArgs) -> Effect {
    Effect::new("flipCoins", move |ctx| {
        // Resolved before a single draw: see the determinism note above.
        let Some(unit) = instance_of(ctx, &args.target) else {
            return;
        };

        let lucky = flipper_lucky(ctx);
        let coins = args.coins.max(0);
        let mut heads = 0;
        for _flip in 0..coins {
            if ctx.rng.lucky_coin(lucky) {
                heads += 1;
            }
        }
        let tails = coins - heads;

        let nothing = BuffAmount::default();
        let per_heads = args.per_heads.as_ref().unwrap_or(&nothing);
        let per_tails = args.per_tails.as_ref().unwrap_or(&nothing);
        // TS `buff({ target: { of: "instance", instanceId }, attack, health })`, written as that literal
        // so it reads the buff's own argument shape.
        let effect = buff(json_as(json!({
            "target": { "of": "instance", "instanceId": unit.id },
            "attack": total_for(heads, tails, per_heads, per_tails, Stat::Attack),
            "health": total_for(heads, tails, per_heads, per_tails, Stat::Health),
        })));
        (effect.apply)(ctx);
    })
}

/// `flip_coin_keyword`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlipCoinKeywordArgs {
    pub target: TargetSpec,
    pub heads_keyword: Keyword,
    pub tails_keyword: Keyword,
}

/// §8.1 #4: flip one coin and grant the target a keyword — `headsKeyword` on heads,
/// `tailsKeyword` on tails (§10.4 granted keywords, §10.7, R32/R130).
///
/// Gary the Gambler's rider after its stat flips: heads gains Divine Shield, tails gains Rush.
/// One `ctx.rng.lucky_coin` at the flipping card's Lucky (R1440: one seeded draw with none, 1 + X
/// with Lucky X, heads kept), resolved against the target FIRST so a missing target takes zero
/// draws, like `flip_coins`.
pub fn flip_coin_keyword(args: FlipCoinKeywordArgs) -> Effect {
    Effect::new("flipCoinKeyword", move |ctx| {
        // Resolved before the single draw: see the determinism note above.
        let Some(unit) = instance_of(ctx, &args.target) else {
            return;
        };

        let lucky = flipper_lucky(ctx);
        let heads = ctx.rng.lucky_coin(lucky);
        let keyword = if heads {
            args.heads_keyword.clone()
        } else {
            args.tails_keyword.clone()
        };
        let effect = grant_keyword(json_as(json!({
            "target": { "of": "instance", "instanceId": unit.id },
            "keyword": keyword,
        })));
        (effect.apply)(ctx);
    })
}
