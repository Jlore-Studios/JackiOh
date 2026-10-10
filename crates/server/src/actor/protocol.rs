//! The WebSocket protocol (BUILD M6-T4, SPEC §9.1, §9.3, §10.8). Port of
//! `apps/server/src/match/protocol.ts`.
//!
//! BUILD M6-T4 fixes the message names — hello, view, action, ack, error, prompt, clock — and SPEC
//! fixes what they may carry:
//!
//!  - §9.1: "the client sends intent, never state", and the client may only read
//!    `viewFor(state, playerId)`. So `view` carries a `PlayerView` and, beside it, the *same
//!    viewer's* `legalActions(state, viewer)` — no `GameState`, no opponent hand, no library order,
//!    and never the other seat's array (see `ServerMessage::View`).
//!  - §9.3: "every action carries a client nonce, deduped server-side", and "`reduce` refuses
//!    illegal actions itself and returns the reason". `ack` reports the nonce and the log seq the
//!    action was written at; `error` relays the reducer's reason verbatim and never restates a rule.
//!  - §9.5: "the grace countdown is stored on the match so both clients show it". `clock` carries
//!    `MatchClocks` (the same shape `matches.clocks` persists — one shape, not two) plus the
//!    server's `now`, so a client renders remaining time without trusting its own clock skew.
//!  - §10.6, §10.8: with a prompt open the opponent "sees only that a prompt is open", so the
//!    `prompt` message is split on `forYou` and the non-holder's arm carries neither the choiceId
//!    nor the kind, let alone the options.
//!
//! The one security-relevant asymmetry: a client's `action` frame is parsed into `{ nonce, body }`
//! and its `playerId` is *discarded here*, not trusted and not forwarded. The actor stamps the
//! authenticated seat. A client therefore cannot submit an action as the other player even if it
//! puts `playerId: "p2"` on the wire (`actor.test.ts` asserts it).
//!
//! SURFACE §11.3's deltas from TS: the action nonce is read only inside `action` (the client's
//! form, `{"type":"action","action":{…,"nonce":"…"}}`), never from the frame's top level; and a
//! `joinRoom` frame is answered `malformed` (it was `unsupported`), so there is no `JoinRoomMessage`.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use jackioh_engine::{
    ActionBody, ActionType, Aim, EmoteId, PlagueSpend, PlayerId, PlayerView, PortraitId, PromptKind, Row,
    Selection, ZoneChoice, is_emote_id, parse_aim,
};

use crate::db::store::MatchClocks;

// ---------------------------------------------------------------------------
// Client -> server
// ---------------------------------------------------------------------------

/// The first frame on a socket. Authentication happens at the upgrade (`ws_server.rs`), so the
/// actor treats `hello` purely as "push me a fresh full view" (§9.5: "Reconnect gets a fresh full
/// view, never a log replay") and ignores every field on it — a token here can never re-seat a
/// socket that is already attached to a seat.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct HelloMessage {
    pub token: Option<String>,
    pub match_id: Option<String>,
    pub room_code: Option<String>,
}

/// An action the client wants applied. `playerId` never survives parsing (see the file header).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionMessage {
    pub nonce: String,
    pub body: ActionBody,
}

/// R643: a cosmetic emote. Top-level on purpose — it is NOT an `ActionBody`, so it never reaches
/// `reduce`, the action log or the replay hash, and it carries no nonce because there is nothing to
/// ack: a rate-limited emote is silently dropped, and silence is exactly what a drop needs (R643).
/// An `emote` value outside the pool's `EMOTE_IDS` is `malformed`, the only way this frame can fail
/// to parse; one in the pool but outside the sender's dealt hand parses, and the actor drops it as
/// silently as a rate-limited one (R1342).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmoteMessage {
    pub emote: EmoteId,
}

/// R738: what the sender is aiming a play, an Activate or an attack at, for the opponent's board to
/// draw — `None` (TS `null`) when the aim ends. Cosmetic like an emote: top-level, never an
/// `ActionBody`, so it never reaches `reduce`, the log or the replay hash, carries no nonce and is
/// never answered. Its ends are public handles only (`jackioh_engine::wire::aim`); anything else is
/// `malformed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AimMessage {
    pub aim: Option<Aim>,
}

