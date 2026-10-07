//! `cargo jackioh agent`: this build's AI as an arena agent, on JSON lines (SURFACE §14.1;
//! docs/v0.3.0/README.md §8). The training arena's referee (`arena.rs`) spawns `<path> agent` for a
//! `bin:<path>` seat, which is how a promotion run plays the AI on `main` (the parent) against the
//! candidate built from the lane's branch.
//!
//! One request per stdin line, one answer per stdout line:
//!
//! ```text
//! → {"op":"info"}
//! ← {"generation":7,"lane":"improve","shadowBan":["core-042", …]}
//! → {"op":"decide","state":<redact(state, seat) JSON>,"seat":"p1","rngSeed":"…","rngCursor":0}
//! ← {"action":<ActionBody JSON>,"rngCursor":3}
//! → {"op":"quit"}
//! ```
//!
//! The agent never sees the true state: the referee holds it and sends `redact(state, seat)` (R185),
//! and the AI decides on that. The AI's own stream is the referee's to keep: a request names the seed
//! and the cursor to resume it at, and the answer hands the cursor back, so the referee can fall back
//! to the random policy on the same stream when the AI has no move (match.ts's `playMatch`), and a
//! game played through this protocol is the game the in-process `self` agent plays.
//!
//! `{"action":null,…}` means the AI had no move. A request the agent cannot read, or a decision that
//! panicked, is answered `{"error":"…"}`, which the referee records as a thrown controller.

use std::any::Any;
use std::io::{BufRead, Write};

use anyhow::Context;
use jackioh_ai::{AI_GATE_BUDGET, AiOptions, SHADOW_BAN, decide};
use jackioh_engine::{ActionBody, GameState, PlayerId, Rng};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// This build's generation record (SURFACE §14.3), compiled in: the binary is the AI, so it reports
/// the generation of the source it was built from, whatever the checkout holds now.
const GENERATION_JSON: &str = include_str!("../../ai/generation.json");

/// `cargo jackioh agent` takes no flags: everything it needs arrives on stdin.
#[derive(clap::Args)]
pub struct Args {}

/// `info`'s answer: who this agent is, and the shadow ban (R186) its own decks are built without.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentInfo {
    pub generation: i64,
    pub lane: String,
    pub shadow_ban: Vec<String>,
}

/// One request line. Only `op` is always present; `decide` carries the rest. Unknown keys are ignored.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Request {
    op: String,
    #[serde(default)]
    state: Option<GameState>,
    #[serde(default)]
    seat: Option<PlayerId>,
    #[serde(default)]
    rng_seed: Option<String>,
    #[serde(default)]
    rng_cursor: Option<u32>,
}

/// `decide`'s answer. `action` is `null` when the AI had no move.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DecideAnswer {
    action: Option<ActionBody>,
    rng_cursor: u32,
}

/// Answers requests until `quit` or the end of stdin.
pub fn run(_args: Args) -> anyhow::Result<()> {
    jackioh_cards::register_all();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line.context("agent: reading stdin")?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Request = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(error) => {
                reply(&mut stdout, &json!({ "error": format!("agent: not a request: {error}") }))?;
                continue;
            }
        };
        match request.op.as_str() {
            "info" => reply(&mut stdout, &serde_json::to_value(own_info())?)?,
            "decide" => reply(&mut stdout, &answer_decide(request))?,
            "quit" => break,
            other => reply(&mut stdout, &json!({ "error": format!("agent: unknown op \"{other}\"") }))?,
        }
    }
    Ok(())
}

/// Writes one answer line and flushes, so the referee never waits on a buffer.
fn reply(out: &mut impl Write, answer: &Value) -> anyhow::Result<()> {
    serde_json::to_writer(&mut *out, answer).context("agent: writing stdout")?;
    out.write_all(b"\n").context("agent: writing stdout")?;
    out.flush().context("agent: flushing stdout")?;
    Ok(())
}

