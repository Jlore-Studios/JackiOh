//! `cargo jackioh luau surface [--json] [--card <id>]…` (#560, groundwork for #442's part 6, #448): the
//! engine API every card script calls, card by card.
//!
//! For each card script (`crates/cards/src/scripts/<set>/<file>.rs`, above its `#[cfg(test)]`) it
//! reports, per face: the `Script` hooks the face sets, and whether the Radiant face reuses the base
//! face (`radiant: base.clone()`, `..base.clone()`); the effect verbs it calls (the `pub fn`s of
//! `crates/engine/src/effects/` that return an `Effect`); the read helpers it calls (the other `pub fn`s
//! the engine's prelude re-exports, `subsystems::…` paths, `remember` and the ctx readers); and the
//! random pools it uses (`crates/cards/src/query.rs`). Names that appear in the file but in neither
//! face are the card's `elsewhere`. The cards that stay in Rust (#442's L14) are marked. A summary then
//! lists every distinct hook, verb, reader and pool with the cards that use it, the L14 cards counted
//! apart.
//!
//! It reads source text, as part 6's grep would, and is not a Rust parser. Comments, string and
//! character literals and everything from `#[cfg(test)]` on are blanked before any scan. What it follows
//! is the shapes the scripts use: a `CardScripts { base, radiant }` literal in `script()`, faces that are
//! `Script { … }` literals (spread `..Script::default()` or `..base.clone()`), `let` bindings and helper
//! functions of the same file, and `base.field` borrowed into the Radiant face. A name it cannot classify
//! is listed as unclassified, and a card it cannot read as unreadable with its reason (a card ported to
//! Luau, #442, as one whose script is its `.luau`); neither fails the command, and nothing is dropped.
//! Only the report is printed: no file is written, so it cannot go stale.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::{ErrorKind, Write as _};
use std::ops::Range;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use jackioh_cards::CATALOG_IDS;
use serde::Serialize;

use crate::catalog::{Quote, char_literal_length, raw_string_open};
use crate::patches::repo_root;

/// #442's L14: the cards that stay in Rust, by catalog id.
const STAY_IN_RUST: [&str; 14] = [
    "core-093",
    "core-095",
    "core-096",
    "core-097",
    "core-098",
    "core-099",
    "classic-057",
    "classic-090",
    "classicplus-042",
    "classicplus-044",
    "classicplus-045",
    "classicplus-062",
    "classicplus-073",
    "classicplus-074",
];

/// Names a script calls as readers though they return an `Effect`: `remember` is the issue's own example.
const NAMED_READERS: [&str; 1] = ["remember"];

/// Method calls that count as readers: the ctx's own, whatever the receiver.
const CTX_METHODS: [&str; 1] = ["live_self"];

/// How much of a face's code an unreadable card's reason quotes.
const FACE_TEXT_CHARS: usize = 60;

/// Rust's keywords, which are never a call to a name of the engine's.
const KEYWORDS: [&str; 35] = [
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self",
    "static", "struct", "super", "trait", "type", "unsafe", "use", "where", "while",
];

const SCRIPTS_DIR: &str = "crates/cards/src/scripts";
const EFFECTS_DIR: &str = "crates/engine/src/effects";
const ENGINE_SRC: &str = "crates/engine/src";
const PRELUDE: &str = "crates/engine/src/prelude.rs";
const SCRIPT_TYPES: &str = "crates/engine/src/script.rs";
const ENGINE_CATALOG: &str = "crates/engine/src/catalog.rs";
const POOLS: &str = "crates/cards/src/query.rs";
const CARDS_LIB: &str = "crates/cards/src/lib.rs";

/// The return type of an effect verb.
const EFFECT_TYPE: &str = "Effect";

/// The marker of the unit tests, below which nothing is read.
const TESTS_MARKER: &str = "#[cfg(test)]";

/// A top-level function's declaration, the way every module of the engine writes one.
const PUB_FN: &str = "pub fn ";

/// The `Script` hooks live in this struct of `script.rs`.
const SCRIPT_STRUCT: &str = "Script";

/// The pool object's name in a script: `catalog.pool(…)`, `catalog.trap_types`.
const CATALOG_OBJECT: &str = "catalog";

#[derive(clap::Args)]
pub(super) struct SurfaceArgs {
    /// Print the report as JSON instead of Markdown tables.
    #[arg(long)]
    json: bool,
    /// Report only this catalog id; repeatable. Every card without it.
    #[arg(long = "card", value_name = "ID")]
    cards: Vec<String>,
}

/// What one face's code uses, each list sorted.
#[derive(Serialize, Debug, Default, PartialEq)]
struct Face {
    hooks: BTreeSet<String>,
    verbs: BTreeSet<String>,
    readers: BTreeSet<String>,
    pools: BTreeSet<String>,
    unclassified: BTreeSet<String>,
}

impl Face {
    /// What this face names that neither of `others` does, kind by kind. A face's hooks are never
    /// read from the code at large, so none are kept.
    fn without(mut self, others: [&Face; 2]) -> Face {
        self.hooks.clear();
        for other in others {
            self.verbs.retain(|name| !other.verbs.contains(name));
            self.readers.retain(|name| !other.readers.contains(name));
            self.pools.retain(|name| !other.pools.contains(name));
            self.unclassified
                .retain(|name| !other.unclassified.contains(name));
        }
        self
    }
}

/// How the Radiant face is made from the base face.
#[derive(Serialize, Debug, Default, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Reuse {
    /// Its own `Script`, or one that only borrows a field of the base's.
    #[default]
    No,
    /// `radiant: base.clone()`, or the base itself.
    Whole,
    /// `Script { …, ..base.clone() }`.
    Spread,
}

impl Reuse {
    fn label(self) -> &'static str {
        match self {
            Reuse::No => "no",
            Reuse::Whole => "whole",
            Reuse::Spread => "spread",
        }
    }
}

#[derive(Serialize, Debug, Default, PartialEq)]
struct CardSurface {
    radiant_reuses_base: Reuse,
    base: Face,
    radiant: Face,
    /// Names the file holds that neither face reaches: helpers nobody calls, or a shape the reader
    /// did not follow.
    elsewhere: Face,
}

#[derive(Serialize, Debug, Default, PartialEq)]
struct CardReport {
    id: String,
    file: String,
    stays_in_rust: bool,
    #[serde(flatten)]
    surface: CardSurface,
}

#[derive(Serialize, Debug, Default, PartialEq)]
struct Unreadable {
    id: String,
    file: Option<String>,
    reason: String,
}

/// The cards that use one name; the cards that stay in Rust are counted apart.
#[derive(Serialize, Debug, Default, PartialEq)]
struct Usage {
    cards: usize,
    ids: Vec<String>,
    stay_in_rust_cards: usize,
    stay_in_rust_ids: Vec<String>,
}

#[derive(Serialize, Debug, Default, PartialEq)]
struct Summary {
    hooks: BTreeMap<String, Usage>,
    verbs: BTreeMap<String, Usage>,
    readers: BTreeMap<String, Usage>,
    pools: BTreeMap<String, Usage>,
    unclassified: BTreeMap<String, Usage>,
}

#[derive(Serialize, Debug, Default, PartialEq)]
struct Report {
    cards: Vec<CardReport>,
    unreadable: Vec<Unreadable>,
    summary: Summary,
}

pub(super) fn run(args: &SurfaceArgs) -> Result<()> {
    let report = survey(&repo_root(), &args.cards)?;
    let text = if args.json {
        serde_json::to_string_pretty(&report)? + "\n"
    } else {
        markdown(&report)
    };
    // `… | head` closes the pipe early, which is no failure of the report.
    match std::io::stdout().lock().write_all(text.as_bytes()) {
        Err(error) if error.kind() == ErrorKind::BrokenPipe => Ok(()),
        written => Ok(written?),
    }
}

