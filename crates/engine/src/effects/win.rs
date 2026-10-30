//! Win effects: winning outright and holding an alternative win (SPEC §2.5, ME-WIN, R848–R850), and
//! issue #1843's two endings, a vote to remove a player and a flight from the game (R1843000–R1843002;
//! Meditative #8, #20, Stinkyscape #18.1–#18.3, #148.1). Each only records what it did; the state
//! check's game-end point ends the game (`win_conditions::win_check`).

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

/// R1870000 (Stinkyscape #4 Olivian Expanse): a card whose `memory` holds `true` under this key wins
/// its player the game as it is played, a cast too (R70), at §10.5 step 4 (`play_steps::place_card`).
/// The key rides that one instance, never a copy or Transform, and goes with its memory (R215).
pub const WIN_ON_PLAY_KEY: &str = "winOnPlay";

/// R850: win the game outright, at the state check closing this list (R59); a hero at 0 still loses first.
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

/// R1843000: this card votes to remove the player from the game. Each card definition counts once a
/// turn per player, and once `VOTE_TALLY_SIZE` have voted the closing state check removes the player,
/// a loss for them (R1843001). The card's hook holds the condition as its own `if`.
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
/// loss when `opponent` says so; like `win_game`, it takes hold at the closing state check (R59, R850).
pub fn flee(args: FleeArgs) -> Effect {
    Effect::new("flee", move |ctx| {
        if ctx.sink.state.result.is_some() {
            return;
        }
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        ctx.sink.state.players[player].fled = Some(args.opponent.unwrap_or(FleeOpponent::Draw));
    })
}
