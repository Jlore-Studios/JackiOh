//! Mint a crafted card (ME-CRAFT, Meditative #17 True Craft a Card, R882): validate the recipe and
//! compile it into a transient definition, kept in `state.transient_defs` as a Fuse's is (R77) so its
//! scripts rebuild in any process, then add a fresh instance to the hand (§2.4, R4).

use serde::{Deserialize, Serialize};

use crate::draw::add_to_hand;
use crate::script::Effect;
use crate::state::new_instance;
use crate::subsystems::craft::{compile_crafted_def, validate_recipe};
use crate::wire::{CraftRecipe, Zone};

/// `craft_card`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CraftCardArgs {
    pub recipe: CraftRecipe,
    /// The Radiant face crafts the card Radiant (R882).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
}

/// An invalid recipe makes nothing: the answer check (`prompts`) has already refused it, so this
/// guard is unreachable from an answered prompt.
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
        crate::generators::record_generator(ctx, &card);
        let _ = add_to_hand(&mut ctx.sink, &mut card);
    })
}
