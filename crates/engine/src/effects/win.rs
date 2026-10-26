//! Win effects: winning outright and holding an alternative win (SPEC §2.5, ME-WIN, R848–R850), and
//! issue #1843's two endings, a vote to remove a player and a flight from the game (R1843000–R1843002).
//!
//! Port of nothing shipped: Meditative #8 and #20 are the first cards that win the game another
//! way, and Stinkyscape's Brains (#18.1–#18.3) and Edge of the Universe (#148.1) the first that vote
//! a player out and flee. A card file reaches these as
//! `jackioh_engine::effects::{win_game, alt_win, vote_out, flee}` or through `jackioh_engine::prelude`.
//! Each only records what it did; the state check's game-end point ends the game
//! (`win_conditions::win_check`).

use serde::{Deserialize, Serialize};

use crate::effects::targets::{PlayerSpec, player_of};
use crate::script::Effect;
use crate::state::{AltWinCondition, ModifierExpiry, ModifierKind};
use crate::wire::{Ballot, FleeOpponent};

/// `winGame`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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

/// `voteOut`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct VoteOutArgs {
    /// Whom the vote would remove. Default the card's controller ("votes to remove you").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// R1843000: this card votes to remove the player from the game. The tally is the engine's, per player
/// and per turn: each card definition counts once a turn, and once `VOTE_TALLY_SIZE` have voted the
/// state check that closes this effect list removes the player, a loss for them (R1843001). A card
/// writes its condition as its hook's own `if` ("votes to remove you if the enemy has more Units than
/// you") and returns this verb when it holds. A vote with no card behind it, or once the game is over,
/// counts nothing.
pub fn vote_out(args: VoteOutArgs) -> Effect {
    Effect::new("voteOut", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let Some(card) = ctx.self_.as_ref() else {
            return;
        };
        let ballot = Ballot {
            instance_id: card.id.clone(),
            def_id: card.def_id.clone(),
        };
        crate::win_conditions::cast_vote(&mut ctx.sink, player, ballot);
    })
}

/// `flee`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FleeArgs {
    /// Who flees. Default the card's controller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    /// What it leaves the other player. Default a draw (`fled`); Edge of the Universe's Radiant face
    /// passes the fleer's choice (a `mode` prompt the card asks first, R1843003).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opponent: Option<FleeOpponent>,
}

/// R1843002: the player escapes the universe and wins the game, leaving the other player a draw, or a
/// loss when `opponent` says so. Like `win_game` it takes hold at the state check that closes this
/// effect list (R59), where a hero at 0 still loses first (R850). Once the game is over it does nothing.
pub fn flee(args: FleeArgs) -> Effect {
    Effect::new("flee", move |ctx| {
        if ctx.sink.state.result.is_some() {
            return;
        }
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        ctx.sink.state.players[player].fled = Some(args.opponent.unwrap_or(FleeOpponent::Draw));
    })
}
