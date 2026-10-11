//! Mint a crafted card (ME-CRAFT, Meditative #17 True Craft a Card, R882): validate the
//! recipe, compile it into a transient definition and add a fresh instance to the hand.
//!
//! The definition lands in `state.transient_defs` as a Fuse's does (R77), so its scripts rebuild
//! from the def in any process; a full hand burns the card like any other arrival (§2.4, R4).

use serde::{Deserialize, Serialize};

use crate::draw::add_to_hand;
use crate::script::Effect;
use crate::state::new_instance;
use crate::subsystems::craft::{compile_crafted_def, validate_recipe};
use crate::wire::{CraftRecipe, Zone};

/// `craft_card`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CraftCardArgs {
    pub recipe: CraftRecipe,
    /// The Radiant face crafts the card Radiant (R882).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
}

/// Compile the recipe and add a fresh card of it to the controller's hand at its cost, Radiant
/// on the Radiant face. An invalid recipe makes nothing — the answer check (`prompts`) has
/// already refused it, so this guard is unreachable from an answered prompt.
pub fn craft_card(args: CraftCardArgs) -> Effect {
    Effect::new("craftCard", move |ctx| {
        if validate_recipe(&args.recipe, args.recipe.cost).is_err() {
            return;
        }
        let def = compile_crafted_def(&args.recipe);
        if !ctx.sink.state.transient_defs.contains_key(&def.id) {
            ctx.sink.state.transient_defs.insert(def.id.clone(), def.clone());
        }
        let player = ctx.controller;
        let mut card = new_instance(&mut *ctx.sink.state, &def.id, player, Zone::Hand { player });
        if args.radiant == Some(true) {
            card.radiant = true;
        }
        let _ = add_to_hand(&mut ctx.sink, &mut card);
    })
}
