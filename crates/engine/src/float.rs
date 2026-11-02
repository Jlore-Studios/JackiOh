//! Float (SPEC §6.2, R1563000): a player Floats when their turn ends with mana unspent, and the Float
//! is that much mana, the `turnEnded` event's `unspentMana` (§2.2's end step). "Reveals when any player
//! Floats" is a trap trigger (#18 Bread and Butter); "Float reduces cost" and "Float reduces Tribute
//! requirement by 1" (R1841003) are this module's, registered as a hand and a deck trigger (R464),
//! each under its own id (#29 GIGA Glowy Jelly Bean, M #49.1 YileGPT Prime).

use serde_json::{Value, json};

use crate::config::FLOAT_TRIBUTE_REDUCTION;
use crate::effects::set_cost_mod;
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext, TriggerDef};
use crate::state::{CardInstance, find_instance_mut};
use crate::wire::{GameEvent, GameEventType, PlayerId};

/// R1841003: where a card keeps what its owner's Floats took off its Tribute X.
pub const TRIBUTE_LOWERED_KEY: &str = "tributeLowered";

/// The Float an event carries: the player who ended the turn and the mana they left unspent. Every
/// other event, and a turn that ends with nothing unspent, is no Float.
pub fn float_of(event: &GameEvent) -> Option<(PlayerId, i32)> {
    match event {
        GameEvent::TurnEnded {
            player, unspent_mana, ..
        } if *unspent_mana > 0 => Some((*player, *unspent_mana)),
        _ => None,
    }
}

/// "Float reduces cost" (R1563000): whenever the card's owner Floats, the card costs that much less, a
/// `costMod` of minus the Float on the card itself, kept in hand and deck until a graveyard or exile
/// resets it (R766); the price floors at 0. The opponent's Float lowers nothing.
pub fn float_reduces_cost(id: &str) -> TriggerDef {
    TriggerDef::new(id, &[GameEventType::TurnEnded], discount)
        .with_when(|ctx, event| float_of(event).is_some_and(|(player, _)| player == ctx.controller))
}

fn discount(ctx: &mut EffectContext<'_>, event: &GameEvent) -> Vec<Effect> {
    match float_of(event) {
        Some((player, float)) if player == ctx.controller => vec![set_cost_mod(json_as(json!({
            "target": { "of": "self" },
            "amount": -float,
        })))],
        _ => vec![],
    }
}

/// R1841003: what its owner's Floats have taken off the card's Tribute X so far, 0 when nothing has.
/// `play_choices::tribute_cost_of` takes it off, never below 0.
pub fn tribute_lowered(card: &CardInstance) -> i32 {
    card.memory
        .get(TRIBUTE_LOWERED_KEY)
        .and_then(Value::as_i64)
        .map_or(0, |lowered| i32::try_from(lowered).unwrap_or(0).max(0))
}

/// "Float reduces Tribute requirement by 1" (R1841003): whenever the card's owner Floats, its Tribute X
/// is `FLOAT_TRIBUTE_REDUCTION` lower, whatever the Float's size, kept in hand and deck on the card's
/// `memory` until a graveyard, an exile or the field's reset takes it (R78, R215).
pub fn float_lowers_tribute(id: &str) -> TriggerDef {
    TriggerDef::new(id, &[GameEventType::TurnEnded], lower_tribute)
        .with_when(|ctx, event| float_of(event).is_some_and(|(player, _)| player == ctx.controller))
}

fn lower_tribute(ctx: &mut EffectContext<'_>, event: &GameEvent) -> Vec<Effect> {
    match float_of(event) {
        Some((player, _)) if player == ctx.controller => vec![Effect::new("lowerTribute", |ctx| {
            let Some(id) = ctx.self_.as_ref().map(|card| card.id.clone()) else {
                return;
            };
            if let Some(card) = find_instance_mut(ctx.sink.state, &id) {
                let lowered = tribute_lowered(card) + FLOAT_TRIBUTE_REDUCTION;
                card.memory
                    .insert(TRIBUTE_LOWERED_KEY.to_string(), json!(lowered));
            }
        })],
        _ => vec![],
    }
}
