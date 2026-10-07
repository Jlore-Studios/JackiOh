//! One effect for each card of a set a clause reads off the board (R113, R66).
//!
//! A hook is a pure builder, so a list a prompt split is continued by building it again and skipping
//! what already ran (`prompts.runResume`). That is exact for a list whose shape does not hang on the
//! board — and wrong for one that does, when its own head has moved the cards it was built from:
//! #94 Genn's Greed's "draw every 2-cost card" built one draw per card, a drawn card that was cast and
//! asked had left the library by the answer, and the rebuilt list, one draw shorter, skipped a Bigot
//! by index. So a clause over such a set is a part of the list (`resolve.lazyPart`): it reads its set
//! once, as the list reaches it, and keeps the ids as the part's memo, which a continuation hands
//! back to its rebuild — so a pause inside it resumes over the very set it began with.
//!
//! Port of `packages/engine/src/effects/each.ts`. TS's `cards` answered instances or ids; here it
//! answers the ids (a card file maps its instances to `card.id`).

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

/// `each(id)` for every card `cards` names, read once as the list reaches the clause and never again,
/// so a prompt inside one of them resumes over the same set in the same order (R113). A card's list
/// whose length depends on the board — a draw per matching library card (#94), a draw of each of two
/// named cards (#30 radiant) — writes that part with this rather than spreading a `.map` into the list.
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
