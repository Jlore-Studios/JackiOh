//! R366: the words and the shape every card's printed text uses (patch v0.1.1, issue #27: "Card text
//! should be short, neatly formatted, and use consistent terminology").
//!
//! The checks are the ruling's, one by one: the library is the deck and a tribute a Tribute; an
//! embiggen card's bigger price is "Paid (N): …"; the printed keywords lead the text as one
//! comma-separated list on a line of their own; each labelled ability starts a line of its own; and
//! every other line is sentences that end with a full stop. Patch v0.2.0 turned the cost words round
//! (R432, issue #40): "(N) Cost" is the noun ("a (1) Cost or less card", "(4)+ Cost cards") and
//! "costs (N)" the verb ("costs (1) less", "costs (0)"). Every face is read with its `params` filled
//! in (B3.4 rule 5), as a player reads it. Text is presentation (CLAUDE.md rule 7): no rule reads it,
//! which is why the proof is a scan of the catalog rather than a game.
//!
//! Port of `packages/cards/test/card-text.test.ts`. There is no regex crate in a pure crate (SURFACE
//! §3), so each regular expression the TS wrote is a small hand-written matcher below, named after it
//! and checked against it at the bottom of the file. The patch history (`readSnapshot`,
//! `readPatches`) is compiled in with `include_str!` (a pure crate's tests read no files).

use indexmap::{IndexMap, IndexSet};
use jackioh_cards::{CATALOG, catalog_json, chinese_terms_json};
use jackioh_engine::{CardDef, FaceKind, Keyword, KeywordKind, fill_params};
use serde_json::{Map, Value, json};

fn entries() -> Vec<&'static CardDef> {
    CATALOG.values().collect()
}

struct Face {
    card: &'static CardDef,
    face: FaceKind,
    text: String,
    keywords: Vec<Keyword>,
}

fn faces_of(cards: &[&'static CardDef]) -> Vec<Face> {
    cards
        .iter()
        .copied()
        .flat_map(|card| {
            [FaceKind::Base, FaceKind::Radiant]
                .into_iter()
                .map(move |face| Face {
                    card,
                    face,
                    text: fill_params(card, face, None),
                    keywords: card.face(face).keywords.clone(),
                })
        })
        .collect()
}

/// How a printed keyword reads in the text: "Armor 7", "Lucky 1", "Brittle 4", the Bread Token's
/// "Armor X", and "Spell Damage +2" (E6 prints the plus).
fn keyword_label(keyword: &Keyword) -> String {
    let Some(n) = keyword.n() else {
        return keyword.kind().as_str().to_string();
    };
    if keyword.kind() == KeywordKind::SpellDamage {
        return format!("Spell Damage +{n}");
    }
    if keyword.kind() == KeywordKind::Armor && n == 0 {
        "Armor X".to_string()
    } else {
        format!("{} {n}", keyword.kind().as_str())
    }
}

/// The labels that start a line of their own (R366; Activate is B3.2's, Quest Classic #90's).
pub(super) const LABELS: &[&str] = &[
    "Cry:",
    "Death:",
    "Start of turn:",
    "End of turn:",
    "Aura:",
    "Paid (4):",
    "Activate:",
    "Activate 2:",
    "Activate ♾️:",
    "Cast on draw:",
    "Quest:",
];

/// R746 (issue #354): Bounce is a permanent's return from the field (R692). These faces return a card
/// to hand from anywhere else (a Spell after it resolves or at the end of the turn, a card from the
/// graveyard or exile) and say "Return … to hand", as they did before patch v0.2.10.
const RETURNS_OFF_THE_FIELD: &[&str] = &[
    "core-023 base",
    "core-023 radiant",
    "core-024 base",
    "core-024 radiant",
    "core-031 base",
    "core-031 radiant",
    "classic-034 base",
    "classic-034 radiant",
    "classic-047 base",
    "classic-047 radiant",
    "classicplus-014 base",
    "classicplus-014 radiant",
    "classicplus-021 radiant",
    "meditative-096-1 base",
    "meditative-096-1 radiant",
    "meditative-008 base",
    "meditative-008 radiant",
    "meditative-057 base",
    "meditative-057 radiant",
];

// ---------------------------------------------------------------------------------------------
// The regular expressions, by hand. Every pattern is ASCII, so the text is scanned as bytes: a
// non-ASCII character's bytes are never a word character and never equal a pattern byte, which is
// what JS's non-Unicode `\b`, `\d` and `[^.\n]` read them as too. `/i` lowers ASCII only, which keeps
// every byte offset where it was.
// ---------------------------------------------------------------------------------------------

/// `\w`: a word character for `\b`.
fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// `\b` at byte `at` of `text`.
fn boundary(text: &[u8], at: usize) -> bool {
    let before = at > 0 && text.get(at - 1).is_some_and(|b| is_word(*b));
    let after = text.get(at).is_some_and(|b| is_word(*b));
    before != after
}

/// The text as the pattern reads it: lowered for `/i`, as is otherwise.
fn folded(text: &str, insensitive: bool) -> String {
    if insensitive {
        text.to_ascii_lowercase()
    } else {
        text.to_string()
    }
}

/// Every byte offset at which `needle` starts in `text`, overlapping matches included (a regex scans
/// from every position, where `match_indices` skips past each match).
fn every_start(text: &str, needle: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(offset) = text[from..].find(needle) {
        let at = from + offset;
        out.push(at);
        from = at + text[at..].chars().next().map_or(1, char::len_utf8);
    }
    out
}

/// `/\b<phrase>\b/` (with `/i` when `insensitive`): the phrase standing as whole words.
fn has_words(text: &str, phrase: &str, insensitive: bool) -> bool {
    let hay = folded(text, insensitive);
    let needle = folded(phrase, insensitive);
    let bytes = hay.as_bytes();
    every_start(&hay, &needle)
        .into_iter()
        .any(|at| boundary(bytes, at) && boundary(bytes, at + needle.len()))
}

/// `/\b<phrase>/`: the phrase with a word boundary before it only.
fn has_word_start(text: &str, phrase: &str, insensitive: bool) -> bool {
    let hay = folded(text, insensitive);
    let needle = folded(phrase, insensitive);
    let bytes = hay.as_bytes();
    every_start(&hay, &needle)
        .into_iter()
        .any(|at| boundary(bytes, at))
}

/// `/<phrase>/i` anywhere.
fn has_text(text: &str, phrase: &str) -> bool {
    text.to_ascii_lowercase().contains(&phrase.to_ascii_lowercase())
}

/// `/librar(y|ies)/i`.
fn says_library(text: &str) -> bool {
    has_text(text, "library") || has_text(text, "libraries")
}

/// `/sacrific/i`.
fn says_sacrifice(text: &str) -> bool {
    has_text(text, "sacrific")
}

/// The start of the run of bytes `class` takes that ends at `end` (exclusive).
fn run_start(bytes: &[u8], end: usize, class: impl Fn(u8) -> bool) -> usize {
    let mut start = end;
    while start > 0 && class(bytes[start - 1]) {
        start -= 1;
    }
    start
}

/// `/\b\d+-cost\b/i`.
fn digits_dash_cost(text: &str) -> bool {
    let hay = text.to_ascii_lowercase();
    let bytes = hay.as_bytes();
    every_start(&hay, "-cost").into_iter().any(|at| {
        let start = run_start(bytes, at, |b| b.is_ascii_digit());
        start < at && boundary(bytes, start) && boundary(bytes, at + "-cost".len())
    })
}

/// `/\bcost(s|ing)? \d/i`.
fn cost_then_digit(text: &str) -> bool {
    let hay = text.to_ascii_lowercase();
    let bytes = hay.as_bytes();
    every_start(&hay, "cost").into_iter().any(|at| {
        if !boundary(bytes, at) {
            return false;
        }
        let rest = &hay[at + "cost".len()..];
        ["s ", "ing ", " "].iter().any(|tail| {
            rest.strip_prefix(tail)
                .is_some_and(|after| after.bytes().next().is_some_and(|b| b.is_ascii_digit()))
        })
    })
}

/// `/\(\S+\)\+? cost\b/`: a bracketed number, then the noun "cost" in lowercase.
fn bracket_then_lowercase_cost(text: &str) -> bool {
    let bytes = text.as_bytes();
    for (open, _) in text.match_indices('(') {
        let mut at = open + 1;
        // `\S+` runs over every non-whitespace character; any `)` inside that run may close it.
        while at < bytes.len() && !bytes[at].is_ascii_whitespace() {
            if bytes[at] == b')' && at > open + 1 {
                let after = &text[at + 1..];
                for tail in ["+ cost", " cost"] {
                    if let Some(rest) = after.strip_prefix(tail)
                        && !rest.bytes().next().is_some_and(is_word)
                    {
                        return true;
                    }
                }
            }
            at += 1;
        }
    }
    false
}

/// `/\b[A-Za-z]+-cost\b/`: a kind of cost written as one hyphenated word ("odd-cost").
fn word_dash_cost(text: &str) -> bool {
    let bytes = text.as_bytes();
    every_start(text, "-cost").into_iter().any(|at| {
        let start = run_start(bytes, at, |b| b.is_ascii_alphabetic());
        start < at && boundary(bytes, start) && boundary(bytes, at + "-cost".len())
    })
}

/// `/\(paid \d/i`.
fn paid_then_digit(text: &str) -> bool {
    let hay = text.to_ascii_lowercase();
    every_start(&hay, "(paid ").into_iter().any(|at| {
        hay[at + "(paid ".len()..]
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_digit())
    })
}