/// TS `ClientMessage`, minus `joinRoom` (SURFACE §11.3: answered `malformed`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientMessage {
    Hello(HelloMessage),
    Action(ActionMessage),
    Emote(EmoteMessage),
    Aim(AimMessage),
}

/// TS `{ type: "malformed"; reason }`: what `parse_client_message` answers instead of a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MalformedMessage {
    pub reason: String,
}

/// R79: `timeout`, `disconnectExpired` and `ceilingReached` are server-only — "never sent by a
/// client" (`wire/actions.rs`). Accepting one from a socket would let a player end their
/// opponent's turn or the match, so parsing rejects them outright.
pub const SERVER_ONLY_ACTION_TYPES: &[ActionType] = &[
    ActionType::Timeout,
    ActionType::DisconnectExpired,
    ActionType::CeilingReached,
];

/// Everything else in the §10.2 union that a socket may carry: every type `legal_actions` can list
/// among them. R384's `activate` is the one every Activate ability and, since R752, every Heroic
/// Power's power is listed as, so a whitelist without it refused them all online (#491).
pub const CLIENT_ACTION_TYPES: &[ActionType] = &[
    ActionType::Mulligan,
    ActionType::Play,
    ActionType::Attack,
    ActionType::SwitchPosition,
    ActionType::Activate,
    ActionType::ActivatePower,
    ActionType::Answer,
    ActionType::OfferDraw,
    ActionType::AnswerDraw,
    ActionType::Concede,
    ActionType::EndTurn,
    ActionType::SetAutoEndTurn,
];

// ---------------------------------------------------------------------------
// Server -> client
// ---------------------------------------------------------------------------

/// TS `SocketErrorCode`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SocketErrorCode {
    /// The frame was not a protocol message.
    Malformed,
    /// §9.3: the reducer refused the action; `message` is its reason, relayed verbatim.
    IllegalAction,
    /// A server-only action type, or an action for a seat this socket does not hold.
    Forbidden,
    /// §9.8: the per-match action rate limit.
    RateLimited,
    /// A protocol message this endpoint does not serve.
    Unsupported,
    /// The match already has a result; no further action will be applied.
    MatchOver,
    Internal,
}

