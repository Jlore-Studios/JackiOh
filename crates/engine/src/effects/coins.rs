//! Coin flips that buff a unit or grant it a keyword (SPEC §8.1 #4 Gary the Gambler, §10.4
//! layer 4, §10.7, R32, R130, R1440).
//!
//! Every flip is `ctx.rng.lucky_coin(lucky)`, the only randomness §10.7 allows (CLAUDE.md rule 4,
//! SURFACE §3): a card file may not roll for itself, which is why these verbs exist. Totals and the
//! rider go through `super::buff`, the one implementation of §10.4's layer 4. §6.1's Lucky X keeps the
//! best of X more rolls, and R1440 (over R32/R130) makes heads a coin's best: at the flipping card's
//! Lucky X (`query::lucky_on`, R1438) a coin flips X more and lands heads if any does; with none, one draw.
//!
//! THE FIZZLE IS TOTAL, a determinism rule: §10.7 stores `rngCursor` in state, so a flip taken only
//! when a target was there would make the rest of the match's draws depend on the board, and §9.3
//! requires two replays of one action list never diverge. The target is resolved FIRST; a missing one
//! takes no draws.

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
    // R987: the controller's Luck flips extra times beside the card's own Lucky.
    ctx.live_self()
        .map_or(0, |card| crate::query::lucky_on(&*ctx.sink.state, card))
        + crate::query::luck_of(&*ctx.sink.state, ctx.controller)
}

/// Which stat `total_for` sums.
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FlipCoinsArgs {
    pub target: TargetSpec,
    pub coins: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_heads: Option<BuffAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_tails: Option<BuffAmount>,
}

/// §8.1 #4: flip `coins` coins and buff the target by `perHeads` per heads and `perTails` per tails, as
/// ONE layer-4 buff, so a single `buffed` event carries the whole result (§10.10); +0/+0 is nothing.
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FlipCoinKeywordArgs {
    pub target: TargetSpec,
    pub heads_keyword: Keyword,
    pub tails_keyword: Keyword,
}

/// §8.1 #4: flip one coin and grant the target `headsKeyword` on heads, `tailsKeyword` on tails (Gary's
/// rider, Divine Shield or Rush): one `lucky_coin` at the flipping card's Lucky (R1440), like `flip_coins`.
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