/* ---------------------------------------------------------------------------------- the lexer */

/// A blanked character: a space, or the newline itself so lines keep their numbers.
fn blank(out: &mut String, ch: char) {
    out.push(if ch == '\n' { '\n' } else { ' ' });
}

/// The code of a source file and nothing else, as ASCII of the same length: every comment, the inside
/// of every string and character literal (their quotes stay) and every non-ASCII character is a space,
/// newlines are kept, and the text stops at the first line that starts `#[cfg(test)]`.
fn code_text(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut block_depth = 0usize;
    let mut quote: Option<Quote> = None;
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        let next = chars.get(i + 1).copied();
        if block_depth > 0 {
            if ch == '*' && next == Some('/') {
                block_depth -= 1;
                out.push_str("  ");
                i += 2;
            } else if ch == '/' && next == Some('*') {
                block_depth += 1;
                out.push_str("  ");
                i += 2;
            } else {
                blank(&mut out, ch);
                i += 1;
            }
            continue;
        }
        match quote {
            Some(Quote::Plain) => {
                if ch == '\\' {
                    blank(&mut out, ch);
                    if let Some(escaped) = next {
                        blank(&mut out, escaped);
                    }
                    i += 2;
                    continue;
                }
                if ch == '"' {
                    out.push('"');
                    quote = None;
                } else {
                    blank(&mut out, ch);
                }
                i += 1;
                continue;
            }
            Some(Quote::Raw(hashes)) => {
                let closes = ch == '"'
                    && chars.len() > i + hashes
                    && chars[i + 1..=i + hashes].iter().all(|c| *c == '#');
                if closes {
                    out.push('"');
                    out.extend(std::iter::repeat_n('#', hashes));
                    quote = None;
                    i += 1 + hashes;
                } else {
                    blank(&mut out, ch);
                    i += 1;
                }
                continue;
            }
            None => {}
        }
        if ch == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                out.push(' ');
                i += 1;
            }
            continue;
        }
        if ch == '/' && next == Some('*') {
            block_depth += 1;
            out.push_str("  ");
            i += 2;
            continue;
        }
        if ch == '"' {
            quote = Some(Quote::Plain);
            out.push('"');
            i += 1;
            continue;
        }
        if let Some((length, hashes)) = raw_string_open(&chars, i) {
            out.extend(&chars[i..i + length]);
            quote = Some(Quote::Raw(hashes));
            i += length;
            continue;
        }
        if ch == '\'' {
            let length = char_literal_length(&chars, i);
            out.push('\'');
            if length > 1 {
                out.extend(std::iter::repeat_n(' ', length - 2));
                out.push('\'');
            }
            i += length;
            continue;
        }
        out.push(if ch.is_ascii() { ch } else { ' ' });
        i += 1;
    }
    let mut offset = 0;
    let mut cut = None;
    for line in out.split_inclusive('\n') {
        if line.trim().starts_with(TESTS_MARKER) {
            cut = Some(offset);
            break;
        }
        offset += line.len();
    }
    if let Some(at) = cut {
        out.truncate(at);
    }
    out
}

/* --------------------------------------------------------------------------- scanning helpers */

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_opener(b: u8) -> bool {
    matches!(b, b'{' | b'(' | b'[')
}

fn is_closer(b: u8) -> bool {
    matches!(b, b'}' | b')' | b']')
}

fn skip_space(text: &[u8], mut at: usize) -> usize {
    while at < text.len() && text[at].is_ascii_whitespace() {
        at += 1;
    }
    at
}

/// The end of the identifier run that starts at `at`.
fn run_end(text: &[u8], mut at: usize) -> usize {
    while at < text.len() && is_ident_byte(text[at]) {
        at += 1;
    }
    at
}

/// The last byte before `at` that is not whitespace.
fn prev_byte(text: &[u8], at: usize) -> Option<usize> {
    text[..at].iter().rposition(|b| !b.is_ascii_whitespace())
}

/// The identifier that ends right before `at`, whitespace skipped.
fn prev_word(text: &[u8], at: usize) -> Option<Range<usize>> {
    let last = prev_byte(text, at)?;
    if !is_ident_byte(text[last]) {
        return None;
    }
    let mut first = last;
    while first > 0 && is_ident_byte(text[first - 1]) {
        first -= 1;
    }
    Some(first..last + 1)
}

fn word(text: &[u8], range: &Range<usize>) -> String {
    String::from_utf8_lossy(&text[range.clone()]).into_owned()
}

/// The matching closer of the bracket at `open`, counting `{([` against `})]`.
fn close_of(text: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (at, &b) in text.iter().enumerate().skip(open) {
        if is_opener(b) {
            depth += 1;
        } else if is_closer(b) {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(at);
            }
        }
    }
    None
}

/// The index after the `>` that closes the `<` at `open`; a `->` inside is not a closer.
fn angle_end(text: &[u8], open: usize) -> usize {
    let mut depth = 0usize;
    for (at, &b) in text.iter().enumerate().skip(open) {
        match b {
            b'<' => depth += 1,
            b'>' if text[at - 1] != b'-' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return at + 1;
                }
            }
            _ => {}
        }
    }
    text.len()
}

/// Every identifier in `range`: a token that starts with a letter or `_`, whole (a number, with its
/// suffix, is skipped).
fn idents(text: &[u8], range: Range<usize>) -> Vec<Range<usize>> {
    let end = range.end.min(text.len());
    let mut found = Vec::new();
    let mut at = range.start;
    if at > 0 && is_ident_byte(text[at - 1]) {
        at = run_end(text, at);
    }
    while at < end {
        if is_ident_byte(text[at]) {
            let start = at;
            at = run_end(text, at).min(end);
            if is_ident_start(text[start]) {
                found.push(start..at);
            }
        } else {
            at += 1;
        }
    }
    found
}

/// A path `a::b::c` that starts at `start`: its segments, where it ends (after any `::<…>`) and
/// whether `(` follows.
struct PathAt {
    segments: Vec<String>,
    end: usize,
    call: bool,
}

fn path_at(text: &[u8], start: usize) -> PathAt {
    let mut segments = Vec::new();
    let mut at = start;
    loop {
        let from = at;
        at = run_end(text, at);
        segments.push(String::from_utf8_lossy(&text[from..at]).into_owned());
        let mut next = at;
        if text[next..].starts_with(b"::<") {
            next = angle_end(text, next + 2);
        }
        let separated = text[next..].starts_with(b"::");
        if separated && text.get(next + 2).is_some_and(|b| is_ident_start(*b)) {
            at = next + 2;
            continue;
        }
        at = next;
        break;
    }
    let call = text.get(skip_space(text, at)) == Some(&b'(');
    PathAt {
        segments,
        end: at,
        call,
    }
}

/// `.name`, a method or a field, but not `..name`.
fn after_single_dot(text: &[u8], at: usize) -> bool {
    at >= 1 && text[at - 1] == b'.' && !(at >= 2 && text[at - 2] == b'.')
}

fn after_path_sep(text: &[u8], at: usize) -> bool {
    at >= 2 && &text[at - 2..at] == b"::"
}

/// `name:` as a struct key, a parameter or a binding's type, but not `name::`.
fn before_single_colon(text: &[u8], end: usize) -> bool {
    let at = skip_space(text, end);
    text.get(at) == Some(&b':') && text.get(at + 1) != Some(&b':')
}