/// TS `ServerMessage`: every frame the server sends, tagged on `type`, keys in TS's order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ServerMessage {
    /// §10.8's `PlayerView`, plus the array the client greys the board out with.
    ///
    /// WHY `legal` RIDES HERE RATHER THAN IN A FRAME OF ITS OWN. §10.2: "`legalActions(state,
    /// playerId)` is exported and is what both the client UI and the My Pawn AI consume", and BUILD
    /// M5-T2: "The client never computes legality itself; it asks `legalActions` and greys out the
    /// rest." So the browser cannot render a usable board without it — with an empty array
    /// `end-turn` is disabled and no hand card is clickable. Two things settle where it goes:
    ///
    ///  - BUILD §1 fixes this file's message names as "hello, view, action, ack, error, prompt,
    ///    clock". A seventh name would be a protocol BUILD does not list; a field on a frame it does
    ///    list is not.
    ///  - The array is only ever true *of one view*. Carried together they can never disagree, and a
    ///    client cannot render a board narrowed by an array minted against a state one action older.
    ///
    /// It is NOT hidden information and it is not a second channel for any: `legalActions(state, p)`
    /// enumerates the actions `p` itself may take, from `p`'s own hand, units and backrow, against
    /// targets the same `PlayerView` already shows. The actor passes the viewer as the player
    /// (`push_view`), so a socket never sees the other seat's array — which would leak the
    /// opponent's hand by naming every `play` in it (§9.1's "Hidden: ... opponent hand").
    View {
        view: Box<PlayerView>,
        legal: Vec<ActionBody>,
    },
    /// §9.3: the nonce that was accepted and the append-only log seq it was written at.
    Ack { nonce: String, seq: i64 },
    /// `nonce` is present exactly when the failure belongs to an action the client sent.
    Error {
        code: SocketErrorCode,
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        nonce: Option<String>,
    },
    /// §10.6: exactly one prompt is open at a time, and the player who does not hold it learns only
    /// that it exists. `deadline` is the prompt clock (R79) and is public: both clients show it.
    /// TS's two arms in one variant: `for_you: true` carries `choiceId` and `kind`, `for_you: false`
    /// carries neither (both are then absent from the JSON, as TS wrote them).
    Prompt {
        for_you: bool,
        pending_for: PlayerId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        choice_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<PromptKind>,
        deadline: Option<i64>,
    },
    /// §9.5: the deadlines both clients render, including the per-player disconnect grace and the
    /// hard ceiling. `now` is the server's clock at send time, so the client computes remaining time
    /// as `deadline - now` against its own monotonic delta instead of trusting its wall clock.
    Clock { now: i64, clocks: MatchClocks },
    /// R642: both seats' hero portraits, sent on join and again on reconnect — a row on the match,
    /// never a field of `PlayerView` (portraits are cosmetic; `PlayerView` is a rules surface).
    /// R1342: `emotes` is the receiving account's own hand of `EMOTE_HAND_SIZE`, dealt from the match
    /// seed (`deal_emote_hand`), never the opponent's: a socket learns nothing it does not show.
    Portraits {
        p1: PortraitId,
        p2: PortraitId,
        emotes: Vec<EmoteId>,
    },
    /// R643: an opponent's emote, relayed. The sender already sees their own locally and gets
    /// nothing back — the one asymmetry this frame has.
    Emote { from: PlayerId, emote: EmoteId },
    /// R738: the opponent's aim, relayed — never to the sender, never part of `PlayerView`, and only
    /// once the actor has checked that every end names something the receiver may see.
    Aim { from: PlayerId, aim: Option<Aim> },
}

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

/// `legal` must be `legal_actions(state, view.viewer)`; the actor is the only caller (`push_view`).
pub fn view_message(view: PlayerView, legal: &[ActionBody]) -> ServerMessage {
    ServerMessage::View {
        view: Box::new(view),
        legal: legal.to_vec(),
    }
}

pub fn ack_message(nonce: &str, seq: i64) -> ServerMessage {
    ServerMessage::Ack {
        nonce: nonce.to_string(),
        seq,
    }
}

pub fn error_message(code: SocketErrorCode, message: &str, nonce: Option<&str>) -> ServerMessage {
    ServerMessage::Error {
        code,
        message: message.to_string(),
        nonce: nonce.map(str::to_string),
    }
}

pub fn clock_message(now: i64, clocks: MatchClocks) -> ServerMessage {
    ServerMessage::Clock { now, clocks }
}

/// R642, R1342: the portraits frame one account receives, carrying that account's own emote hand.
pub fn portraits_message(p1: PortraitId, p2: PortraitId, emotes: &[EmoteId]) -> ServerMessage {
    ServerMessage::Portraits {
        p1,
        p2,
        emotes: emotes.to_vec(),
    }
}

pub fn emote_relay_message(from: PlayerId, emote: EmoteId) -> ServerMessage {
    ServerMessage::Emote { from, emote }
}

pub fn aim_relay_message(from: PlayerId, aim: Option<Aim>) -> ServerMessage {
    ServerMessage::Aim { from, aim }
}

/// For the holder of the prompt: the choiceId it must answer and the kind to render.
pub fn prompt_for_you(
    pending_for: PlayerId,
    choice_id: &str,
    kind: PromptKind,
    deadline: Option<i64>,
) -> ServerMessage {
    ServerMessage::Prompt {
        for_you: true,
        pending_for,
        choice_id: Some(choice_id.to_string()),
        kind: Some(kind),
        deadline,
    }
}

/// For the other player: that a prompt is open, and nothing about it (§10.6).
pub fn prompt_for_opponent(pending_for: PlayerId, deadline: Option<i64>) -> ServerMessage {
    ServerMessage::Prompt {
        for_you: false,
        pending_for,
        choice_id: None,
        kind: None,
        deadline,
    }
}

