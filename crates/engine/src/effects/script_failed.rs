//! Custom cards' effects (#915, SPEC §12.5, R915004).
//!
//! A custom card's hook that fails does nothing but say so: the Luau host resolves this verb in
//! place of the effects the hook would have made. A card file never calls it.

use crate::script::Effect;
use crate::wire::GameEvent;

/// R915004: a custom card's hook failed. Emits `scriptFailed`, naming the card running it, its
/// controller and `message` (the hook, and the line when there is one, then what went wrong).
pub fn script_failed(message: String) -> Effect {
    Effect::new("scriptFailed", move |ctx| {
        let (instance_id, def_id) = match &ctx.self_ {
            Some(card) => (Some(card.id.clone()), card.def_id.clone()),
            None => (None, String::new()),
        };
        let player = ctx.controller;
        ctx.sink.events.push(GameEvent::ScriptFailed {
            player,
            instance_id,
            def_id,
            message: message.clone(),
        });
    })
}
