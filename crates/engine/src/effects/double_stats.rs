//! Doubling a Unit's Attack and Health (SPEC §6.3, R4588005; Infinite Assault #96 Exponential
//! Growth): the gains a `buff` of the doubled stats, capped at `MULTIPLIED_STAT_CAP` (R4079000), read
//! off the Unit's shown stats (R1868001, R1867003).

use serde::{Deserialize, Serialize};

use crate::config::MULTIPLIED_STAT_CAP;
use crate::damage::DamageTarget;
use crate::layers::unit_view;
use crate::script::Effect;

use super::buff::{BuffArgs, buff};
use super::targets::{TargetSpec, resolve_target};

/// `doubleStats`' argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DoubleStatsArgs {
    /// The Unit to double.
    pub target: TargetSpec,
}

/// R4079000: what doubling a shown stat of `stat` adds, never lifting it above
/// `MULTIPLIED_STAT_CAP` and never below what it is.
fn doubling_gain(stat: i32) -> i32 {
    stat.max(0).min(MULTIPLIED_STAT_CAP - stat).max(0)
}

/// R4588005 (Infinite Assault #96 Exponential Growth): double the target Unit's Attack and Health —
/// a `buff` of each stat's doubling gain (R4079000), read off what the Unit shows (R1868001,
/// R1867003); a Unit already at the cap gains nothing.
pub fn double_stats(args: DoubleStatsArgs) -> Effect {
    Effect::new("doubleStats", move |ctx| {
        let Some(DamageTarget::Unit { instance }) = resolve_target(ctx, &args.target) else {
            return;
        };
        let view = unit_view(ctx.state, &instance);
        let (attack, health) = (doubling_gain(view.attack), doubling_gain(view.health));
        if attack == 0 && health == 0 {
            return;
        }
        (buff(BuffArgs {
            target: TargetSpec::Instance {
                instance_id: instance.id.clone(),
            },
            attack: Some(attack),
            health: Some(health),
        })
        .apply)(ctx);
    })
}