/// R679: the close reason both sockets of a voided match carry, with `MATCH_VOIDED_CLOSE_CODE`
/// (`crate::config`). No new frame: the last `view` already shows the game over with reason
/// `voided`, and the close tells the client the match is gone rather than to reconnect.
pub const MATCH_VOIDED_CLOSE_REASON: &str = "voided";

/// TS `JSON.stringify(message)`.
pub fn encode(message: &ServerMessage) -> String {
    serde_json::to_string(message).unwrap_or_else(|error| {
        // Every field of every frame is plain data; this cannot fail. Answer a frame the client
        // can read rather than an empty one.
        format!(r#"{{"type":"error","code":"internal","message":"the frame could not be encoded: {error}"}}"#)
    })
}

// ---------------------------------------------------------------------------
// Parsing: total, never throws, and whitelists every field it keeps
// ---------------------------------------------------------------------------

fn is_string(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str)
}

fn is_string_list(value: Option<&Value>) -> Option<Vec<String>> {
    let list = value?.as_array()?;
    list.iter()
        .map(|entry| entry.as_str().map(str::to_string))
        .collect()
}

fn is_bool(value: Option<&Value>) -> Option<bool> {
    value.and_then(Value::as_bool)
}

/// `typeof value === "number" && Number.isInteger(value)`: a JSON number with no fraction (JS reads
/// `3.0` as the integer 3).
fn is_int(value: Option<&Value>) -> Option<f64> {
    let number = value?.as_f64()?;
    if number.is_finite() && number.fract() == 0.0 {
        Some(number)
    } else {
        None
    }
}

fn is_row(value: Option<&Value>) -> Option<Row> {
    match value.and_then(Value::as_str) {
        Some("units") => Some(Row::Units),
        Some("backrow") => Some(Row::Backrow),
        _ => None,
    }
}

fn is_player_id(value: Option<&Value>) -> Option<PlayerId> {
    match value.and_then(Value::as_str) {
        Some("p1") => Some(PlayerId::P1),
        Some("p2") => Some(PlayerId::P2),
        _ => None,
    }
}

fn malformed(reason: impl Into<String>) -> MalformedMessage {
    MalformedMessage {
        reason: reason.into(),
    }
}

/// A non-negative whole number as the `i32` the action types hold (a value past `i32::MAX`
/// saturates, and the reducer refuses it as it refused the exact number).
fn non_negative_int(value: Option<&Value>) -> Option<i32> {
    let number = is_int(value)?;
    if number < 0.0 { None } else { Some(number as i32) }
}

fn parse_zone(value: &Value) -> Option<ZoneChoice> {
    let record = (value).as_object()?;
    let row = is_row(record.get("row"))?;
    let lane = non_negative_int(record.get("lane"))?;
    Some(ZoneChoice { row, lane })
}

fn parse_plague(value: &Value) -> Option<PlagueSpend> {
    let record = value.as_object()?;
    let from = is_string(record.get("from"))?;
    let tokens = non_negative_int(record.get("tokens"))?;
    Some(PlagueSpend {
        from: from.to_string(),
        tokens,
    })
}

fn parse_selection(value: &Value) -> Option<Selection> {
    let record = (value).as_object()?;
    match record.get("pick").and_then(Value::as_str) {
        Some("instance") => is_string(record.get("instanceId")).map(|instance_id| Selection::Instance {
            instance_id: instance_id.to_string(),
        }),
        Some("hero") => is_player_id(record.get("player")).map(|player| Selection::Hero { player }),
        Some("zone") => {
            let player = is_player_id(record.get("player"))?;
            let row = is_row(record.get("row"))?;
            let lane = non_negative_int(record.get("lane"))?;
            Some(Selection::Zone { player, row, lane })
        }
        Some("mode") => is_string(record.get("option")).map(|option| Selection::Mode {
            option: option.to_string(),
        }),
        Some("none") => Some(Selection::None),
        _ => None,
    }
}

fn parse_selections(value: &Value) -> Option<Vec<Selection>> {
    let list = value.as_array()?;
    let mut out = Vec::with_capacity(list.len());
    for entry in list {
        out.push(parse_selection(entry)?);
    }
    Some(out)
}

