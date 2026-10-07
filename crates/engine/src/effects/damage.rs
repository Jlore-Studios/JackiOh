//! Deal damage: one instance through the §4.4 pipeline, for one target or for a whole scope.
//!
//! Port of `packages/engine/src/effects/damage.ts`. The source of every hit is the card running the
//! script as its context was built (`ctx.self_`, TS `ctx.self`).

use serde::{Deserialize, Serialize};

use crate::damage::{DamageArgs, DamageFlags, DamageTarget, deal_damage};
use crate::effects::targets::{BoardScope, TargetSpec, cards_in_scope, resolve_target, sides_of};
use crate::script::Effect;

/// The §4.4 modifiers an effect may put on its own damage; shared by `damage` and `damageAll`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DamageFlagArgs {
    /// True Strike ignores Armor (§4.4 step 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_armor: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combat: Option<bool>,
    /// R85: this damage has Lifesteal of its own, without the source gaining the keyword.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifesteal: Option<bool>,
    /// B5 E6: this damage has Trample of its own (Classic #83 Flame Lance, "Trample. Deal 11 damage to a
    /// Unit"): the excess over the target Unit's health hits its controller's hero as a new instance. A
    /// Spell's printed Trample is read off it while it resolves too; stating it keeps it on a repeat.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trample: Option<bool>,
}

/// `damage`'s argument: `{ to; amount } & DamageFlagArgs`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DamageEffectArgs {
    pub to: TargetSpec,
    pub amount: i32,
    #[serde(flatten)]
    pub flags: DamageFlagArgs,
}

/// One spelling of the flag block, so `damage` and `damageAll` cannot drift apart: a sweep must put
/// exactly the same instance through §4.4 as a single-target hit does.
fn damage_flags(args: &DamageFlagArgs) -> DamageFlags {
    DamageFlags {
        ignore_armor: Some(args.ignore_armor == Some(true)),
        combat: Some(args.combat == Some(true)),
        lifesteal: Some(args.lifesteal == Some(true)),
        trample: if args.trample == Some(true) { Some(true) } else { None },
    }
}

pub fn damage(args: DamageEffectArgs) -> Effect {
    Effect::new("damage", move |ctx| {
        let Some(target) = resolve_target(ctx, &args.to) else {
            return;
        };
        let source = ctx.self_.clone();
        deal_damage(
            ctx,
            DamageArgs {
                source,
                target,
                amount: args.amount,
                flags: Some(damage_flags(&args.flags)),
            },
        );
    })
}

/// `damageAll`'s argument: `{ amount; heroes? } & DamageFlagArgs & BoardScope`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DamageAllArgs {
    pub amount: i32,
    /// Also hit the hero of each scoped side, after every unit (#13's "and the enemy hero").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heroes: Option<bool>,
    #[serde(flatten)]
    pub flags: DamageFlagArgs,
    #[serde(flatten)]
    pub scope: BoardScope,
}

/// §6.3 Damage over a scope: one §4.4 damage instance per target, in `cardsInScope` order, then the
/// hero of each scoped side in the same side order when `heroes` is set (#13 Jlockeed Shredder-10,
/// "End of turn: deal 2 damage to each enemy unit and the enemy hero", 5 on the radiant face).
///
/// The target list is snapshotted before the first hit. SPEC says one instance per target, and §4.5
/// never runs a state check between the hits of one effect (R59), so nothing that happens mid-sweep
/// may change who is hit: a unit dragged to 0 health by the second hit still takes nothing extra,
/// and a unit that was not on the field when the sweep began is not hit at all. Re-reading the row
/// per hit would make the sweep depend on the order deaths were noticed, which is exactly the
/// dependency R59 exists to remove.
///
/// Each hit is an ordinary `dealDamage`, so Divine Shield, Armor, the hero cap, Indestructible,
/// Poisonous, Lifesteal and Trample all behave per target; a target that absorbs its hit fizzles
/// silently and the sweep continues.
pub fn damage_all(args: DamageAllArgs) -> Effect {
    Effect::new("damageAll", move |ctx| {
        let flags = damage_flags(&args.flags);
        let targets = cards_in_scope(ctx, &args.scope);

        for instance in targets {
            let source = ctx.self_.clone();
            deal_damage(
                ctx,
                DamageArgs {
                    source,
                    target: DamageTarget::Unit { instance },
                    amount: args.amount,
                    flags: Some(flags),
                },
            );
        }

        if args.heroes != Some(true) {
            return;
        }
        for player in sides_of(ctx, args.scope.side) {
            let source = ctx.self_.clone();
            deal_damage(
                ctx,
                DamageArgs {
                    source,
                    target: DamageTarget::Hero { player },
                    amount: args.amount,
                    flags: Some(flags),
                },
            );
        }
    })
}
