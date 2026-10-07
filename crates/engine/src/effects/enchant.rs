//! Enchantments that ride a card (docs/classic-sets.md B5 E39): the verb that puts one on. Classic+ #40
//! Appropriations' Education shuffles in Books that "have Cast on draw and aim at enemies when they
//! harm and at your side when they help" — `enchant({ instanceId, enchantment: { kind: "castOnDraw" } })`
//! and `{ kind: "targetEnemies" }` on each Book it made — and #14 Forever&'s "after this resolves, return
//! it to hand; it can't cost less than (2)" is `{ kind: "returnAfterResolve", floor }` (which the play
//! pipeline stamps on the next Spell played, `enchantments.addEnchantment`).
//!
//! An enchantment is kept in every zone and never reset (R78 leaves it alone): each is read where its
//! rule acts, by the module that owns that rule (`enchantments.ts`). No event reports one: it names no
//! change a board shows, and each viewer reads a card's enchantments off its view where they may read
//! the card (`CardView.enchantments`).
//!
//! Port of `packages/engine/src/effects/enchant.ts`. The enchantment goes on the card as it stands in
//! the state, found by its id (TS wrote through the live object).

use serde::{Deserialize, Serialize};

use crate::damage::DamageTarget;
use crate::effects::card_scope::{CardScope, cards_in_card_scope};
use crate::effects::targets::{TargetSpec, instance_on_its_stay, resolve_target};
use crate::enchantments::add_enchantment;
use crate::script::Effect;
use crate::state::{CardInstance, find_instance_mut};
use crate::wire::{Enchantment, ZoneName};

/// `enchant`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EnchantArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<CardScope>,
    pub enchantment: Enchantment,
}

/// B5 E39: put `enchantment` on one named card (a spec or an id a script captured), or on a scope's cards.
pub fn enchant(args: EnchantArgs) -> Effect {
    Effect::new("enchant", move |ctx| {
        let mut cards: Vec<CardInstance> = Vec::new();
        if let Some(scope) = &args.scope {
            cards = cards_in_card_scope(ctx, scope, Default::default())
                .into_iter()
                .map(|entry| entry.card)
                .collect();
        } else if let Some(instance_id) = &args.instance_id {
            // R174: a card named by id is aimed at the stay it had when the run began.
            cards = instance_on_its_stay(ctx, instance_id).into_iter().collect();
        } else if let Some(target) = &args.target {
            cards = match resolve_target(ctx, target) {
                Some(DamageTarget::Unit { instance }) => vec![instance],
                _ => Vec::new(),
            };
        }
        for card in cards {
            // A card that has ceased to exist (R11, R86) is in no pile to carry anything.
            if card.zone.z() == ZoneName::Gone {
                continue;
            }
            if let Some(live) = find_instance_mut(ctx.sink.state, &card.id) {
                add_enchantment(live, &args.enchantment);
            }
        }
    })
}
