//! The `/ws/duos` frames (fullsend SPEC S6, B35, B39), as `actor/protocol.rs` is `/ws/match`'s.
//!
//! Client to server: `hello` (which match), `action` (a nonce and a `DuosActionBody`) and `ping`.
//! Server to client: `view` (the seat's `DuosView` and, beside it, that same seat's legal actions),
//! `ack`, `error`, `clock`, `teammate` and `ping`.
//!
//! Parsing is total and rebuilds every field it keeps. A `duel` body is the 1v1 action union, so it
//! goes through the 1v1 parser (`actor::protocol::parse_client_message`) itself: the same whitelist,
//! the same refusal of the server-only types (R79's `timeout`, `disconnectExpired`, `ceilingReached`,
//! R1127's `emote`) and of a smuggled `playerId`, and the same nonce rules (R270: a nonce starting
//! `srv-` is the server's), and one more: a nonce starting `duos:` is the engine's own, the one its
//! round's close reaches the duels under (`DUOS_NONCE_PREFIX`), which the engine refuses from a seat
//! too. The seat is never the client's: the actor stamps the authenticated one.
//!
//! A ping is cosmetic, like an emote or an aim (R643, R738): a top-level frame, never an action, so it
//! never reaches the reducer, the log, a replay or a view (B39). Its shape is checked here; whether
//! its hand target names the sender's own side and a card that is there is the actor's call, against
//! the live state.

use serde::Serialize;
use serde_json::{Map, Value, json};

use jackioh_engine::PlayerId;
use jackioh_engine::config::{BACKROW_ZONES, UNIT_ZONES};
use jackioh_engine::duos::DUOS_NONCE_PREFIX;
use jackioh_engine::wire::{DuosActionBody, DuosSeat, DuosView};

use crate::actor::protocol::{
    ClientMessage, MAX_FRAME_BYTES, MAX_NONCE_LENGTH, MalformedMessage, SERVER_NONCE_PREFIX, SocketErrorCode,
    parse_client_message,
};

// ---------------------------------------------------------------------------
// Pings (S6, B39, B64)
// ---------------------------------------------------------------------------

/// S6's `PingKind`.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PingKind {
    #[serde(rename = "thumbsUp")]
    ThumbsUp,
    #[serde(rename = "thumbsDown")]
    ThumbsDown,
    #[serde(rename = "portal")]
    Portal,
    #[serde(rename = "careful")]
    Careful,
    #[serde(rename = "killThis")]
    KillThis,
}

impl PingKind {
    pub const ALL: [PingKind; 5] = [
        PingKind::ThumbsUp,
        PingKind::ThumbsDown,
        PingKind::Portal,
        PingKind::Careful,
        PingKind::KillThis,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            PingKind::ThumbsUp => "thumbsUp",
            PingKind::ThumbsDown => "thumbsDown",
            PingKind::Portal => "portal",
            PingKind::Careful => "careful",
            PingKind::KillThis => "killThis",
        }
    }

    pub fn parse(text: &str) -> Option<PingKind> {
        PingKind::ALL.into_iter().find(|kind| kind.as_str() == text)
    }
}

/// What a ping points at (S6's `PingTarget`'s `at`).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PingAt {
    Hero,
    Unit,
    Backrow,
    Hand,
}

/// S6's `PingTarget`: `{duel, at: "hero", side}`, `{duel, at: "unit" | "backrow", side, lane}` or
/// `{duel, at: "hand", side, index}`, written back in that shape.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PingTarget {
    /// 0 or 1.
    pub duel: u8,
    pub at: PingAt,
    pub side: PlayerId,
    /// A zone of the row, under `UNIT_ZONES` or `BACKROW_ZONES`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lane: Option<usize>,
    /// A position in the hand, under its size (the actor checks the size).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
}

// ---------------------------------------------------------------------------
// Client -> server
// ---------------------------------------------------------------------------

/// A frame a co-op socket may send.
#[derive(Debug)]
pub enum DuosClientMessage {
    /// S6: the match this socket is for. Before the socket is attached it chooses the match; after,
    /// it asks for a fresh view (§9.5: a reconnect gets a fresh view, never a log replay).
    Hello { match_id: Option<String> },
    /// S6: one action for the authenticated seat.
    Action { nonce: String, body: DuosActionBody },
    /// S6, B39: a ping for the teammate.
    Ping { ping: PingKind, target: PingTarget },
}

fn malformed(reason: impl Into<String>) -> MalformedMessage {
    MalformedMessage {
        reason: reason.into(),
    }
}

