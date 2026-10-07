//! "Summon this from your hand or deck" (docs/classic-sets.md B5 E26): the verb a hand or deck
//! trigger ends in (Classic #66 EU Striker, Classic+ #37 Wardrum). It is §6.3's Summon of a card that
//! already exists: no Cry (R1), into its controller's leftmost open, unlocked, unreserved zone of its
//! row (R64), summoning sick for the turn it arrives on (§4.1), and nothing at all when that row is
//! full. Only a card in a hand or a library moves: a trigger that has left those zones by the time it
//! resolves has nothing to summon.
//!
//! Port of `packages/engine/src/effects/summonThis.ts`.

use serde_json::json;

use crate::effects::summon::{SummonArgs, summon};
use crate::effects::targets::TargetSpec;
use crate::prelude::json_as;
use crate::script::Effect;
use crate::wire::ZoneName;

/// B5 E26: summon the card running the script out of its hand or its library (no Cry, R64).
pub fn summon_this() -> Effect {
    Effect::new("summonThis", |ctx| {
        // TS read the live `ctx.self`: the zone the card stands in now.
        let zone = ctx.live_self().map(|me| me.zone.z());
        if zone != Some(ZoneName::Hand) && zone != Some(ZoneName::Library) {
            return;
        }
        // `{ of: "self" }`, built from its JSON so this file names no variant of `TargetSpec`'s own.
        let this: TargetSpec = json_as(json!({ "of": "self" }));
        let effect = summon(SummonArgs {
            instance: Some(this),
            ..SummonArgs::default()
        });
        (effect.apply)(ctx);
    })
}
