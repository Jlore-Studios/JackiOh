//! Animate (docs/classic-sets.md B3.1, R383, R445): the verb an Animated Trap's or Field Trap's effect
//! list ends with — "Then summon this as a Unit in Defense Position" (Classic #5 Tesla), "Summon this as
//! a Unit" (Classic #38 Jackiestan Auctioneer). The move itself is `animated::animate_card`; this is the
//! thin Effect a card script composes (CLAUDE.md rule 5).
//!
//! Port of `packages/engine/src/effects/animate.ts`.

use serde::{Deserialize, Serialize};

use super::targets::{TargetSpec, instance_of};
use crate::script::Effect;
use crate::state::Position;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AnimateArgs {
    /// B3.1 rule 2: Attack Position unless the text says otherwise (Tesla: "DEF").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    /// The card to animate; the card running the script by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
}

/// B3.1 rules 2 and 4: move the card from its backrow zone into its controller's unit zone in that lane,
/// else the leftmost open one, as a Unit — face-up, summoning sick, without leaving the field. A card
/// that is a Unit already stays where it is, in the position it has; with no open unit zone it stays in
/// the backrow (and a Trap stays there face-up, `traps::consume_trap`). Not a summon (R445): it emits
/// `animated`, never `summoned`.
pub fn animate(args: AnimateArgs) -> Effect {
    Effect::new("animate", move |ctx| {
        let this = TargetSpec::SelfCard;
        let Some(card) = instance_of(ctx, args.target.as_ref().unwrap_or(&this)) else {
            return;
        };
        // TS `args.position === undefined ? {} : { position: args.position }`: the options object's one
        // field, passed as it stands.
        let _ = crate::animated::animate_card(ctx, &card, args.position);
    })
}
