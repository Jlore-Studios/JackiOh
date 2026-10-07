//! What a card remembers, on its own instance (§10.1): Carnivorous Cube's meal, Heroic Power's power.
//!
//! R174, R78: what is remembered is remembered by the card on the stay the run began with. A card an
//! earlier effect of the same list took off the field has been reset (R78) — memory included — and
//! what it remembered went with that stay, so a later part of the list writes nothing onto it: a
//! fused Cube part does not write its meal onto a card its Silas part has bounced to a hand.
//!
//! R102: on a fused card each ingredient remembers under its own key (`work::part_memory_key`), so two
//! Carnivorous Cubes crafted into one card remember two meals, and each Death copies its own; a card
//! reads back through `query::recalled`, which applies the same key. A card a Fuse keeps moves what its
//! texts remembered to the path they run at in the new fusion (`work::reroot_remembered`, R77).
//!
//! Port of `packages/engine/src/effects/memory.ts`. TS wrote through the live `ctx.self`; here the
//! card is found again by id in the state, and a card that is no longer anywhere (a Death hook's
//! snapshot) has nothing to write onto, as TS's write onto a detached object reached no state.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::script::Effect;
use crate::state::find_instance_mut;
use crate::work::remember_on;

use super::targets::self_on_its_stay;

/// `remember`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RememberArgs {
    pub key: String,
    pub value: Value,
}

pub fn remember(args: RememberArgs) -> Effect {
    Effect::new("remember", move |ctx| {
        let Some(this) = self_on_its_stay(ctx) else {
            return;
        };
        if let Some(card) = find_instance_mut(ctx.sink.state, &this.id) {
            remember_on(&mut card.memory, &ctx.data, &args.key, args.value.clone());
        }
    })
}

/// `rememberRandom`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RememberRandomArgs {
    pub key: String,
    pub options: Vec<Value>,
}

/// Remember one of `options`, drawn from the match rng so setup replays the same way (R43).
pub fn remember_random(args: RememberRandomArgs) -> Effect {
    Effect::new("rememberRandom", move |ctx| {
        let Some(this) = self_on_its_stay(ctx) else {
            return;
        };
        if args.options.is_empty() {
            return;
        }
        let value = ctx.sink.rng.pick(&args.options).cloned().unwrap_or(Value::Null);
        if let Some(card) = find_instance_mut(ctx.sink.state, &this.id) {
            remember_on(&mut card.memory, &ctx.data, &args.key, value);
        }
    })
}
