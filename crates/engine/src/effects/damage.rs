//! Deal damage: one instance through the §4.4 pipeline, for one target or for a whole scope.
//!
//! Port of `packages/engine/src/effects/damage.ts`. The source of every hit is the card running the
//! script as its context was built (`ctx.self_`, TS `ctx.self`).

use serde::{Deserialize, Serialize};

use crate::damage::{DamageArgs, DamageFlags, DamageTarget, deal_damage};
// R1848001: where a stated Trample's excess goes, as a card names it.
pub use crate::damage::TrampleOnto;
use crate::effects::targets::{BoardScope, TargetSpec, cards_in_scope_aimed, resolve_target, sides_of};
use crate::rng::Rng;
use crate::script::Effect;
use crate::wire::TargetAim;

/// The §4.4 modifiers an effect may put on its own damage; shared by `damage` and `damageAll`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    /// R1848001 (Stinkyscape #3 Meteor): this damage tramples onto the Units adjacent to its target
    /// (`adjacent`), and on to its controller's hero after them (`adjacentAndHero`), instead of onto
    /// that hero alone (`damage::trample_onto_adjacent`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trample_onto: Option<TrampleOnto>,
    /// R1841001: "deals half damage to heroes": a hit on a hero is halved, rounded down, before Armor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub half_to_heroes: Option<bool>,
}

/// `damage`'s argument: `{ to; amount } & DamageFlagArgs`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
        trample: if args.trample == Some(true) {
            Some(true)
        } else {
            None
        },
        poisonous: None,
        attacking: None,
        trample_onto: args.trample_onto,
        half_to_heroes: if args.half_to_heroes == Some(true) {
            Some(true)
        } else {
            None
        },
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

/// R2521001: one face of a sweep's weighted roll — `amount` comes up `weight` times in the odds' sum
/// (Stinkyscape #53, #57, #61 Flak's "60%, 30%, 10%, lowest first").
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DamageOdds {
    pub amount: i32,
    pub weight: i32,
}

/// `damageAll`'s argument: `{ amount; heroes?; odds?; lucky? } & DamageFlagArgs & BoardScope`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DamageAllArgs {
    pub amount: i32,
    /// Also hit the hero of each scoped side, after every unit (#13's "and the enemy hero").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heroes: Option<bool>,
    /// R2521001: "a random N to M damage to each enemy": `amount` is set aside and each target rolls
    /// its own amount over these odds, just before its hit (`sweep_amount`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub odds: Option<Vec<DamageOdds>>,
    /// R2521001, §6.1 Lucky X: each target's roll is rolled this many times more and the highest
    /// amount kept. The script states the whole X (the card's Lucky and its controller's Luck, R987).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lucky: Option<i32>,
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
///
/// R2521001: with `odds`, each target's amount is its own roll, made just before its hit, in the
/// order the targets are hit (the units, then the heroes), as Hearthstone's Lightning Storm rolls
/// each minion apart. Without `odds` nothing is drawn, so a sweep that names none plays and replays
/// exactly as it always did.
pub fn damage_all(args: DamageAllArgs) -> Effect {
    Effect::new("damageAll", move |ctx| {
        let flags = damage_flags(&args.flags);
        // MD-B1, R940: a harmful walk — an immune card in a tribal scope is passed by.
        let targets = cards_in_scope_aimed(ctx, &args.scope, TargetAim::Harm);

        for instance in targets {
            let source = ctx.self_.clone();
            let amount = sweep_amount(&mut *ctx.sink.rng, &args);
            deal_damage(
                ctx,
                DamageArgs {
                    source,
                    target: DamageTarget::Unit { instance },
                    amount,
                    flags: Some(flags),
                },
            );
        }

        if args.heroes != Some(true) {
            return;
        }
        for player in sides_of(ctx, args.scope.side) {
            let source = ctx.self_.clone();
            let amount = sweep_amount(&mut *ctx.sink.rng, &args);
            deal_damage(
                ctx,
                DamageArgs {
                    source,
                    target: DamageTarget::Hero { player },
                    amount,
                    flags: Some(flags),
                },
            );
        }
    })
}

/// R2521001: what one target of a sweep is dealt — the stated `amount`, or with `odds` one weighted
/// roll of its own, rolled `lucky` more times with the highest kept when `lucky` is above 0 (§6.1
/// Lucky X, `Rng::lucky`). Without `odds` it draws nothing.
fn sweep_amount(rng: &mut Rng, args: &DamageAllArgs) -> i32 {
    let Some(odds) = args.odds.as_deref() else {
        return args.amount;
    };
    let roll = |rng: &mut Rng| roll_odds(rng, odds, args.amount);
    let lucky = args.lucky.unwrap_or(0);
    if lucky > 0 {
        rng.lucky(lucky, roll, Ord::max)
    } else {
        roll(rng)
    }
}

/// One weighted draw over `odds`: `rng.int` of the summed weights, walked in list order, each face
/// taking the next `weight` of it. An empty list (nothing to weigh, so nothing drawn) is `fallback`.
fn roll_odds(rng: &mut Rng, odds: &[DamageOdds], fallback: i32) -> i32 {
    let total: i32 = odds.iter().map(|face| face.weight).sum();
    let mut at = rng.int(total);
    for face in odds {
        if at < face.weight {
            return face.amount;
        }
        at -= face.weight;
    }
    odds.last().map_or(fallback, |face| face.amount)
}
