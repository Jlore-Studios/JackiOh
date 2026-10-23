//! `cargo jackioh spec check` and `cargo jackioh spec index`: the spec graph's structural checks
//! (SURFACE §15, part 28). They replace `packages/engine/test/rulings.test.ts`'s index and
//! `packages/engine/scripts/rulings-coverage.ts`.
//!
//! `spec/` is the specification as notes: one per section (`spec/NN-<slug>.md`, its `## N.` and
//! `### N.M` headings the section ids) and one per ruling (`spec/rulings/R<nnnn>.md`), whose front
//! matter is
//!
//! ```text
//! ---
//! id: R195
//! title: When a card glows yellow (`conditionActive`)
//! cards: "§10.8, §10.9, #10, …"
//! proven_in:
//!   - crates/engine/tests/rules/condition_active.rs
//! ---
//! ```
//!
//! `check` holds seven rules:
//!
//! 1. every ruling note's `id` matches its file name (`R195` is `R0195.md`), and ids are unique;
//! 2. every note lists at least one `proven_in` path, and each exists and holds a proof of the id, a
//!    test that runs (#1718): a Rust `fn` whose name has the `_`-delimited token `r<n>`, marked
//!    `#[test]` or `#[tokio::test]` and not `#[ignore]`d but for a `daily:` reason (super.yml runs
//!    those), or listed in a call of a macro the same file defines to make tests (`both_stores!`), or
//!    a `mod` so named whose body holds such a test; a TypeScript `it(`, `test(` or `describe(` whose
//!    title has `R<n>` as a word, with no `.skip`, `.todo`, `.fixme` or `.only` on it or on a call
//!    around it; or an SQL evidence script's `\echo '### R<n>: … ###'` heading (the heading form
//!    `rulings.test.ts` credited; a mention in prose is not a proof);
//! 3. every `r<n>` token of a Rust `fn` or `mod` name, and every `R<n>` cited in `crates/`,
//!    `apps/web/src/` and `e2e/` (comments included, recursively), has a note;
//! 4. every `[[…]]` link in `spec/` resolves: `[[R113]]` to a ruling note, `[[§10.3]]` to a section
//!    heading, anything else to a note's file name. Links inside code are not links;
//! 5. every ruling note's `title` and `cards` are present and not empty;
//! 6. `spec/INDEX.md` is what `index` writes from the notes as they stand (line endings aside), so
//!    a note added or changed without `index` run again fails (#669);
//! 7. no file the third rule reads holds an unresolved-ruling marker (`UNRESOLVED_MARKERS`, in any
//!    case, where a word starts): once a ruling exists, a comment still saying it is to be written
//!    invites a second decision of the same point (#1243).
//!
//! It reads ids, file names, headings and test names, never the wording but rule 7's markers (#133): a reworded sentence
//! fails nothing, though a reworded `title` changes the index (rule 6). `index` rewrites `spec/INDEX.md` from the ruling notes: id, title, note and the
//! files that prove it, one line each.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::patches::repo_root;

/// `jackioh spec check | index`.
#[derive(clap::Args)]
pub struct Args {
    /// The repository root (default: the workspace this binary was built from, as every other
    /// command reads it).
    #[arg(long)]
    pub root: Option<PathBuf>,
    #[command(subcommand)]
    pub command: SpecCommand,
}

#[derive(clap::Subcommand)]
pub enum SpecCommand {
    /// The seven checks (SURFACE §15, #1243): ids and file names, proofs, citations, links, titles and cards, the index, unresolved-ruling markers.
    Check,
    /// Rewrite `spec/INDEX.md` from the ruling notes.
    Index,
}

pub fn run(args: Args) -> Result<()> {
    let root = args.root.unwrap_or_else(repo_root);
    match args.command {
        SpecCommand::Check => check(&root),
        SpecCommand::Index => index(&root),
    }
}

/// The directories `check`'s third rule reads, recursively.
const CITING_ROOTS: &[&str] = &["crates", "apps/web/src", "e2e"];

/// Directories never read: build output, dependencies, test artefacts and generated packages.
const SKIPPED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    "coverage",
    "artifacts",
    "pkg",
    "screenshots",
    "videos",
    "downloads",
];

/// The files whose text counts as code for the third rule. Data (JSON, golden traces) and prose
/// (Markdown) do not cite rulings.
const CODE_EXTENSIONS: &[&str] = &[
    "rs", "sql", "toml", "ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs", "css", "html",
];

/// The file extensions a TypeScript proof may have.
const TS_EXTENSIONS: &[&str] = &["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];

/// The modifiers that keep a TypeScript test from running (`it.skip(…)`, `test.todo(…)`, Playwright's
/// `.fixme`, and `.only`, which turns off the rest of its file and which vitest refuses under CI):
/// no title under one proves a ruling (#1718).
const SKIPPED_TS_MODIFIERS: &[&str] = &["skip", "todo", "fixme", "only"];

/// The phrases rule 7 refuses: each says the point it annotates still waits for its ruling (#1243).
/// Written with `concat!`, because this file is itself in what `check` reads.
const UNRESOLVED_MARKERS: &[&str] = &[concat!("proposed", " ruling"), concat!("no r-row", " yet")];

const INDEX_HEADER: &str = "# Rulings index\n\nGenerated by `cargo jackioh spec index` from the notes in `rulings/`: edit a note, never this file, then run the command again.\n\n| id | title | note | proven in |\n| --- | --- | --- | --- |\n";

/* ------------------------------------------------------------------------------------- notes */

/// One ruling note, as its front matter states it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RulingNote {
    /// The path relative to the root, `spec/rulings/R0195.md`.
    path: String,
    /// The file name without `.md`, `R0195`.
    stem: String,
    id: Option<String>,
    title: Option<String>,
    /// What it affects, as plain text: cards, sections and rulings.
    cards: Option<String>,
    proven_in: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Field {
    Scalar(String),
    List(Vec<String>),
}

/// The front matter of a note: the `---` block it starts with, as `key: value` scalars and
/// `key:` lists of `  - item` lines (or `key: []`). Quoted scalars are unquoted.
fn front_matter(text: &str) -> Result<Vec<(String, Field)>, String> {
    let mut lines = text.lines().map(|line| line.trim_end_matches('\r'));
    if lines.next() != Some("---") {
        return Err("no front matter: the note must start with a `---` line".to_string());
    }
    let mut fields: Vec<(String, Field)> = Vec::new();
    let mut closed = false;
    for line in lines {
        if line == "---" {
            closed = true;
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        let trimmed = line.trim_start();
        if let Some(item) = trimmed.strip_prefix("- ") {
            match fields.last_mut() {
                Some((_, Field::List(items))) => items.push(unquote(item.trim())),
                _ => return Err(format!("a list item outside a list: `{line}`")),
            }
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            return Err(format!("not a `key: value` line: `{line}`"));
        };
        let key = key.trim().to_string();
        let value = value.trim();
        let field = if value.is_empty() {
            Field::List(Vec::new())
        } else if let Some(inner) = value.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
            Field::List(
                inner
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(unquote)
                    .collect(),
            )
        } else {
            Field::Scalar(unquote(value))
        };
        fields.push((key, field));
    }
    if !closed {
        return Err("the front matter has no closing `---` line".to_string());
    }
    Ok(fields)
}

/// A YAML scalar as written: `"…"` with backslash escapes, `'…'` with `''`, or plain.
fn unquote(value: &str) -> String {
    if value.len() < 2 {
        return value.to_string();
    }
    if let Some(inner) = value.strip_prefix('"').and_then(|rest| rest.strip_suffix('"')) {
        let mut out = String::with_capacity(inner.len());
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some(other) => out.push(other),
                    None => out.push('\\'),
                }
            } else {
                out.push(c);
            }
        }
        return out;
    }
    if let Some(inner) = value.strip_prefix('\'').and_then(|rest| rest.strip_suffix('\'')) {
        return inner.replace("''", "'");
    }
    value.to_string()
}

