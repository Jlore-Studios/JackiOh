//! Granted tags (docs/meditative-set.md, group B's Systems; MD-B15, R923): Meditative #35 gives a
//! permanent the CN tag. The tag joins the instance's `granted_tags`, so every instance-level tag
//! read (`query::tags_of`) sees it from then on, while catalog pools — which read definitions — do
//! not. A card that already carries the tag, or one that has ceased to exist (R11, R86), is left
//! alone. No event: the view lists the card's tags where they differ from its definition's.

use serde::{Deserialize, Serialize};

use super::targets::{TargetSpec, instance_of, instance_on_its_stay};
use crate::script::Effect;
use crate::state::find_instance_mut;
use crate::wire::{Tag, ZoneName};

/// Which card gains the tag: one named card (a spec or an id a script captured), like `translate`'s.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GrantTagArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    pub tag: Tag,
}

/// MD-B15, R923: give the named card `tag`, once — a second grant of a tag it carries changes
/// nothing and reports nothing.
pub fn grant_tag(args: GrantTagArgs) -> Effect {
    Effect::new("grantTag", move |ctx| {
        // R174: a card named by id is aimed at the stay it had when the run began.
        let card = if let Some(instance_id) = &args.instance_id {
            instance_on_its_stay(ctx, instance_id)
        } else {
            args.target.as_ref().and_then(|target| instance_of(ctx, target))
        };
        let Some(card) = card else {
            return;
        };
        if card.zone.z() == ZoneName::Gone {
            return;
        }
        if crate::query::tags_of(ctx.state, &card).contains(&args.tag) {
            return;
        }
        let Some(stored) = find_instance_mut(&mut *ctx.state, &card.id) else {
            return;
        };
        match stored.granted_tags.as_mut() {
            Some(granted) => granted.push(args.tag),
            None => stored.granted_tags = Some(vec![args.tag]),
        }
    })
}
