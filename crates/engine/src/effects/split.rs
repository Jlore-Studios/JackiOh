//! Random split damage (docs/classic-sets.md B5 E37): "deal N damage split among enemies".

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::damage::{DamageArgs, DamageTarget, deal_damage};
use crate::effects::targets::{BoardScope, cards_in_scope_aimed};
use crate::layers::unit_view;
use crate::prelude::json_as;
use crate::script::Effect;
use crate::state::{CardInstance, GameState};
use crate::wire::opponent_of;

/// "Still standing": above 0 health and neither marked destroyed nor marked exiled, so the
/// check will not collect it (§4.5). A unit marked for exile (MD-D31, R1124) falls with the
/// destroyed, though no death collects it.
fn standing(state: &GameState, unit: &CardInstance) -> bool {
    unit_view(state, unit).health > 0
        && unit.marked_destroyed != Some(true)
        && unit.marked_exiled != Some(true)
}

/// `damageSplit`'s `among`: every enemy (the hero included), or the enemy Units only.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum SplitAmong {
    Enemies,
    EnemyUnits,
}

/// `damageSplit`'s arguments: `among` defaults to "enemies", `perHit` to 1.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DamageSplitArgs {
    pub amount: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub among: Option<SplitAmong>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_hit: Option<i32>,
}

/// ST-S03-5 (R1860004's Spacemine, issue #1860): `damageRandomEnemies`'s arguments: `amount`
/// damage as one §4.4 instance each to `count` different random enemies.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DamageRandomEnemiesArgs {
    pub count: i32,
    pub amount: i32,
}

/// ST-S03-5 (R1860004): `amount` damage as one §4.4 instance each to `count` different random enemies
/// still `standing` (the hero or a Unit), fewer when fewer stand. Unlike `damage_split`, which re-draws
/// per hit, each enemy here takes at most one hit.
pub fn damage_random_enemies(args: DamageRandomEnemiesArgs) -> Effect {
    Effect::new("damageRandomEnemies", move |ctx| {
        let count = args.count.max(0) as usize;
        let amount = args.amount.max(0);
        if count == 0 || amount == 0 {
            return;
        }
        if ctx.state.result.is_some() {
            return;
        }
        let enemy = opponent_of(ctx.controller);
        let scope: BoardScope = json_as(json!({ "side": "enemy" }));
        let units: Vec<CardInstance> = cards_in_scope_aimed(ctx, &scope, crate::wire::TargetAim::Harm)
            .into_iter()
            .collect();
        let mut pool: Vec<DamageTarget> = Vec::new();
        for instance in units {
            if standing(ctx.state, &instance) {
                pool.push(DamageTarget::Unit { instance });
            }
        }
        pool.push(DamageTarget::Hero { player: enemy });
        let order = ctx.sink.rng.shuffle(&pool);
        let source = ctx.live_self().cloned();
        for target in order.into_iter().take(count) {
            if ctx.state.result.is_some() {
                return;
            }
            deal_damage(
                ctx,
                DamageArgs {
                    source: source.clone(),
                    target,
                    amount,
                    flags: None,
                },
            );
        }
    })
}

/// B5 E37: `amount` damage dealt as hits of `per_hit` (an Upgrade may make Classic+ #3's 2), each its
/// own §4.4 instance on an enemy drawn from the match rng among those still `standing` as that hit is
/// dealt, since the state check does not run between one effect's hits (R59); the hero is left out for
/// "an enemy Unit". A Spell's hits pass Spell-immune units by and take Spell Damage (E6).
pub fn damage_split(args: DamageSplitArgs) -> Effect {
    Effect::new("damageSplit", move |ctx| {
        let per_hit = args.per_hit.unwrap_or(1).max(1);
        let enemy = opponent_of(ctx.controller);
        let scope: BoardScope = json_as(json!({ "side": "enemy" }));
        let mut left = args.amount.max(0);
        while left > 0 {
            if ctx.state.result.is_some() {
                return;
            }
            // MD-B1, R940: a harmful walk — an immune card in a tribal scope is passed by.
            let units: Vec<CardInstance> = cards_in_scope_aimed(ctx, &scope, crate::wire::TargetAim::Harm)
                .into_iter()
                .collect();
            let mut pool: Vec<DamageTarget> = Vec::new();
            for instance in units {
                if standing(ctx.state, &instance) {
                    pool.push(DamageTarget::Unit { instance });
                }
            }
            if args.among != Some(SplitAmong::EnemyUnits) {
                pool.push(DamageTarget::Hero { player: enemy });
            }
            let Some(target) = ctx.sink.rng.pick(&pool).cloned() else {
                return;
            };
            let hit = per_hit.min(left);
            let source = ctx.live_self().cloned();
            deal_damage(
                ctx,
                DamageArgs {
                    source,
                    target,
                    amount: hit,
                    flags: None,
                },
            );
            left -= hit;
        }
    })
}