fn parse_note(path: String, stem: String, text: &str) -> Result<RulingNote, String> {
    let fields = front_matter(text)?;
    let mut note = RulingNote {
        path,
        stem,
        id: None,
        title: None,
        cards: None,
        proven_in: Vec::new(),
    };
    for (key, field) in fields {
        match (key.as_str(), field) {
            ("id", Field::Scalar(value)) => note.id = Some(value),
            ("title", Field::Scalar(value)) => note.title = Some(value),
            ("cards", Field::Scalar(value)) => note.cards = Some(value),
            ("proven_in", Field::List(items)) => note.proven_in = items,
            ("proven_in", Field::Scalar(value)) => note.proven_in = vec![value],
            _ => {}
        }
    }
    Ok(note)
}

/// Rule 5's findings for one note: each of `title` and `cards` its front matter leaves out or
/// empty (`spec/README.md`, Note format).
fn unfilled_fields(note: &RulingNote) -> Vec<String> {
    [("title", &note.title), ("cards", &note.cards)]
        .into_iter()
        .filter(|(_, value)| value.as_deref().is_none_or(|value| value.trim().is_empty()))
        .map(|(field, _)| format!("{}: `{field}` is missing or empty in the front matter", note.path))
        .collect()
}

/// Every ruling note under `spec/rulings/`, in file-name order.
fn read_notes(root: &Path) -> Result<Vec<RulingNote>> {
    let dir = root.join("spec").join("rulings");
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    files.sort();
    let mut notes = Vec::with_capacity(files.len());
    for file in files {
        let text = fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
        let stem = file
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let path = relative(root, &file);
        let note =
            parse_note(path.clone(), stem, &text).map_err(|error| anyhow::anyhow!("{path}: {error}"))?;
        notes.push(note);
    }
    Ok(notes)
}

/// `R195` → 195.
fn ruling_number(id: &str) -> Option<u64> {
    let digits = id.strip_prefix('R')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// The file name a ruling's note must have, without `.md`: zero-padded to four digits at least (`R0195`, `R690000`).
fn note_stem(number: u64) -> String {
    format!("R{number:04}")
}

/* ------------------------------------------------------------------------------- the scanners */

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

pub(crate) fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Every `R<n>` cited as a word in a text, with its byte offset: `R113`, `(R113)`, `R113's`, not
/// `XR113` or `R113a`.
fn citations(text: &str) -> Vec<(usize, u64)> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'R' && (i == 0 || !is_word_byte(bytes[i - 1])) {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end > start && (end == bytes.len() || !is_word_byte(bytes[end])) {
                if let Ok(number) = text[start..end].parse::<u64>() {
                    found.push((i, number));
                }
                i = end;
                continue;
            }
        }
        i += 1;
    }
    found
}

/// Rule 7's findings for one file: each line where an `UNRESOLVED_MARKERS` phrase starts a word,
/// in any ASCII case.
fn unresolved_markers(path: &str, text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let lower = line.to_ascii_lowercase();
        for marker in UNRESOLVED_MARKERS {
            let starts_a_word = lower
                .match_indices(marker)
                .any(|(at, _)| at == 0 || !is_word_byte(lower.as_bytes()[at - 1]));
            if starts_a_word {
                found.push(format!(
                    "{path}:{}: `{marker}` says a ruling is still to be written: cite its note in spec/rulings/, or write one (CLAUDE.md rule 3)",
                    index + 1
                ));
            }
        }
    }
    found
}

/// Rust source with its comments, string literals and char literals blanked out, so a name in a
/// comment or a string is not a test. Each blanked character reads as a space and line breaks are
/// kept, so a character's index here is its index in the source and line numbers still hold.
fn rust_code_only(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        let end = if c == '/' && next == Some('/') {
            let mut j = i;
            while j < chars.len() && chars[j] != '\n' {
                j += 1;
            }
            j
        } else if c == '/' && next == Some('*') {
            block_comment_end(&chars, i)
        } else if c == '"' || (c == 'r' && raw_string_start(&chars, i)) {
            string_end(&chars, i)
        } else if c == '\'' {
            // A char literal ('x', '\n', '"') is blanked; a lifetime ('a) is one quote.
            char_literal_end(&chars, i).unwrap_or(i + 1)
        } else {
            out.push(c);
            i += 1;
            continue;
        };
        // What was skipped reads as one space per character, its line breaks kept.
        let end = end.max(i + 1);
        out.extend(chars[i..end].iter().map(|c| if *c == '\n' { '\n' } else { ' ' }));
        i = end;
    }
    out
}

/// The index just past the (possibly nested) block comment that starts at `i`.
fn block_comment_end(chars: &[char], i: usize) -> usize {
    let mut depth = 0usize;
    let mut j = i;
    while j < chars.len() {
        if chars[j] == '/' && chars.get(j + 1) == Some(&'*') {
            depth += 1;
            j += 2;
        } else if chars[j] == '*' && chars.get(j + 1) == Some(&'/') {
            depth = depth.saturating_sub(1);
            j += 2;
            if depth == 0 {
                return j;
            }
        } else {
            j += 1;
        }
    }
    chars.len()
}

/// Whether `r` at `i` opens a raw string (`r"…"`, `r#"…"#`, `br"…"`) rather than being part of an
/// identifier.
fn raw_string_start(chars: &[char], i: usize) -> bool {
    if i > 0 {
        let before = chars[i - 1];
        let byte_prefix = before == 'b' && (i < 2 || !is_ident_char(chars[i - 2]));
        if is_ident_char(before) && !byte_prefix {
            return false;
        }
    }
    let mut j = i + 1;
    while j < chars.len() && chars[j] == '#' {
        j += 1;
    }
    j < chars.len() && chars[j] == '"'
}

