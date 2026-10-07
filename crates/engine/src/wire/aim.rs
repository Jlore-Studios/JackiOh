//! The opponent's aim (SPEC §9.5, R738): what a player is aiming a play, an Activate or an attack
//! at, as the opponent's board draws it — Hearthstone's targeting arrow, shown to both players.
//!
//! Cosmetic, like an emote (R643): an aim is never an `ActionBody`, never reaches `reduce`, the
//! action log, the replay hash or a game record, is never part of `PlayerView`, and no rule is ever
//! decided from it (CLAUDE.md rule 7). This module holds only the shape both ends of the wire agree
//! on, and its shape check, because `apps/web` and the server may not import each other.
//!
//! Every end is a public handle, so nothing hidden can ride on it (R97, R177):
//!  - a hero, by its seat;
//!  - a zone of the field, by seat, row and lane — a face-down backrow card is named only by the
//!    zone it lies in, and a card on the field is never named by its instance id;
//!  - a card in a hand, by its position in that hand, which the opponent's view draws as the card
//!    back at that position — never its instance id, its definition or anything on its face.
//!
//! Port of `packages/shared/src/aim.ts` (SURFACE §4.1, §10.4: the web keeps its TS copy,
//! `apps/web/src/wire/aim.ts`; the server reads aims through this one).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::wire::catalog_types::{PlayerId, Row};

/// One end of an aim, discriminated on `at`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "at", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AimEnd {
    Hero { player: PlayerId },
    Zone { player: PlayerId, row: Row, lane: i32 },
    Hand { player: PlayerId, index: usize },
}

/// One aim: where it starts and what it is over now. `target` is null while the aim is over
/// nothing it may land on, so the opponent sees what is being aimed with and not yet at what.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct Aim {
    pub source: AimEnd,
    pub target: Option<AimEnd>,
}

fn is_record(value: &Value) -> Option<&Map<String, Value>> {
    // `typeof value === "object" && value !== null && !Array.isArray(value)`.
    value.as_object()
}

fn is_player_id(value: Option<&Value>) -> Option<PlayerId> {
    match value.and_then(Value::as_str) {
        Some("p1") => Some(PlayerId::P1),
        Some("p2") => Some(PlayerId::P2),
        _ => None,
    }
}

fn is_row(value: Option<&Value>) -> Option<Row> {
    match value.and_then(Value::as_str) {
        Some("units") => Some(Row::Units),
        Some("backrow") => Some(Row::Backrow),
        _ => None,
    }
}

/// `typeof value === "number" && Number.isInteger(value) && value >= least`: any JSON number whose
/// value is a whole number at least `least` (JS reads `3.0` as the integer 3).
fn is_index(value: Option<&Value>, least: i64) -> Option<i64> {
    let number = value?.as_f64()?;
    if number.is_finite() && number.fract() == 0.0 && number >= least as f64 {
        Some(number as i64)
    } else {
        None
    }
}

/// One end, rebuilt field by field so nothing else on the wire survives (an instance id, a def id),
/// or `None` when it is not one of the three handles. Lanes count from 1, as the board's do.
pub fn parse_aim_end(value: &Value) -> Option<AimEnd> {
    let record = is_record(value)?;
    let player = is_player_id(record.get("player"))?;
    match record.get("at").and_then(Value::as_str) {
        Some("hero") => Some(AimEnd::Hero { player }),
        Some("zone") => {
            let row = is_row(record.get("row"))?;
            let lane = is_index(record.get("lane"), 1)?;
            Some(AimEnd::Zone {
                player,
                row,
                lane: i32::try_from(lane).ok()?,
            })
        }
        Some("hand") => {
            let index = is_index(record.get("index"), 0)?;
            Some(AimEnd::Hand {
                player,
                index: usize::try_from(index).ok()?,
            })
        }
        _ => None,
    }
}