fn after_attribute_open(text: &[u8], at: usize) -> bool {
    text[..at].ends_with(b"#[") || text[..at].ends_with(b"#![")
}

fn after_word(text: &[u8], at: usize, expected: &str) -> bool {
    prev_word(text, at).is_some_and(|range| text[range] == *expected.as_bytes())
}

fn after_arrow(text: &[u8], at: usize) -> bool {
    prev_byte(text, at).is_some_and(|last| last >= 1 && &text[last - 1..=last] == b"->")
}

fn uppercase_initial(name: &str) -> bool {
    name.starts_with(|c: char| c.is_uppercase())
}

/// `{ key: value, shorthand, ..rest }` at the brace `open`…`close`: each key with its value's range (a
/// shorthand's own name is its value).
fn literal_keys(text: &[u8], open: usize, close: usize) -> Vec<(String, Range<usize>)> {
    fn entry(text: &[u8], range: Range<usize>, keys: &mut Vec<(String, Range<usize>)>) {
        let mut start = skip_space(text, range.start);
        let mut end = range.end;
        while end > start && text[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        if start >= end || text[start..end].starts_with(b"..") || !is_ident_start(text[start]) {
            return;
        }
        let name_end = run_end(text, start);
        let name = String::from_utf8_lossy(&text[start..name_end]).into_owned();
        let colon = skip_space(text, name_end);
        if colon >= end {
            keys.push((name, start..name_end));
        } else if text[colon] == b':' && text.get(colon + 1) != Some(&b':') {
            start = skip_space(text, colon + 1);
            keys.push((name, start..end));
        }
    }

    let mut keys = Vec::new();
    let mut depth = 0usize;
    let mut from = open + 1;
    for at in open + 1..=close {
        let b = text[at];
        if at == close || (depth == 0 && b == b',') {
            entry(text, from..at, &mut keys);
            from = at + 1;
        } else if is_opener(b) {
            depth += 1;
        } else if is_closer(b) {
            depth = depth.saturating_sub(1);
        }
    }
    keys
}

/// The identifiers a script binds, which a call to is not a call to the engine: names after `let`,
/// `let mut`, `const` and `static`, names followed by a `:`, closure parameters (next to a `|`) and a
/// name alone inside `Some(…)`, `Ok(…)` or `Err(…)`.
fn find_locals(text: &[u8]) -> BTreeSet<String> {
    let mut locals = BTreeSet::new();
    for range in idents(text, 0..text.len()) {
        let binds = match prev_word(text, range.start) {
            Some(prev) => {
                let prev_text = &text[prev.clone()];
                matches!(prev_text, b"let" | b"const" | b"static")
                    || (prev_text == b"mut" && after_word(text, prev.start, "let"))
            }
            None => false,
        };
        let next = text.get(skip_space(text, range.end)).copied();
        let piped = next == Some(b'|') || prev_byte(text, range.start).is_some_and(|at| text[at] == b'|');
        let alone = next == Some(b')')
            && prev_byte(text, range.start).is_some_and(|open| {
                text[open] == b'('
                    && prev_word(text, open).is_some_and(|ctor| {
                        ctor.end == open && matches!(&text[ctor], b"Some" | b"Ok" | b"Err")
                    })
            });
        if binds || piped || alone || before_single_colon(text, range.end) {
            locals.insert(word(text, &range));
        }
    }
    locals
}

/// A function of the file: the whole item, from its `fn` to its closing brace, and the brace it opens.
struct FnItem {
    whole: Range<usize>,
    open: usize,
}

/// Every `fn NAME` with a body: `fn(` is a type, and a declaration that a `;` ends has no body.
fn find_fns(text: &[u8]) -> BTreeMap<String, Vec<FnItem>> {
    let mut fns: BTreeMap<String, Vec<FnItem>> = BTreeMap::new();
    for keyword in idents(text, 0..text.len()) {
        if &text[keyword.clone()] != b"fn" {
            continue;
        }
        let name_start = skip_space(text, keyword.end);
        if !text.get(name_start).is_some_and(|b| is_ident_start(*b)) {
            continue;
        }
        let name_end = run_end(text, name_start);
        let mut depth = 0usize;
        let mut open = None;
        for (at, &b) in text.iter().enumerate().skip(name_end) {
            match b {
                b'(' | b'[' => depth += 1,
                b')' | b']' => depth = depth.saturating_sub(1),
                b'{' if depth == 0 => {
                    open = Some(at);
                    break;
                }
                b';' if depth == 0 => break,
                _ => {}
            }
        }
        let Some(open) = open else { continue };
        let Some(close) = close_of(text, open) else {
            continue;
        };
        fns.entry(String::from_utf8_lossy(&text[name_start..name_end]).into_owned())
            .or_default()
            .push(FnItem {
                whole: keyword.start..close + 1,
                open,
            });
    }
    fns
}

/// `let [mut] NAME [: T] = VALUE;` at depth 0 of `body`: each name with its value's range.
fn find_lets(text: &[u8], body: &Range<usize>) -> BTreeMap<String, Range<usize>> {
    /// The binding after the `let` that ends at `from`: its name, its value's range and where it ends.
    fn binding(text: &[u8], from: usize, limit: usize) -> Option<(String, Range<usize>, usize)> {
        let mut at = skip_space(text, from);
        if text[at..].starts_with(b"mut") && !text.get(at + 3).is_some_and(|b| is_ident_byte(*b)) {
            at = skip_space(text, at + 3);
        }
        if !text.get(at).is_some_and(|b| is_ident_start(*b)) {
            return None;
        }
        let name_end = run_end(text, at);
        let name = String::from_utf8_lossy(&text[at..name_end]).into_owned();
        let mut depth = 0usize;
        let mut equals = None;
        for index in name_end..limit {
            let b = text[index];
            if is_opener(b) {
                depth += 1;
            } else if is_closer(b) {
                depth = depth.saturating_sub(1);
            } else if depth == 0 && b == b';' {
                return None;
            } else if depth == 0 && b == b'=' {
                let before = text[index - 1];
                let after = text.get(index + 1).copied();
                if !b"=!<>+-*/%&|^".contains(&before) && after != Some(b'=') && after != Some(b'>') {
                    equals = Some(index);
                    break;
                }
            }
        }
        let equals = equals?;
        let mut depth = 0usize;
        for (index, &b) in text.iter().enumerate().take(limit).skip(equals + 1) {
            if is_opener(b) {
                depth += 1;
            } else if is_closer(b) {
                depth = depth.saturating_sub(1);
            } else if depth == 0 && b == b';' {
                return Some((name, equals + 1..index, index + 1));
            }
        }
        None
    }

    let mut lets = BTreeMap::new();
    let mut depth = 0usize;
    let mut at = body.start;
    while at < body.end {
        let b = text[at];
        if is_opener(b) {
            depth += 1;
        } else if is_closer(b) {
            depth = depth.saturating_sub(1);
        } else if is_ident_byte(b) {
            let end = run_end(text, at);
            if depth == 0
                && &text[at..end] == b"let"
                && let Some((name, value, after)) = binding(text, end, body.end)
            {
                lets.insert(name, value);
                at = after;
                continue;
            }
            at = end;
            continue;
        }
        at += 1;
    }
    lets
}

/// The statements at depth 0 of `body` that change a `let` binding after it is made, `NAME.insert(…);`
/// or `NAME.hook = …;`: the range of each, by the binding's name.
fn find_edits(
    text: &[u8],
    body: &Range<usize>,
    lets: &BTreeMap<String, Range<usize>>,
) -> BTreeMap<String, Vec<Range<usize>>> {
    let mut edits: BTreeMap<String, Vec<Range<usize>>> = BTreeMap::new();
    let mut depth = 0usize;
    let mut from = body.start;
    for at in body.start..body.end {
        let b = text[at];
        if is_opener(b) {
            depth += 1;
        } else if is_closer(b) {
            depth = depth.saturating_sub(1);
        } else if depth == 0 && b == b';' {
            let start = skip_space(text, from);
            let end = run_end(text, start);
            if end > start && is_ident_start(text[start]) && text.get(end) == Some(&b'.') {
                let name = String::from_utf8_lossy(&text[start..end]).into_owned();
                if lets.contains_key(&name) {
                    edits.entry(name).or_default().push(start..at);
                }
            }
            from = at + 1;
        }
    }
    edits
}

/* --------------------------------------------------------------------------- the engine's names */

/// What the engine and the cards crate offer a script, found in their sources.
#[derive(Debug, Default)]
struct Names {
    /// The `pub fn`s of `effects/` that return an `Effect`, and the subsystem functions it re-exports.
    verbs: BTreeSet<String>,
    /// The other `pub fn`s the prelude re-exports.
    readers: BTreeSet<String>,
    /// The fields of `Script`.
    hook_fields: BTreeSet<String>,
    /// The functions of `crates/cards/src/query.rs`.
    pool_fns: BTreeSet<String>,
    /// Its constants and types.
    pool_words: BTreeSet<String>,
    /// The functions of the cards crate's root.
    cards_fns: BTreeSet<String>,
}

/// The text after the closing parenthesis of the parameters at `name_end`, up to the body: the return
/// type, as the whitespace-collapsed text before any `where`. Empty when there is none.
fn return_type(text: &[u8], name_end: usize) -> String {
    let mut at = skip_space(text, name_end);
    if text.get(at) == Some(&b'<') {
        at = skip_space(text, angle_end(text, at));
    }
    if text.get(at) != Some(&b'(') {
        return String::new();
    }
    let Some(close) = close_of(text, at) else {
        return String::new();
    };
    let arrow = skip_space(text, close + 1);
    if !text[arrow..].starts_with(b"->") {
        return String::new();
    }
    let end = text[arrow..]
        .iter()
        .position(|b| matches!(b, b'{' | b';'))
        .map_or(text.len(), |offset| arrow + offset);
    String::from_utf8_lossy(&text[arrow + 2..end])
        .split_whitespace()
        .take_while(|token| *token != "where")
        .collect::<Vec<_>>()
        .join(" ")
}

/// The `pub fn`s that start a line (column 0), each with its return type.
fn column_zero_fns(text: &str) -> Vec<(String, String)> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut line_start = 0;
    for line in text.split_inclusive('\n') {
        if line.starts_with(PUB_FN) {
            let name_at = line_start + PUB_FN.len();
            let name_end = run_end(bytes, name_at);
            found.push((text[name_at..name_end].to_string(), return_type(bytes, name_end)));
        }
        line_start += line.len();
    }
    found
}

