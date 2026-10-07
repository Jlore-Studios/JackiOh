//! Draw: takes the top card, with cast-on-draw, fatigue, the hand cap and R58's chain cap (§2.4),
//! and the named-card form a script computes for itself (#30 Archivist, #94 Genn's Greed).
//!
//! Port of `packages/engine/src/effects/draw.ts`. The engine's draw is `crate::draw::draw` (TS
//! `draw as drawCards`); this module's `draw` is the effect verb.

use serde::{Deserialize, Serialize};

use crate::draw::{complete_draw, draw_blocked};
use crate::effects::targets::{PlayerSpec, player_of};
use crate::script::Effect;

/// `draw`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DrawArgs {
    pub count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

pub fn draw(args: DrawArgs) -> Effect {
    Effect::new("draw", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        crate::draw::draw(ctx, player, args.count);
    })
}

/// `drawFromLibrary`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DrawFromLibraryArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub def_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// §6.3 Draw of a card a script has NAMED, rather than of the top of the library: #30 Archivist's
/// "draw the highest-cost card in your library", #94 Genn's Greed's "draw every 2-cost card from your
/// library". Neither of the neighbouring verbs is this one — `draw({ count })` takes the top and
/// `addToHand({ defId })` creates a fresh card of a definition and leaves the original where it is,
/// which is a second copy and not a draw — so a card that names its own card out of a library needs
/// this, and the two halves of "it is a DRAW" fall out of sharing `../draw`'s pipeline: the card
/// really leaves the library, `state.counters.drawn` moves (R55), a `drawn` event fires for anything
/// watching, the hand cap burns the overflow (§2.4, R4) and a cast-on-draw card casts (R58).
///
/// `instanceId` names one card; `defId` takes the first card of that definition from the top of the
/// library, which is the form a card that knows only what it wants can write. A spec that matches
/// nothing — an empty library, a card already drawn by an earlier effect in the same list — fizzles
/// and the card still resolves (§6.3). A script that wants several cards returns several of these,
/// one per card, so each is its own draw in its own order (R135's per-card rule for the exile half).
pub fn draw_from_library(args: DrawFromLibraryArgs) -> Effect {
    Effect::new("drawFromLibrary", move |ctx| {
        if args.instance_id.is_none() && args.def_id.is_none() {
            return;
        }

        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let at = ctx.sink.state.players[player]
            .library
            .iter()
            .position(|card| match &args.instance_id {
                None => args.def_id.as_deref() == Some(card.def_id.as_str()),
                Some(instance_id) => &card.id == instance_id,
            });
        let Some(at) = at else {
            return;
        };
        // B5 E3, R457: a draw past the player's limit this turn does not happen, and leaves the card.
        if draw_blocked(ctx, player) {
            return;
        }

        // Out of the pile first, exactly as `drawOne` does it, so a cast-on-draw card resolves against
        // a library that no longer holds it (§2.4, R58).
        let card = ctx.sink.state.players[player].library.remove(at);
        complete_draw(ctx, player, card, None);
    })
}
