//! Set health, heals turned into damage (docs/classic-sets.md B5 E7, E8), and a Unit's Attack and
//! Health set outright (R1185002, M+ #21 Harmony Spirit).
//!
//! Port of `packages/engine/src/effects/health.ts`.

use serde::{Deserialize, Serialize};

use crate::damage::{DamageTarget, set_hero_health};
use crate::effects::targets::{BoardScope, PlayerSpec, TargetSpec, cards_in_scope, player_of, resolve_target};
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

/// B5 E7: "Set a hero's health to N" (Classic #29 Book of Vital Kill, either hero, a declared target).
/// No pipeline, not damage and not a heal: no Armor, no cap, no replacement, nothing that answers a hit
/// or a heal (R18's lose health is the nearest rule). A target that is not a hero fizzles. `healthSet`.
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

/// B5 E8: "for the rest of this turn, healing on enemies deals that much Pierce damage to them
/// instead" — a this-turn player modifier on `player` (default "self"), converting every heal on that
/// player's enemies from then on, the damage from this card. Classic+ #22 Blood Moon's base face does
/// the same through its replacement's `lasting: "thisTurn"`, which also converts the heal that set it
/// off; this verb is the plain effect for a card that says it outright.
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

/// R1185002: Hearthstone's "set": the Unit's §10.4 layer 1 becomes `stats` (`statsOverride`, which
/// wins over its printed and X stats, R41), its buffs and the stat half of its `tuning` go
/// (`to_printed_stats`, R781003) and its damage is healed, so it stands at exactly `stats` before the
/// auras of layer 5 apply on top. A face that wears its X/X doubled (the Radiant Ghoul Token, R349)
/// keeps the difference as tuning, so it too wears `stats`. R78's reset takes the override off with
/// the card leaving the field, so a set Unit bounced is its printed card again.
fn set_stats_on(ctx: &mut crate::script::EffectContext<'_>, id: &str, stats: AttackHealth) {
    let Some(mut set) = find_instance(ctx.state, id).cloned() else {
        return;
    };
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

/// §6.3 Set stats (R1185002): every Unit the scope names (`cards_in_scope` order, the tops of piles,
/// carried and standing Units included, R13, R446, R1081000) is set to `attack`/`health`
/// (`set_stats_on`). An Immutable Unit is untouched (R23), and a Spell's set passes a Unit Immune to
/// Spells by, as every scope does. Each Unit set reports its two numbers as it now wears them, auras
/// included, with `numberChanged` ("attack", "health"): public, since a Unit on the field is.
pub fn set_stats_all(args: SetStatsAllArgs) -> Effect {
    Effect::new("setStatsAll", move |ctx| {
        let stats = AttackHealth {
            attack: args.attack,
            health: args.health,
        };
        for unit in cards_in_scope(ctx, &args.scope) {
            if !crate::zones::in_units_row_or_standing(ctx.state, &unit) && !crate::zones::is_carried(ctx.state, &unit)
            {
                continue;
            }
            if unit_has(ctx.state, &unit, KeywordKind::Immutable) {
                continue;
            }
            set_stats_on(ctx, &unit.id, stats);
            let Some(now) = find_instance(ctx.state, &unit.id).cloned() else {
                continue;
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
    })
}
