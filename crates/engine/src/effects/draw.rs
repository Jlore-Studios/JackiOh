//! Draw: takes the top card, with cast-on-draw, fatigue, the hand cap and R58's chain cap (§2.4),
//! and the named-card form a script computes for itself (#30 Archivist, #94 Genn's Greed). The
//! engine's draw is `crate::draw::draw`; this module's `draw` is the effect verb.

use serde::{Deserialize, Serialize};

use crate::draw::{DrawOutcome, complete_draw, draw_blocked};
use crate::effects::targets::{PlayerSpec, player_of};
use crate::script::Effect;

/// `draw`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DrawFromLibraryArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub def_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    /// R1061: once the drawn card is in its holder's hand it is made Radiant; one the hand cap burned or cast on draw stays as it was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
}

/// §6.3 Draw of a card a script has NAMED rather than the top of the library. Not `addToHand`, which
/// makes a second copy: it shares `crate::draw`'s pipeline, so the card leaves the library,
/// `state.counters.drawn` moves (R55), `drawn` fires, the hand cap burns (§2.4, R4) and a cast-on-draw
/// card casts (R58). `defId` takes the first of that definition from the top; a spec matching nothing
/// fizzles; several cards are several of these, each its own draw (R135).
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

        // Out of the pile first, so a cast-on-draw card resolves against a library without it.
        let card = ctx.sink.state.players[player].library.remove(at);
        let drawn_id = card.id.clone();
        if complete_draw(ctx, player, card, None) == DrawOutcome::Drawn
            && args.radiant == Some(true)
            && let Some(drawn) = crate::state::find_instance(ctx.state, &drawn_id).cloned()
        {
            crate::effects::radiant::make_radiant(ctx, &drawn);
        }
    })
}
