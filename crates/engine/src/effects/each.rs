//! One effect for each card of a set a clause reads off the board (R113, R66).
//!
//! A hook is a pure builder, so a list a prompt split is continued by building it again and skipping
//! what already ran: wrong for a list whose shape hangs on the board once its own head has moved the
//! cards it was built from (#94 Genn's Greed's rebuilt draws skipped a Bigot by index). So such a
//! clause is a part of the list (`resolve::lazy_part`): it reads its set once, as the list reaches it,
//! and keeps the ids as the part's memo, so a pause inside it resumes over the set it began with.

use std::sync::Arc;

use serde_json::{Value, json};

use crate::resolve::lazy_part;
use crate::script::{Effect, EffectContext, EffectPart};

/// The set `forEachCard` reads once: the cards' ids, in order.
pub type ForEachCardCards = Arc<dyn Fn(&mut EffectContext<'_>) -> Vec<String> + Send + Sync>;

/// The effect `forEachCard` makes for one card, by its id.
pub type ForEachCardEach = Arc<dyn Fn(&str) -> Effect + Send + Sync>;

/// `forEachCard`'s argument.
#[derive(Clone)]
pub struct ForEachCardArgs {
    pub cards: ForEachCardCards,
    pub each: ForEachCardEach,
}

/// `each(id)` for every card `cards` names, read once (R113). A card's list whose length depends on
/// the board (a draw per matching library card, #94; a draw of each of two named cards, #30 radiant)
/// writes that part with this rather than spreading a `.map` into the list.
pub fn for_each_card(args: ForEachCardArgs) -> Effect {
    lazy_part("forEachCard", move |ctx, memo| {
        let ids: Vec<String> = match memo {
            Some(Value::Array(items)) => items
                .iter()
                .filter_map(|id| id.as_str().map(str::to_string))
                .collect(),
            _ => (args.cards)(ctx),
        };
        EffectPart {
            effects: ids.iter().map(|id| (args.each)(id)).collect(),
            memo: Some(json!(ids)),
        }
    })
}