/// The identifier after a line's `prefix`, for the lines of `text` that start with it.
fn declared_after(text: &str, prefixes: &[&str]) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for line in text.lines() {
        for prefix in prefixes {
            if let Some(rest) = line.strip_prefix(prefix) {
                let end = run_end(rest.as_bytes(), 0);
                found.insert(rest[..end].to_string());
            }
        }
    }
    found
}

/// The `pub` fields of `pub struct NAME { … }`, each on a line of its own: `    pub field: Type,`.
fn struct_fields(text: &str, name: &str) -> BTreeSet<String> {
    let header = format!("pub struct {name} {{");
    let Some(at) = text.find(&header) else {
        return BTreeSet::new();
    };
    let open = at + header.len() - 1;
    let close = close_of(text.as_bytes(), open).unwrap_or(text.len());
    text[open..close]
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("    pub ")?;
            let end = run_end(rest.as_bytes(), 0);
            (rest[end..].starts_with(':')).then(|| rest[..end].to_string())
        })
        .collect()
}

/// A source file of the repository as its code text.
fn read_code(root: &Path, relative: &str) -> Result<String> {
    let path = root.join(relative);
    let source = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    Ok(code_text(&source))
}

/// The `*.rs` files directly in `dir`, sorted.
fn rust_files_in(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn load_names(root: &Path) -> Result<Names> {
    let mut names = Names::default();

    for file in rust_files_in(&root.join(EFFECTS_DIR))? {
        let relative = format!(
            "{EFFECTS_DIR}/{}",
            file.file_name().unwrap_or_default().to_string_lossy()
        );
        let text = read_code(root, &relative)?;
        for (name, returns) in column_zero_fns(&text) {
            if returns == EFFECT_TYPE {
                names.verbs.insert(name);
            } else {
                names.readers.insert(name);
            }
        }
        if relative.ends_with("/mod.rs") {
            for line in text.lines() {
                let line = line.trim();
                if let Some(path) = line.strip_prefix("pub use crate::subsystems::")
                    && let Some(last) = path.strip_suffix(';')
                    && !last.contains(['{', '*'])
                {
                    names
                        .verbs
                        .insert(last.rsplit("::").next().unwrap_or(last).to_string());
                }
            }
        }
    }

    let prelude = read_code(root, PRELUDE)?;
    for line in prelude.lines() {
        let Some(module) = line
            .trim()
            .strip_prefix("pub use crate::")
            .and_then(|rest| rest.strip_suffix("::*;"))
        else {
            continue;
        };
        if module == "effects" {
            continue;
        }
        let file = root.join(ENGINE_SRC).join(format!("{module}.rs"));
        let files = if file.is_file() {
            vec![file]
        } else {
            rust_files_in(&root.join(ENGINE_SRC).join(module))?
        };
        for file in files {
            let source = fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
            for (name, _) in column_zero_fns(&code_text(&source)) {
                names.readers.insert(name);
            }
        }
    }
    if let Some(start) = prelude.find("pub use crate::catalog::{") {
        let list = &prelude[start..];
        let list = &list[..list.find('}').unwrap_or(list.len())];
        let catalog_fns: BTreeSet<String> = column_zero_fns(&read_code(root, ENGINE_CATALOG)?)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        for name in list.split(['{', ',']).skip(1).map(str::trim) {
            if catalog_fns.contains(name) {
                names.readers.insert(name.to_string());
            }
        }
    }
    names.readers.insert("json_as".to_string());
    names.readers.retain(|name| !names.verbs.contains(name));

    names.hook_fields = struct_fields(&read_code(root, SCRIPT_TYPES)?, SCRIPT_STRUCT);
    let pools = read_code(root, POOLS)?;
    names.pool_fns = column_zero_fns(&pools)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    names.pool_words = declared_after(&pools, &["pub const ", "pub type "]);
    names.cards_fns = column_zero_fns(&read_code(root, CARDS_LIB)?)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    Ok(names)
}

/* ------------------------------------------------------------------------------ one card file */

/// One script file's code, with what the reading of a face needs.
struct Code<'a> {
    text: &'a [u8],
    names: &'a Names,
    fns: BTreeMap<String, Vec<FnItem>>,
    lets: BTreeMap<String, Range<usize>>,
    /// The statements that change a `let` binding after it is made, by its name.
    edits: BTreeMap<String, Vec<Range<usize>>>,
    locals: BTreeSet<String>,
    /// The inside of `script()`.
    body: Range<usize>,
}

