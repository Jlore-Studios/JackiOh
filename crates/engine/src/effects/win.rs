//! Win effects: winning outright and holding an alternative win (SPEC §2.5, ME-WIN, R848–R850).
//!
//! Port of nothing shipped: Meditative #8 and #20 are the first cards that win the game another
//! way. A card file reaches these as `jackioh_engine::effects::{win_game, alt_win}` or through
//! `jackioh_engine::prelude`.

use serde::{Deserialize, Serialize};

use crate::effects::targets::{PlayerSpec, player_of};
use crate::script::Effect;
use crate::state::{AltWinCondition, ModifierExpiry, ModifierKind};

/// `winGame`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WinGameArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// R850: win the game outright for the player. The flag takes hold at the state check that closes
/// this effect list (R59), where a hero at 0 still loses first.
pub fn win_game(args: WinGameArgs) -> Effect {
    Effect::new("winGame", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        ctx.sink.state.players[player].won_by_effect = Some(true);
    })
}

/// `altWin`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AltWinArgs {
    pub condition: AltWinCondition,
    pub threshold: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// R848: hold a chosen alternative win condition for the rest of the game (never expiring, R458).
/// Several stack, each its own modifier, and any one met wins.
pub fn alt_win(args: AltWinArgs) -> Effect {
    Effect::new("altWin", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        crate::modifiers::add_modifier(
            &mut ctx.sink,
            player,
            ModifierExpiry::Never,
            ModifierKind::AltWin {
                condition: args.condition,
                threshold: args.threshold,
            },
        );
    })
}
