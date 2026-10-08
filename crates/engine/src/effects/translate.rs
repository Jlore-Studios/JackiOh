//! ME-CN's verb (docs/meditative-set.md, group B's Systems, ME-CN; R1300, R1301): "Translate all cards
//! in your and your opponent's hand and deck into Chinese" (Meditative #32), "convert its text to
//! Chinese" (#35). It sets the instance's `chinese` flag and nothing else: the flag is presentation
//! only, so no rule reads it, and Immutable does not stop it (the text is the same, only the language
//! it is shown in changes, MD-B11).
//!
//! Events (R440, R1301): `translated` is sent only for a card both players read where it sits (a
//! unit, a face-up backrow card). A change in a hand, a deck or on a face-down trap is silent, and its
//! owner reads it off their own view (`CardView.chinese`).

use serde::{Deserialize, Serialize};

use super::card_scope::{CardScope, Readers, cards_in_card_scope, readers_of};
use super::targets::{TargetSpec, instance_of, instance_on_its_stay};
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, find_instance_mut};
use crate::wire::{GameEvent, ZoneName};

/// Which cards `translate` reaches: one named card (a spec or an id a script captured), or a scope.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TranslateArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<CardScope>,
}

fn reached(ctx: &EffectContext<'_>, args: &TranslateArgs) -> Vec<CardInstance> {
    if let Some(scope) = &args.scope {
        return cards_in_card_scope(ctx, scope, None)
            .into_iter()
            .map(|entry| entry.card)
            .collect();
    }
    // R174: a card named by id is aimed at the stay it had when the run began.
    let card = if let Some(instance_id) = &args.instance_id {
        instance_on_its_stay(ctx, instance_id)
    } else {
        args.target.as_ref().and_then(|target| instance_of(ctx, target))
    };
    card.into_iter().collect()
}

/// ME-CN: show the cards in Chinese from now on (R1300). A card already Chinese, or one that has
/// ceased to exist (R11, R86), is left alone and reported by nothing.
pub fn translate(args: TranslateArgs) -> Effect {
    Effect::new("translate", move |ctx| {
        for card in reached(ctx, &args) {
            if card.zone.z() == ZoneName::Gone || card.chinese == Some(true) {
                continue;
            }
            let public = readers_of(ctx.state, &card) == Readers::Everyone;
            let Some(stored) = find_instance_mut(&mut *ctx.state, &card.id) else {
                continue;
            };
            stored.chinese = Some(true);
            if public {
                ctx.events.push(GameEvent::Translated { instance_id: card.id });
            }
        }
    })
}