impl Code<'_> {
    fn word(&self, range: &Range<usize>) -> String {
        word(self.text, range)
    }

    /// The first `FACE_TEXT_CHARS` characters of a range's code, its whitespace collapsed.
    fn excerpt(&self, range: &Range<usize>) -> String {
        word(self.text, range)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(FACE_TEXT_CHARS)
            .collect()
    }

    /// The value of the `Script { … }` literal's key `field` in `value`, if that range is one.
    fn script_key(&self, value: &Range<usize>, field: &str) -> Option<Range<usize>> {
        for ident in idents(self.text, value.clone()) {
            if self.word(&ident) != SCRIPT_STRUCT || after_path_sep(self.text, ident.start) {
                continue;
            }
            let open = skip_space(self.text, ident.end);
            if self.text.get(open) != Some(&b'{') {
                continue;
            }
            let close = close_of(self.text, open)?;
            return literal_keys(self.text, open, close)
                .into_iter()
                .find_map(|(key, range)| (key == field).then_some(range));
        }
        None
    }

    /// The `FIELD` of `NAME.FIELD` after the identifier ending at `end`, when it is a hook.
    fn borrowed_hook(&self, end: usize) -> Option<String> {
        if self.text.get(end) != Some(&b'.') {
            return None;
        }
        let start = end + 1;
        let field = String::from_utf8_lossy(&self.text[start..run_end(self.text, start)]).into_owned();
        self.names.hook_fields.contains(&field).then_some(field)
    }

    /// Everything a face's start range reaches: the ranges of the helper functions and `let` bindings
    /// of the script it names, the statements that change those bindings, and theirs in turn.
    /// `NAME.hook` reaches only that hook of a `Script` literal. Also the hooks the changing statements
    /// assign (`NAME.hook = …;`).
    fn region(&self, start: Range<usize>) -> (Vec<Range<usize>>, BTreeSet<String>) {
        let mut seen: BTreeSet<(usize, usize)> = BTreeSet::from([(start.start, start.end)]);
        let mut ranges = vec![start.clone()];
        let mut assigned = BTreeSet::new();
        let mut queue = vec![start];
        while let Some(range) = queue.pop() {
            for ident in idents(self.text, range) {
                if after_single_dot(self.text, ident.start)
                    || after_path_sep(self.text, ident.start)
                    || before_single_colon(self.text, ident.end)
                {
                    continue;
                }
                let name = self.word(&ident);
                let mut reached: Vec<Range<usize>> = Vec::new();
                if name != "script" {
                    for item in self.fns.get(&name).into_iter().flatten() {
                        reached.push(item.whole.clone());
                    }
                }
                if self.body.contains(&ident.start)
                    && let Some(value) = self.lets.get(&name)
                {
                    let narrowed = self
                        .borrowed_hook(ident.end)
                        .and_then(|field| self.script_key(value, &field));
                    if narrowed.is_none() {
                        for edit in self.edits.get(&name).into_iter().flatten() {
                            if let Some(field) = self.borrowed_hook(run_end(self.text, edit.start)) {
                                assigned.insert(field);
                            }
                            reached.push(edit.clone());
                        }
                    }
                    reached.push(narrowed.unwrap_or_else(|| value.clone()));
                }
                for next in reached {
                    if seen.insert((next.start, next.end)) {
                        ranges.push(next.clone());
                        queue.push(next);
                    }
                }
            }
        }
        (ranges, assigned)
    }

    /// The hooks the `Script` literals in `ranges` set, and whether any literal (or a
    /// `Script::default()`) is there at all.
    fn hooks(&self, ranges: &[Range<usize>]) -> (BTreeSet<String>, bool) {
        let mut hooks = BTreeSet::new();
        let mut found = false;
        for range in ranges {
            for ident in idents(self.text, range.clone()) {
                if after_path_sep(self.text, ident.start) || after_single_dot(self.text, ident.start) {
                    continue;
                }
                let path = path_at(self.text, ident.start);
                if let [.., owner, last] = path.segments.as_slice()
                    && path.call
                    && last == "default"
                    && (owner == SCRIPT_STRUCT || owner == "Default")
                {
                    found = true;
                }
                let open = skip_space(self.text, ident.end);
                if self.word(&ident) == SCRIPT_STRUCT
                    && self.text.get(open) == Some(&b'{')
                    && !after_arrow(self.text, ident.start)
                    && let Some(close) = close_of(self.text, open)
                {
                    found = true;
                    for (key, _) in literal_keys(self.text, open, close) {
                        if self.names.hook_fields.contains(&key) {
                            hooks.insert(key);
                        }
                    }
                }
            }
        }
        (hooks, found)
    }

    /// Sort the names called in `range` into `face`.
    fn classify(&self, range: Range<usize>, face: &mut Face) {
        let names = self.names;
        for ident in idents(self.text, range) {
            if after_path_sep(self.text, ident.start) {
                continue;
            }
            let path = path_at(self.text, ident.start);
            let first = path.segments[0].as_str();
            let last = path.segments[path.segments.len() - 1].as_str();
            if after_single_dot(self.text, ident.start) {
                if path.call && CTX_METHODS.contains(&first) {
                    face.readers.insert(first.to_string());
                }
                continue;
            }
            let single = path.segments.len() == 1;
            let local = self.locals.contains(first);
            if single && first == CATALOG_OBJECT && !local {
                let method = (self.text.get(path.end) == Some(&b'.'))
                    .then(|| {
                        let start = path.end + 1;
                        word(self.text, &(start..run_end(self.text, start)))
                    })
                    .filter(|method| !method.is_empty());
                face.pools.insert(match method {
                    Some(method) => format!("{CATALOG_OBJECT}.{method}"),
                    None => CATALOG_OBJECT.to_string(),
                });
                continue;
            }
            if names.pool_words.contains(last) && ((single && !local) || first == "crate") {
                face.pools.insert(last.to_string());
                continue;
            }
            if !path.call {
                // A bare engine name passed as a value (`map_or(0, given_lucky_on)`): a reader
                // all the same (#464 gap 5). Definitions and attributes are not uses.
                if single
                    && !local
                    && !after_word(self.text, ident.start, "fn")
                    && !after_attribute_open(self.text, ident.start)
                    && names.readers.contains(first)
                {
                    face.readers.insert(first.to_string());
                }
                continue;
            }
            if after_word(self.text, ident.start, "fn") || after_attribute_open(self.text, ident.start) {
                continue;
            }
            if single {
                self.classify_bare(first, local, face);
            } else {
                self.classify_path(&path.segments, face);
            }
        }
    }

    /// A call to a bare name: the first match of this file's functions, the named readers, the pools,
    /// the verbs, the readers, a local binding, and else unclassified.
    fn classify_bare(&self, name: &str, local: bool, face: &mut Face) {
        let names = self.names;
        if KEYWORDS.contains(&name) || uppercase_initial(name) || self.fns.contains_key(name) {
            return;
        }
        if NAMED_READERS.contains(&name) {
            face.readers.insert(name.to_string());
        } else if names.pool_fns.contains(name) {
            face.pools.insert(name.to_string());
        } else if names.verbs.contains(name) {
            face.verbs.insert(name.to_string());
        } else if names.readers.contains(name) {
            face.readers.insert(name.to_string());
        } else if !local {
            face.unclassified.insert(name.to_string());
        }
    }

    /// A call by a path of two segments or more. A path through a type (`Arc::new`,
    /// `subsystems::quests::QuestBook::new`) is not a function of the engine's and is skipped.
    fn classify_path(&self, segments: &[String], face: &mut Face) {
        let names = self.names;
        let full = segments.join("::");
        let last = segments[segments.len() - 1].as_str();
        if segments[1..].iter().any(|segment| uppercase_initial(segment)) {
            return;
        }
        match segments[0].as_str() {
            "subsystems" => {
                face.readers.insert(full);
            }
            "crate" => {
                if names.pool_fns.contains(last) {
                    face.pools.insert(last.to_string());
                } else if segments.len() == 2 && names.cards_fns.contains(last) {
                    face.readers.insert(full);
                } else {
                    face.unclassified.insert(full);
                }
            }
            "jackioh_engine" if segments[1] == "effects" => {
                if names.verbs.contains(last) {
                    face.verbs.insert(last.to_string());
                } else if names.readers.contains(last) {
                    face.readers.insert(last.to_string());
                } else {
                    face.unclassified.insert(full);
                }
            }
            "jackioh_engine" => {
                face.readers.insert(full);
            }
            _ => {}
        }
    }

    /// One face, read from the range its `CardScripts` key's value starts at.
    fn face(&self, which: &str, start: &Range<usize>) -> Result<Face, String> {
        let (ranges, assigned) = self.region(start.clone());
        let (mut hooks, found) = self.hooks(&ranges);
        let found = found || !assigned.is_empty();
        hooks.extend(assigned);
        if !found {
            return Err(format!(
                "the {which} face (`{}`) names no `Script` literal in this file",
                self.excerpt(start)
            ));
        }
        let mut face = Face {
            hooks,
            ..Face::default()
        };
        for range in ranges {
            self.classify(range, &mut face);
        }
        Ok(face)
    }

    /// How the Radiant value is made from the base value: its text against `base`, `base.clone()` and
    /// `..base`, one `let` followed when it is a lone name.
    fn reuse(&self, base: &Range<usize>, radiant: &Range<usize>) -> Reuse {
        let squeeze = |range: &Range<usize>| -> String {
            self.text[range.clone()]
                .iter()
                .filter(|b| !b.is_ascii_whitespace())
                .map(|b| *b as char)
                .collect()
        };
        let base = squeeze(base);
        if !is_lone_ident(&base) {
            return Reuse::No;
        }
        let cloned = format!("{base}.clone()");
        let mut radiant = squeeze(radiant);
        if radiant == base || radiant == cloned {
            return Reuse::Whole;
        }
        let mut edited = false;
        if is_lone_ident(&radiant)
            && let Some(value) = self.lets.get(&radiant)
        {
            edited = self.edits.contains_key(&radiant);
            radiant = squeeze(value);
        }
        if radiant == base || radiant == cloned {
            return if edited { Reuse::Spread } else { Reuse::Whole };
        }
        let spread = format!("..{base}");
        let spreads = radiant.match_indices(&spread).any(|(at, _)| {
            !radiant
                .as_bytes()
                .get(at + spread.len())
                .is_some_and(|b| is_ident_byte(*b))
        });
        if spreads { Reuse::Spread } else { Reuse::No }
    }
}