/// The index just past the string literal that starts at `i` (`"…"` or a raw string).
fn string_end(chars: &[char], i: usize) -> usize {
    if chars[i] == '"' {
        let mut j = i + 1;
        while j < chars.len() {
            match chars[j] {
                '\\' => j += 2,
                '"' => return j + 1,
                _ => j += 1,
            }
        }
        return chars.len();
    }
    let mut j = i + 1;
    let mut hashes = 0;
    while j < chars.len() && chars[j] == '#' {
        hashes += 1;
        j += 1;
    }
    j += 1; // the opening quote
    while j < chars.len() {
        if chars[j] == '"' && (0..hashes).all(|k| chars.get(j + 1 + k) == Some(&'#')) {
            return j + 1 + hashes;
        }
        j += 1;
    }
    chars.len()
}

/// The index just past a char literal starting at `i`, or `None` for a lifetime.
fn char_literal_end(chars: &[char], i: usize) -> Option<usize> {
    match chars.get(i + 1) {
        // '\n', '\'', '\\', '\u{1F600}': the escape's first character is never the closing quote.
        Some('\\') => (i + 3..chars.len().min(i + 14))
            .find(|j| chars[*j] == '\'')
            .map(|j| j + 1),
        Some(_) if chars.get(i + 2) == Some(&'\'') => Some(i + 3),
        _ => None,
    }
}

/// A `fn` or `mod` in Rust source.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RustItem {
    line: usize,
    name: String,
    /// Whether it is a test that runs (#1718): a `fn` marked `#[test]` or `#[<path>::test]`
    /// (`#[tokio::test]`) and not `#[ignore]`d but for a `daily:` reason, which super.yml runs; a
    /// `fn` a call of a test macro of the same file lists (one whose `macro_rules!` marks a
    /// `fn $…` as a test, as `both_stores!` does); or a `mod` whose body holds either.
    runs: bool,
}

/// Every `fn` and `mod` in Rust source (comments and strings excluded), with its line and whether
/// it is a test that runs.
fn rust_item_names(src: &str) -> Vec<RustItem> {
    let source: Vec<char> = src.chars().collect();
    let code: Vec<char> = rust_code_only(src).chars().collect();
    let mut items: Vec<RustItem> = Vec::new();
    // The outer attributes read since the last item, `;`, `{` or `}`, as written between `#[` and `]`.
    let mut attributes: Vec<String> = Vec::new();
    // The `mod` just named, until its `{` or `;`.
    let mut pending_mod: Option<usize> = None;
    // Each open `mod` body: the mod's index in `items` and the brace depth inside it.
    let mut open_mods: Vec<(usize, usize)> = Vec::new();
    // The macros this file defines whose body marks a `fn $…` as a test, the `macro_rules!` just
    // named until its `{`, the one being read with the brace depth inside it, and the names its
    // calls list.
    let mut test_macros: Vec<String> = Vec::new();
    let mut pending_macro: Option<String> = None;
    let mut current_macro: Option<(String, usize)> = None;
    let mut macro_tests: Vec<String> = Vec::new();
    let mut depth = 0usize;
    let mut previous: Option<String> = None;
    let mut line = 1;
    let mut i = 0;
    while i < code.len() {
        let c = code[i];
        if is_ident_char(c) {
            let start = i;
            while i < code.len() && is_ident_char(code[i]) {
                i += 1;
            }
            let word: String = code[start..i].iter().collect();
            match previous.as_deref() {
                Some("fn") => {
                    let runs = attributes.iter().any(|a| is_test_attribute(a))
                        && !attributes.iter().any(|a| ignored_but_not_daily(a));
                    if runs {
                        for (index, _) in &open_mods {
                            items[*index].runs = true;
                        }
                    }
                    items.push(RustItem {
                        line,
                        name: word.clone(),
                        runs,
                    });
                    attributes.clear();
                }
                Some("mod") => {
                    pending_mod = Some(items.len());
                    items.push(RustItem {
                        line,
                        name: word.clone(),
                        runs: false,
                    });
                    attributes.clear();
                }
                _ => {}
            }
            if word == "macro_rules" {
                pending_macro = ident_after_bang(&code, i);
            } else if test_macros.contains(&word)
                && let Some(names) = macro_arguments(&code, i)
            {
                for (index, _) in &open_mods {
                    items[*index].runs = true;
                }
                macro_tests.extend(names);
            }
            previous = Some(word);
            continue;
        }
        if c == '#'
            && let Some((inner, start, end)) = attribute_at(&code, i)
        {
            if !inner {
                attributes.push(source[start..end].iter().collect());
            }
            line += code[i..end].iter().filter(|c| **c == '\n').count();
            previous = None;
            i = end + 1;
            continue;
        }
        match c {
            '$' if previous.as_deref() == Some("fn") && attributes.iter().any(|a| is_test_attribute(a)) => {
                if let Some((name, _)) = &current_macro {
                    test_macros.push(name.clone());
                }
            }
            '{' => {
                depth += 1;
                if let Some(index) = pending_mod.take() {
                    open_mods.push((index, depth));
                }
                if let Some(name) = pending_macro.take() {
                    current_macro = Some((name, depth));
                }
                attributes.clear();
            }
            '}' => {
                if open_mods.last().is_some_and(|(_, inside)| *inside == depth) {
                    open_mods.pop();
                }
                if current_macro.as_ref().is_some_and(|(_, inside)| *inside == depth) {
                    current_macro = None;
                }
                depth = depth.saturating_sub(1);
                attributes.clear();
            }
            ';' => {
                pending_mod = None;
                attributes.clear();
            }
            '\n' => line += 1,
            _ => {}
        }
        if !c.is_whitespace() {
            previous = None;
        }
        i += 1;
    }
    for item in &mut items {
        item.runs |= macro_tests.contains(&item.name);
    }
    items
}

fn skip_whitespace(code: &[char], i: usize) -> usize {
    i + code[i..].iter().take_while(|c| c.is_whitespace()).count()
}

/// The name after the `!` that follows index `i`, whitespace aside: `macro_rules! both_stores`.
fn ident_after_bang(code: &[char], i: usize) -> Option<String> {
    let at = skip_whitespace(code, i);
    if code.get(at) != Some(&'!') {
        return None;
    }
    let start = skip_whitespace(code, at + 1);
    let name: String = code[start..].iter().take_while(|c| is_ident_char(**c)).collect();
    (!name.is_empty()).then_some(name)
}

