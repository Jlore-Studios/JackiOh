//! A secret pick from the opponent's hand (Stinkyscape+ #31 Soul Read, SPEC §6.3 Secret pick,
//! R3550008).
//!
//! The trap's Cry shows its owner the opponent's hand, and they secretly choose one card
//! (`secret_pick`, a `choose_from_hand` of the enemy's hand answered by its owner alone, §10.8).
//! The resume keeps the pick (`keep_secret_pick`): the chosen id and every id seen, on the card's own
//! memory — instance memory, so no view and no event ever names them.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::choose::{ChooseFromHandArgs, choose_from_hand};
use crate::script::Effect;
use crate::state::find_instance_mut;
use crate::wire::{Selection, opponent_of};

/// R3550008: where a card keeps its secret pick, `{ chosen, seen }`. Instance memory, so it reaches
/// no view, no event and no AI (§10.8).
pub const SECRET_PICK_KEY: &str = "@secretPick";

/// `secret_pick`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SecretPickArgs {
    pub step: String,
}

/// R3550008: "Look at your opponent's hand and choose a card secretly" — one card of the enemy
/// hand, picked by the card's owner alone. An empty hand opens no prompt (`choose_from_hand`).
pub fn secret_pick(args: SecretPickArgs) -> Effect {
    choose_from_hand(
        serde_json::from_value::<ChooseFromHandArgs>(json!({
            "of": "enemy",
            "count": 1,
            "step": args.step,
            "prompt": "Look at your opponent's hand and choose a card secretly",
        }))
        .expect("a secret pick is a hand pick"),
    )
}

/// R3550008: keep the secret pick — the chosen id and every id of the enemy hand then seen, on the
/// card's own memory.
pub fn keep_secret_pick() -> Effect {
    Effect::new("keepSecretPick", move |ctx| {
        let chosen = ctx.targets.first().and_then(|picked| match picked {
            Selection::Instance { instance_id } => Some(instance_id.clone()),
            _ => None,
        });
        let Some(chosen) = chosen else {
            return;
        };
        let enemy = opponent_of(ctx.controller);
        let seen: Vec<Value> = ctx.state.players[enemy]
            .hand
            .iter()
            .map(|card| Value::String(card.id.clone()))
            .collect();
        let record = json!({ "chosen": chosen, "seen": seen });
        if let Some(self_id) = ctx.live_self().map(|card| card.id.clone())
            && let Some(live) = find_instance_mut(ctx.sink.state, &self_id)
        {
            live.memory.insert(SECRET_PICK_KEY.to_string(), record);
        }
    })
}