/// A text that is one identifier and nothing else.
fn is_lone_ident(text: &str) -> bool {
    text.bytes().next().is_some_and(is_ident_start) && text.bytes().all(is_ident_byte)
}

/// What one card script's source says about its two faces. An error is the reason it could not be read.
fn read_card(source: &str, names: &Names) -> Result<CardSurface, String> {
    let text = code_text(source);
    let bytes = text.as_bytes();
    let fns = find_fns(bytes);
    let script = fns
        .get("script")
        .and_then(|items| items.first())
        .ok_or("no `pub fn script()`")?;
    let body = script.open + 1..script.whole.end - 1;
    let lets = find_lets(bytes, &body);
    let code = Code {
        text: bytes,
        names,
        edits: find_edits(bytes, &body, &lets),
        lets,
        locals: find_locals(bytes),
        fns,
        body: body.clone(),
    };

    let literal = idents(bytes, body)
        .into_iter()
        .rev()
        .find_map(|ident| {
            let open = skip_space(bytes, ident.end);
            (word(bytes, &ident) == "CardScripts" && bytes.get(open) == Some(&b'{')).then_some(open)
        })
        .ok_or("`script()` builds no `CardScripts { … }` literal")?;
    let close = close_of(bytes, literal).ok_or("the `CardScripts { … }` literal never closes")?;
    let keys = literal_keys(bytes, literal, close);
    let key = |name: &str| {
        keys.iter()
            .find(|(key, _)| key == name)
            .map(|(_, range)| range.clone())
            .ok_or(format!("`CardScripts` sets no `{name}`"))
    };
    let (base_at, radiant_at) = (key("base")?, key("radiant")?);

    let base = code.face("base", &base_at)?;
    let radiant = code.face("radiant", &radiant_at)?;
    let mut whole = Face::default();
    code.classify(0..bytes.len(), &mut whole);
    let elsewhere = whole.without([&base, &radiant]);
    Ok(CardSurface {
        radiant_reuses_base: code.reuse(&base_at, &radiant_at),
        base,
        radiant,
        elsewhere,
    })
}

/* ------------------------------------------------------------------------------ the survey */

/// The id a card file declares, as `crates/cards/build.rs` reads it: `pub const ID: &str = "…";`.
fn read_id(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("pub const ID")?.trim_start();
        let rest = rest
            .strip_prefix(':')?
            .trim_start()
            .strip_prefix("&str")?
            .trim_start();
        let rest = rest.strip_prefix('=')?.trim_start().strip_prefix('"')?;
        let (id, tail) = rest.split_once('"')?;
        (tail.trim_start().starts_with(';') && !id.is_empty()).then(|| id.to_string())
    })
}

