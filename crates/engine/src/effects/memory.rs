//! What a card remembers, on its own instance (§10.1): Carnivorous Cube's meal, Heroic Power's power.
//!
//! R174, R78: what is remembered is remembered by the card on the stay the run began with: a card an
//! earlier effect of the same list took off the field has been reset, memory included, so a later
//! part writes nothing onto it (a fused Cube part, onto a card its Silas part bounced). A card no
//! longer anywhere (a Death hook's snapshot) has nothing to write onto either.
//!
//! R102: on a fused card each ingredient remembers under its own key (`work::part_memory_key`), which
//! `query::recalled` applies too; a card a Fuse keeps moves what its texts remembered to their path in
//! the new fusion (`work::reroot_remembered`, R77).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::script::Effect;
use crate::state::find_instance_mut;
use crate::work::remember_on;

use super::targets::self_on_its_stay;

/// R2247000: the memory key a card keeps a word under that it shows in place of its text in its owner's
/// hand (the view's `CardView.label`): Anomaly Found!'s board code, remembered as it arrives.
pub const CARD_LABEL_KEY: &str = "cardLabel";

/// `remember`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