/// The identifiers inside the macro call whose `!` follows index `i`, whitespace aside:
/// `both_stores!(a, b)` gives `a` and `b`. `None` when no `!(…)`, `![…]` or `!{…}` follows.
fn macro_arguments(code: &[char], i: usize) -> Option<Vec<String>> {
    let at = skip_whitespace(code, i);
    if code.get(at) != Some(&'!') {
        return None;
    }
    let open_at = skip_whitespace(code, at + 1);
    let open = *code.get(open_at)?;
    let close = match open {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        _ => return None,
    };
    let mut depth = 0usize;
    let mut names = Vec::new();
    let mut word = String::new();
    for &c in &code[open_at..] {
        if is_ident_char(c) {
            word.push(c);
            continue;
        }
        if !word.is_empty() {
            names.push(std::mem::take(&mut word));
        }
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(names);
            }
        }
    }
    None
}

/// The attribute `#[…]` or `#![…]` that starts at `i`: whether it is an inner one (`#!`), and the
/// indices its contents start and end at, between its brackets. `None` when `#` opens none.
fn attribute_at(code: &[char], i: usize) -> Option<(bool, usize, usize)> {
    let inner = code.get(i + 1) == Some(&'!');
    let open = if inner { i + 2 } else { i + 1 };
    if code.get(open) != Some(&'[') {
        return None;
    }
    let mut depth = 0usize;
    for (at, c) in code.iter().enumerate().skip(open) {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some((inner, open + 1, at));
                }
            }
            _ => {}
        }
    }
    None
}

/// Whether an attribute, as written between its brackets, marks a test: `test`, or a path ending
/// in `::test` (`tokio::test(flavor = "multi_thread")`).
fn is_test_attribute(attribute: &str) -> bool {
    let path = attribute.split(['(', '=']).next().unwrap_or("").trim();
    path == "test" || path.ends_with("::test")
}

/// Whether an attribute, as written between its brackets, ignores a test for any reason but a
/// daily run: `ignore`, or `ignore = "…"` whose reason does not start with `daily:`.
fn ignored_but_not_daily(attribute: &str) -> bool {
    let (path, reason) = attribute.split_once('=').unwrap_or((attribute, ""));
    path.trim() == "ignore" && !reason.trim().trim_start_matches('"').starts_with("daily:")
}

/// The ruling numbers a Rust item name carries as `_`-delimited `r<n>` tokens: `r90_r81_x` → 90, 81.
fn name_tokens(name: &str) -> Vec<u64> {
    name.split('_')
        .filter_map(|token| {
            let digits = token.strip_prefix('r')?;
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            digits.parse().ok()
        })
        .collect()
}

/// The title of every `it(…)`, `test(…)` and `describe(…)` in TypeScript source that runs, as
/// written between its quotes: `.concurrent` and the like may stand between the name and the
/// parenthesis, but a call marked with one of `SKIPPED_TS_MODIFIERS` is left out, along with every
/// call inside it (#1718).
fn ts_test_titles(src: &str) -> Vec<String> {
    let bytes = src.as_bytes();
    let mut titles = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &bytes[i..];
        let name_len = ["describe", "test", "it"]
            .iter()
            .find(|name| rest.starts_with(name.as_bytes()))
            .map(|name| name.len());
        let Some(len) = name_len else {
            i += 1;
            continue;
        };
        if i > 0 && is_word_byte(bytes[i - 1]) {
            i += 1;
            continue;
        }
        let mut j = i + len;
        if j < bytes.len() && is_word_byte(bytes[j]) {
            i += 1;
            continue;
        }
        // `.concurrent`, `.each`, …, or one of SKIPPED_TS_MODIFIERS.
        let mut skipped = false;
        while j < bytes.len() && bytes[j] == b'.' {
            let mut k = j + 1;
            while k < bytes.len() && is_word_byte(bytes[k]) {
                k += 1;
            }
            if k == j + 1 {
                break;
            }
            skipped |= SKIPPED_TS_MODIFIERS.contains(&&src[j + 1..k]);
            j = k;
        }
        if j >= bytes.len() || bytes[j] != b'(' {
            i += 1;
            continue;
        }
        if skipped {
            // Nothing inside a skipped call runs either: a `describe.skip(…)`'s tests among it.
            i = ts_call_end(bytes, j);
            continue;
        }
        j += 1;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j >= bytes.len() || !matches!(bytes[j], b'"' | b'\'' | b'`') {
            i = j;
            continue;
        }
        let quote = bytes[j];
        let start = j + 1;
        let mut k = start;
        while k < bytes.len() && bytes[k] != quote {
            if bytes[k] == b'\\' {
                k += 1;
            }
            k += 1;
        }
        let end = k.min(bytes.len());
        titles.push(src[start..end].to_string());
        i = end + 1;
    }
    titles
}

/// The index just past the `)` that closes the call whose `(` is at `open`, string literals and
/// comments skipped (the end of the source if none closes it).
fn ts_call_end(bytes: &[u8], open: usize) -> usize {
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            quote @ (b'"' | b'\'' | b'`') => {
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i < bytes.len() && !bytes[i..].starts_with(b"*/") {
                    i += 1;
                }
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    bytes.len()
}

/// The headings an SQL evidence script announces its checks with: `\echo '### … ###'` or
/// `\echo '=== … ==='`, as the text between the delimiters. Prose between checks is not read.
fn sql_headings(src: &str) -> Vec<String> {
    let mut headings = Vec::new();
    let mut rest = src;
    while let Some(at) = rest.find("\\echo") {
        rest = &rest[at + "\\echo".len()..];
        let after = rest.trim_start();
        let Some(quoted) = after.strip_prefix('\'') else {
            continue;
        };
        let quoted = quoted.trim_start();
        let Some(body) = quoted.strip_prefix("###").or_else(|| quoted.strip_prefix("===")) else {
            continue;
        };
        let Some(close) = body.find('\'') else { continue };
        let heading = body[..close].trim();
        let heading = heading
            .strip_suffix("###")
            .or_else(|| heading.strip_suffix("==="))
            .unwrap_or(heading);
        headings.push(heading.trim().to_string());
    }
    headings
}

/// Whether an SQL heading names ruling `number` where a name goes: `R104: …` or `(R105): …`.
fn sql_heading_names(heading: &str, number: u64) -> bool {
    citations(heading).into_iter().any(|(at, cited)| {
        if cited != number {
            return false;
        }
        let digits = heading[at + 1..].bytes().take_while(u8::is_ascii_digit).count();
        let after = heading[at + 1 + digits..].trim_start();
        after.starts_with(':') || after.starts_with(')')
    })
}

/// Whether a file holds a proof of ruling `number`, in its own idiom for a test name.
fn proves(path: &str, src: &str, number: u64) -> Result<bool, String> {
    let extension = Path::new(path)
        .extension()
        .map(|ext| ext.to_string_lossy().into_owned())
        .unwrap_or_default();
    if extension == "rs" {
        return Ok(rust_item_names(src)
            .iter()
            .any(|item| item.runs && name_tokens(&item.name).contains(&number)));
    }
    if extension == "sql" {
        return Ok(sql_headings(src)
            .iter()
            .any(|heading| sql_heading_names(heading, number)));
    }
    if TS_EXTENSIONS.contains(&extension.as_str()) {
        return Ok(ts_test_titles(src)
            .iter()
            .any(|title| citations(title).iter().any(|(_, cited)| *cited == number)));
    }
    Err("is not a Rust, TypeScript or SQL test file".to_string())
}

/* --------------------------------------------------------------------------------- the walk */

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Every file under `dir`, recursively and in name order, skipping SKIPPED_DIRS.
fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    paths.sort();
    for path in paths {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if path.is_dir() {
            if !SKIPPED_DIRS.contains(&name.as_str()) {
                walk(&path, out);
            }
        } else {
            out.push(path);
        }
    }
}