/// `decide`: the AI's move on the state it was sent, from the stream at the cursor it was sent.
fn answer_decide(request: Request) -> Value {
    let Some(state) = request.state else {
        return json!({ "error": "agent: decide needs a state" });
    };
    let Some(seat) = request.seat else {
        return json!({ "error": "agent: decide needs a seat p1|p2" });
    };
    let Some(rng_seed) = request.rng_seed else {
        return json!({ "error": "agent: decide needs an rngSeed" });
    };
    let rng = Rng::new(&rng_seed, request.rng_cursor.unwrap_or(0));
    let decided = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| decide_with(&state, seat, rng)));
    match decided {
        Ok((action, rng)) => serde_json::to_value(DecideAnswer {
            action,
            rng_cursor: rng.cursor(),
        })
        .unwrap_or_else(|error| json!({ "error": format!("agent: answer did not serialise: {error}") })),
        Err(panic) => json!({ "error": format!("agent: decide panicked: {}", panic_message(&*panic)) }),
    }
}

/// The AI's move for `seat` on `state` (the referee's `redact(state, seat)`), at the quality gates'
/// budget (`AI_GATE_BUDGET`, the browser's own, so a promotion measures the AI that ships), and the
/// stream after it. `arena.rs`'s in-process `self` agent calls this too, so `self` and `bin:<this
/// binary>` play the same games.
pub(crate) fn decide_with(state: &GameState, seat: PlayerId, rng: Rng) -> (Option<ActionBody>, Rng) {
    let mut options = AiOptions {
        rng,
        budget: AI_GATE_BUDGET,
        should_stop: None,
    };
    let decision = decide(state, seat, &mut options);
    (decision.map(|decision| decision.action), options.rng)
}

/// This build's `info`: its generation and lane from the compiled-in `crates/ai/generation.json`, and
/// its shadow ban's ids, sorted (`SHADOW_BAN_IDS`).
pub(crate) fn own_info() -> AgentInfo {
    let record: Value = serde_json::from_str(GENERATION_JSON).expect("crates/ai/generation.json is not JSON");
    let generation = record
        .get("generation")
        .and_then(Value::as_i64)
        .expect("crates/ai/generation.json has no integer \"generation\"");
    let lane = record
        .get("lane")
        .and_then(Value::as_str)
        .expect("crates/ai/generation.json has no string \"lane\"")
        .to_string();
    AgentInfo {
        generation,
        lane,
        shadow_ban: own_shadow_ban(),
    }
}

/// `SHADOW_BAN_IDS`: the shadow ban's ids (R186), sorted.
pub(crate) fn own_shadow_ban() -> Vec<String> {
    let mut ids: Vec<String> = SHADOW_BAN.iter().map(|(id, _)| id.to_string()).collect();
    ids.sort();
    ids
}

/// match.ts's `messageOf`: a panic's message, as TS read a thrown error's.
pub(crate) fn panic_message(panic: &(dyn Any + Send)) -> String {
    if let Some(text) = panic.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = panic.downcast_ref::<String>() {
        text.clone()
    } else {
        "a panic without a message".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_reports_the_compiled_generation_and_the_sorted_shadow_ban() {
        let info = own_info();
        let record: Value = serde_json::from_str(GENERATION_JSON).unwrap();
        assert_eq!(Some(info.generation), record["generation"].as_i64());
        assert_eq!(Some(info.lane.as_str()), record["lane"].as_str());
        assert_eq!(info.shadow_ban.len(), SHADOW_BAN.len());
        let mut sorted = info.shadow_ban.clone();
        sorted.sort();
        assert_eq!(info.shadow_ban, sorted);
        let text = serde_json::to_string(&info).unwrap();
        assert!(text.starts_with("{\"generation\":"));
        assert!(text.contains("\"shadowBan\":["));
    }

    #[test]
    fn a_decide_without_a_state_is_answered_with_an_error() {
        let request: Request = serde_json::from_str(r#"{"op":"decide","seat":"p1","rngSeed":"s","rngCursor":0}"#).unwrap();
        let answer = answer_decide(request);
        assert_eq!(answer["error"], json!("agent: decide needs a state"));
    }

    #[test]
    fn panic_messages_read_both_payload_kinds() {
        let text: Box<dyn Any + Send> = Box::new("static text");
        assert_eq!(panic_message(&*text), "static text");
        let owned: Box<dyn Any + Send> = Box::new(String::from("owned text"));
        assert_eq!(panic_message(&*owned), "owned text");
    }
}
