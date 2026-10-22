//! Float (SPEC §6.2, R1563000): a player Floats when their turn ends with mana unspent, and the Float
//! is that much mana, the `turnEnded` event's `unspentMana` (§2.2's end step). "Reveals when any player
//! Floats" is a trap trigger on `turnEnded` (#18 Bread and Butter); "Float reduces cost" is this
//! module's trigger, which a card registers as a hand trigger and as a deck trigger (R464), each under
//! its own id (#29 GIGA Glowy Jelly Bean, M #49.1 YileGPT Prime).

use serde_json::json;

use crate::effects::set_cost_mod;
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext, TriggerDef};
use crate::wire::{GameEvent, GameEventType, PlayerId};

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