fn line_of(text: &str, offset: usize) -> usize {
    text[..offset].bytes().filter(|b| *b == b'\n').count() + 1
}

/* ------------------------------------------------------------------------------- the links */

/// Every `[[…]]` link outside code in a Markdown note, with its line number and its target (the
/// part before any `|alias` or `#heading`).
fn wiki_links(text: &str) -> Vec<(usize, String, String)> {
    let mut links = Vec::new();
    let mut fenced = false;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let prose = without_code_spans(line);
        let mut rest = prose.as_str();
        while let Some(open) = rest.find("[[") {
            let after = &rest[open + 2..];
            let Some(close) = after.find("]]") else { break };
            let inner = &after[..close];
            let target = inner
                .split('|')
                .next()
                .unwrap_or("")
                .split('#')
                .next()
                .unwrap_or("")
                .trim();
            links.push((index + 1, inner.to_string(), target.to_string()));
            rest = &after[close + 2..];
        }
    }
    links
}

/// A line with its inline code spans (`…`) removed.
fn without_code_spans(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(open) = rest.find('`') {
        out.push_str(&rest[..open]);
        let ticks = rest[open..].bytes().take_while(|b| *b == b'`').count();
        let fence = &rest[open..open + ticks];
        let after = &rest[open + ticks..];
        match after.find(fence) {
            Some(close) => rest = &after[close + ticks..],
            None => {
                out.push_str(&rest[open..]);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The section ids a note's headings define: `## 10.` is `10`, `### 10.3 …` is `10.3`.
fn section_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("### ") {
            let id: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            let id = id.trim_end_matches('.');
            if !id.is_empty() && rest[id.len()..].starts_with([' ', '.']) {
                ids.push(id.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("## ") {
            let id: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if !id.is_empty() && rest[id.len()..].starts_with('.') {
                ids.push(id);
            }
        }
    }
    ids
}

/* ------------------------------------------------------------------------------- the checks */

fn check(root: &Path) -> Result<()> {
    let notes = read_notes(root)?;
    let mut problems: Vec<String> = Vec::new();

    // 1. Every note's id matches its file name, and ids are unique.
    let mut ids: BTreeMap<u64, String> = BTreeMap::new();
    for note in &notes {
        let Some(id) = note.id.as_deref() else {
            problems.push(format!("{}: no `id` in the front matter", note.path));
            continue;
        };
        let Some(number) = ruling_number(id) else {
            problems.push(format!("{}: `id: {id}` is not a ruling id (R<n>)", note.path));
            continue;
        };
        if note.stem != note_stem(number) {
            problems.push(format!(
                "{}: `id: {id}` belongs in {}.md",
                note.path,
                note_stem(number)
            ));
        }
        if let Some(other) = ids.get(&number) {
            problems.push(format!("{}: R{number} is also the id of {other}", note.path));
        } else {
            ids.insert(number, note.path.clone());
        }
    }

    // 2. Every note is proved, and every proof path exists and names the ruling.
    let mut proofs = 0usize;
    let mut sources: BTreeMap<String, Option<String>> = BTreeMap::new();
    for note in &notes {
        let Some(number) = note.id.as_deref().and_then(ruling_number) else {
            continue;
        };
        if note.proven_in.is_empty() {
            problems.push(format!("{}: R{number} lists no `proven_in` file", note.path));
        }
        for proof in &note.proven_in {
            let src = sources
                .entry(proof.clone())
                .or_insert_with(|| fs::read_to_string(root.join(proof)).ok());
            let Some(src) = src.as_deref() else {
                problems.push(format!("{}: `proven_in` {proof} does not exist", note.path));
                continue;
            };
            match proves(proof, src, number) {
                Ok(true) => proofs += 1,
                Ok(false) => problems.push(format!(
                    "{}: `proven_in` {proof} has no test that runs named after R{number} (a #[test] fn, a fn a test macro lists or a mod holding one, with the token r{number}; an it/test/describe title with R{number} and no .skip, .todo, .fixme or .only; or an SQL `### R{number}: …` heading)",
                    note.path
                )),
                Err(why) => problems.push(format!("{}: `proven_in` {proof} {why}", note.path)),
            }
        }
    }

    // 3. Every r<n> test token and every R<n> cited in code has a note.
    let mut files = Vec::new();
    for dir in CITING_ROOTS {
        walk(&root.join(dir), &mut files);
    }
    let mut cited = 0usize;
    let mut citing_files = 0usize;
    for file in &files {
        let extension = file
            .extension()
            .map(|ext| ext.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !CODE_EXTENSIONS.contains(&extension.as_str()) {
            continue;
        }
        let Ok(text) = fs::read_to_string(file) else {
            continue;
        };
        let path = relative(root, file);
        // 7. No unresolved-ruling marker, in the same files.
        problems.extend(unresolved_markers(&path, &text));
        let found = citations(&text);
        if !found.is_empty() {
            citing_files += 1;
        }
        for (offset, number) in found {
            cited += 1;
            if !ids.contains_key(&number) {
                problems.push(format!(
                    "{path}:{}: R{number} has no note in spec/rulings/",
                    line_of(&text, offset)
                ));
            }
        }
        if extension == "rs" {
            for item in rust_item_names(&text) {
                for number in name_tokens(&item.name) {
                    if !ids.contains_key(&number) {
                        problems.push(format!(
                            "{path}:{}: `{}` names r{number}, which has no note in spec/rulings/",
                            item.line, item.name
                        ));
                    }
                }
            }
        }
    }

    // 4. Every [[…]] link in spec/ resolves to a note or a section.
    let mut spec_files = Vec::new();
    walk(&root.join("spec"), &mut spec_files);
    spec_files.retain(|path| path.extension().is_some_and(|ext| ext == "md"));
    let mut texts: Vec<(String, String)> = Vec::new();
    let mut sections: BTreeSet<String> = BTreeSet::new();
    let mut stems: BTreeSet<String> = BTreeSet::new();
    for file in &spec_files {
        let text = fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
        sections.extend(section_ids(&text));
        if let Some(stem) = file.file_stem() {
            stems.insert(stem.to_string_lossy().into_owned());
        }
        texts.push((relative(root, file), text));
    }
    let mut links = 0usize;
    for (path, text) in &texts {
        for (line, inner, target) in wiki_links(text) {
            links += 1;
            let resolves = if let Some(number) = ruling_number(&target) {
                ids.contains_key(&number)
            } else if let Some(section) = target.strip_prefix('§') {
                sections.contains(section)
            } else {
                stems.contains(&target)
            };
            if !resolves {
                problems.push(format!(
                    "{path}:{line}: [[{inner}]] resolves to no note or section"
                ));
            }
        }
    }

    // 5. Every note fills in its `title` and `cards`.
    problems.extend(notes.iter().flat_map(unfilled_fields));

    // 6. spec/INDEX.md is what `index` writes from the notes. A note with no ruling id has no row
    // to render, and rule 1 has named it already.
    if let Ok(rendered) = render_index(&notes) {
        let committed = fs::read_to_string(root.join("spec").join("INDEX.md")).ok();
        problems.extend(index_problem(committed.as_deref(), &rendered));
    }

    for problem in &problems {
        println!("{problem}");
    }
    if !problems.is_empty() {
        bail!("spec check: {} problem(s)", problems.len());
    }
    println!(
        "spec check: {} rulings, {proofs} proofs, {cited} citations in {citing_files} files, {links} links: ok",
        notes.len()
    );
    Ok(())
}

/* -------------------------------------------------------------------------------- the index */

fn render_index(notes: &[RulingNote]) -> Result<String> {
    let mut rows: Vec<(u64, &RulingNote)> = Vec::with_capacity(notes.len());
    for note in notes {
        let id = note
            .id
            .as_deref()
            .with_context(|| format!("{}: no `id` in the front matter", note.path))?;
        let number =
            ruling_number(id).with_context(|| format!("{}: `id: {id}` is not a ruling id", note.path))?;
        rows.push((number, note));
    }
    rows.sort_by_key(|(number, _)| *number);
    let mut out = String::from(INDEX_HEADER);
    for (_, note) in rows {
        let id = note.id.as_deref().unwrap_or("");
        let title = note.title.as_deref().unwrap_or("").replace('|', "\\|");
        let stem = &note.stem;
        let proofs: Vec<String> = note.proven_in.iter().map(|path| format!("`{path}`")).collect();
        writeln!(
            out,
            "| {id} | {title} | [{stem}](rulings/{stem}.md) | {} |",
            proofs.join(", ")
        )?;
    }
    Ok(out)
}

/// Rule 6's finding: `None` when `spec/INDEX.md` as it stands (`committed`, `None` when the file is
/// missing) is what `index` writes, `rendered`. Line endings aside: a checkout that turned them
/// into `\r\n` is not stale, as `front_matter` reads such a note.
fn index_problem(committed: Option<&str>, rendered: &str) -> Option<String> {
    match committed {
        Some(text) if text.replace("\r\n", "\n") == rendered => None,
        Some(_) => Some(
            "spec/INDEX.md is stale: run `cargo jackioh spec index` to rewrite it from the notes".to_string(),
        ),
        None => Some(
            "spec/INDEX.md is missing: run `cargo jackioh spec index` to write it from the notes".to_string(),
        ),
    }
}

fn index(root: &Path) -> Result<()> {
    let notes = read_notes(root)?;
    let out = render_index(&notes)?;
    let path = root.join("spec").join("INDEX.md");
    fs::write(&path, out).with_context(|| format!("writing {}", path.display()))?;
    println!("spec index: wrote spec/INDEX.md ({} rulings)", notes.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTE: &str = "---\nid: R195\ntitle: When a card glows yellow (`conditionActive`)\ncards: \"§10.8, §10.9, #10\"\nproven_in:\n  - crates/engine/tests/rules/condition_active.rs\n  - crates/cards/tests/cross/condition_active.rs\n---\nThe ruling ([[R113]], [[§10.8]]).\n";

    #[test]
    fn reads_a_ruling_notes_front_matter() {
        let note = parse_note("spec/rulings/R0195.md".into(), "R0195".into(), NOTE).unwrap();
        assert_eq!(note.id.as_deref(), Some("R195"));
        assert_eq!(
            note.title.as_deref(),
            Some("When a card glows yellow (`conditionActive`)")
        );
        assert_eq!(
            note.proven_in,
            vec![
                "crates/engine/tests/rules/condition_active.rs".to_string(),
                "crates/cards/tests/cross/condition_active.rs".to_string()
            ]
        );
        assert_eq!(note.cards.as_deref(), Some("§10.8, §10.9, #10"));
    }

    #[test]
    fn names_a_note_without_a_title_or_cards() {
        let path = "spec/rulings/R0195.md";
        let note = |text: &str| parse_note(path.into(), "R0195".into(), text).unwrap();
        let no_title = format!("{path}: `title` is missing or empty in the front matter");
        let no_cards = format!("{path}: `cards` is missing or empty in the front matter");

        assert!(unfilled_fields(&note(NOTE)).is_empty());
        assert_eq!(
            unfilled_fields(&note("---\nid: R195\nproven_in: []\n---\n")),
            vec![no_title.clone(), no_cards.clone()]
        );
        assert_eq!(
            unfilled_fields(&note(
                "---\nid: R195\ntitle: \"  \"\ncards:\nproven_in: []\n---\n"
            )),
            vec![no_title, no_cards.clone()]
        );
        assert_eq!(
            unfilled_fields(&note(
                "---\nid: R195\ntitle: Glow\ncards: \"\"\nproven_in: []\n---\n"
            )),
            vec![no_cards]
        );
    }

    #[test]
    fn flags_a_missing_or_stale_index() {
        let note = |text: &str| parse_note("spec/rulings/R0195.md".into(), "R0195".into(), text).unwrap();
        let rendered = render_index(&[note(NOTE)]).unwrap();
        assert_eq!(index_problem(Some(rendered.as_str()), &rendered), None);

        let crlf = rendered.replace('\n', "\r\n");
        assert_eq!(index_problem(Some(crlf.as_str()), &rendered), None);

        let retitled = render_index(&[note(&NOTE.replace("glows yellow", "glows gold"))]).unwrap();
        let stale = index_problem(Some(rendered.as_str()), &retitled).unwrap();
        assert!(stale.contains("spec/INDEX.md is stale"));
        assert!(stale.contains("run `cargo jackioh spec index`"));

        assert!(index_problem(Some(INDEX_HEADER), &rendered).is_some());

        let missing = index_problem(None, &rendered).unwrap();
        assert!(missing.contains("spec/INDEX.md is missing"));
        assert!(missing.contains("run `cargo jackioh spec index`"));
    }

    #[test]
    fn unquotes_yaml_scalars() {
        assert_eq!(unquote("\"\\\"Lose health\\\"\""), "\"Lose health\"");
        assert_eq!(unquote("'it''s'"), "it's");
        assert_eq!(unquote("C #15 Nose Hunter"), "C #15 Nose Hunter");
        let fields = front_matter("---\nproven_in: []\ncards: \"\"\n---\n").unwrap();
        assert_eq!(fields[0], ("proven_in".to_string(), Field::List(Vec::new())));
        assert_eq!(fields[1], ("cards".to_string(), Field::Scalar(String::new())));
    }

    #[test]
    fn refuses_a_note_without_front_matter() {
        assert!(front_matter("id: R1\n").is_err());
        assert!(front_matter("---\nid: R1\n").is_err());
    }

    #[test]
    fn names_a_rulings_file_with_four_digits() {
        assert_eq!(note_stem(1), "R0001");
        assert_eq!(note_stem(195), "R0195");
        assert_eq!(ruling_number("R195"), Some(195));
        assert_eq!(ruling_number("R"), None);
        assert_eq!(ruling_number("§10.3"), None);
    }

    #[test]
    fn r690000_a_block_number_names_its_note_proof_and_index_row() {
        let text = "---\nid: R690000\ntitle: A new ruling takes a number from its issue's block (issue #690)\ncards: \"§11\"\nproven_in:\n  - crates/tools/src/spec.rs\n---\n";
        let block = parse_note("spec/rulings/R690000.md".into(), "R690000".into(), text).unwrap();
        let number = block.id.as_deref().and_then(ruling_number).unwrap();
        assert_eq!(number, 690_000);
        assert_eq!(note_stem(number), block.stem);
        assert!(unfilled_fields(&block).is_empty());
        assert_eq!(name_tokens("r690000_x"), vec![690_000]);
        let sequential = parse_note("spec/rulings/R0195.md".into(), "R0195".into(), NOTE).unwrap();
        let out = render_index(&[block, sequential]).unwrap();
        assert_eq!(
            &out[INDEX_HEADER.len()..],
            "| R195 | When a card glows yellow (`conditionActive`) | [R0195](rulings/R0195.md) | `crates/engine/tests/rules/condition_active.rs`, `crates/cards/tests/cross/condition_active.rs` |\n| R690000 | A new ruling takes a number from its issue's block (issue #690) | [R690000](rulings/R690000.md) | `crates/tools/src/spec.rs` |\n"
        );
    }

    #[test]
    fn finds_citations_as_words_only() {
        // Only ids with a note here: this file is itself in what `check` reads (rule 3).
        let found: Vec<u64> = citations("R1, (R113), R113's, R244–R245; XR5, R5a, R_6, R0195")
            .into_iter()
            .map(|(_, n)| n)
            .collect();
        assert_eq!(found, vec![1, 113, 113, 244, 245, 195]);
    }

    #[test]
    fn refuses_an_unresolved_ruling_marker_and_passes_ordinary_prose() {
        // Built from UNRESOLVED_MARKERS: this file is itself in what `check` reads (rule 7).
        let proposed = UNRESOLVED_MARKERS[0];
        let unwritten = UNRESOLVED_MARKERS[1];
        let path = "crates/server/src/api/x.rs";
        let stale = format!(
            "// NOT IN SPEC, and {} — {} for §11:\nfn f() {{}}\n/// {}{} for §11: a ban is server state.\n",
            unwritten.replace("r-row", "R-row"),
            proposed.to_uppercase(),
            proposed[..1].to_uppercase(),
            &proposed[1..],
        );
        let found = unresolved_markers(path, &stale);
        assert_eq!(found.len(), 3);
        assert!(found[0].starts_with(&format!("{path}:1: `{proposed}`")));
        assert!(found[1].starts_with(&format!("{path}:1: `{unwritten}`")));
        assert!(found[2].starts_with(&format!("{path}:3: `{proposed}`")));

        let prose = format!(
            "// Not in SPEC, and no R-row: wording only.\n// R159: only the positive answer is remembered.\n/// Proposed R82 excludes the play.\n//! the checks a proposed name passes\n// un{proposed}, pia{unwritten}\n"
        );
        assert!(unresolved_markers(path, &prose).is_empty());
    }

    #[test]
    fn reads_ruling_tokens_from_rust_item_names() {
        let src = "#[test]\nfn r90_r81_no_legal_target() {}\nmod r195_glow {\n    fn plain() {}\n}\n// fn r999_commented() {}\nfn arr195_not() {}\nlet s = \"fn r998_in_a_string\";\n";
        let names: Vec<String> = rust_item_names(src).into_iter().map(|item| item.name).collect();
        assert_eq!(
            names,
            vec!["r90_r81_no_legal_target", "r195_glow", "plain", "arr195_not"]
        );
        assert_eq!(name_tokens("r90_r81_no_legal_target"), vec![90, 81]);
        assert_eq!(name_tokens("arr195_not"), Vec::<u64>::new());
        assert_eq!(name_tokens("glows_r195"), vec![195]);
        // A `mod` holding only a plain `fn` is no proof (#1718).
        assert!(!proves("x.rs", src, 195).unwrap());
        assert!(proves("x.rs", src, 90).unwrap());
        assert!(proves("x.rs", src, 81).unwrap());
        assert!(!proves("x.rs", src, 999).unwrap());
        assert!(!proves("x.rs", src, 998).unwrap());
    }

    #[test]
    fn keeps_line_numbers_through_comments() {
        let src = "/* a\nblock */\nfn r7_x() {}\n";
        assert_eq!(
            rust_item_names(src),
            vec![RustItem {
                line: 3,
                name: "r7_x".to_string(),
                runs: false
            }]
        );
        let lifetimes = "fn r8_y<'a>(x: &'a str) -> char { '\"' }\nfn r9_z() {}\n";
        let names: Vec<String> = rust_item_names(lifetimes)
            .into_iter()
            .map(|item| item.name)
            .collect();
        assert_eq!(names, vec!["r8_y", "r9_z"]);
        let attributed = "#[tokio::test(\n    flavor = \"multi_thread\"\n)]\nasync fn r8_y() {}\n";
        assert_eq!(
            rust_item_names(attributed),
            vec![RustItem {
                line: 4,
                name: "r8_y".to_string(),
                runs: true
            }]
        );
    }

    #[test]
    fn credits_only_a_rust_test_that_runs() {
        let src = "fn r58_helper() {}\n#[test]\n#[ignore = \"daily: a sweep\"]\nfn r90_daily() {}\n#[test]\n#[ignore]\nfn r81_bare_ignore() {}\n#[ignore = \"slow\"]\n#[test]\nfn r112_slow() {}\n#[tokio::test(flavor = \"multi_thread\")]\npub async fn r143_async() {}\n#[cfg(test)]\nmod r195_tests {\n    #[test]\n    fn plain() {}\n}\nmod r191_helpers {\n    fn plain() {}\n}\n";
        let items: Vec<(String, bool)> = rust_item_names(src)
            .into_iter()
            .map(|item| (item.name, item.runs))
            .collect();
        let expected = [
            ("r58_helper", false),
            ("r90_daily", true),
            ("r81_bare_ignore", false),
            ("r112_slow", false),
            ("r143_async", true),
            ("r195_tests", true),
            ("plain", true),
            ("r191_helpers", false),
            ("plain", false),
        ];
        assert_eq!(
            items,
            expected
                .iter()
                .map(|(name, runs)| (name.to_string(), *runs))
                .collect::<Vec<_>>()
        );
        for number in [90, 143, 195] {
            assert!(proves("x.rs", src, number).unwrap(), "{number}");
        }
        for number in [58, 81, 112, 191] {
            assert!(!proves("x.rs", src, number).unwrap(), "{number}");
        }
    }

    #[test]
    fn credits_the_cases_a_test_macro_lists() {
        let src = "macro_rules! both_stores {\n    ($($case:ident),+) => {\n        mod memory {\n            $(\n                #[tokio::test]\n                async fn $case() {}\n            )+\n        }\n    };\n}\nmacro_rules! helpers {\n    ($($case:ident),+) => { $(fn $case() {})+ };\n}\nmod decks {\n    async fn r256_case(harness: &Harness) {}\n    async fn r250_case(harness: &Harness) {}\n    both_stores!(r256_case);\n    helpers!(r250_case);\n}\n";
        let items: Vec<(String, bool)> = rust_item_names(src)
            .into_iter()
            .map(|item| (item.name, item.runs))
            .collect();
        let expected = [
            ("memory", false),
            ("decks", true),
            ("r256_case", true),
            ("r250_case", false),
        ];
        assert_eq!(
            items,
            expected
                .iter()
                .map(|(name, runs)| (name.to_string(), *runs))
                .collect::<Vec<_>>()
        );
        assert!(proves("x.rs", src, 256).unwrap());
        assert!(!proves("x.rs", src, 250).unwrap());
    }

    #[test]
    fn reads_typescript_test_titles() {
        let src = "describe(\"R143 — the optional seed\", () => {\n  it(\"plays (R112)\", () => {});\n  it.skip('R5 skipped', () => {});\n  test(`R191 B5 ${row.name}`, () => {});\n  submit(\"R7 not a test\");\n});\n";
        assert_eq!(
            ts_test_titles(src),
            vec!["R143 — the optional seed", "plays (R112)", "R191 B5 ${row.name}"]
        );
        assert!(proves("a.test.ts", src, 112).unwrap());
        assert!(proves("a.test.tsx", src, 191).unwrap());
        assert!(!proves("a.test.ts", src, 5).unwrap());
        assert!(!proves("a.test.ts", src, 7).unwrap());
        assert!(!proves("a.test.ts", src, 11).unwrap());
    }

    #[test]
    fn refuses_a_typescript_title_that_does_not_run() {
        // Only ids with a note here: this file is itself in what `check` reads (rule 3).
        let src = "describe.skip(\"R143 group (a paren: ')')\", () => {\n  // a stray ) in a comment\n  it(\"R112 inside a skipped group\", () => {});\n});\nit.todo(\"R191 later\");\ntest.only(\"R195 focused\", () => {});\ntest.fixme(\"R58 broken\", () => {});\nit.concurrent(\"R7 runs\", () => {});\ndescribe(\"R1 after\", () => {});\n";
        assert_eq!(ts_test_titles(src), vec!["R7 runs", "R1 after"]);
        for number in [143, 112, 191, 195, 58] {
            assert!(!proves("a.test.ts", src, number).unwrap(), "{number}");
        }
        for number in [7, 1] {
            assert!(proves("a.test.ts", src, number).unwrap(), "{number}");
        }
    }

    #[test]
    fn credits_an_sql_proof_only_from_a_heading() {
        let src = "-- R107, R108 and R109 are not proved here\n\\echo '### R104: the alphabet ###'\n\\echo '=== CHECK 15 (R105): stale version ==='\n\\echo '### R110: reusable (R142) ###'\n\\echo '-- R109 prose'\n";
        assert!(proves("x.sql", src, 104).unwrap());
        assert!(proves("x.sql", src, 105).unwrap());
        assert!(proves("x.sql", src, 142).unwrap());
        assert!(!proves("x.sql", src, 107).unwrap());
        assert!(!proves("x.sql", src, 109).unwrap());
        assert!(!proves("x.sql", src, 10).unwrap());
    }

    #[test]
    fn refuses_a_proof_that_is_not_a_test_file() {
        assert!(proves("notes.md", "R1", 1).is_err());
    }

    #[test]
    fn finds_links_outside_code() {
        let text = "See [[R113]] and [[§10.3]], `[[R1]]` is code.\n```\n[[R2]]\n```\n[[INDEX|the index]] [[11-rulings#provenance]]\n";
        let targets: Vec<String> = wiki_links(text)
            .into_iter()
            .map(|(_, _, target)| target)
            .collect();
        assert_eq!(targets, vec!["R113", "§10.3", "INDEX", "11-rulings"]);
        assert_eq!(wiki_links(text)[2].0, 5);
    }

    #[test]
    fn reads_section_ids_from_headings() {
        let text = "## 10. Engine implementation guide\n\n### 10.3 Events and triggers\n### 10.10 Client rendering\n## Not a section\n#### 10.3.1 too deep\n";
        assert_eq!(section_ids(text), vec!["10", "10.3", "10.10"]);
    }

    #[test]
    fn renders_the_index_in_id_order() {
        let notes = vec![
            parse_note(
                "spec/rulings/R0010.md".into(),
                "R0010".into(),
                "---\nid: R10\ntitle: a | b\nproven_in: []\n---\n",
            )
            .unwrap(),
            parse_note(
                "spec/rulings/R0002.md".into(),
                "R0002".into(),
                "---\nid: R2\ntitle: Two\nproven_in:\n  - x.rs\n  - y.ts\n---\n",
            )
            .unwrap(),
        ];
        let out = render_index(&notes).unwrap();
        assert!(out.starts_with(INDEX_HEADER));
        assert_eq!(
            &out[INDEX_HEADER.len()..],
            "| R2 | Two | [R0002](rulings/R0002.md) | `x.rs`, `y.ts` |\n| R10 | a \\| b | [R0010](rulings/R0010.md) |  |\n"
        );
    }
}
