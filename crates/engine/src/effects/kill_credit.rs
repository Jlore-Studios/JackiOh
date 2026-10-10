//! Kills an effect watches, and kills it credits to another unit (R42, R412): Classic+ #19.2 Jungle
//! Loser's "If it destroys the enemy Unit across from your Bot Loser, your Bot Loser goes Berserk" and
//! its Radiant "your Bot Loser gets the kill instead".
//!
//! One effect wraps the one it watches (`during`, a forced attack): the credit is in force while that
//! effect's hits land and gone after it, and the kills are read off that effect's own `destroyed`
//! events in the same `apply`, so a Death that asks (a prompt pausing the list) loses none of them.

use std::sync::Arc;

use serde_json::{Value, json};

use super::targets::{TargetSpec, instance_of};
use crate::kill_credit::{KILL_CREDIT_KEY, KillCredit};
use crate::resolve::apply_effects;
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, find_instance_mut};
use crate::wire::{GameEvent, ZoneName};

/// `with_kill_credit`'s `pairs`: read before `during`, each victim that matters and the unit it is
/// paired with.
pub type KillCreditPairs =
    Arc<dyn Fn(&mut EffectContext<'_>, &CardInstance) -> Vec<KillCredit> + Send + Sync>;

/// `with_kill_credit`'s `then`: the effects for one pair whose victim `during` destroyed.
pub type KillCreditThen = Arc<dyn Fn(&KillCredit) -> Vec<Effect> + Send + Sync>;

/// `with_kill_credit`'s arguments. Two callbacks and an effect, so it is not data: a card builds it in Rust.
#[derive(Clone)]
pub struct WithKillCreditArgs {
    pub killer: TargetSpec,
    pub pairs: KillCreditPairs,
    pub transfer: bool,
    pub during: Effect,
    pub then: Option<KillCreditThen>,
}

impl std::fmt::Debug for WithKillCreditArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WithKillCreditArgs")
            .field("killer", &self.killer)
            .field("transfer", &self.transfer)
            .field("during", &self.during)
            .field("then", &self.then.is_some())
            .finish()
    }
}

/// R42, R412: apply `during` with `killer`'s kills watched. `pairs`, read before `during`, names each
/// victim that matters and the unit it is paired with. With `transfer`, `killer`'s lethal hit on a
/// paired victim names that unit as R42's killer (the `destroyed` event's `killerId` and its kill
/// triggers). Then `then(pair)` applies for each pair whose victim `during` destroyed, killed by the
/// credited unit with `transfer` or by `killer` without; its effects must not ask. A killer not on the
/// field watches nothing, and `during` still applies.
pub fn with_kill_credit(args: WithKillCreditArgs) -> Effect {
    Effect::new("withKillCredit", move |ctx| {
        let killer = instance_of(ctx, &args.killer);
        let pairs: Vec<KillCredit> = match &killer {
            Some(card) if card.zone.z() == ZoneName::Field => (args.pairs)(ctx, card),
            _ => Vec::new(),
        };
        if let Some(card) = &killer
            && args.transfer
            && !pairs.is_empty()
        {
            // A copy of each credit, as plain JSON.
            let credits: Vec<Value> = pairs
                .iter()
                .map(|pair| json!({ "victimId": pair.victim_id, "toId": pair.to_id }))
                .collect();
            if let Some(live) = find_instance_mut(&mut *ctx.state, &card.id) {
                live.memory
                    .insert(KILL_CREDIT_KEY.to_string(), Value::Array(credits));
            }
        }
        let from = ctx.events.len();
        (args.during.apply)(ctx);
        let Some(killer) = killer else {
            return;
        };
        if let Some(live) = find_instance_mut(&mut *ctx.state, &killer.id) {
            // The other keys keep their order.
            live.memory.shift_remove(KILL_CREDIT_KEY);
        }
        // The events `during` pushed, as they stood before any `then`.
        let destroyed: Vec<(String, Option<String>)> = ctx.events[from..]
            .iter()
            .filter_map(|event| match event {
                GameEvent::Destroyed {
                    instance_id,
                    killer_id,
                    ..
                } => Some((instance_id.clone(), killer_id.clone())),
                _ => None,
            })
            .collect();
        for (victim_id, killer_id) in destroyed {
            let Some(pair) = pairs.iter().find(|each| each.victim_id == victim_id) else {
                continue;
            };
            let credited = if args.transfer {
                pair.to_id.as_str()
            } else {
                killer.id.as_str()
            };
            if killer_id.as_deref() != Some(credited) {
                continue;
            }
            if let Some(then) = &args.then {
                let effects = then(pair);
                apply_effects(&effects, ctx);
            }
        }
    })
}