/// A JSON number with no fraction, as the 1v1 parser reads one (JS reads `3.0` as 3).
fn whole(value: Option<&Value>) -> Option<f64> {
    let number = value?.as_f64()?;
    (number.is_finite() && number.fract() == 0.0).then_some(number)
}

/// A whole number in `0..below`.
fn index_under(value: Option<&Value>, below: usize) -> Option<usize> {
    let number = whole(value)?;
    if number < 0.0 || number >= below as f64 {
        return None;
    }
    Some(number as usize)
}

fn side_of(value: Option<&Value>) -> Option<PlayerId> {
    match value?.as_str()? {
        "p1" => Some(PlayerId::P1),
        "p2" => Some(PlayerId::P2),
        _ => None,
    }
}

/// The two duels (S2: duel 0 is A1 against B1, duel 1 A2 against B2).
const DUELS: usize = 2;

fn parse_target(value: &Value) -> Option<PingTarget> {
    let record = value.as_object()?;
    let duel = u8::try_from(index_under(record.get("duel"), DUELS)?).ok()?;
    let side = side_of(record.get("side"))?;
    let zones = |count: i32| usize::try_from(count).unwrap_or(0);
    let (at, lane, index) = match record.get("at")?.as_str()? {
        "hero" => (PingAt::Hero, None, None),
        "unit" => (
            PingAt::Unit,
            Some(index_under(record.get("lane"), zones(UNIT_ZONES))?),
            None,
        ),
        "backrow" => (
            PingAt::Backrow,
            Some(index_under(record.get("lane"), zones(BACKROW_ZONES))?),
            None,
        ),
        "hand" => (
            PingAt::Hand,
            None,
            Some(index_under(record.get("index"), usize::MAX)?),
        ),
        _ => return None,
    };
    Some(PingTarget {
        duel,
        at,
        side,
        lane,
        index,
    })
}

/// R270 and §9.8's nonce rules, as the 1v1 parser holds them, and the Duos wrapper's own namespace.
fn check_nonce(value: Option<&Value>) -> Result<String, MalformedMessage> {
    let nonce = match value.and_then(Value::as_str) {
        Some(nonce) if !nonce.is_empty() => nonce,
        _ => return Err(malformed("every action carries a client nonce (SPEC §9.3)")),
    };
    if nonce.encode_utf16().count() > MAX_NONCE_LENGTH {
        return Err(malformed("that nonce is too long"));
    }
    if nonce.starts_with(SERVER_NONCE_PREFIX) {
        return Err(malformed(format!(
            r#"a nonce starting "{SERVER_NONCE_PREFIX}" is the server's own (R270)"#
        )));
    }
    if nonce.starts_with(DUOS_NONCE_PREFIX) {
        return Err(malformed(format!(
            r#"a nonce starting "{DUOS_NONCE_PREFIX}" is the round's own"#
        )));
    }
    Ok(nonce.to_string())
}

