//! Reveal (Classic #88 Siphon Squad's "Start of Turn: Reveal", Classic #65 Ace in the Hole's Radiant
//! "Revealed regardless of the coin flip"; R686): a backrow Trap or Field Trap whose identity is
//! public while the card stays armed.
//!
//! A revealed card is no longer face-down (`preview::is_face_down`), so both players read its face —
//! but it is NOT face-up: a Trap that has not fired still fires (`traps::is_spent` keys on `faceUp`
//! alone, as do the replacement and text guards that read it), and a Field Trap keeps firing either
//! way. R78's reset clears it with the card leaving the field. No event is emitted: the next view
//! shows the card, which is the whole of the reveal.
//!
//! Port of `packages/engine/src/effects/reveal.ts`.

use std::borrow::Borrow;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::brittle_count::start_brittle_on_field;
use crate::effects::targets::{TargetSpec, instance_of};
use crate::prelude::json_as;
use crate::script::Effect;
use crate::state::{CardInstance, find_instance, find_instance_mut};
use crate::wire::{Row, Zone};

/// `{ of: "self" }`, built from its JSON so this file names no variant of `TargetSpec`'s own.
fn self_spec() -> TargetSpec {
    json_as(json!({ "of": "self" }))
}

/// `reveal`'s arguments: the target defaults to the running card.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RevealArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
}

/// Show a backrow card's face to both players without firing it (default the running card).
pub fn reveal(args: RevealArgs) -> Effect {
    Effect::new("reveal", move |ctx| {
        let spec = args.target.clone().unwrap_or_else(self_spec);
        let Some(card) = instance_of(ctx, &spec) else {
            return;
        };
        if !matches!(card.zone, Zone::Field { row: Row::Backrow, .. }) {
            return;
        }
        let Some(mut shown) = find_instance(ctx.state, &card.id).cloned() else {
            return;
        };
        shown.revealed = Some(true);
        // R687: a printed Brittle a face-down arrival never started begins now that it shows.
        start_brittle_on_field(ctx.state, &mut shown, false);
        if let Some(live) = find_instance_mut(ctx.state, &card.id) {
            *live = shown;
        }
    })
}
