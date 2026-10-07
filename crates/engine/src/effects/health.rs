//! Set health, and heals turned into damage (docs/classic-sets.md B5 E7, E8).
//!
//! Port of `packages/engine/src/effects/health.ts`.

use serde::{Deserialize, Serialize};

use crate::damage::{DamageTarget, set_hero_health};
use crate::effects::targets::{PlayerSpec, TargetSpec, player_of, resolve_target};
use crate::modifiers::add_modifier;
use crate::script::Effect;
use crate::state::{ModifierExpiry, ModifierKind};

/// `setHealth`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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