/// An aim, null (the aim has ended), or neither — the shape check the server parses a client's
/// frame with and the client parses a relay with. A hand is never a target: no play, Activate or
/// attack aims at a hand card through the arrow.
///
/// TS answers `Aim | null | undefined`; here `Some(Some(aim))` is an aim, `Some(None)` is null (the
/// aim has ended) and `None` is undefined (the value is not an aim).
pub fn parse_aim(value: &Value) -> Option<Option<Aim>> {
    if value.is_null() {
        return Some(None);
    }
    let record = is_record(value)?;
    let source = parse_aim_end(record.get("source").unwrap_or(&Value::Null))?;
    if let Some(Value::Null) = record.get("target") {
        return Some(Some(Aim { source, target: None }));
    }
    let target = parse_aim_end(record.get("target").unwrap_or(&Value::Null))?;
    if let AimEnd::Hand { .. } = target {
        return None;
    }
    Some(Some(Aim {
        source,
        target: Some(target),
    }))
}

/// A stable key for an aim, so two equal aims compare equal (the sender sends only changes).
pub fn aim_key(aim: Option<&Aim>) -> String {
    let Some(aim) = aim else {
        return "none".to_string();
    };
    let target = match &aim.target {
        None => "none".to_string(),
        Some(target) => end_key(target),
    };
    format!("{}>{}", end_key(&aim.source), target)
}

fn end_key(end: &AimEnd) -> String {
    match end {
        AimEnd::Hero { player } => format!("hero:{player}"),
        AimEnd::Zone { player, row, lane } => format!("zone:{player}:{row}:{lane}"),
        AimEnd::Hand { player, index } => format!("hand:{player}:{index}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_each_handle_and_drops_everything_else() {
        let parsed = parse_aim(&json!({
            "source": { "at": "hand", "player": "p1", "index": 2, "instanceId": "c9" },
            "target": { "at": "zone", "player": "p2", "row": "backrow", "lane": 3.0, "defId": "core-002" }
        }));
        let aim = Aim {
            source: AimEnd::Hand {
                player: PlayerId::P1,
                index: 2,
            },
            target: Some(AimEnd::Zone {
                player: PlayerId::P2,
                row: Row::Backrow,
                lane: 3,
            }),
        };
        assert_eq!(parsed, Some(Some(aim)));
        assert_eq!(
            serde_json::to_value(aim).unwrap(),
            json!({
                "source": { "at": "hand", "player": "p1", "index": 2 },
                "target": { "at": "zone", "player": "p2", "row": "backrow", "lane": 3 }
            })
        );
        assert_eq!(aim_key(Some(&aim)), "hand:p1:2>zone:p2:backrow:3");
    }

    #[test]
    fn null_ends_an_aim_and_anything_else_is_not_one() {
        assert_eq!(parse_aim(&Value::Null), Some(None));
        assert_eq!(aim_key(None), "none");
        let open = parse_aim(&json!({ "source": { "at": "hero", "player": "p2" }, "target": null }));
        assert_eq!(
            open,
            Some(Some(Aim {
                source: AimEnd::Hero { player: PlayerId::P2 },
                target: None
            }))
        );
        assert_eq!(aim_key(open.unwrap().as_ref()), "hero:p2>none");
        // A missing target is not null.
        assert_eq!(
            parse_aim(&json!({ "source": { "at": "hero", "player": "p2" } })),
            None
        );
        // A hand is never a target.
        assert_eq!(
            parse_aim(&json!({
                "source": { "at": "hero", "player": "p1" },
                "target": { "at": "hand", "player": "p2", "index": 0 }
            })),
            None
        );
        // Lanes count from 1; indexes from 0; whole numbers only.
        assert_eq!(
            parse_aim_end(&json!({ "at": "zone", "player": "p1", "row": "units", "lane": 0 })),
            None
        );
        assert_eq!(
            parse_aim_end(&json!({ "at": "hand", "player": "p1", "index": 1.5 })),
            None
        );
        assert_eq!(parse_aim_end(&json!({ "at": "hero", "player": "p3" })), None);
        assert_eq!(parse_aim(&json!([1, 2])), None);
        assert_eq!(parse_aim(&json!("aim")), None);
    }
}