/// Structural validation only. Whether the action is *legal* is the reducer's call (§9.3), so this
/// checks shapes and nothing else — but it rebuilds the body field by field, which is what keeps a
/// client-supplied `playerId` (or any other smuggled key) from reaching `reduce`.
fn parse_action_body(raw: &Map<String, Value>) -> Result<ActionBody, MalformedMessage> {
    let Some(type_) = is_string(raw.get("type")) else {
        return Err(malformed(r#""action.type" must be a string"#));
    };
    let parsed = type_.parse::<ActionType>().ok();
    if parsed.is_some_and(|kind| SERVER_ONLY_ACTION_TYPES.contains(&kind)) {
        return Err(malformed(format!(r#""{type_}" is a server-only action (R79)"#)));
    }
    let Some(kind) = parsed.filter(|kind| CLIENT_ACTION_TYPES.contains(kind)) else {
        return Err(malformed(format!(r#""{type_}" is not an action type"#)));
    };

    match kind {
        ActionType::Mulligan => {
            let Some(keep) = is_string_list(raw.get("keep")) else {
                return Err(malformed(r#""mulligan.keep" must be an array of ids"#));
            };
            Ok(ActionBody::Mulligan { keep })
        }
        ActionType::Play => {
            let Some(instance_id) = is_string(raw.get("instanceId")) else {
                return Err(malformed(r#""play.instanceId" must be a string"#));
            };
            let mut zone = None;
            if let Some(value) = raw.get("zone") {
                let Some(parsed) = parse_zone(value) else {
                    return Err(malformed(r#""play.zone" must be { row, lane }"#));
                };
                zone = Some(parsed);
            }
            let mut x = None;
            if let Some(value) = raw.get("x") {
                let Some(parsed) = non_negative_int(Some(value)) else {
                    return Err(malformed(r#""play.x" must be a non-negative integer"#));
                };
                x = Some(parsed);
            }
            let mut embiggen = None;
            if let Some(value) = raw.get("embiggen") {
                let Some(parsed) = is_bool(Some(value)) else {
                    return Err(malformed(r#""play.embiggen" must be a boolean"#));
                };
                embiggen = Some(parsed);
            }
            let mut tributes = None;
            if let Some(value) = raw.get("tributes") {
                let Some(parsed) = is_string_list(Some(value)) else {
                    return Err(malformed(r#""play.tributes" must be an array of ids"#));
                };
                tributes = Some(parsed);
            }
            let mut targets = None;
            if let Some(value) = raw.get("targets") {
                let Some(parsed) = parse_selections(value) else {
                    return Err(malformed(r#""play.targets" must be an array of selections"#));
                };
                targets = Some(parsed);
            }
            let mut modes = None;
            if let Some(value) = raw.get("modes") {
                let Some(parsed) = is_string_list(Some(value)) else {
                    return Err(malformed(r#""play.modes" must be an array of strings"#));
                };
                modes = Some(parsed);
            }
            // B5 E11, R454: the Plague Counters that pay for a play from the graveyard (Classic #74
            // Corpse Plantation), as `legal_actions` lists them; whether they may pay is the reducer's.
            let mut plague = None;
            if let Some(value) = raw.get("plague") {
                let Some(parsed) = parse_plague(value) else {
                    return Err(malformed(r#""play.plague" must be { from, tokens }"#));
                };
                plague = Some(parsed);
            }
            Ok(ActionBody::Play {
                instance_id: instance_id.to_string(),
                zone,
                x,
                embiggen,
                tributes,
                targets,
                modes,
                plague,
            })
        }
        ActionType::Attack => {
            let (Some(attacker_id), Some(target_id)) =
                (is_string(raw.get("attackerId")), is_string(raw.get("targetId")))
            else {
                return Err(malformed(r#""attack" needs "attackerId" and "targetId""#));
            };
            Ok(ActionBody::Attack {
                attacker_id: attacker_id.to_string(),
                target_id: target_id.to_string(),
            })
        }
        ActionType::SwitchPosition => {
            let Some(instance_id) = is_string(raw.get("instanceId")) else {
                return Err(malformed(r#""switchPosition.instanceId" must be a string"#));
            };
            Ok(ActionBody::SwitchPosition {
                instance_id: instance_id.to_string(),
            })
        }
        ActionType::Activate => {
            // R384: the ability, its declared targets and modes and a Tribute cost's units travel in
            // the action, as a play's do (R81); whether they are legal is the reducer's call.
            let Some(instance_id) = is_string(raw.get("instanceId")) else {
                return Err(malformed(r#""activate.instanceId" must be a string"#));
            };
            let mut ability = None;
            if let Some(value) = raw.get("ability") {
                let Some(parsed) = value.as_str() else {
                    return Err(malformed(r#""activate.ability" must be a string"#));
                };
                ability = Some(parsed.to_string());
            }
            let mut targets = None;
            if let Some(value) = raw.get("targets") {
                let Some(parsed) = parse_selections(value) else {
                    return Err(malformed(r#""activate.targets" must be an array of selections"#));
                };
                targets = Some(parsed);
            }
            let mut modes = None;
            if let Some(value) = raw.get("modes") {
                let Some(parsed) = is_string_list(Some(value)) else {
                    return Err(malformed(r#""activate.modes" must be an array of strings"#));
                };
                modes = Some(parsed);
            }
            let mut tributes = None;
            if let Some(value) = raw.get("tributes") {
                let Some(parsed) = is_string_list(Some(value)) else {
                    return Err(malformed(r#""activate.tributes" must be an array of ids"#));
                };
                tributes = Some(parsed);
            }
            Ok(ActionBody::Activate {
                instance_id: instance_id.to_string(),
                ability,
                targets,
                modes,
                tributes,
            })
        }
        ActionType::ActivatePower => {
            let Some(instance_id) = is_string(raw.get("instanceId")) else {
                return Err(malformed(r#""activatePower.instanceId" must be a string"#));
            };
            let mut targets = None;
            if let Some(value) = raw.get("targets") {
                let Some(parsed) = parse_selections(value) else {
                    return Err(malformed(r#""activatePower.targets" must be selections"#));
                };
                targets = Some(parsed);
            }
            Ok(ActionBody::ActivatePower {
                instance_id: instance_id.to_string(),
                targets,
            })
        }
        ActionType::Answer => {
            let Some(choice_id) = is_string(raw.get("choiceId")) else {
                return Err(malformed(r#""answer.choiceId" must be a string"#));
            };
            let Some(selection) = raw.get("selection").and_then(parse_selections) else {
                return Err(malformed(r#""answer.selection" must be an array of selections"#));
            };
            Ok(ActionBody::Answer {
                choice_id: choice_id.to_string(),
                selection,
            })
        }
        ActionType::AnswerDraw => {
            let Some(accept) = is_bool(raw.get("accept")) else {
                return Err(malformed(r#""answerDraw.accept" must be a boolean"#));
            };
            Ok(ActionBody::AnswerDraw { accept })
        }
        ActionType::OfferDraw => Ok(ActionBody::OfferDraw),
        ActionType::Concede => Ok(ActionBody::Concede),
        ActionType::EndTurn => Ok(ActionBody::EndTurn),
        ActionType::SetAutoEndTurn => {
            let Some(enabled) = is_bool(raw.get("enabled")) else {
                return Err(malformed(r#""setAutoEndTurn.enabled" must be a boolean"#));
            };
            Ok(ActionBody::SetAutoEndTurn { enabled })
        }
        // Unreachable: `CLIENT_ACTION_TYPES` admitted none of these.
        ActionType::Timeout | ActionType::DisconnectExpired | ActionType::CeilingReached => {
            Err(malformed(format!(r#""{type_}" is not an action type"#)))
        }
    }
}

/// The longest nonce the server will hold in its dedupe map (§9.8: nothing unbounded), in UTF-16
/// code units as TS's `string.length` counts them.
pub const MAX_NONCE_LENGTH: usize = 128;

/// SPEC §11 R270: the prefix of every nonce the server mints for its own actions (a clock's
/// `timeout`, `disconnectExpired`, `ceilingReached`; `match_actor.rs`). A client may not send one:
/// the actor answers a known nonce with its stored ack and applies nothing, so a client that sent
/// the nonce its clock was about to mint would swallow that expiry and stall its own turn, or the
/// other seat's mulligan, until the ceiling.
pub const SERVER_NONCE_PREFIX: &str = "srv-";

/// A frame larger than this is rejected before it is parsed (§9.8). The transport closes a larger
/// frame with 1009 (SURFACE §11.3, `ws_server.rs`); this check is the same bound, held again here
/// so the parser stays total on its own.
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

/// TS `parseClientMessage`: total, never panics, and every field it keeps is rebuilt by hand.
pub fn parse_client_message(text: &str) -> Result<ClientMessage, MalformedMessage> {
    if text.encode_utf16().count() > MAX_FRAME_BYTES {
        return Err(malformed("that frame is too large"));
    }

    let Ok(parsed) = serde_json::from_str::<Value>(text) else {
        return Err(malformed("every frame must be JSON"));
    };
    let Some(parsed) = parsed.as_object() else {
        return Err(malformed("every frame must be a JSON object"));
    };

    let Some(type_) = is_string(parsed.get("type")) else {
        return Err(malformed(r#""type" must be a string"#));
    };

    match type_ {
        "hello" => {
            let mut message = HelloMessage::default();
            if let Some(value) = parsed.get("token") {
                let Some(token) = value.as_str() else {
                    return Err(malformed(r#""hello.token" must be a string"#));
                };
                message.token = Some(token.to_string());
            }
            if let Some(value) = parsed.get("matchId") {
                let Some(match_id) = value.as_str() else {
                    return Err(malformed(r#""hello.matchId" must be a string"#));
                };
                message.match_id = Some(match_id.to_string());
            }
            if let Some(value) = parsed.get("roomCode") {
                let Some(room_code) = value.as_str() else {
                    return Err(malformed(r#""hello.roomCode" must be a string"#));
                };
                message.room_code = Some(room_code.to_string());
            }
            Ok(ClientMessage::Hello(message))
        }
        // SURFACE §11.3: joining a room is `POST /api/rooms/:code/join` (`rooms.rs`), because the
        // atomic single-claim and the loadout re-check are HTTP concerns and a socket is opened for a
        // match that already exists. The frame is answered `malformed`, with the pointer TS gave.
        "joinRoom" => Err(malformed(
            "join a room with POST /api/rooms/:code/join, not over the socket",
        )),
        "action" => {
            let Some(raw) = parsed.get("action").and_then(Value::as_object) else {
                return Err(malformed(r#""action" must be an object"#));
            };
            // SURFACE §11.3: the nonce is read inside `action` only (the client's form).
            let nonce = match is_string(raw.get("nonce")) {
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
            let body = parse_action_body(raw)?;
            Ok(ClientMessage::Action(ActionMessage {
                nonce: nonce.to_string(),
                body,
            }))
        }
        "emote" => {
            // R643, R1340: the pool's ids are the whole vocabulary; anything else is malformed, and there
            // is nothing else on the frame to validate (no nonce, no seat — the actor stamps the seat).
            let value = parsed.get("emote").unwrap_or(&Value::Null);
            let emote = if is_emote_id(value) {
                value.as_str().and_then(|text| text.parse::<EmoteId>().ok())
            } else {
                None
            };
            match emote {
                Some(emote) => Ok(ClientMessage::Emote(EmoteMessage { emote })),
                None => Err(malformed(r#""emote" must be a known emote id"#)),
            }
        }
        "aim" => {
            // R738: the shape check only, rebuilt field by field. Whether each end names something the
            // opponent may see is the actor's call, against the live state (`match_actor.rs`). A
            // missing `aim` is TS's `undefined`, not `null`: malformed.
            match parsed.get("aim").and_then(parse_aim) {
                Some(aim) => Ok(ClientMessage::Aim(AimMessage { aim })),
                None => Err(malformed(
                    r#""aim" must be null or { source, target } of public handles"#,
                )),
            }
        }
        other => Err(malformed(format!(r#""{other}" is not a client message"#))),
    }
}
