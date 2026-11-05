//! The opponent's aim (R738) and choice hover (R2739001): the two cosmetic relays, and what each may
//! show. Cosmetic frames never reach `reduce` or the log; the machine relays them to the opponent at
//! most once per `AIM_RELAY_INTERVAL_MS` per seat, coalescing faster ones (`DuelHost::receive_aim`,
//! `DuelHost::receive_hover`), after checking them against what the receiver may see.

use serde::{Deserialize, Serialize};

use jackioh_engine::{Aim, AimEnd, HandView, PendingView, PlayerId, PlayerView};

/// R738: one seat's aim relay.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AimSlot {
    /// Newest aim received: absent if none, `null` if aim ended.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::absent_or_null"
    )]
    #[cfg_attr(feature = "ts", ts(optional, type = "Aim | null"))]
    pub pending: Option<Option<Aim>>,
    /// The id of the one wait armed to relay it.
    pub timer: Option<u32>,
    /// Timestamp of the last relay, or `None` if never sent.
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub last_at: Option<i64>,
    /// What the opponent was last told, so a repeat is never sent and a seat that leaves mid-aim has
    /// its arrow cleared.
    pub relayed: Option<Aim>,
}

/// R2739001: one seat's choice-hover relay, [`AimSlot`]'s shape.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct HoverSlot {
    /// Newest index received: absent if none, `null` if the hover ended.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::absent_or_null"
    )]
    #[cfg_attr(feature = "ts", ts(optional, type = "number | null"))]
    pub pending: Option<Option<i32>>,
    pub timer: Option<u32>,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub last_at: Option<i64>,
    /// What the opponent was last told, so a repeat is never sent and an answered prompt, a seat that
    /// leaves or the game's end has its outline cleared.
    pub relayed: Option<i32>,
    /// The id of the seat's own prompt the index points into, so a hover never outlives it.
    pub prompt: Option<String>,
}

/// R738, R97, R177: checks whether the aim's endpoints are visible to the receiving viewer (a hero or
/// a zone is public, a face-down card named by its zone; a hand card is checked by index in the
/// sender's hand).
pub fn aim_is_public(aim: &Aim, sender: PlayerId, view: &PlayerView) -> bool {
    match &aim.source {
        AimEnd::Hand { player, index } => {
            if *player != sender {
                return false;
            }
            let hand = if sender == view.viewer {
                &view.you.hand
            } else {
                &view.opponent.hand
            };
            let count = match hand {
                HandView::Cards(cards) => cards.len(),
                HandView::Count { count } => usize::try_from(*count).unwrap_or(0),
            };
            if *index >= count {
                return false;
            }
        }
        source => {
            if !end_is_public(source, view) {
                return false;
            }
        }
    }
    match &aim.target {
        None => true,
        Some(AimEnd::Hand { .. }) => false,
        Some(target) => end_is_public(target, view),
    }
}

fn end_is_public(end: &AimEnd, view: &PlayerView) -> bool {
    match end {
        AimEnd::Hand { .. } => false,
        AimEnd::Hero { .. } => true,
        AimEnd::Zone { player, row, lane } => {
            let side = if *player == view.viewer {
                &view.you
            } else {
                &view.opponent
            };
            i64::from(*lane) <= side.locks.row(*row).len() as i64
        }
    }
}

/// R2739001, R2739000: whether `view` draws a back at `index` for `sender`'s open card picker: the
/// prompt is the sender's, its view carries the option count, and the index is under it.
pub fn hover_is_shown(index: i32, sender: PlayerId, view: &PlayerView) -> bool {
    match &view.pending {
        Some(PendingView::Elsewhere(elsewhere)) => {
            elsewhere.pending_for == sender
                && elsewhere
                    .option_count
                    .is_some_and(|count| (0..count).contains(&index))
        }
        _ => false,
    }
}
