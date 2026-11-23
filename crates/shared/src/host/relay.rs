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

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::PlayerId::{P1, P2};
    use jackioh_engine::wire::Row;
    use serde_json::{Value, json};

    /// A side whose rows have `zones` zones each, holding `hand`.
    fn side(player: &str, zones: usize, hand: Value) -> Value {
        let flags = vec![false; zones];
        let empty = vec![Value::Null; zones];
        json!({
            "player": player,
            "hero": { "health": 30, "armor": 0, "powers": [] },
            "modifiers": [],
            "mana": { "current": 4, "max": 4 },
            "hand": hand,
            "libraryCount": 12,
            "graveyard": [], "exile": [], "resolving": [],
            "units": empty, "backrow": empty,
            "locks": { "units": flags, "backrow": flags },
            "reserved": { "units": flags, "backrow": flags },
            "fatigueCount": 0,
        })
    }

    /// p2's view: its own five-zone rows and an empty hand, p1's rows of `p1_zones` and three cards in hand,
    /// and `pending`.
    fn p2_view(p1_zones: usize, pending: Value) -> PlayerView {
        serde_json::from_value(json!({
            "viewer": "p2", "turn": 3, "active": "p1", "phase": "main",
            "you": side("p2", 5, json!([])),
            "opponent": side("p1", p1_zones, json!({ "count": 3 })),
            "pending": pending, "events": [], "result": null, "clockMs": null,
        }))
        .expect("a PlayerView")
    }

    fn aim(source: AimEnd, target: Option<AimEnd>) -> Aim {
        Aim { source, target }
    }

    /// R738, R97: a hand card aims from an index of its sender's hand as the viewer counts it, the other seat's
    /// by its count; a zone is public while the side it names has that lane in that row.
    #[test]
    fn r738_an_aims_ends_are_public_by_the_senders_hand_and_the_named_sides_zones() {
        let view = p2_view(3, Value::Null);
        let hand = |player, index| AimEnd::Hand { player, index };
        assert!(aim_is_public(&aim(hand(P1, 2), None), P1, &view));
        assert!(!aim_is_public(&aim(hand(P1, 3), None), P1, &view));
        assert!(!aim_is_public(&aim(hand(P2, 0), None), P2, &view));
        let zone = |player, lane| AimEnd::Zone {
            player,
            row: Row::Units,
            lane,
        };
        let hero = AimEnd::Hero { player: P1 };
        assert!(aim_is_public(&aim(zone(P2, 5), Some(hero)), P2, &view));
        assert!(!aim_is_public(&aim(zone(P1, 5), Some(hero)), P1, &view));
        assert!(aim_is_public(&aim(hero, Some(zone(P1, 3))), P2, &view));
        assert!(!aim_is_public(&aim(hero, Some(zone(P1, 4))), P2, &view));
    }

    /// R2739001, R2739000: a hover is shown over the sender's own open picker only, at an index under the count
    /// the view gives it.
    #[test]
    fn r2739001_a_hover_is_shown_only_over_the_senders_picker_under_its_count() {
        let elsewhere = |count: Option<i32>| {
            p2_view(
                5,
                json!({ "forYou": false, "pendingFor": "p1", "optionCount": count }),
            )
        };
        let view = elsewhere(Some(3));
        assert!(hover_is_shown(0, P1, &view) && hover_is_shown(2, P1, &view));
        assert!(!hover_is_shown(3, P1, &view) && !hover_is_shown(-1, P1, &view));
        assert!(!hover_is_shown(0, P2, &view));
        assert!(!hover_is_shown(0, P1, &elsewhere(None)));
        assert!(!hover_is_shown(0, P1, &p2_view(5, Value::Null)));
    }
}
