//! What a card face prints, worked out in Rust (#3106, Part B of #2956): the face model and everything it
//! decides, in the reader's words, as data a client lays out. The browser calls it through
//! `jackioh-wasm-core`'s `face` body, Roblox through the same body in the C module, and neither restates a
//! rule of what a face says (README.md, the host contract).
//!
//! Inputs: the catalog (the engine's registered one, and the match's own definitions the view carries), a
//! `FaceSource` or the cards a `PlayerView` lists, and a locale. The model sees only what `view_for` gives
//! (CLAUDE.md rule 7), so it cannot reveal a hidden card. Output: final text as spans (`types::Span`): plain
//! words, a keyword with its glossary entry, a number with whether it moved up or down from the printed one,
//! a reference to a card, the words a Radiant face adds. No HTML, Markdown or CSS: a client maps spans to its
//! markup and decides nothing. Layout, art, animation and audio stay in the clients.
//!
//! The face and glossary strings of every locale are compiled in (`build.rs`, `data/`), not the clients'.

pub mod chinese;
pub mod data;
pub mod diff;
pub mod glitch;
pub mod glossary;
pub mod live;
pub mod model;
pub mod power;
pub mod quest;
pub mod refs;
pub mod short;
pub mod spans;
pub mod state;
pub mod text;
pub mod tuning;
pub mod types;
pub mod words;

use indexmap::IndexMap;
use jackioh_engine::wire::{CardDef, PlayerView};
use serde::{Deserialize, Serialize};

pub use live::{LiveCard, LiveFacts, NamedCard};
pub use model::{ALL_TRIBES, Lookup, face_model};
pub use types::{DefMap, FaceModel, FaceSource, Locale, RichText, Span};
pub use words::Words;

/// A stretch of a text by UTF-16 code units, as a JavaScript string counts them: `start` inclusive, `end` exclusive.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct Utf16Range {
    pub start: i32,
    pub end: i32,
}

/// One call of the face model: a tagged union in (`FaceCall`), one out (`FaceAnswer`), so a host has two
/// types to read for the whole body (README.md, host contract 2).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "call", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum FaceCall {
    /// One card as printed (the collection, a preview) or as a host built its source: `defs` are definitions
    /// the catalog does not hold.
    Card {
        source: FaceSource,
        #[serde(default)]
        locale: Locale,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        defs: Option<DefMap>,
    },
    /// One card the view lists: the face in play the view says it has (R243).
    Live {
        card: LiveCard,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        def: Option<CardDef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        name: Option<String>,
        #[serde(default)]
        facts: LiveFacts,
        #[serde(default)]
        locale: Locale,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        defs: Option<DefMap>,
    },
    /// Every face a view shows, by instance id.
    View {
        view: Box<PlayerView>,
        #[serde(default)]
        locale: Locale,
    },
    /// A card an event or a prompt names.
    Named {
        view: Box<PlayerView>,
        named: NamedCard,
        #[serde(default)]
        locale: Locale,
    },
    /// Words from the engine or a card with their glossary terms in bold: a prompt's detail, a quest's text.
    Text {
        text: String,
        #[serde(default)]
        locale: Locale,
    },
    /// What a newer text adds to an older and what it loses, by word (R277, R388).
    WordDiff { before: String, after: String },
}

/// What a [`FaceCall`] answers.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "answer", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum FaceAnswer {
    /// `card` and `live`, and `named` for a card it names.
    Face { face: Box<FaceModel> },
    /// `named` for the sentinel, which names no card: the caller draws a back.
    NoFace,
    /// `view`.
    Faces { faces: IndexMap<String, FaceModel> },
    /// `text`.
    Rich { text: RichText },
    /// `wordDiff`, by UTF-16 code units.
    Diff {
        added: Vec<Utf16Range>,
        removed: Vec<Utf16Range>,
    },
}

/// The UTF-16 offset of each character offset of `chars`, for a client that slices strings as JavaScript does.
fn utf16_ranges(chars: &[char], ranges: &[diff::TextRange]) -> Vec<Utf16Range> {
    let mut at = Vec::with_capacity(chars.len() + 1);
    let mut units = 0_i32;
    for c in chars {
        at.push(units);
        units += c.len_utf16() as i32;
    }
    at.push(units);
    ranges
        .iter()
        .map(|range| Utf16Range {
            start: at[range.start.min(chars.len())],
            end: at[range.end.min(chars.len())],
        })
        .collect()
}

/// Answers one call. The one function behind the `face` body (`crates/wasm/core`).
pub fn apply(call: &FaceCall) -> FaceAnswer {
    let no_defs = DefMap::new();
    match call {
        FaceCall::Card { source, locale, defs } => {
            let defs = defs.as_ref().unwrap_or(&no_defs);
            FaceAnswer::Face {
                face: Box::new(face_model(Words::new(*locale), source, &Lookup { defs })),
            }
        }
        FaceCall::Live {
            card,
            def,
            name,
            facts,
            locale,
            defs,
        } => {
            let defs = defs.as_ref().unwrap_or(&no_defs);
            FaceAnswer::Face {
                face: Box::new(live::live_face(
                    Words::new(*locale),
                    card,
                    def.clone(),
                    name.clone(),
                    facts,
                    defs,
                )),
            }
        }
        FaceCall::View { view, locale } => FaceAnswer::Faces {
            faces: live::view_faces(Words::new(*locale), view),
        },
        FaceCall::Named { view, named, locale } => match live::named_face(Words::new(*locale), view, named) {
            Some(face) => FaceAnswer::Face { face: Box::new(face) },
            None => FaceAnswer::NoFace,
        },
        FaceCall::Text { text, locale } => FaceAnswer::Rich {
            text: spans::rich_words(text, Words::new(*locale)),
        },
        FaceCall::WordDiff { before, after } => {
            let (before, after): (Vec<char>, Vec<char>) = (before.chars().collect(), after.chars().collect());
            let found = diff::word_diff(&before, &after);
            FaceAnswer::Diff {
                added: utf16_ranges(&after, &found.added),
                removed: utf16_ranges(&before, &found.removed),
            }
        }
    }
}
