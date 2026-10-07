//! Random split damage (docs/classic-sets.md B5 E37): "deal N damage split among enemies".
//!
//! Port of `packages/engine/src/effects/split.ts`.

use std::borrow::Borrow;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::damage::{DamageArgs, DamageTarget, deal_damage};
use crate::effects::targets::{BoardScope, cards_in_scope};
use crate::layers::unit_view;
use crate::prelude::json_as;
use crate::script::Effect;
use crate::state::{CardInstance, GameState};
use crate::wire::opponent_of;

/// "Still standing": above 0 health and not marked destroyed, so the check will not collect it (§4.5).
fn standing(state: &GameState, unit: &CardInstance) -> bool {
    unit_view(state, unit).health > 0 && unit.marked_destroyed != Some(true)
}

/// `damageSplit`'s `among`: every enemy (the hero included), or the enemy Units only.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum SplitAmong {
    Enemies,
    EnemyUnits,
}

/// `damageSplit`'s arguments: `among` defaults to "enemies", `perHit` to 1.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DamageSplitArgs {
    pub amount: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub among: Option<SplitAmong>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_hit: Option<i32>,
}

/// B5 E37: `amount` damage dealt as hits of `per_hit` (1 unless the card says otherwise — Classic+ #3's
/// "1 damage for each Plague Counter", which an Upgrade may make 2), each hit a damage instance of its
/// own (§4.4) on an enemy drawn from the match rng among the ones still standing as that hit is dealt:
/// the enemy hero ("an enemy"; left out for "an enemy Unit") and every enemy unit acting on the field
/// still standing — above 0 health and not marked destroyed, since the state check does not run
/// between the hits of one effect (R59). A last hit smaller than `per_hit` takes what is left. A Spell's split passes
/// a unit immune to Spells by, as every scope does (`effects/targets.rs`), and each hit is raised by
/// Spell Damage like any other hit of a Spell (E6).
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
            let units: Vec<CardInstance> = cards_in_scope(ctx, &scope).into_iter().collect();
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
                &DamageArgs {
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