/// S2's `DuosActionBody`, rebuilt field by field. A `duel` body is parsed by the 1v1 parser, so
/// exactly what a 1v1 socket may send reaches the duel.
fn parse_body(raw: &Value, nonce: &str) -> Result<DuosActionBody, MalformedMessage> {
    let Some(record) = raw.as_object() else {
        return Err(malformed(r#""body" must be an object"#));
    };
    match record.get("kind").and_then(Value::as_str) {
        Some("duel") => {
            let Some(inner) = record.get("body").and_then(Value::as_object) else {
                return Err(malformed(r#""body.body" must be an action object"#));
            };
            let mut action: Map<String, Value> = inner.clone();
            action.insert("nonce".to_string(), Value::String(nonce.to_string()));
            let frame = json!({ "type": "action", "action": Value::Object(action) });
            match parse_client_message(&frame.to_string()) {
                Ok(ClientMessage::Action(parsed)) => Ok(DuosActionBody::Duel { body: parsed.body }),
                Ok(_) => Err(malformed(r#""body.body" must be an action"#)),
                Err(refused) => Err(refused),
            }
        }
        Some("pass") => match record.get("instanceId").and_then(Value::as_str) {
            Some(instance_id) if !instance_id.is_empty() => Ok(DuosActionBody::Pass {
                instance_id: instance_id.to_string(),
            }),
            _ => Err(malformed(r#""pass.instanceId" must be a string"#)),
        },
        Some("concede") => Ok(DuosActionBody::Concede),
        _ => Err(malformed(r#""body.kind" must be "duel", "pass" or "concede""#)),
    }
}

/// Total, never panics, and every field it keeps is rebuilt by hand.
pub fn parse_duos_message(text: &str) -> Result<DuosClientMessage, MalformedMessage> {
    if text.encode_utf16().count() > MAX_FRAME_BYTES {
        return Err(malformed("that frame is too large"));
    }
    let Ok(parsed) = serde_json::from_str::<Value>(text) else {
        return Err(malformed("every frame must be JSON"));
    };
    let Some(parsed) = parsed.as_object() else {
        return Err(malformed("every frame must be a JSON object"));
    };
    match parsed.get("type").and_then(Value::as_str) {
        Some("hello") => match parsed.get("matchId") {
            None | Some(Value::Null) => Ok(DuosClientMessage::Hello { match_id: None }),
            Some(Value::String(match_id)) if !match_id.is_empty() => Ok(DuosClientMessage::Hello {
                match_id: Some(match_id.clone()),
            }),
            Some(_) => Err(malformed(r#""hello.matchId" must be a string"#)),
        },
        Some("action") => {
            let nonce = check_nonce(parsed.get("nonce"))?;
            let Some(body) = parsed.get("body") else {
                return Err(malformed(r#""body" must be an object"#));
            };
            let body = parse_body(body, &nonce)?;
            Ok(DuosClientMessage::Action { nonce, body })
        }
        Some("ping") => {
            let Some(ping) = parsed
                .get("ping")
                .and_then(Value::as_str)
                .and_then(PingKind::parse)
            else {
                return Err(malformed(
                    r#""ping" must be "thumbsUp", "thumbsDown", "portal", "careful" or "killThis""#,
                ));
            };
            let Some(target) = parsed.get("target").and_then(parse_target) else {
                return Err(malformed(
                    r#""target" must be { duel, at: "hero", side }, { duel, at: "unit" | "backrow", side, lane } or { duel, at: "hand", side, index }"#,
                ));
            };
            Ok(DuosClientMessage::Ping { ping, target })
        }
        Some(other) => Err(malformed(format!(r#""{other}" is not a co-op message"#))),
        None => Err(malformed(r#""type" must be a string"#)),
    }
}

// ---------------------------------------------------------------------------
// Server -> client
// ---------------------------------------------------------------------------

/// S6's `graceDeadlines`: each person's disconnect grace, epoch ms, or null.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraceDeadlines {
    pub a1: Option<i64>,
    pub a2: Option<i64>,
}

/// Every frame the co-op socket sends, tagged on `type`.
#[derive(Serialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DuosServerMessage {
    /// B23, B35: the seat's own `duos_view_for` and `duos_legal_actions` for that same seat, read off
    /// one state, so the array is always true of the view it travels with.
    View {
        view: Box<DuosView>,
        legal: Vec<DuosActionBody>,
    },
    /// §9.3: the nonce accepted and the log seq it was written at.
    Ack { nonce: String, seq: i64 },
    /// `nonce` is present exactly when the failure belongs to an action the client sent.
    Error {
        code: SocketErrorCode,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        nonce: Option<String>,
    },
    /// S6: the deadline the seat is playing against (its round's, its prompt's, or the mulligans'),
    /// both people's disconnect graces, and the server's `now` to measure them by.
    Clock {
        now: i64,
        round_deadline: Option<i64>,
        grace_deadlines: GraceDeadlines,
    },
    /// S6, B37: whether the teammate's socket is open, and whether the AI plays their seat.
    Teammate { connected: bool, ai_playing: bool },
    /// B39: the teammate's ping, relayed to this socket only.
    Ping {
        from: DuosSeat,
        ping: PingKind,
        target: PingTarget,
    },
}

pub fn ack_message(nonce: &str, seq: i64) -> DuosServerMessage {
    DuosServerMessage::Ack {
        nonce: nonce.to_string(),
        seq,
    }
}

pub fn error_message(code: SocketErrorCode, message: &str, nonce: Option<&str>) -> DuosServerMessage {
    DuosServerMessage::Error {
        code,
        message: message.to_string(),
        nonce: nonce.map(str::to_string),
    }
}

/// The frame's JSON text.
pub fn encode(message: &DuosServerMessage) -> String {
    serde_json::to_string(message).unwrap_or_else(|error| {
        format!(r#"{{"type":"error","code":"internal","message":"the frame could not be encoded: {error}"}}"#)
    })
}

#[cfg(test)]
mod tests {
    use jackioh_engine::ActionBody;

    use super::*;

    #[test]
    fn a_pass_and_a_concede_parse_and_the_seat_is_never_read() {
        let pass = parse_duos_message(
            r#"{"type":"action","nonce":"n1","seat":"b1","body":{"kind":"pass","instanceId":"i7"}}"#,
        );
        assert!(matches!(
            &pass,
            Ok(DuosClientMessage::Action { nonce, body: DuosActionBody::Pass { instance_id } })
                if nonce == "n1" && instance_id == "i7"
        ));
        let concede = parse_duos_message(r#"{"type":"action","nonce":"n2","body":{"kind":"concede"}}"#);
        assert!(matches!(
            &concede,
            Ok(DuosClientMessage::Action { nonce, body: DuosActionBody::Concede }) if nonce == "n2"
        ));
    }

    #[test]
    fn a_duel_body_goes_through_the_1v1_parser() {
        let end_turn = parse_duos_message(
            r#"{"type":"action","nonce":"n4","body":{"kind":"duel","body":{"type":"endTurn","playerId":"p2"}}}"#,
        );
        assert!(matches!(
            &end_turn,
            Ok(DuosClientMessage::Action {
                body: DuosActionBody::Duel {
                    body: ActionBody::EndTurn
                },
                ..
            })
        ));
    }

    #[test]
    fn a_server_nonce_and_a_server_only_duel_action_are_malformed() {
        assert!(
            parse_duos_message(r#"{"type":"action","nonce":"srv-1","body":{"kind":"concede"}}"#).is_err()
        );
        // R79, R1127: every server-only 1v1 body, as a client would smuggle it into a duel.
        for body in [
            r#"{"type":"timeout"}"#,
            r#"{"type":"disconnectExpired","player":"p2"}"#,
            r#"{"type":"ceilingReached"}"#,
            r#"{"type":"emote","emote":"hello"}"#,
        ] {
            let frame = format!(r#"{{"type":"action","nonce":"n3","body":{{"kind":"duel","body":{body}}}}}"#);
            assert!(parse_duos_message(&frame).is_err(), "{frame}");
        }
    }

    #[test]
    fn a_nonce_in_the_duos_wrappers_namespace_is_malformed_and_one_merely_like_it_is_not() {
        for nonce in ["duos:", "duos:begin", "duos:end:r1:d0:mine"] {
            let frame = format!(
                r#"{{"type":"action","nonce":"{nonce}","body":{{"kind":"duel","body":{{"type":"endTurn"}}}}}}"#
            );
            assert!(parse_duos_message(&frame).is_err(), "{frame}");
        }
        // The old round-end shape is an ordinary client nonce now: the round's own are `duos:`'s.
        let frame = r#"{"type":"action","nonce":"mine:end:r1:d0","body":{"kind":"concede"}}"#;
        assert!(parse_duos_message(frame).is_ok(), "{frame}");
    }

    #[test]
    fn a_ping_target_is_shape_checked() {
        let hero = parse_duos_message(
            r#"{"type":"ping","ping":"careful","target":{"duel":1,"at":"hero","side":"p2"}}"#,
        );
        assert!(matches!(
            hero,
            Ok(DuosClientMessage::Ping {
                ping: PingKind::Careful,
                ..
            })
        ));
        for bad in [
            r#"{"type":"ping","ping":"wave","target":{"duel":0,"at":"hero","side":"p1"}}"#,
            r#"{"type":"ping","ping":"portal","target":{"duel":2,"at":"hero","side":"p1"}}"#,
            r#"{"type":"ping","ping":"portal","target":{"duel":0,"at":"unit","side":"p1","lane":9}}"#,
            r#"{"type":"ping","ping":"portal","target":{"duel":0,"at":"hand","side":"p3","index":0}}"#,
            r#"{"type":"ping","ping":"portal","target":{"duel":0,"at":"hand","side":"p1","index":-1}}"#,
        ] {
            assert!(parse_duos_message(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_ping_frame_is_written_back_in_the_spec_shape() {
        let frame = DuosServerMessage::Ping {
            from: DuosSeat::A2,
            ping: PingKind::KillThis,
            target: PingTarget {
                duel: 0,
                at: PingAt::Unit,
                side: PlayerId::P2,
                lane: Some(3),
                index: None,
            },
        };
        assert_eq!(
            serde_json::from_str::<Value>(&encode(&frame)).ok(),
            Some(json!({
                "type": "ping",
                "from": "a2",
                "ping": "killThis",
                "target": { "duel": 0, "at": "unit", "side": "p2", "lane": 3 },
            }))
        );
    }
}