/// Where `return` or `returns` ends when it stands as a word at `at` (`/\breturns?\b/`), if it does.
fn return_word_end(bytes: &[u8], at: usize) -> Option<usize> {
    if !boundary(bytes, at) {
        return None;
    }
    let end = at + "return".len();
    if bytes.get(end) == Some(&b's') {
        boundary(bytes, end + 1).then_some(end + 1)
    } else {
        boundary(bytes, end).then_some(end)
    }
}

/// The first byte at or after `from` that `stop` takes, or the end.
fn segment_end(bytes: &[u8], from: usize, stop: impl Fn(u8) -> bool) -> usize {
    (from..bytes.len())
        .find(|at| stop(bytes[*at]))
        .unwrap_or(bytes.len())
}

/// `/\breturns?\b[^.\n]*\bto\b[^.\n]*\bhand\b/i`.
fn return_to_hand_in_a_sentence(text: &str) -> bool {
    let hay = text.to_ascii_lowercase();
    let bytes = hay.as_bytes();
    every_start(&hay, "return").into_iter().any(|at| {
        let Some(end) = return_word_end(bytes, at) else {
            return false;
        };
        let stop = segment_end(bytes, end, |b| b == b'.' || b == b'\n');
        let segment = &hay[..stop];
        every_start(segment, "to")
            .into_iter()
            .filter(|to| *to >= end && boundary(bytes, *to) && boundary(bytes, *to + 2))
            .any(|to| {
                every_start(segment, "hand")
                    .into_iter()
                    .any(|hand| hand >= to + 2 && boundary(bytes, hand) && boundary(bytes, hand + 4))
            })
    })
}

/// `/\breturn\b[^.\n]*\bto your hand\b/i`.
fn return_to_your_hand(text: &str) -> bool {
    let hay = text.to_ascii_lowercase();
    let bytes = hay.as_bytes();
    every_start(&hay, "return").into_iter().any(|at| {
        let end = at + "return".len();
        if !boundary(bytes, at) || !boundary(bytes, end) {
            return false;
        }
        let stop = segment_end(bytes, end, |b| b == b'.' || b == b'\n');
        let segment = &hay[..stop];
        every_start(segment, "to your hand")
            .into_iter()
            .any(|to| to >= end && boundary(bytes, to) && boundary(bytes, to + "to your hand".len()))
    })
}

/// `/\breturns? [^.]*\bto <tail>\b/i` for each `tail` (`hand`, or the owner's or your hand): a Return
/// that reaches a hand before the sentence's full stop (a line break does not end it).
fn returns_then_to(text: &str, tails: &[&str]) -> bool {
    let hay = text.to_ascii_lowercase();
    let bytes = hay.as_bytes();
    every_start(&hay, "return").into_iter().any(|at| {
        if !boundary(bytes, at) {
            return false;
        }
        let after = &hay[at + "return".len()..];
        let start = if after.starts_with("s ") {
            at + "returns ".len()
        } else if after.starts_with(' ') {
            at + "return ".len()
        } else {
            return false;
        };
        let stop = segment_end(bytes, start, |b| b == b'.');
        let segment = &hay[..stop];
        every_start(segment, "to ")
            .into_iter()
            .filter(|to| *to >= start && boundary(bytes, *to))
            .any(|to| {
                tails.iter().any(|tail| {
                    let end = to + "to ".len() + tail.len();
                    segment[to + "to ".len()..].starts_with(tail) && end <= stop && boundary(bytes, end)
                })
            })
    })
}

/// `/\bBounced?\b/`.
fn says_bounce_or_bounced(text: &str) -> bool {
    let bytes = text.as_bytes();
    every_start(text, "Bounce").into_iter().any(|at| {
        if !boundary(bytes, at) {
            return false;
        }
        let end = at + "Bounce".len();
        if bytes.get(end) == Some(&b'd') {
            boundary(bytes, end + 1)
        } else {
            boundary(bytes, end)
        }
    })
}

/// `/\b(at the (start|end)( and end)? of your turn|(start|end) of your turn)\b/i`.
fn your_turn_trigger(text: &str) -> bool {
    [
        "at the start of your turn",
        "at the end of your turn",
        "at the start and end of your turn",
        "at the end and end of your turn",
        "start of your turn",
        "end of your turn",
    ]
    .iter()
    .any(|phrase| has_words(text, phrase, true))
}

