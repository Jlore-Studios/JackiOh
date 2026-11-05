//! The game log's lines, written in Rust (#3108, Part B of #2956): the events a player's view gives, as
//! readable lines in the reader's words, for the browser and for Roblox alike. The browser calls it through
//! `jackioh-wasm-core`'s `log` body, Roblox through the same body in the C module, and neither writes or
//! translates a line (README.md, the host contract).
//!
//! Input: a `PlayerView`, whose `events` are the window of events the viewer was sent (SPEC §10.8, R168),
//! and a locale. Output: for each event its line, or none for an event the log leaves out, as the same spans
//! a card face's text is made of (`face::types::Span`): plain words, and a glossary term where a line is
//! about one. A line that is about a card says which, as the `NamedCard` a client resolves to the card's face
//! for its hover and its sheet (`FaceCall::Named`). The log sees only what `view_for` gives (CLAUDE.md rule
//! 7), and a line about a hidden card never names it (R97): a card the viewer may not read is "a card" or
//! "a hidden card", an id is never printed, and the sentinel opens nothing.
//!
//! The log's sentences are English keys that `data/zh-CN.json` translates, beside the face model's.
//! Keeping the lines of events that have left the window (R745) is the client's: a history is a list of the
//! lines this returned, not a decision about any of them.

pub mod chaos;
pub mod curve;
pub mod lines;
pub mod names;
pub mod say;

use indexmap::IndexMap;
use jackioh_engine::wire::PlayerView;
use serde::{Deserialize, Serialize};

use crate::face::live::NamedCard;
use crate::face::types::{DefMap, Locale, RichText, Span};
use crate::face::words::Words;

use names::{Names, announced_aims};
use say::rich;

/// The `event` of the line that says events may be missing.
pub const GAP_EVENT: &str = "gap";

/// R745: what the log says in place of events a view's window never carried.
const GAP_TEXT: &str = "Some events may be missing here";

/// One line of the log, final and in the reader's words.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    /// The event's type (`GameEvent.type`), or `gap`: what a client keys a line's look on.
    pub event: String,
    /// The line's words: `full` the plain sentence, `spans` the same words with the glossary term it is about.
    pub text: RichText,
    /// The card the line is about, if it names one the viewer may read: the card to open on hover or tap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub card: Option<NamedCard>,
}

/// One call of the log: a tagged union in (`LogCall`), one out (`LogAnswer`), so a host has two types to
/// read for the whole body (README.md, host contract 2).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "call", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum LogCall {
    /// A line for every event of the view's window, in order.
    View {
        view: Box<PlayerView>,
        #[serde(default)]
        locale: Locale,
    },
    /// The line a history keeps where it cannot join a window to the one before (R745).
    Gap {
        #[serde(default)]
        locale: Locale,
    },
}

/// What a [`LogCall`] answers.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "answer", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum LogAnswer {
    /// `view`: one entry per event of the window, `null` for an event with no line (bookkeeping).
    Lines { lines: Vec<Option<LogLine>> },
    /// `gap`.
    Line { line: LogLine },
}

/// The line of every event in `view.events`, in order; None where the log leaves an event out.
pub fn view_lines(words: Words, view: &PlayerView) -> Vec<Option<LogLine>> {
    let no_defs: DefMap = IndexMap::new();
    let defs = view.defs.as_ref().unwrap_or(&no_defs);
    let names = Names::new(view, words, defs);
    let aims = announced_aims(&view.events);
    view.events
        .iter()
        .zip(aims)
        .map(|(event, aims)| {
            let spans: Vec<Span> = lines::describe(&names, event, aims)?;
            Some(LogLine {
                event: event.event_type().as_str().to_string(),
                text: rich(spans),
                card: names.card_of(event),
            })
        })
        .collect()
}

/// R745: the line that says events may be missing where a window shares none with the one before.
pub fn gap_line(words: Words) -> LogLine {
    LogLine {
        event: GAP_EVENT.to_string(),
        text: rich(vec![Span::Text {
            text: words.say(GAP_TEXT),
        }]),
        card: None,
    }
}

/// Answers one call. The one function behind the `log` body (`crates/wasm/core`).
pub fn apply(call: &LogCall) -> LogAnswer {
    match call {
        LogCall::View { view, locale } => LogAnswer::Lines {
            lines: view_lines(Words::new(*locale), view),
        },
        LogCall::Gap { locale } => LogAnswer::Line {
            line: gap_line(Words::new(*locale)),
        },
    }
}
