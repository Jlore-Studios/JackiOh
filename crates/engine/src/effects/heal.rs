//! Heal (§6.3): a unit loses damage, never past its max health; a hero simply gains health, with no
//! cap (§3). R19 lets a heal name any unit or hero. The `healed` event comes from damage.rs.
//!
//! Port of `packages/engine/src/effects/heal.ts`.

use serde::{Deserialize, Serialize};

use super::targets::{TargetSpec, resolve_target};
use crate::damage::{DamageTarget, heal_hero, heal_hero_up_to, heal_to_full, heal_unit};
use crate::layers::unit_view;
use crate::script::Effect;

/// `heal`'s arguments: TS's three object shapes, told apart by the key each carries (`"amount" in
/// args`, then `"toFull" in args`), which is what an untagged enum tried in this order does.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged, rename_all_fields = "camelCase")]
pub enum HealArgs {
    /// "Heal X": up to X damage off a unit, or X health onto a hero with no cap (#5, #47, R19).
    Amount { target: TargetSpec, amount: i32 },
    /// "Heal to full": all of a unit's damage (#19 Midrange Menace). TS `toFull: true`.
    ToFull { target: TargetSpec, to_full: bool },
    /// "Heal up to N": raise health to at least N, still bounded by a unit's max health (§6.3).
    UpTo { target: TargetSpec, up_to: i32 },
}

impl HealArgs {
    /// The target every shape names.
    fn target(&self) -> &TargetSpec {
        match self {
            HealArgs::Amount { target, .. } | HealArgs::ToFull { target, .. } | HealArgs::UpTo { target, .. } => {
                target
            }
        }
    }
}

pub fn heal(args: HealArgs) -> Effect {
    Effect::new("heal", move |ctx| {
        let Some(target) = resolve_target(ctx, args.target()) else {
            return;
        };

        match &args {
            HealArgs::Amount { amount, .. } => {
                match target {
                    DamageTarget::Hero { player } => {
                        heal_hero(ctx, player, *amount);
                    }
                    DamageTarget::Unit { instance } => {
                        heal_unit(ctx, &instance, *amount);
                    }
                }
            }
            HealArgs::ToFull { .. } => {
                // §3 gives a hero no maximum health, so a hero has no "full" to be healed to; no Core card
                // asks for one. The hero form of this reading is `upTo` (§6.3's "Heal up to 30").
                if let DamageTarget::Unit { instance } = target {
                    heal_to_full(ctx, &instance);
                }
            }
            HealArgs::UpTo { up_to, .. } => match target {
                DamageTarget::Hero { player } => {
                    heal_hero_up_to(ctx, player, *up_to);
                }
                DamageTarget::Unit { instance } => {
                    // A unit reaches N only if its own damage is in the way; healing never raises max
                    // health. B5 E8: the heal is what it would restore, which is what a conversion into
                    // damage deals (R462).
                    let missing = *up_to - unit_view(ctx.state, &instance).health;
                    heal_unit(ctx, &instance, instance.damage.min(missing));
                }
            },
        }
    })
}