/// `/^(Tribute \d+|Echo (\d+|X))$/`: a computed Echo X (R802) prints "Echo X".
fn is_tribute_or_echo_count(item: &str) -> bool {
    if let Some(n) = item.strip_prefix("Echo ") {
        return !n.is_empty() && (n == "X" || n.bytes().all(|b| b.is_ascii_digit()));
    }
    item.strip_prefix("Tribute ")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// `/\.["”]?$/`.
fn ends_with_a_full_stop(line: &str) -> bool {
    line.ends_with('.') || line.ends_with(".\"") || line.ends_with(".\u{201d}")
}

/// Every R366 check a face fails, by name; empty when it passes them all. A face printed in Chinese
/// is read by Chinese punctuation instead (R1302).
fn failures(face: &Face) -> Vec<String> {
    let text = face.text.as_str();
    let keywords = &face.keywords;
    // R988: a face printed in English that names the element glyphs (Meditative #40) is R366's
    // English shape. A face is printed in Chinese when it is punctuated in Chinese — R1302 ends
    // every other line with 。, so a Chinese face always is — not when it merely names a glyph.
    if text.chars().any(is_cjk_punctuation) {
        return cjk_failures(text, keywords);
    }
    let mut out: Vec<String> = Vec::new();
    if says_library(text) {
        out.push("says library, not deck".to_string());
    }
    if says_sacrifice(text) {
        out.push("says sacrifice, not Tribute".to_string());
    }
    if digits_dash_cost(text) || cost_then_digit(text) {
        out.push("writes a cost without (N)".to_string());
    }
    // R432: "(N) Cost" is the noun — never the old "Cost (N)", never a lowercase "(N) cost".
    if has_word_start(text, "Cost (", false) {
        out.push("writes the noun as \"Cost (N)\", not \"(N) Cost\"".to_string());
    }
    if bracket_then_lowercase_cost(text) {
        out.push("writes the noun \"(N) cost\" without its capital".to_string());
    }
    if has_word_start(text, "costing (", false) {
        out.push("writes a price as \"costing (N)\", not \"costs (N)\"".to_string());
    }
    if has_word_start(text, "that costs (", true) {
        out.push("writes \"that costs (N)\", not \"(N) Cost\"".to_string());
    }
    if word_dash_cost(text) {
        out.push("writes a kind of cost as \"odd-cost\", not \"odd Cost\"".to_string());
    }
    if paid_then_digit(text) {
        out.push("writes an embiggen price as \"(paid N\" rather than \"Paid (N):\"".to_string());
    }
    // Vocabulary table (patch v0.2.1, issue #45; balance patch 1 retires "Return … to hand" for
    // Bounce, R692): a face that returns a permanent from the field says Bounce; only the faces that
    // return a card from anywhere else keep "Return … to hand" (R746).
    if return_to_hand_in_a_sentence(text)
        && !RETURNS_OFF_THE_FIELD.contains(&format!("{} {}", face.card.id, face.face).as_str())
    {
        out.push("says \"Return … to hand\", not Bounce".to_string());
    }
    if has_words(text, "backrow zone", true) {
        out.push("says backrow zone, not backrow".to_string());
    }
    if your_turn_trigger(text) {
        out.push("writes turn trigger as (Start|End) of your turn, not (Start|End) of turn:".to_string());
    }
    if has_words(text, "Start of Game", false) {
        out.push("writes \"Start of Game\", not \"Start of game\"".to_string());
    }
    if has_words(text, "Once per Turn", false) {
        out.push("writes \"Once per Turn\", not \"Once per turn\"".to_string());
    }
    if has_words(text, "Cannot be in Defense Position", true) {
        out.push(
            "writes \"Cannot be in Defense Position\", not \"Can't be in Defense Position\"".to_string(),
        );
    }
    if has_words(text, "Trigger the Cry", true) {
        out.push("writes \"Trigger the Cry\", not \"Trigger a Cry\"".to_string());
    }
    if has_words(text, "Set a hero's health", true) {
        out.push("writes \"Set a hero's health\", not \"Set health\"".to_string());
    }
    if has_words(text, "End your turn", true) {
        out.push("writes \"End your turn\", not \"End the turn\"".to_string());
    }
    // Return sends a card to its owner's hand, so no Return face names "your" hand (Add keeps it).
    if return_to_your_hand(text) {
        out.push("writes \"to your hand\", not \"to hand\"".to_string());
    }
    let lines: Vec<&str> = if text.is_empty() {
        Vec::new()
    } else {
        text.split('\n').collect()
    };
    // The keyword list: the first line, when every item on it is a printed keyword or a Tribute cost,
    // which the list carries too (#55, #66). A face with keywords must lead with them.
    let labels: Vec<String> = keywords.iter().map(keyword_label).collect();
    let list_item = |item: &str| labels.iter().any(|label| label == item) || is_tribute_or_echo_count(item);
    let lead: Vec<&str> = lines.first().copied().unwrap_or("").split(", ").collect();
    let has_list = !lines.is_empty() && lead.iter().copied().all(list_item);
    for label in &labels {
        if !has_list || !lead.contains(&label.as_str()) {
            out.push(format!("does not lead with its keyword {label}"));
        }
    }
    // Every other line is sentences, each ending with a full stop, and a labelled ability ("Death:",
    // "Aura:", "Paid (4):") starts a line of its own.
    for (at, line) in lines.iter().enumerate() {
        if at == 0 && has_list {
            if line.ends_with('.') {
                out.push("ends its keyword line with a full stop".to_string());
            }
            continue;
        }
        if !ends_with_a_full_stop(line) {
            out.push(format!("does not end the line \"{line}\" with a full stop"));
        }
        for label in LABELS {
            if matches!(line.find(label), Some(index) if index > 0) {
                out.push(format!("does not start \"{label}\" on a line of its own"));
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// R1302 (MD-B13, ME-CN): a face printed in Chinese (Meditative #32's Radiant face, and every face of
// the Chinese table, `chinese.rs`) is R366's shape in Chinese punctuation: the keyword list is joined
// with "，" on a line of its own with no full stop, a labelled ability ("战吼：") starts a sentence of
// its own, every other line ends with "。", and a cost stays "(N)". No ASCII "." "," ";" or ":" is
// written, and no full-width bracket.
// ---------------------------------------------------------------------------------------------

/// R988: a character that punctuates a Chinese face: CJK punctuation (U+3000–U+303F) and the
/// full-width forms (U+FF00–U+FFEF: "，", "："). The element glyphs are unified ideographs
/// (U+3400–U+4DBF, U+4E00–U+9FFF), not punctuation, so an English face that names them is not
/// punctuated in Chinese.
pub(super) fn is_cjk_punctuation(c: char) -> bool {
    matches!(c, '\u{3000}'..='\u{303f}' | '\u{ff00}'..='\u{ffef}')
}

/// `chinese-terms.json` (R1303), the frame's words in Chinese.
pub(super) fn chinese_terms() -> Value {
    serde_json::from_str(chinese_terms_json()).expect("crates/cards/chinese-terms.json is JSON")
}

/// How a printed keyword reads in Chinese text: its word with the number written on ("护甲7",
/// "幸运1"), the Bread Token's "护甲X" and "法术伤害+2", as `keyword_label` reads the English.
pub(super) fn chinese_keyword_label(keyword: &Keyword, terms: &Value) -> String {
    let word = terms["keywords"][keyword.kind().as_str()]
        .as_str()
        .unwrap_or_else(|| panic!("chinese-terms.json has no keyword {}", keyword.kind().as_str()));
    match keyword.n() {
        None => word.to_string(),
        Some(n) if keyword.kind() == KeywordKind::SpellDamage => format!("{word}+{n}"),
        Some(0) if keyword.kind() == KeywordKind::Armor => format!("{word}X"),
        Some(n) => format!("{word}{n}"),
    }
}

/// `/^(献祭|回响)\d+$/`: a Tribute or Echo count in the keyword list, as `is_tribute_or_echo_count`.
fn is_chinese_tribute_or_echo_count(item: &str) -> bool {
    ["献祭", "回响"].iter().any(|word| {
        item.strip_prefix(word)
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    })
}

/// `/。[”」]?$/`.
fn ends_with_a_chinese_full_stop(line: &str) -> bool {
    line.ends_with('。') || line.ends_with("。”") || line.ends_with("。」")
}

/// Every R1302 check a Chinese face fails, by name; empty when it passes them all. `keywords` are the
/// face's printed keywords, which the first line must list.
pub(super) fn cjk_failures(text: &str, keywords: &[Keyword]) -> Vec<String> {
    let terms = chinese_terms();
    let ability_labels: Vec<&str> = LABELS
        .iter()
        .map(|label| {
            terms["labels"][*label]
                .as_str()
                .unwrap_or_else(|| panic!("chinese-terms.json has no label {label}"))
        })
        .collect();
    let mut out: Vec<String> = Vec::new();
    let lines: Vec<&str> = if text.is_empty() {
        Vec::new()
    } else {
        text.split('\n').collect()
    };
    let labels: Vec<String> = keywords
        .iter()
        .map(|keyword| chinese_keyword_label(keyword, &terms))
        .collect();
    let list_item =
        |item: &str| labels.iter().any(|label| label == item) || is_chinese_tribute_or_echo_count(item);
    let lead: Vec<&str> = lines.first().copied().unwrap_or("").split('，').collect();
    let has_list = !lines.is_empty() && lead.iter().copied().all(list_item);
    for label in &labels {
        if !has_list || !lead.contains(&label.as_str()) {
            out.push(format!("does not lead with its keyword {label}"));
        }
    }
    for (at, line) in lines.iter().enumerate() {
        if let Some(mark) = line.chars().find(|c| matches!(c, '.' | ',' | ';' | ':')) {
            out.push(format!("writes the ASCII \"{mark}\" in \"{line}\""));
        }
        if line.contains(['（', '）']) {
            out.push(format!("writes a full-width bracket in \"{line}\", not \"(N)\""));
        }
        if at == 0 && has_list {
            if line.ends_with('。') {
                out.push("ends its keyword line with a full stop".to_string());
            }
            continue;
        }
        if !ends_with_a_chinese_full_stop(line) {
            out.push(format!("does not end the line \"{line}\" with \"。\""));
        }
        // A label opens its ability: at a line's start or after another label's words ("战吼和回合开始
        // 时：", C+ #7's "Cry and start of turn:"), never after a sentence that ended on the line.
        for label in &ability_labels {
            for (index, _) in line.match_indices(label) {
                if line[..index].ends_with(['。', '；']) {
                    out.push(format!("does not start \"{label}\" on a line of its own"));
                }
            }
        }
    }
    out
}

fn described(face: &Face) -> String {
    format!("{} {}: {}", face.card.id, face.face, face.text)
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}

// ---------------------------------------------------------------------------------------------
// The patch history (B4.2, R388, R646), as `scripts/patches-io.ts` read it: `crates/cards/patches/`
// compiled in.
// ---------------------------------------------------------------------------------------------

/// One catalog entry as a snapshot holds it: its fields in the file's order.
type Entry = IndexMap<String, Value>;

/// TS `Catalog = Record<string, Record<string, unknown>>`: a whole catalog, by id, in file order.
type Catalog = IndexMap<String, Entry>;

/// Every shipped snapshot (`patches/<version>.json`), compiled in. A patch shipped after this file
/// was written has no row here until one is added; `read_snapshot` names it when a test asks.
const SNAPSHOTS: &[(&str, &str)] = &[
    ("v0.1.0", include_str!("../../patches/v0.1.0.json")),
    ("v0.1.0b", include_str!("../../patches/v0.1.0b.json")),
    ("v0.1.0c", include_str!("../../patches/v0.1.0c.json")),
    ("v0.1.0d", include_str!("../../patches/v0.1.0d.json")),
    ("v0.1.1", include_str!("../../patches/v0.1.1.json")),
    ("v0.2.0", include_str!("../../patches/v0.2.0.json")),
    ("v0.2.1", include_str!("../../patches/v0.2.1.json")),
    ("v0.2.2", include_str!("../../patches/v0.2.2.json")),
    ("v0.2.3", include_str!("../../patches/v0.2.3.json")),
    ("v0.2.4", include_str!("../../patches/v0.2.4.json")),
    ("v0.2.5", include_str!("../../patches/v0.2.5.json")),
    ("v0.2.6", include_str!("../../patches/v0.2.6.json")),
    ("v0.2.7", include_str!("../../patches/v0.2.7.json")),
    ("v0.2.7b", include_str!("../../patches/v0.2.7b.json")),
    ("v0.2.8", include_str!("../../patches/v0.2.8.json")),
    ("v0.2.8b", include_str!("../../patches/v0.2.8b.json")),
    ("v0.2.9", include_str!("../../patches/v0.2.9.json")),
    ("v0.2.10", include_str!("../../patches/v0.2.10.json")),
    ("v0.2.10b", include_str!("../../patches/v0.2.10b.json")),
    ("v0.2.10c", include_str!("../../patches/v0.2.10c.json")),
    ("v0.2.10d", include_str!("../../patches/v0.2.10d.json")),
    ("v0.2.11", include_str!("../../patches/v0.2.11.json")),
];

/// `patches/patches.json`: every shipped patch in ship order.
const PATCHES_JSON: &str = include_str!("../../patches/patches.json");

/// TS `readSnapshot(version)`: the whole catalog as that patch left it.
fn read_snapshot(version: &str) -> Catalog {
    match SNAPSHOTS.iter().find(|(shipped, _)| *shipped == version) {
        Some((_, raw)) => serde_json::from_str(raw)
            .unwrap_or_else(|error| panic!("patches/{version}.json is not a catalog: {error}")),
        None => panic!("patches/{version}.json is not compiled into card_text.rs's SNAPSHOTS"),
    }
}

/// TS `readPatches().map((patch) => patch.version)`: every shipped version, in ship order.
fn shipped_versions() -> Vec<String> {
    let patches: Vec<Value> = serde_json::from_str(PATCHES_JSON).expect("patches.json is a list of patches");
    patches
        .iter()
        .filter_map(|patch| patch.get("version").and_then(Value::as_str).map(str::to_string))
        .collect()
}

/// TS `CATALOG`, read the way a snapshot is: the current catalog.json, its fields in file order.
fn current_catalog() -> Catalog {
    serde_json::from_str(catalog_json()).expect("catalog.json is a catalog")
}

/// TS `JSON.stringify(x)`: the top level in its own order, every nested object canonical.
fn stringify<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("a catalog entry serialises")
}

/// `entry[face]`, an object.
fn face_of<'a>(entry: &'a Entry, face: &str) -> &'a Map<String, Value> {
    match entry.get(face).and_then(Value::as_object) {
        Some(object) => object,
        None => panic!("an entry's {face} face is an object"),
    }
}

/// `{ ...object, [key]: value }` where `value` may be `undefined` (then the key is gone, which is how
/// `toEqual` reads an undefined property).
fn set_or_remove<K: Into<String>>(object: &mut IndexMap<String, Value>, key: K, value: Option<&Value>) {
    let key = key.into();
    match value {
        Some(value) => {
            object.insert(key, value.clone());
        }
        None => {
            object.shift_remove(&key);
        }
    }
}

/// The same on a nested face object.
fn set_or_remove_in(object: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    match value {
        Some(value) => {
            object.insert(key.to_string(), value.clone());
        }
        None => {
            object.remove(key);
        }
    }
}

/// `{ ...card, base: { ...card.base, text: "" }, radiant: { ...card.radiant, text: "" } }`.
fn without_text(card: &Entry) -> Entry {
    let mut out = card.clone();
    for face in ["base", "radiant"] {
        let mut printed = face_of(card, face).clone();
        printed.insert("text".to_string(), Value::from(""));
        out.insert(face.to_string(), Value::Object(printed));
    }
    out
}

/// `{ ...current, base: { ...current.base, <fields>: prior.base.<field> }, radiant: { … } }`: the
/// current card with the named face fields put back as the prior card printed them.
fn restore_face_fields(current: &Entry, prior: &Entry, fields: &[&str]) -> Entry {
    let mut out = current.clone();
    for face in ["base", "radiant"] {
        let mut printed = face_of(current, face).clone();
        let before = face_of(prior, face);
        for field in fields {
            set_or_remove_in(&mut printed, field, before.get(*field));
        }
        out.insert(face.to_string(), Value::Object(printed));
    }
    out
}

/// `entry[face][field]`.
fn face_field<'a>(entry: &'a Entry, face: &str, field: &str) -> Option<&'a Value> {
    face_of(entry, face).get(field)
}