/// Every `*.rs` under `dir`, sorted, subdirectories included.
fn rust_files_under(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<_>>()?;
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files_under(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

/// Each card id that has a script file, with the file's path from the repository root.
fn script_files(root: &Path) -> Result<BTreeMap<String, String>> {
    let mut paths = Vec::new();
    rust_files_under(&root.join(SCRIPTS_DIR), &mut paths)?;
    let mut by_id = BTreeMap::new();
    for path in paths {
        let source = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        if let Some(id) = read_id(&source) {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            by_id.insert(id, relative.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(by_id)
}

/// Add each of `names` to `usage` for `card`, an L14 card to the fields of its own.
fn count_usage(usage: &mut BTreeMap<String, Usage>, names: BTreeSet<&String>, card: &CardReport) {
    for name in names {
        let entry = usage.entry(name.clone()).or_default();
        if card.stays_in_rust {
            entry.stay_in_rust_cards += 1;
            entry.stay_in_rust_ids.push(card.id.clone());
        } else {
            entry.cards += 1;
            entry.ids.push(card.id.clone());
        }
    }
}

/// Every name once per card, over the card's faces and what is elsewhere in its file.
fn summarise(cards: &[CardReport]) -> Summary {
    let mut summary = Summary::default();
    for card in cards {
        let faces = [&card.surface.base, &card.surface.radiant, &card.surface.elsewhere];
        let union = |pick: fn(&Face) -> &BTreeSet<String>| -> BTreeSet<&String> {
            faces.iter().flat_map(|face| pick(face)).collect()
        };
        count_usage(&mut summary.hooks, union(|face| &face.hooks), card);
        count_usage(&mut summary.verbs, union(|face| &face.verbs), card);
        count_usage(&mut summary.readers, union(|face| &face.readers), card);
        count_usage(&mut summary.pools, union(|face| &face.pools), card);
        count_usage(&mut summary.unclassified, union(|face| &face.unclassified), card);
    }
    summary
}

/// The report over the catalog's cards, or over the ids in `only` when it is not empty.
fn survey(root: &Path, only: &[String]) -> Result<Report> {
    let names = load_names(root)?;
    let files = script_files(root)?;
    if let Some(unknown) = only.iter().find(|id| !CATALOG_IDS.contains(id)) {
        bail!("luau surface: no catalog card {unknown}");
    }
    let mut report = Report::default();
    for id in CATALOG_IDS
        .iter()
        .filter(|id| only.is_empty() || only.contains(id))
    {
        let Some(file) = files.get(id) else {
            report.unreadable.push(Unreadable {
                id: id.clone(),
                file: None,
                reason: format!("no script file under {SCRIPTS_DIR}"),
            });
            continue;
        };
        let source = fs::read_to_string(root.join(file)).with_context(|| format!("reading {file}"))?;
        match read_card(&source, &names) {
            Ok(surface) => report.cards.push(CardReport {
                id: id.clone(),
                file: file.clone(),
                stays_in_rust: STAY_IN_RUST.contains(&id.as_str()),
                surface,
            }),
            Err(reason) => {
                // A ported card's `.rs` holds its tests alone; its script is the `.luau` beside it.
                let luau = Path::new(file).with_extension("luau");
                let reason = if root.join(&luau).is_file() {
                    format!("ported: its script is {}", luau.display())
                } else {
                    reason
                };
                report.unreadable.push(Unreadable {
                    id: id.clone(),
                    file: Some(file.clone()),
                    reason,
                });
            }
        }
    }
    report.summary = summarise(&report.cards);
    Ok(report)
}

/* ------------------------------------------------------------------------------ the printer */

/// A table cell holding names, `—` when there are none.
fn cell<'a>(names: impl IntoIterator<Item = &'a String>) -> String {
    let joined = names
        .into_iter()
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ");
    if joined.is_empty() {
        "—".to_string()
    } else {
        joined
    }
}

fn ids_cell(ids: &[String]) -> String {
    if ids.is_empty() {
        "—".to_string()
    } else {
        ids.join(", ")
    }
}

fn usage_table(out: &mut String, title: &str, usage: &BTreeMap<String, Usage>) {
    let _ = writeln!(out, "## {title}\n");
    let _ = writeln!(out, "| Name | Cards | Which | L14 cards | Which (L14) |");
    let _ = writeln!(out, "|---|---|---|---|---|");
    for (name, usage) in usage {
        let _ = writeln!(
            out,
            "| `{name}` | {} | {} | {} | {} |",
            usage.cards,
            ids_cell(&usage.ids),
            usage.stay_in_rust_cards,
            ids_cell(&usage.stay_in_rust_ids)
        );
    }
    let _ = writeln!(out);
}

/// The report as Markdown: a table of the cards, then one of each kind of name, then the unreadable.
fn markdown(report: &Report) -> String {
    let mut out = String::new();
    let stay = report.cards.iter().filter(|card| card.stays_in_rust).count();
    let _ = writeln!(
        out,
        "# Luau surface: {} cards ({stay} stay in Rust), {} unreadable, {} unclassified names\n",
        report.cards.len() + report.unreadable.len(),
        report.unreadable.len(),
        report.summary.unclassified.len()
    );
    let _ = writeln!(out, "## Cards\n");
    let _ = writeln!(
        out,
        "| Card | Rust | Radiant reuses base | Base hooks | Radiant hooks | Base verbs | Radiant verbs | Readers | Pools | Unclassified |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|---|---|");
    for card in &report.cards {
        let surface = &card.surface;
        let faces = [&surface.base, &surface.radiant, &surface.elsewhere];
        let union = |pick: fn(&Face) -> &BTreeSet<String>| -> BTreeSet<String> {
            faces.iter().flat_map(|face| pick(face).iter().cloned()).collect()
        };
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            card.id,
            if card.stays_in_rust { "yes" } else { "—" },
            surface.radiant_reuses_base.label(),
            cell(&surface.base.hooks),
            cell(&surface.radiant.hooks),
            cell(&surface.base.verbs),
            cell(&surface.radiant.verbs),
            cell(&union(|face| &face.readers)),
            cell(&union(|face| &face.pools)),
            cell(&union(|face| &face.unclassified)),
        );
    }
    let _ = writeln!(out);
    usage_table(&mut out, "Hook types", &report.summary.hooks);
    usage_table(&mut out, "Verbs", &report.summary.verbs);
    usage_table(&mut out, "Readers", &report.summary.readers);
    usage_table(&mut out, "Pools", &report.summary.pools);
    usage_table(&mut out, "Unclassified", &report.summary.unclassified);
    let _ = writeln!(out, "## Unreadable\n");
    let _ = writeln!(out, "| Card | File | Reason |");
    let _ = writeln!(out, "|---|---|---|");
    for card in &report.unreadable {
        let _ = writeln!(
            out,
            "| {} | {} | {} |",
            card.id,
            card.file.as_deref().unwrap_or("—"),
            card.reason.replace('|', "\\|")
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> Names {
        load_names(&repo_root()).unwrap()
    }

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    fn face(hooks: &[&str], verbs: &[&str], readers: &[&str]) -> Face {
        Face {
            hooks: set(hooks),
            verbs: set(verbs),
            readers: set(readers),
            ..Face::default()
        }
    }

    #[test]
    fn luau_code_text_blanks_everything_but_code() {
        let source = "let a = f(\"x(\"); // g(1)\nlet b = r#\"h(\"#; /* i( /* j( */ k( */ let c = '(' + é;\n#[cfg(test)]\nl(1);\n";
        let expected = format!(
            "let a = f(\"  \");{}\nlet b = r#\"  \"#;{}let c = ' ' +  ;\n",
            " ".repeat(8),
            " ".repeat(22)
        );
        let code = code_text(source);
        assert_eq!(code, expected);
        assert_eq!(code.len(), code.chars().count());
    }

    #[test]
    fn luau_literals_read_each_face() {
        let surface = read_card(include_str!("../../tests/luau/literals.rs"), &names()).unwrap();
        assert_eq!(
            surface.base,
            face(&["cry", "targets"], &["destroy"], &["hook", "json_as"])
        );
        assert_eq!(
            surface.radiant,
            face(&["cry"], &["destroy_all"], &["hook", "json_as"])
        );
        assert_eq!(surface.radiant_reuses_base, Reuse::No);
        assert_eq!(surface.elsewhere, Face::default());
    }

    #[test]
    fn luau_helper_faces() {
        let surface = read_card(include_str!("../../tests/luau/helpers.rs"), &names()).unwrap();
        let both = face(&["cry", "end_of_turn"], &["draw"], &["hook", "json_as", "param"]);
        assert_eq!(surface.base, both);
        assert_eq!(surface.radiant, both);
        assert_eq!(surface.radiant_reuses_base, Reuse::No);
        assert_eq!(surface.elsewhere, face(&[], &["heal"], &[]));
    }

    #[test]
    fn luau_radiant_clones_base() {
        let surface = read_card(include_str!("../../tests/luau/clone.rs"), &names()).unwrap();
        assert_eq!(surface.radiant_reuses_base, Reuse::Whole);
        assert_eq!(surface.base, face(&["death"], &["draw"], &["hook", "json_as"]));
        assert_eq!(surface.radiant, surface.base);
        assert_eq!(surface.elsewhere, Face::default());
    }

    #[test]
    fn luau_radiant_spreads_base() {
        let surface = read_card(include_str!("../../tests/luau/spread.rs"), &names()).unwrap();
        assert_eq!(surface.radiant_reuses_base, Reuse::Spread);
        assert_eq!(surface.base, face(&["death"], &["draw"], &["hook", "json_as"]));
        assert_eq!(
            surface.radiant,
            face(&["death", "end_of_turn"], &["draw", "heal"], &["hook", "json_as"])
        );
    }

    #[test]
    fn luau_radiant_borrows_one_field() {
        let surface = read_card(include_str!("../../tests/luau/field.rs"), &names()).unwrap();
        assert_eq!(surface.radiant, face(&["cry"], &["recruit_all"], &["hook"]));
        assert_eq!(
            surface.base,
            face(&["cry", "death"], &["draw", "recruit_all"], &["hook", "json_as"])
        );
        assert_eq!(surface.radiant_reuses_base, Reuse::No);
        assert_eq!(surface.elsewhere, Face::default());
    }

    #[test]
    fn luau_radiant_edited_after_the_clone() {
        let surface = read_card(include_str!("../../tests/luau/edit.rs"), &names()).unwrap();
        assert_eq!(surface.radiant_reuses_base, Reuse::Spread);
        assert_eq!(surface.base, face(&["cry"], &["draw"], &["hook", "json_as"]));
        assert_eq!(
            surface.radiant,
            face(&["cry", "death"], &["draw", "heal"], &["hook", "json_as"])
        );
        assert_eq!(surface.elsewhere, Face::default());
    }

    #[test]
    fn luau_classifies_every_call() {
        let surface = read_card(include_str!("../../tests/luau/calls.rs"), &names()).unwrap();
        let expected = Face {
            hooks: set(&["cry"]),
            verbs: set(&["damage"]),
            readers: set(&[
                "chosen_options",
                "crate::card_def",
                "given_lucky_on",
                "hook",
                "jackioh_engine::work::part_memory_key",
                "json_as",
                "live_self",
                "remember",
                "subsystems::call_to_chaos",
            ]),
            pools: set(&["TRAP_TYPES", "pool"]),
            unclassified: set(&["mystery_helper"]),
        };
        assert_eq!(surface.base, expected);
        assert_eq!(surface.radiant, expected);
        assert_eq!(surface.radiant_reuses_base, Reuse::Whole);
        assert_eq!(surface.elsewhere, Face::default());
    }

    #[test]
    fn luau_unreadable_faces_give_reasons() {
        let names = names();
        let reason = read_card(include_str!("../../tests/luau/unreadable.rs"), &names).unwrap_err();
        assert!(reason.contains("the base face (`shared::face()`)"), "{reason}");
        let reason = read_card("pub fn script() -> CardScripts { make() }", &names).unwrap_err();
        assert!(reason.contains("builds no `CardScripts"), "{reason}");
        let reason = read_card("pub const ID: &str = \"x\";", &names).unwrap_err();
        assert!(reason.contains("no `pub fn script()`"), "{reason}");
        let reason = read_card(
            "pub fn script() -> CardScripts { CardScripts { base: Script::default() } }",
            &names,
        )
        .unwrap_err();
        assert!(reason.contains("sets no `radiant`"), "{reason}");
    }

    #[test]
    fn luau_surface_reads_the_engine() {
        let names = names();
        for verb in ["destroy", "destroy_all", "glitch", "gain_mana"] {
            assert!(names.verbs.contains(verb), "verb {verb}");
        }
        for reader in ["chosen_options", "param", "zone_cards", "json_as", "hook"] {
            assert!(names.readers.contains(reader), "reader {reader}");
        }
        assert!(!names.readers.contains("destroy") && !names.readers.contains("gain_mana"));
        for field in ["cry", "targets", "start_of_opponent_turn"] {
            assert!(names.hook_fields.contains(field), "hook {field}");
        }
        assert!(
            names.pool_fns.is_superset(&set(&["pool", "query"])),
            "{:?}",
            names.pool_fns
        );
        assert!(
            names
                .pool_words
                .is_superset(&set(&["CardQuery", "TRAP_TYPES", "catalog"])),
            "{:?}",
            names.pool_words
        );
        assert!(names.cards_fns.contains("card_def"));
    }

    #[test]
    fn luau_surveys_every_catalog_card() {
        let report = survey(&repo_root(), &[]).unwrap();
        for id in CATALOG_IDS.iter() {
            let reported = report.cards.iter().filter(|card| &card.id == id).count();
            let unreadable = report
                .unreadable
                .iter()
                .filter(|card| &card.id == id)
                .collect::<Vec<_>>();
            assert_eq!(reported + unreadable.len(), 1, "{id}");
        }
        assert_eq!(report.cards.len() + report.unreadable.len(), CATALOG_IDS.len());
        let files = script_files(&repo_root()).unwrap();
        for card in &report.unreadable {
            assert_eq!(card.file.is_some(), files.contains_key(&card.id), "{}", card.id);
        }
        for card in &report.cards {
            assert_eq!(files.get(&card.id), Some(&card.file));
        }
    }

    #[test]
    fn luau_marks_the_cards_that_stay_in_rust() {
        let report = survey(&repo_root(), &[]).unwrap();
        let marked: BTreeSet<&str> = report
            .cards
            .iter()
            .filter(|card| card.stays_in_rust)
            .map(|card| card.id.as_str())
            .collect();
        assert_eq!(marked, STAY_IN_RUST.into_iter().collect::<BTreeSet<_>>());
        assert_eq!(marked.len(), 14);
        let hook = &report.summary.hooks["quests"];
        assert_eq!(hook.cards, 0);
        assert_eq!(hook.stay_in_rust_ids, ["classic-090"]);
    }

    /// Classic #32 Felinor Feelings, a card kept in Rust (`docs/v0.4.0/README.md` §4), whose faces
    /// call different verbs. A ported card's `.rs` holds no script for the survey to read.
    #[test]
    fn luau_classic_032_felinor_feelings() {
        let report = survey(&repo_root(), &["classic-032".into()]).unwrap();
        assert_eq!(report.cards.len(), 1);
        let feelings = &report.cards[0].surface;
        assert!(feelings.base.hooks.contains("cry") && feelings.radiant.hooks.contains("cry"));
        assert!(feelings.base.verbs.contains("steal"));
        assert!(!feelings.base.verbs.contains("choose_target_where"));
        assert!(feelings.radiant.verbs.contains("choose_target_where"));
        assert_eq!(report.summary.verbs["steal"].cards, 1);
        assert_eq!(report.summary.verbs["steal"].ids, ["classic-032"]);
        assert!(survey(&repo_root(), &["nope".into()]).is_err());
        // Every card is Rust again since #902, Bigot (once the first card ported) among them.
        let bigot = survey(&repo_root(), &["core-002".into()]).unwrap();
        assert_eq!(bigot.cards.len(), 1);
        assert!(bigot.unreadable.is_empty());
    }

    #[test]
    fn luau_reports_print_as_markdown_and_json() {
        let report = survey(&repo_root(), &["classic-032".into(), "classic-090".into()]).unwrap();
        let text = markdown(&report);
        assert!(
            text.starts_with(
                "# Luau surface: 2 cards (1 stay in Rust), 0 unreadable, 0 unclassified names\n"
            )
        );
        for heading in [
            "Cards",
            "Hook types",
            "Verbs",
            "Readers",
            "Pools",
            "Unclassified",
            "Unreadable",
        ] {
            assert!(text.contains(&format!("\n## {heading}\n")), "{heading}");
        }
        assert!(
            text.contains(
                "| classic-032 | — | no | `cry`, `target_checks`, `targets` | `cry`, `resume` | `steal` | \
                 `choose_target_where`, `steal`, `summon` |"
            ),
            "{text}"
        );
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["cards"][0]["id"], "classic-032");
        assert_eq!(json["cards"][0]["stays_in_rust"], false);
        assert_eq!(json["cards"][0]["radiant_reuses_base"], "no");
        assert_eq!(json["cards"][0]["base"]["verbs"][0], "steal");
        assert_eq!(json["cards"][1]["id"], "classic-090");
        assert_eq!(json["cards"][1]["stays_in_rust"], true);
        assert_eq!(json["summary"]["verbs"]["steal"]["ids"][0], "classic-032");
    }
}
