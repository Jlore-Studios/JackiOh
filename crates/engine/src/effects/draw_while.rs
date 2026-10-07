//! "Draw until …" (Classic #46 Divine Favor, SPEC §8.6 row 46): one draw at a time while a condition the
//! card reads holds, asked again before each draw, ending at the first draw that adds no card to the
//! hand — a fatigue hit, a burn at the hand cap, a card cast on draw (R58), a draw a draw limit stops
//! (§2.4) — so it never loops. A card-specific verb of the cards-classic-b workstream.
//!
//! Each draw is §2.4's own (`draw.draw`). A cast on draw ends it whatever its chain then draws, and a
//! cast that asks a question leaves the rest of its chain owed to the answer (R113) and ends it too.
//! The hand holds at most `HAND_CAP` cards, which bounds the loop.
//!
//! Port of `packages/engine/src/effects/drawWhile.ts`.

use std::sync::Arc;

use crate::config::HAND_CAP;
use crate::draw::DrawOutcome;
use crate::effects::targets::{PlayerSpec, player_of};
use crate::script::{Effect, EffectContext};

/// `drawWhile`'s condition, asked before each draw.
pub type DrawWhileMore = Arc<dyn Fn(&mut EffectContext<'_>) -> bool + Send + Sync>;

/// `drawWhile`'s argument.
#[derive(Clone)]
pub struct DrawWhileArgs {
    pub more: DrawWhileMore,
    pub player: Option<PlayerSpec>,
}

/// Draw one card at a time while `more(ctx)` holds, stopping at a draw that adds no card to the hand.
pub fn draw_while(args: DrawWhileArgs) -> Effect {
    Effect::new("drawWhile", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        // TS compared the prompt objects; a prompt's id names it alone.
        let asked = ctx.sink.state.pending.as_ref().map(|pending| pending.id.clone());
        for _draws in 0..=HAND_CAP {
            if ctx.sink.state.result.is_some() || !(args.more)(ctx) {
                return;
            }
            let held = ctx.sink.state.players[player].hand.len();
            let outcomes = crate::draw::draw(ctx, player, 1);
            let now = ctx.sink.state.pending.as_ref().map(|pending| pending.id.clone());
            if now != asked {
                return;
            }
            let added = matches!(
                outcomes.first(),
                Some(DrawOutcome::Drawn) | Some(DrawOutcome::Token)
            );
            if !added || ctx.sink.state.players[player].hand.len() <= held {
                return;
            }
        }
    })
}