/// `entry[face].text`, for a card the catalog holds.
fn face_text(catalog: &Catalog, id: &str, face: &str) -> String {
    catalog
        .get(id)
        .and_then(|entry| face_field(entry, face, "text"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// TS: `shipped ? readSnapshot(version) : CATALOG` — a patch pending until `patches ship` promotes it
/// (R646) is the current catalog until then, its snapshot after.
fn after_patch(version: &str) -> (bool, Catalog) {
    let shipped = shipped_versions().iter().any(|shipped| shipped == version);
    let after = if shipped {
        read_snapshot(version)
    } else {
        current_catalog()
    };
    (shipped, after)
}

mod r366_the_words_a_card_s_text_uses_spec_11_patch_v0_1_1 {
    use super::*;

    fn swept() -> Vec<Face> {
        faces_of(&entries())
    }

    #[test]
    fn r366_calls_the_library_the_deck_and_a_tribute_a_tribute_on_every_face() {
        let wrong: Vec<String> = swept()
            .iter()
            .filter(|face| says_library(&face.text) || says_sacrifice(&face.text))
            .map(described)
            .collect();
        assert_eq!(wrong, Vec::<String>::new());
    }

    #[test]
    fn r432_writes_a_specific_cost_as_the_noun_n_cost_a_price_as_the_verb_costs_n_and_an_embiggen_price_as_paid_n()
     {
        let wrong: Vec<String> = swept()
            .iter()
            .filter(|face| {
                failures(face)
                    .iter()
                    .any(|why| why.contains("cost") || why.contains("embiggen"))
            })
            .map(described)
            .collect();
        assert_eq!(wrong, Vec::<String>::new());
    }

    #[test]
    fn r366_leads_with_the_face_s_printed_keywords_on_a_line_of_their_own_starts_each_labelled_ability_on_its_own_line_and_ends_every_other_line_with_a_full_stop()
     {
        let wrong: Vec<String> = swept()
            .iter()
            .flat_map(|face| {
                failures(face)
                    .into_iter()
                    .map(|why| format!("{} {} {why}: {}", face.card.id, face.face, face.text))
                    .collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(wrong, Vec::<String>::new());
    }

    #[test]
    fn r692_prints_bounce_for_a_permanent_s_return_to_hand_from_the_field_and_no_other_face_says_return_to_hand()
     {
        let faces = faces_of(&entries());
        let stale: Vec<String> = faces
            .iter()
            .filter(|f| {
                returns_then_to(
                    &f.text,
                    &["its owner's hand", "their owner's hand", "your hand", "hand"],
                ) && !RETURNS_OFF_THE_FIELD.contains(&format!("{} {}", f.card.id, f.face).as_str())
            })
            .map(described)
            .collect();
        assert_eq!(stale, Vec::<String>::new());
        let bouncing: &[(&str, FaceKind)] = &[
            ("core-017", FaceKind::Base),
            ("core-017", FaceKind::Radiant),
            ("core-052", FaceKind::Radiant),
            ("classic-014", FaceKind::Base),
            ("classic-022", FaceKind::Base),
            ("classic-022", FaceKind::Radiant),
            ("classic-066", FaceKind::Base),
            ("classic-066", FaceKind::Radiant),
        ];
        for (id, face) in bouncing {
            let text = faces
                .iter()
                .find(|f| f.card.id == *id && f.face == *face)
                .map(|f| f.text.clone())
                .unwrap_or_default();
            assert!(says_bounce_or_bounced(&text), "{id} {face}");
        }
    }

    #[test]
    fn r746_says_return_to_hand_never_bounce_where_a_card_returns_from_the_graveyard_exile_or_a_resolved_spell_issue_354()
     {
        let faces = faces_of(&entries());
        let returning: Vec<String> = faces
            .iter()
            .filter(|f| returns_then_to(&f.text, &["hand"]))
            .map(|f| format!("{} {}", f.card.id, f.face))
            .collect();
        assert_eq!(sorted(returning), sorted(strings(RETURNS_OFF_THE_FIELD)));
        for key in RETURNS_OFF_THE_FIELD {
            let text = faces
                .iter()
                .find(|f| format!("{} {}", f.card.id, f.face) == *key)
                .map(|f| f.text.clone())
                .unwrap_or_default();
            assert!(!has_word_start(&text, "Bounce", false), "{key}");
        }
    }

    #[test]
    fn r366_patch_v0_2_1_only_changes_base_text_and_radiant_text_between_v0_2_0_and_v0_2_1() {
        let before = read_snapshot("v0.2.0");
        let after = read_snapshot("v0.2.1");
        let mut differing_cards: Vec<String> = Vec::new();
        for (id, current_card) in &after {
            let prior_card = before.get(id);
            assert!(prior_card.is_some(), "card {id} existed in v0.2.0");
            let Some(prior_card) = prior_card else { continue };

            let prior_non_text = without_text(prior_card);
            let current_non_text = without_text(current_card);
            assert_eq!(current_non_text, prior_non_text, "non-text fields of {id}");

            if stringify(prior_card) != stringify(current_card) {
                differing_cards.push(id.clone());
            }
        }
        assert_eq!(
            differing_cards,
            strings(&[
                "core-017",
                "core-023",
                "core-024",
                "core-031",
                "core-067",
                "classic-010",
                "classic-014",
                "classic-022",
                "classic-025",
                "classic-029",
                "classic-034",
                "classic-047",
                "classic-054",
                "classic-063",
                "classic-065",
                "classic-066",
                "classicplus-014",
                "classicplus-019-5",
                "classicplus-021",
                "classicplus-026",
                "classicplus-034",
                "classicplus-052",
            ])
        );
    }

    #[test]
    fn r366_patch_v0_2_2_changes_only_its_mechanics_cards_between_v0_2_1_and_v0_2_2() {
        let before = read_snapshot("v0.2.1");
        let after = read_snapshot("v0.2.2");
        let mut non_text_changed: IndexMap<String, Vec<String>> = IndexMap::new();
        let mut differing_cards: Vec<String> = Vec::new();
        for (id, current_card) in &after {
            let prior_card = before.get(id);
            assert!(prior_card.is_some(), "card {id} existed in v0.2.1");
            let Some(prior_card) = prior_card else { continue };

            let prior_non_text = without_text(prior_card);
            let current_non_text = without_text(current_card);
            let fields: IndexSet<&String> = prior_non_text.keys().chain(current_non_text.keys()).collect();
            let changed: Vec<String> = fields
                .into_iter()
                .filter(|field| prior_non_text.get(*field) != current_non_text.get(*field))
                .cloned()
                .collect();
            if !changed.is_empty() {
                non_text_changed.insert(id.clone(), changed);
            }

            if stringify(prior_card) != stringify(current_card) {
                differing_cards.push(id.clone());
            }
        }
        // Seventeen cards gain the Plague tag and nothing else; Exile moves its threshold, Blade Storm
        // gains its Whirlwind ref, Adaptive Growth its numbers, Chaos Machine its lines of code.
        let expected_fields: &[(&str, &[&str])] = &[
            ("core-091", &["tags"]),
            ("classic-010", &["params"]),
            ("classic-027", &["tags"]),
            ("classic-039", &["tags"]),
            ("classic-042", &["tags"]),
            ("classic-043", &["tags"]),
            ("classic-053", &["tags"]),
            ("classic-059", &["tags"]),
            ("classic-061", &["tags"]),
            ("classic-062", &["tags"]),
            ("classic-063", &["tags"]),
            ("classic-069", &["tags"]),
            ("classic-070", &["tags"]),
            ("classic-074", &["tags"]),
            ("classic-076", &["tags"]),
            ("classic-078", &["tags"]),
            ("classic-087", &["tags"]),
            ("classicplus-003", &["tags"]),
            ("classicplus-032-3", &["refs"]),
            ("classicplus-050", &["params", "loc"]),
            ("classicplus-070", &["loc"]),
        ];
        let expected: IndexMap<String, Vec<String>> = expected_fields
            .iter()
            .map(|(id, fields)| ((*id).to_string(), strings(fields)))
            .collect();
        assert_eq!(non_text_changed, expected);
        assert_eq!(
            differing_cards,
            strings(&[
                "core-091",
                "classic-010",
                "classic-027",
                "classic-033",
                "classic-039",
                "classic-042",
                "classic-043",
                "classic-053",
                "classic-059",
                "classic-061",
                "classic-062",
                "classic-063",
                "classic-069",
                "classic-070",
                "classic-074",
                "classic-076",
                "classic-078",
                "classic-087",
                "classicplus-003",
                "classicplus-032-3",
                "classicplus-050",
                "classicplus-070",
            ])
        );
    }

    /// The eighteen Field Spells patch v0.2.3 Animated and patch v0.2.5 put back.
    const ANIMATED_IN_V0_2_3: &[&str] = &[
        "core-014",
        "core-033",
        "core-038",
        "core-065",
        "core-073",
        "classic-004",
        "classic-007",
        "classic-062",
        "classic-064",
        "classic-087",
        "classicplus-007",
        "classicplus-012-5",
        "classicplus-012-7",
        "classicplus-031",
        "classicplus-061",
        "classicplus-063",
        "classicplus-070",
        "classicplus-078",
    ];

    fn is_number(value: Option<&Value>) -> bool {
        value.is_some_and(Value::is_number)
    }

    #[test]
    fn r366_patch_v0_2_3_only_animates_eighteen_field_spells_rewords_ivory_tower_and_moves_final_gambit_s_loc_between_v0_2_2_and_v0_2_3()
     {
        let before = read_snapshot("v0.2.2");
        let after = read_snapshot("v0.2.3");
        let animated: IndexSet<&str> = ANIMATED_IN_V0_2_3.iter().copied().collect();
        let mut changed: Vec<String> = Vec::new();
        for (id, current_card) in &after {
            let prior_card = before.get(id);
            assert!(prior_card.is_some(), "card {id} existed in v0.2.2");
            let Some(prior_card) = prior_card else { continue };
            if stringify(prior_card) != stringify(current_card) {
                changed.push(id.clone());
            }

            if animated.contains(id.as_str()) {
                for face in ["base", "radiant"] {
                    assert!(
                        face_field(prior_card, face, "attack").is_none(),
                        "{id} {face} had no stats before v0.2.3"
                    );
                    assert!(
                        face_field(prior_card, face, "health").is_none(),
                        "{id} {face} had no stats before v0.2.3"
                    );
                    assert_eq!(
                        face_field(prior_card, face, "keywords"),
                        Some(&json!([])),
                        "{id} {face} had no keywords before v0.2.3"
                    );
                    assert_eq!(
                        face_field(current_card, face, "keywords"),
                        Some(&json!([{ "kind": "Animated" }])),
                        "{id} {face} gains Animated"
                    );
                    assert!(
                        is_number(face_field(current_card, face, "attack")),
                        "{id} {face} gains attack"
                    );
                    assert!(
                        is_number(face_field(current_card, face, "health")),
                        "{id} {face} gains health"
                    );
                    let prior_text = face_field(prior_card, face, "text")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    assert_eq!(
                        face_field(current_card, face, "text").and_then(Value::as_str),
                        Some(format!("Animated\n{prior_text}").as_str()),
                        "{id} {face} prints Animated"
                    );
                }
                // Nothing else on the card moves: normalizing the eight Animated fields restores the prior card.
                assert_eq!(
                    restore_face_fields(
                        current_card,
                        prior_card,
                        &["attack", "health", "keywords", "text"]
                    ),
                    *prior_card,
                    "only Animated fields differ on {id}"
                );
            } else if id == "classicplus-033" {
                // Ivory Tower (issue #113): the Stack aura is gone, replaced by the fusion text; it gains no stats.
                for face in ["base", "radiant"] {
                    assert_eq!(
                        face_field(current_card, face, "keywords"),
                        Some(&json!([])),
                        "{id} {face} gains no keywords"
                    );
                    assert!(
                        face_field(current_card, face, "attack").is_none(),
                        "{id} {face} gains no stats"
                    );
                    assert!(
                        face_field(current_card, face, "health").is_none(),
                        "{id} {face} gains no stats"
                    );
                }
                assert_eq!(
                    face_field(current_card, "base", "text").and_then(Value::as_str),
                    Some("The first Unit you stack onto this is fused into it.")
                );
                assert_eq!(
                    face_field(current_card, "radiant", "text").and_then(Value::as_str),
                    Some("The first Unit you stack onto this becomes Radiant and is fused into it.")
                );
                // Only the two texts and the script's loc move.
                let mut restored = restore_face_fields(current_card, prior_card, &["text"]);
                set_or_remove(&mut restored, "loc", prior_card.get("loc"));
                assert_eq!(restored, *prior_card, "only text and loc differ on {id}");
            } else if id == "classic-052" {
                // Final Gambit's follow-up gained its R216 guard: only the script's loc moves.
                let mut restored = current_card.clone();
                set_or_remove(&mut restored, "loc", prior_card.get("loc"));
                assert_eq!(restored, *prior_card, "only loc differs on {id}");
            } else {
                assert_eq!(current_card, prior_card, "card {id} unchanged by v0.2.3");
            }
        }
        let mut expected = strings(ANIMATED_IN_V0_2_3);
        expected.extend(strings(&["classicplus-033", "classic-052"]));
        assert_eq!(sorted(changed), sorted(expected));
    }

    #[test]
    fn r366_patch_v0_2_4_aims_solarius_prime_and_appropriations_keywords_deft_duelist_and_moves_two_locs_between_v0_2_3_and_v0_2_4()
     {
        let before = read_snapshot("v0.2.3");
        // Pending until `patches ship` promotes it (R646): the current catalog until then, its snapshot after.
        let (_, after) = after_patch("v0.2.4");
        let five: IndexSet<&str> = [
            "classic-003",
            "classicplus-010",
            "classicplus-038-1",
            "classicplus-040",
            "core-045",
        ]
        .into_iter()
        .collect();
        let mut changed: Vec<String> = Vec::new();
        for (id, current_card) in &after {
            let prior_card = before.get(id);
            assert!(prior_card.is_some(), "card {id} existed in v0.2.3");
            let Some(prior_card) = prior_card else { continue };
            if stringify(prior_card) != stringify(current_card) {
                changed.push(id.clone());
            }
        }
        // Another pending fragment's cards legitimately differ beside these five (R646), so the diff is
        // read on this patch's claims.
        let claimed: Vec<String> = changed
            .into_iter()
            .filter(|id| five.contains(id.as_str()))
            .collect();
        assert_eq!(
            sorted(claimed),
            sorted(five.iter().map(|id| (*id).to_string()).collect())
        );
        // Deft Duelist prints the Deft keyword on both faces (R49).
        assert_eq!(face_text(&after, "core-045", "base"), "Charge, Deft");
        assert_eq!(face_text(&after, "core-045", "radiant"), "Charge, Armor 1, Deft");
        // The aimed casts say so on the face (R656).
        for face in ["base", "radiant"] {
            assert!(
                face_text(&after, "classicplus-038-1", face)
                    .contains("Each aims at enemies when it harms and at your side when it helps.")
            );
            assert!(
                face_text(&after, "classicplus-040", face)
                    .contains("aim at enemies when they harm and at your side when they help.")
            );
        }
        // Book of Heal and New Wraps move only their script's loc.
        for id in ["classic-003", "classicplus-010"] {
            let prior_card = &before[id];
            let mut restored = after[id].clone();
            set_or_remove(&mut restored, "loc", prior_card.get("loc"));
            assert_eq!(restored, *prior_card, "{id}");
        }
    }

    #[test]
    fn r366_patch_v0_2_10_issue_88_changes_exactly_the_balance_patch_cards_against_the_patch_before_it() {
        // v0.2.10 (made as v0.2.15, renumbered by R743) ships after v0.2.9: its baseline is the newest
        // shipped snapshot while it is pending and the patch before it once `patches ship` has
        // promoted it, and the patch is the current catalog until then, its snapshot after.
        let versions = shipped_versions();
        let at = versions.iter().position(|version| version == "v0.2.10");
        let baseline = match at {
            None => versions.last().cloned().expect("a shipped patch"),
            Some(at) => versions[at - 1].clone(),
        };
        let before = read_snapshot(&baseline);
        let after = if at.is_none() {
            current_catalog()
        } else {
            read_snapshot("v0.2.10")
        };
        // The top-level fields each balance card may move (balance patch 1, issue #88); every other
        // field restores the baseline card, so no card smuggles an unlisted change.
        let allowed_fields: &[(&str, &[&str])] = &[
            ("core-017", &["base", "radiant"]),
            ("core-021", &["loc"]),
            ("core-023", &["base", "radiant"]),
            ("core-024", &["base", "radiant"]),
            ("core-031", &["base", "radiant"]),
            ("core-032", &["base", "radiant"]),
            ("core-052", &["radiant"]),
            ("classic-004", &["tags", "loc", "base"]),
            ("classic-008", &["loc"]),
            ("classic-010", &["cost"]),
            ("classic-014", &["base"]),
            ("classic-015", &["base", "radiant"]),
            ("classic-018", &["loc", "base"]),
            ("classic-020", &["loc"]),
            ("classic-021", &["base", "radiant"]),
            ("classic-022", &["params", "loc", "base", "radiant"]),
            ("classic-023", &["loc"]),
            ("classic-025", &["loc", "base"]),
            ("classic-026", &["loc"]),
            ("classic-028", &["base"]),
            ("classic-033", &["radiant"]),
            ("classic-034", &["loc", "base", "radiant"]),
            ("classic-037", &["cost"]),
            ("classic-038", &["base", "radiant"]),
            ("classic-043", &["cost"]),
            ("classic-046", &["base"]),
            ("classic-047", &["base", "radiant"]),
            ("classic-048", &["radiant"]),
            ("classic-054", &["base"]),
            ("classic-064", &["loc"]),
            ("classic-065", &["loc", "radiant"]),
            ("classic-066", &["base", "radiant"]),
            ("classic-074", &["loc", "base", "radiant"]),
            ("classic-075", &["cost"]),
            ("classic-080", &["base", "radiant"]),
            ("classic-083", &["params", "base", "radiant"]),
            ("classic-088", &["loc", "base", "radiant"]),
            ("classic-090", &["base", "radiant"]),
            ("classicplus-007", &["base"]),
            ("classicplus-012-6", &["type", "base", "radiant"]),
            ("classicplus-014", &["params", "loc", "base", "radiant"]),
            ("classicplus-019", &["loc"]),
            ("classicplus-021", &["radiant"]),
            ("classicplus-030", &["cost"]),
            ("classicplus-031", &["base", "radiant"]),
            ("classicplus-038", &["params", "loc", "base", "radiant"]),
            ("classicplus-039", &["params", "loc", "base", "radiant"]),
            ("classicplus-040", &["base", "radiant"]),
            ("classicplus-042", &["refs", "base", "radiant"]),
            ("classicplus-042-1", &["loc"]),
            ("classicplus-046", &["params", "loc", "base", "radiant"]),
            ("classicplus-053", &["params", "loc", "base", "radiant"]),
            ("classicplus-056", &["loc"]),
            ("classicplus-060", &["loc", "base", "radiant"]),
            ("classicplus-063", &["params", "loc", "base", "radiant"]),
            ("classicplus-065", &["refs", "params", "base", "radiant"]),
            ("classicplus-065-2", &["params", "base", "radiant"]),
            ("classicplus-065-4", &["radiant"]),
            ("classicplus-066", &["refs", "base", "radiant"]),
            ("classicplus-073-1", &["loc", "base", "radiant"]),
            ("classicplus-074", &["base", "radiant"]),
            ("classicplus-078", &["params", "loc", "base", "radiant"]),
        ];
        let allowed: IndexMap<&str, &[&str]> = allowed_fields.iter().copied().collect();
        let mut changed: Vec<String> = Vec::new();
        for (id, current_card) in &after {
            let prior_card = before.get(id);
            assert!(prior_card.is_some(), "card {id} is not new in v0.2.10");
            let Some(prior_card) = prior_card else { continue };
            if stringify(prior_card) != stringify(current_card) {
                changed.push(id.clone());
            }
            let Some(fields) = allowed.get(id.as_str()) else {
                assert_eq!(current_card, prior_card, "card {id} unchanged by v0.2.10");
                continue;
            };
            let mut restored = current_card.clone();
            for field in *fields {
                set_or_remove(&mut restored, *field, prior_card.get(*field));
            }
            assert_eq!(restored, *prior_card, "only {} differ on {id}", fields.join(", "));
        }
        assert_eq!(
            sorted(changed),
            sorted(allowed.keys().map(|id| (*id).to_string()).collect())
        );
    }

    #[test]
    fn r366_patch_v0_2_5_removes_animated_from_the_eighteen_v0_2_3_field_spells_between_v0_2_3_and_v0_2_5() {
        let before = read_snapshot("v0.2.3");
        // Pending until `patches ship` promotes it (R646): the current catalog until then, its snapshot after.
        let (_, after) = after_patch("v0.2.5");
        let unanimated: IndexSet<&str> = ANIMATED_IN_V0_2_3.iter().copied().collect();
        let mut changed: Vec<String> = Vec::new();
        for (id, current_card) in &after {
            let prior_card = before.get(id);
            assert!(prior_card.is_some(), "card {id} existed in v0.2.3");
            let Some(prior_card) = prior_card else { continue };
            if stringify(prior_card) != stringify(current_card) {
                changed.push(id.clone());
            }
            if !unanimated.contains(id.as_str()) {
                continue;
            }

            for face in ["base", "radiant"] {
                assert!(
                    face_field(current_card, face, "attack").is_none(),
                    "{id} {face} loses its stats"
                );
                assert!(
                    face_field(current_card, face, "health").is_none(),
                    "{id} {face} loses its stats"
                );
                assert_eq!(
                    face_field(current_card, face, "keywords"),
                    Some(&json!([])),
                    "{id} {face} loses Animated"
                );
                assert_eq!(
                    face_field(prior_card, face, "keywords"),
                    Some(&json!([{ "kind": "Animated" }])),
                    "{id} {face} printed Animated"
                );
                assert!(
                    is_number(face_field(prior_card, face, "attack")),
                    "{id} {face} printed attack"
                );
                assert!(
                    is_number(face_field(prior_card, face, "health")),
                    "{id} {face} printed health"
                );
                let current_text = face_field(current_card, face, "text")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                assert_eq!(
                    face_field(prior_card, face, "text").and_then(Value::as_str),
                    Some(format!("Animated\n{current_text}").as_str()),
                    "{id} {face} printed Animated"
                );
            }
            // Nothing else on the card moves: restoring the eight Animated fields restores the snapshot.
            assert_eq!(
                restore_face_fields(
                    current_card,
                    prior_card,
                    &["attack", "health", "keywords", "text"]
                ),
                *prior_card,
                "only Animated fields differ on {id}"
            );
        }
        // Another pending fragment's cards legitimately differ beside these eighteen (R646), so the
        // diff is read on this patch's claims.
        let claimed: Vec<String> = changed
            .into_iter()
            .filter(|id| unanimated.contains(id.as_str()))
            .collect();
        assert_eq!(sorted(claimed), sorted(strings(ANIMATED_IN_V0_2_3)));
    }

    #[test]
    fn r366_patch_v0_2_7_takes_craft_a_card_s_radiant_draw_off_and_moves_plague_chalice_s_loc_between_v0_2_6_and_v0_2_7()
     {
        let before = read_snapshot("v0.2.6");
        // Pending until `patches ship` promotes it (R646): the current catalog until then, its snapshot after.
        let (shipped, after) = after_patch("v0.2.7");
        // The current catalog also carries every other pending patch's claims (R646), which are theirs
        // to prove. Those claims live in `patches/pending/`, a directory this test cannot list (a pure
        // crate's tests read no files, SURFACE §3); `cargo jackioh patches check` proves them. v0.2.7
        // has shipped, so its snapshot is read and no other claim is in it.
        assert!(
            shipped,
            "v0.2.7 is pending: its baseline needs the other pending fragments' claims, which `cargo jackioh patches check` reads"
        );
        let others: IndexSet<String> = IndexSet::new();
        let changed: Vec<String> = after
            .iter()
            .filter(|(id, _)| !others.contains(id.as_str()))
            .filter(|(id, card)| before.get(id.as_str()).map(stringify) != Some(stringify(card)))
            .map(|(id, _)| id.clone())
            .collect();
        assert_eq!(sorted(changed), strings(&["classic-087", "core-099"]));
        // Craft a Card: the Radiant face keeps its three Discovers and loses "Draw 1"; the base face is as it was.
        let craft_before = &before["core-099"];
        assert_eq!(
            face_text(&after, "core-099", "radiant"),
            "Discover 3 Units. Fuse them and add the result to your hand. It costs (0)."
        );
        assert_eq!(after["core-099"].get("base"), craft_before.get("base"));
        // Plague Chalice moves only its script's loc (R667's `wouldCounter`).
        let chalice_before = &before["classic-087"];
        let mut restored = after["classic-087"].clone();
        set_or_remove(&mut restored, "loc", chalice_before.get("loc"));
        assert_eq!(restored, *chalice_before);
    }

    #[test]
    fn r366_patch_v0_2_1_no_printed_face_uses_any_word_the_vocabulary_table_retired() {
        let wrong: Vec<String> = swept()
            .iter()
            .filter(|face| {
                failures(face).iter().any(|why| {
                    why.contains("not Bounce")
                        || why.contains("backrow zone")
                        || why.contains("turn trigger")
                        || why.contains("that costs")
                        || why.contains("Start of Game")
                        || why.contains("Once per Turn")
                        || why.contains("Defense Position")
                        || why.contains("Trigger the Cry")
                        || why.contains("Set a hero")
                        || why.contains("End your turn")
                        || why.contains("to your hand")
                })
            })
            .map(described)
            .collect();
        assert_eq!(wrong, Vec::<String>::new());
    }

    #[test]
    fn r366_patch_v0_2_1_the_failure_detector_catches_retired_vocabulary_terms() {
        let dummy_card = entries()[0];
        let check = |text: &str| -> Vec<String> {
            failures(&Face {
                card: dummy_card,
                face: FaceKind::Base,
                text: text.to_string(),
                keywords: Vec::new(),
            })
        };
        let has = |found: Vec<String>, why: &str| found.iter().any(|reason| reason == why);
        let turn = "writes turn trigger as (Start|End) of your turn, not (Start|End) of turn:";
        assert!(has(
            check("Return a target Unit to hand."),
            "says \"Return … to hand\", not Bounce"
        ));
        assert_eq!(check("Bounce a target Unit."), Vec::<String>::new());
        assert!(has(
            check("Destroy a backrow zone."),
            "says backrow zone, not backrow"
        ));
        assert!(has(check("Start of your turn: Draw 1."), turn));
        assert!(has(check("End of your turn: Deal 1 damage."), turn));
        assert!(has(check("At the start of your turn, Draw 1."), turn));
        assert!(has(check("At the end of your turn, Draw 1."), turn));
        assert!(has(
            check("At the start and end of your turn, this attacks."),
            turn
        ));
        assert!(has(
            check("Discover a Spell that costs (1)."),
            "writes \"that costs (N)\", not \"(N) Cost\""
        ));
        assert!(has(
            check("Start of Game: Draw 1."),
            "writes \"Start of Game\", not \"Start of game\""
        ));
        assert!(has(
            check("Once per Turn: Gain 1 mana."),
            "writes \"Once per Turn\", not \"Once per turn\""
        ));
        assert!(has(
            check("Cannot be in Defense Position."),
            "writes \"Cannot be in Defense Position\", not \"Can't be in Defense Position\""
        ));
        assert!(has(
            check("Trigger the Cry of a Unit."),
            "writes \"Trigger the Cry\", not \"Trigger a Cry\""
        ));
        assert!(has(
            check("Set a hero's health to 13."),
            "writes \"Set a hero's health\", not \"Set health\""
        ));
        assert!(has(
            check("Cast on draw: End your turn."),
            "writes \"End your turn\", not \"End the turn\""
        ));
        assert!(has(
            check("End of turn: Return this to your hand."),
            "writes \"to your hand\", not \"to hand\""
        ));
        assert!(!has(
            check("End of turn: Return this to hand."),
            "writes \"to your hand\", not \"to hand\""
        ));
        assert!(!has(
            check("Add a card to your hand."),
            "writes \"to your hand\", not \"to hand\""
        ));
        assert_eq!(
            check("Cry: Add 2 random Units to your hand. They cost (1)."),
            Vec::<String>::new()
        );
    }
}

mod patch_v0_2_9_wording_issue_44 {
    use super::*;

    fn swept() -> Vec<Face> {
        faces_of(&entries())
    }

    #[test]
    fn r740_no_printed_face_says_plague_token_every_counter_a_player_reads_is_a_plague_counter() {
        let wrong: Vec<String> = swept()
            .iter()
            .filter(|face| has_text(&face.text, "Plague Token"))
            .map(described)
            .collect();
        assert_eq!(wrong, Vec::<String>::new());
    }

    #[test]
    fn r741_no_trap_fires_by_activating_every_trap_condition_reads_reveals() {
        let wrong: Vec<String> = swept()
            .iter()
            .filter(|face| {
                [
                    "activate when",
                    "activates when",
                    "this activates",
                    "has activated",
                    "Traps activates",
                ]
                .iter()
                .any(|phrase| has_text(&face.text, phrase))
            })
            .map(described)
            .collect();
        assert_eq!(wrong, Vec::<String>::new());
    }

    #[test]
    fn r751_no_printed_face_names_a_target_it_picks() {
        // Issue #355 swept Core as issue #88 swept Classic and Classic+: "Deal 3 damage.", "Destroy a
        // Unit.", "Steal an enemy permanent." "Heal a target N" stays, as Book of Heal's did, and so does
        // "target" the verb (R394), which no pattern here reads.
        let wrong: Vec<String> = swept()
            .iter()
            .filter(|face| {
                [
                    "to a target",
                    "to any target",
                    "target enemy",
                    "target Unit",
                    "target permanent",
                    "target backrow",
                    "target card",
                ]
                .iter()
                .any(|phrase| has_words(&face.text, phrase, true))
                    || has_text(&face.text, "targets are highlighted")
            })
            .map(described)
            .collect();
        assert_eq!(wrong, Vec::<String>::new());
    }
}

mod r1302_a_face_printed_in_chinese {
    use super::*;

    /// A face of `card` that prints `text`, as `faces_of` builds one.
    fn face(id: &str, text: &str, keywords: Vec<Keyword>) -> Face {
        Face {
            card: &CATALOG[id],
            face: FaceKind::Base,
            text: text.to_string(),
            keywords,
        }
    }

    #[test]
    fn r1302_a_face_printed_in_chinese_is_read_by_chinese_punctuation() {
        // Meditative #32's Radiant face (docs/meditative-set.md, #32), the first printed in Chinese.
        let printed = "随机一项效果：准备好学中文；将你手牌中的每张牌变形为同一张随机的光辉中国牌，它们的法力值消耗为(0)；召唤{units}个随机的光辉中国单位。";
        assert_eq!(failures(&face("core-005", printed, vec![])), Vec::<String>::new());
        // The keyword list leads, joined with "，" and with no full stop; a label opens its line.
        let lead = "嘲讽，护甲7\n亡语：抽一张牌。";
        let keywords = vec![Keyword::Taunt, Keyword::Armor { n: 7 }];
        assert_eq!(
            failures(&face("core-005", lead, keywords.clone())),
            Vec::<String>::new()
        );
        // C+ #7's "Cry and start of turn:": a label after another label's words is still its opening.
        assert_eq!(
            cjk_failures("战吼和回合开始时：召唤一个单位。", &[]),
            Vec::<String>::new()
        );
        for wrong in [
            "抽两张牌.",
            "抽一张牌。战吼：抽一张牌。",
            "法力值消耗为（0）。",
            "抽一张牌",
            "抽一张牌，然后,弃一张牌。",
            "嘲讽，护甲7。\n亡语：抽一张牌。",
        ] {
            assert_ne!(
                failures(&face("core-005", wrong, keywords.clone())),
                Vec::<String>::new(),
                "{wrong}"
            );
        }
        assert!(
            cjk_failures("亡语：抽一张牌。", &keywords)
                .iter()
                .any(|why| why.contains("嘲讽"))
        );
        // English stays English: a face with no Chinese character is read by R366's checks.
        assert!(
            failures(&face("core-005", "Draw a card", vec![]))
                .iter()
                .any(|why| why.contains("full stop"))
        );
    }

    #[test]
    fn r988_an_english_face_that_names_the_element_glyphs_is_read_as_english() {
        // Meditative #40 names the five glyphs but is punctuated in English (R366): no Chinese
        // punctuation, so both printed faces read as English and pass.
        let faces: Vec<Face> = faces_of(&entries())
            .into_iter()
            .filter(|face| face.card.id == "meditative-040")
            .collect();
        assert_eq!(faces.len(), 2);
        for face in &faces {
            assert_eq!(failures(face), Vec::<String>::new(), "{}", face.text);
        }
        // And a punctuated face still reads as Chinese, glyphs or not: an ASCII comma fails the
        // Chinese punctuation, never the English list.
        assert!(
            failures(&face("core-005", "抽一张牌,水。", vec![]))
                .iter()
                .any(|why| why.contains("ASCII"))
        );
    }
}

/// The hand-written matchers against the regular expressions they stand for, on the cases that tell
/// a near miss from a match (not in the TS file, which had the regex engine).
mod the_hand_written_patterns_read_as_their_regular_expressions {
    use super::*;

    #[test]
    fn the_cost_patterns() {
        assert!(digits_dash_cost("a 2-cost card"));
        assert!(!digits_dash_cost("a x2-cost card"));
        assert!(!digits_dash_cost("a 2-costly card"));
        assert!(cost_then_digit("it costs 2 less"));
        assert!(cost_then_digit("costing 3"));
        assert!(cost_then_digit("Cost 4"));
        assert!(!cost_then_digit("costs (2)"));
        assert!(!cost_then_digit("recost 2"));
        assert!(has_word_start("a Cost (1) card", "Cost (", false));
        assert!(bracket_then_lowercase_cost("a (4)+ cost card"));
        assert!(bracket_then_lowercase_cost("a (1) cost card"));
        assert!(!bracket_then_lowercase_cost("a (1) Cost card"));
        assert!(!bracket_then_lowercase_cost("a ( ) cost card"));
        assert!(word_dash_cost("an odd-cost card"));
        assert!(!word_dash_cost("a 3odd-cost card"));
        assert!(paid_then_digit("(Paid 4: draw)"));
        assert!(!paid_then_digit("Paid (4): draw"));
    }

    #[test]
    fn the_return_patterns() {
        assert!(return_to_hand_in_a_sentence("Return it to its owner's hand."));
        assert!(!return_to_hand_in_a_sentence("Return it. Then to hand."));
        assert!(!return_to_hand_in_a_sentence("Returned to hand."));
        assert!(return_to_your_hand("Return this to your hand."));
        assert!(!return_to_your_hand("Returns this to your hand."));
        assert!(returns_then_to("Returns it\nto hand.", &["hand"]));
        assert!(!returns_then_to("Return it. To hand.", &["hand"]));
        assert!(returns_then_to(
            "Return it to their owner's hand.",
            &["their owner's hand", "hand"]
        ));
        assert!(says_bounce_or_bounced("Bounced units"));
        assert!(!says_bounce_or_bounced("Bouncer"));
    }

    #[test]
    fn the_line_patterns() {
        assert!(is_tribute_or_echo_count("Tribute 2"));
        assert!(!is_tribute_or_echo_count("Tribute X"));
        assert!(is_tribute_or_echo_count("Echo X"));
        assert!(ends_with_a_full_stop("He said \"go.\""));
        assert!(ends_with_a_full_stop("He said \u{201c}go.\u{201d}"));
        assert!(!ends_with_a_full_stop("Draw 1"));
        assert!(your_turn_trigger(
            "At the start and end of your turn, this attacks."
        ));
        assert!(!your_turn_trigger("Start of turn: draw."));
    }
}

/// R1320 (patch v0.3.4, issue #543): players read Nerf for the engine's Degrade and Buff for its
/// Upgrade (`degrade`, `upgrade`, `tune_once` and the events `degraded` and `upgraded` keep their
/// names, as R373 kept `library`). Every entry of every set is read, the sets that have not shipped
/// included, so a Meditative card that lands later is held to it too: it says Nerf and Buff.
mod r1320_players_read_nerf_and_buff_issue_543 {
    use super::*;

    /// `/degrad|upgrad/i` anywhere: the old words in any form ("Degrade", "upgraded", "Upgrades").
    fn says_degrade_or_upgrade(text: &str) -> bool {
        has_text(text, "degrad") || has_text(text, "upgrad")
    }

    /// The faces patch v0.3.4 renamed, each with the words it prints now.
    const RENAMED: &[(&str, FaceKind, &[&str])] = &[
        ("classicplus-008", FaceKind::Base, &["Nerf"]),
        ("classicplus-008", FaceKind::Radiant, &["Nerf"]),
        ("classicplus-069", FaceKind::Base, &["Buff"]),
        ("classicplus-069", FaceKind::Radiant, &["Buff"]),
        ("classicplus-070", FaceKind::Base, &["Buff", "Nerf"]),
        ("classicplus-070", FaceKind::Radiant, &["Buff", "Nerf"]),
        ("classicplus-071", FaceKind::Base, &["Buff"]),
        ("classicplus-071", FaceKind::Radiant, &["Buff"]),
        ("classicplus-072", FaceKind::Base, &["Nerf"]),
        ("classicplus-072", FaceKind::Radiant, &["Nerf"]),
        ("classicplus-073", FaceKind::Base, &["Buff", "Nerf"]),
        ("classicplus-073", FaceKind::Radiant, &["Buff", "Nerf"]),
        ("classicplus-t-ai-10", FaceKind::Base, &["Buff"]),
        ("classicplus-t-ai-10", FaceKind::Radiant, &["Buff"]),
    ];

    /// Core #98's Steady Shot raises its own damage (R754), which is no Buff: its Radiant face says so
    /// in words that are no keyword's (R1320).
    #[test]
    fn r1320_steady_shot_says_its_raise_in_plain_words() {
        let card = CATALOG.get("core-098").expect("the catalog has Heroic Power");
        let text = fill_params(card, FaceKind::Radiant, None);
        assert!(text.contains("This permanently deals 2 more damage."), "{text}");
        assert!(!has_words(&text, "Buff", false), "{text}");
    }

    #[test]
    fn r1320_no_face_and_no_name_of_any_entry_says_degrade_or_upgrade() {
        let wrong: Vec<String> = faces_of(&entries())
            .iter()
            .filter(|face| says_degrade_or_upgrade(&face.text))
            .map(described)
            .collect();
        assert_eq!(wrong, Vec::<String>::new());
        let named: Vec<String> = entries()
            .iter()
            .filter(|card| says_degrade_or_upgrade(&card.name))
            .map(|card| format!("{} {}", card.id, card.name))
            .collect();
        assert_eq!(named, Vec::<String>::new());
    }

    #[test]
    fn r1320_the_renamed_faces_print_nerf_and_buff_as_whole_words() {
        for (id, face, words) in RENAMED {
            let card = CATALOG
                .get(*id)
                .unwrap_or_else(|| panic!("the catalog has no {id}"));
            let text = fill_params(card, *face, None);
            for word in *words {
                assert!(has_words(&text, word, false), "{id} {face}: {word} in {text}");
            }
        }
    }

    #[test]
    fn r1320_the_guard_flags_the_old_words_in_any_form_and_passes_the_new_ones() {
        assert!(says_degrade_or_upgrade("Degrade a permanent 5 times."));
        assert!(says_degrade_or_upgrade("Cry: Upgrade this X times."));
        assert!(says_degrade_or_upgrade("It was upgraded."));
        assert!(!says_degrade_or_upgrade("Nerf a permanent 5 times."));
        assert!(!says_degrade_or_upgrade("Cry: Buff this X times."));
        // Core #93 Combo-Index's grades are no tuning words.
        assert!(!says_degrade_or_upgrade("go up a grade and trigger every step"));
    }
}
