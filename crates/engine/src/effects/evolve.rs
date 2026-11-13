//! Evolve and "evolve all cards in your deck" as card-script verbs (SPEC §6.3's "Evolve" row,
//! R1849001, R3536000, R3536002; Stinkyscape #91 Wasp, #92 Hornet). A card's own `evolve` hook is
//! what evolving it does, run only by the `evolve` verb, the resumable way (`prompts::run_evolve`),
//! so a question in it pauses the list that evolved the unit — once per instance (`evolution`).

use serde::{Deserialize, Serialize};

use super::targets::{PlayerSpec, TargetSpec, instance_of, player_of};
use crate::script::Effect;

/// `evolve`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct EvolveArgs {
    pub target: TargetSpec,
}

/// R1849001, R3536000: evolve the target: its `evolve` hook runs for its controller once per
/// instance, with the multiplier's extra runs queued behind it (`evolution::evolve_now`).
pub fn evolve(args: EvolveArgs) -> Effect {
    Effect::new("evolve", move |ctx| {
        let Some(card) = instance_of(ctx, &args.target) else {
            return;
        };
        crate::evolution::evolve_now(ctx, &card);
    })
}

/// `evolveDeck`'s arguments: `player` defaults to "self".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct EvolveDeckArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// R3536002: "Evolve all cards in your deck": mark every library card of `player` that prints an
/// Evolve text to enter the field evolved (`evolution::mark_library`).
pub fn evolve_deck(args: EvolveDeckArgs) -> Effect {
    Effect::new("evolveDeck", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        crate::evolution::mark_library(ctx, player);
    })
}
