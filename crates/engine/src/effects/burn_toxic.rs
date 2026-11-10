//! Burn and Toxic as verbs (SPEC §6.3, Stinkyscape+ E2; R3540000): "Burn N an enemy unit" (Fire Beam) and
//! "Toxic N an enemy unit / all enemies" (Scourge Mortar, Toxopid, Call to Chaos) add N to the count
//! already there, through `afflictions::afflict`, which owns the counters, the report and what may carry
//! them. A verb is no damage, so no Armor, Divine Shield or Spell Damage meets it.

use serde::{Deserialize, Serialize};

use crate::afflictions::{Affliction, afflict};
use crate::damage::DamageTarget;
use crate::effects::targets::{BoardScope, TargetSpec, cards_in_scope_aimed, resolve_target, sides_of};
use crate::script::Effect;
use crate::wire::TargetAim;

/// `burn`'s and `toxic`'s argument: `{ to; amount }`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AfflictArgs {
    pub to: TargetSpec,
    pub amount: i32,
}

/// `burnAll`'s and `toxicAll`'s argument: `{ amount; heroes? } & BoardScope`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AfflictAllArgs {
    pub amount: i32,
    /// Also afflict the hero of each scoped side, after every Unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heroes: Option<bool>,
    #[serde(flatten)]
    pub scope: BoardScope,
}

fn afflict_one(args: AfflictArgs, kind: Affliction, name: &'static str) -> Effect {
    Effect::new(name, move |ctx| {
        let Some(target) = resolve_target(ctx, &args.to) else {
            return;
        };
        afflict(ctx, &target, kind, args.amount);
    })
}

fn afflict_all(args: AfflictAllArgs, kind: Affliction, name: &'static str) -> Effect {
    Effect::new(name, move |ctx| {
        // MD-B1, R940: a harmful walk — an immune card in a tribal scope is passed by. The targets are
        // read before the first one is afflicted, as `damageAll` reads its own.
        let units = cards_in_scope_aimed(ctx, &args.scope, TargetAim::Harm);
        for instance in units {
            afflict(ctx, &DamageTarget::Unit { instance }, kind, args.amount);
        }
        if args.heroes != Some(true) {
            return;
        }
        for player in sides_of(ctx, args.scope.side) {
            afflict(ctx, &DamageTarget::Hero { player }, kind, args.amount);
        }
    })
}

/// "Burn N a Unit or hero": adds `amount` Burn to the target (R3540000).
pub fn burn(args: AfflictArgs) -> Effect {
    afflict_one(args, Affliction::Burn, "burn")
}

/// "Toxic N a Unit or hero": adds `amount` Toxic to the target (R3540000).
pub fn toxic(args: AfflictArgs) -> Effect {
    afflict_one(args, Affliction::Toxic, "toxic")
}

/// "Burn N each Unit in a scope", and each scoped side's hero with `heroes` (R3540000).
pub fn burn_all(args: AfflictAllArgs) -> Effect {
    afflict_all(args, Affliction::Burn, "burnAll")
}

/// "Toxic N each Unit in a scope", and each scoped side's hero with `heroes` (R3540000).
pub fn toxic_all(args: AfflictAllArgs) -> Effect {
    afflict_all(args, Affliction::Toxic, "toxicAll")
}
