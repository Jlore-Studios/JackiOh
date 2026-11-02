//! Set health, heals turned into damage (docs/classic-sets.md B5 E7, E8), and a Unit's Attack and
//! Health set outright (R1185002, M+ #21 Harmony Spirit).

use serde::{Deserialize, Serialize};

use crate::damage::{DamageTarget, set_hero_health};
use crate::effects::targets::{
    BoardScope, PlayerSpec, TargetSpec, cards_in_scope, player_of, resolve_target,
};
use crate::layers::{unit_has, unit_view, worn_stats_override};
use crate::modifiers::add_modifier;
use crate::script::Effect;
use crate::state::{ModifierExpiry, ModifierKind, find_instance, find_instance_mut};
use crate::wire::{AttackHealth, GameEvent, KeywordKind, Tuning};

/// `setHealth`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SetHealthArgs {
    pub to: TargetSpec,
    pub value: i32,
}

/// B5 E7: "Set a hero's health to N" (Classic #29 Book of Vital Kill). Neither damage nor a heal, so no
/// Armor, cap, replacement or answer to either (R18's lose health is the nearest rule).
pub fn set_health(args: SetHealthArgs) -> Effect {
    Effect::new("setHealth", move |ctx| {
        let Some(DamageTarget::Hero { player }) = resolve_target(ctx, &args.to) else {
            return;
        };
        let source_id = ctx.self_.as_ref().map(|card| card.id.clone());
        set_hero_health(ctx, player, args.value, source_id);
    })
}

/// `convertHealing`'s argument.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ConvertHealingArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// B5 E8: this turn, heals on `player`'s enemies deal that much Pierce damage instead, from this card
/// (a this-turn player modifier). Classic+ #22 Blood Moon does it through a replacement instead, which
/// also converts the heal that set it off.
pub fn convert_healing(args: ConvertHealingArgs) -> Effect {
    Effect::new("convertHealing", move |ctx| {
        let Some(converter_id) = ctx.self_.as_ref().map(|card| card.id.clone()) else {
            return;
        };
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let turn = ctx.sink.state.turn;
        add_modifier(
            ctx,
            player,
            ModifierExpiry::ThisTurn { turn },
            ModifierKind::HealToDamage { converter_id },
        );
    })
}

/// `setStatsAll`'s argument: a board scope (its keys beside the stats) and the Attack and Health
/// every Unit in it is set to.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SetStatsAllArgs {
    #[serde(flatten)]
    pub scope: BoardScope,
    pub attack: i32,
    pub health: i32,
}

/// R1185002: Hearthstone's "set": §10.4 layer 1 becomes `stats` (`statsOverride`, R41), its buffs and
/// stat tuning go (R781003) and its damage is healed, so it stands at `stats` under layer 5's auras. A
/// face wearing its X/X doubled (R349) keeps the difference as tuning; R78's reset takes it all off.
fn set_stats_on(ctx: &mut crate::script::EffectContext<'_>, id: &str, stats: AttackHealth) {
    let Some(mut set) = find_instance(ctx.state, id).cloned() else {
        return;
    };
    // R175, R444: Set Stats is temporary across a reset, but an X/X token must still know the face it
    // was summoned with when Reborn or Flicker returns it.
    crate::zones::remember_summoned_token_face(ctx.state, &mut set);
    crate::effects::transform::to_printed_stats(&mut set);
    set.stats_override = Some(stats);
    set.damage = 0;
    let def = crate::catalog::def_of(Some(&*ctx.state), &set.def_id);
    if let Some(worn) = worn_stats_override(def, &set)
        && worn != stats
    {
        let tuning = set.tuning.get_or_insert_with(Tuning::default);
        tuning.attack = Some(stats.attack - worn.attack);
        tuning.health = Some(stats.health - worn.health);
    }
    if let Some(live) = find_instance_mut(ctx.state, id) {
        *live = set;
    }
}

/// R1185002: set one Unit on the field (or carried) to `stats`, unless Immutable (R23), and report the
/// numbers it now wears, auras included, with a public `numberChanged`.
fn set_one(
    ctx: &mut crate::script::EffectContext<'_>,
    unit: &crate::state::CardInstance,
    stats: AttackHealth,
) {
    if !crate::zones::in_units_row_or_standing(ctx.state, unit) && !crate::zones::is_carried(ctx.state, unit)
    {
        return;
    }
    if unit_has(ctx.state, unit, KeywordKind::Immutable) {
        return;
    }
    set_stats_on(ctx, &unit.id, stats);
    let Some(now) = find_instance(ctx.state, &unit.id).cloned() else {
        return;
    };
    let view = unit_view(ctx.state, &now);
    for (key, value) in [("attack", view.attack), ("health", view.health)] {
        ctx.events.push(GameEvent::NumberChanged {
            instance_id: now.id.clone(),
            def_id: now.def_id.clone(),
            key: key.to_string(),
            value,
            hidden_from: None,
        });
    }
}

/// §6.3 Set stats (R1185002) on every Unit the scope names (pile tops, carried and standing Units: R13,
/// R446, R1081000); a Spell's set passes a Unit Immune to Spells by, as every scope does.
pub fn set_stats_all(args: SetStatsAllArgs) -> Effect {
    Effect::new("setStatsAll", move |ctx| {
        let stats = AttackHealth {
            attack: args.attack,
            health: args.health,
        };
        for unit in cards_in_scope(ctx, &args.scope) {
            set_one(ctx, &unit, stats);
        }
    })
}

/// `setStats`' argument: one Unit and the Attack and Health it is set to.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SetStatsArgs {
    pub target: TargetSpec,
    pub attack: i32,
    pub health: i32,
}

/// §6.3 Set stats on one Unit (R2198000, Training Grounds #34 Dino-size), with `set_one`'s guards; who
/// may be targeted is the declared target's business (R81), so Immune to Spells needs no check here.
pub fn set_stats(args: SetStatsArgs) -> Effect {
    Effect::new("setStats", move |ctx| {
        let Some(DamageTarget::Unit { instance: unit }) = resolve_target(ctx, &args.target) else {
            return;
        };
        let stats = AttackHealth {
            attack: args.attack,
            health: args.health,
        };
        set_one(ctx, &unit, stats);
    })
}
