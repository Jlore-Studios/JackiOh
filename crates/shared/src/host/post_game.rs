//! What a seat's open match socket is told after the result (SPEC §9.5, R3213000): the frames that stand
//! in for the post-game polls while the socket is up. A game of no series is told its rematch status, a
//! Conquest game its series' view; each holds exactly what that seat's own `GET /api/matches/:id/rematch`
//! or `GET /api/matches/:id/series` answers, so the opponent's lean and the opponent's sealed pick never
//! ride it (CLAUDE.md rule 7, R336). `v` is the host's frame version (the server's
//! `POST_GAME_FRAME_VERSION`): a client reads only the one it knows and polls otherwise, and a client
//! older than the frames ignores both `type`s.

use serde::{Deserialize, Serialize};

use super::rematch::RematchStakes;
use crate::series::SeriesView;

/// R1372: the mode a rematch of the match plays, as its status names it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum RematchMode {
    Bo1,
    Random,
}

/// R3213000: one post-game frame for one seat.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PostGameFrame {
    /// R672, R2685000: the seat's own offer, the opponent's, whether the opponent is here and the game
    /// two equal offers made, as the rematch status answers them.
    Rematch {
        v: u32,
        you_offered: Option<RematchStakes>,
        opponent_offer: Option<RematchStakes>,
        opponent_here: bool,
        match_id: Option<String>,
        mode: RematchMode,
    },
    /// R336, R3205000: the seat's own view of the series the match is a game of.
    Series { v: u32, series: SeriesView },
}
