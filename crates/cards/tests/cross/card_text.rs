//! R366: words and shape of printed card text (issue #27, #40, #1175, R432, R1175000, B3.4 rule 5;
//! CLAUDE.md rule 7). Hand-written matchers over catalog text (pure crate, SURFACE §3).

use indexmap::{IndexMap, IndexSet};
use jackioh_cards::{CATALOG, catalog_json, chinese_json, chinese_terms_json};
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

/// The labels that start a line of their own (R366; Activate is B3.2's, Quest Classic #90's). An
/// prints (R1175000). The Stinkyscape set prints five more: "Embiggened (5):" (S #79 Orca) and
/// "Activate 3:" to "Activate 6:" (R1857003).
pub(super) const LABELS: &[&str] = &[
    "Cry:",
    "Death:",
    "Start of turn:",
    "End of turn:",
    "Aura:",
    "Embiggened (1):",
    "Embiggened (2):",
    "Embiggened (3):",
    "Embiggened (4):",
    "Embiggened (5):",
    "Activate:",
    "Activate 2:",
    "Activate 3:",
    "Activate 4:",
    "Activate 5:",
    "Activate 6:",
    "Activate ♾️:",
    "Cast on draw:",
    "Quest:",
];

/// R746 (issue #354): Bounce is a permanent's return from the field (R692). These faces return a card
/// graveyard or exile) and say "Return … to hand", as they did before patch v0.2.10. Core #37
/// Gravedigger's base face joined them in #911: it said "Add a random card from your graveyard to
const RETURNS_OFF_THE_FIELD: &[&str] = &[
    "core-023 base",
    "core-023 radiant",
    "core-024 base",
    "core-024 radiant",
    "core-031 base",
    "core-031 radiant",
    "core-037 base",
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
    // R2195003: Core #23's snapshot, TG #49.
    "traininggrounds-049 base",
    "traininggrounds-049 radiant",
];

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn boundary(text: &[u8], at: usize) -> bool {
    let before = at > 0 && text.get(at - 1).is_some_and(|b| is_word(*b));
    let after = text.get(at).is_some_and(|b| is_word(*b));
    before != after
}

fn folded(text: &str, insensitive: bool) -> String {
    if insensitive {
        text.to_ascii_lowercase()
    } else {
        text.to_string()
    }
}

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

fn has_words(text: &str, phrase: &str, insensitive: bool) -> bool {
    let hay = folded(text, insensitive);
    let needle = folded(phrase, insensitive);
    let bytes = hay.as_bytes();
    every_start(&hay, &needle)
        .into_iter()
        .any(|at| boundary(bytes, at) && boundary(bytes, at + needle.len()))
}

fn has_word_start(text: &str, phrase: &str, insensitive: bool) -> bool {
    let hay = folded(text, insensitive);
    let needle = folded(phrase, insensitive);
    let bytes = hay.as_bytes();
    every_start(&hay, &needle)
        .into_iter()
        .any(|at| boundary(bytes, at))
}

fn has_text(text: &str, phrase: &str) -> bool {
    text.to_ascii_lowercase().contains(&phrase.to_ascii_lowercase())
}

fn says_library(text: &str) -> bool {
    has_text(text, "library") || has_text(text, "libraries")
}

fn says_sacrifice(text: &str) -> bool {
    has_text(text, "sacrific")
}

fn run_start(bytes: &[u8], end: usize, class: impl Fn(u8) -> bool) -> usize {
    let mut start = end;
    while start > 0 && class(bytes[start - 1]) {
        start -= 1;
    }
    start
}

fn digits_dash_cost(text: &str) -> bool {
    let hay = text.to_ascii_lowercase();
    let bytes = hay.as_bytes();
    every_start(&hay, "-cost").into_iter().any(|at| {
        let start = run_start(bytes, at, |b| b.is_ascii_digit());
        start < at && boundary(bytes, start) && boundary(bytes, at + "-cost".len())
    })
}

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

fn bracket_then_lowercase_cost(text: &str) -> bool {
    let bytes = text.as_bytes();
    for (open, _) in text.match_indices('(') {
        let mut at = open + 1;
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

fn word_dash_cost(text: &str) -> bool {
    let bytes = text.as_bytes();
    every_start(text, "-cost").into_iter().any(|at| {
        let start = run_start(bytes, at, |b| b.is_ascii_alphabetic());
        start < at && boundary(bytes, start) && boundary(bytes, at + "-cost".len())
    })
}

fn paid_then_digit(text: &str) -> bool {
    let hay = text.to_ascii_lowercase();
    every_start(&hay, "(paid ").into_iter().any(|at| {
        hay[at + "(paid ".len()..]
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_digit())
    })
}

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

fn segment_end(bytes: &[u8], from: usize, stop: impl Fn(u8) -> bool) -> usize {
    (from..bytes.len())
        .find(|at| stop(bytes[*at]))
        .unwrap_or(bytes.len())
}

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

fn ends_with_a_full_stop(line: &str) -> bool {
    line.ends_with('.') || line.ends_with(".\"") || line.ends_with(".\u{201d}")
}

/// R1445: whether the text before a label ends on an opening double quote (`“`, or a `"` that opens
fn opens_a_quote(before: &str) -> bool {
    before.ends_with('\u{201c}') || (before.ends_with('"') && before.matches('"').count() % 2 == 1)
}

/// Every R366 check a face fails, by name; empty when it passes them all. A face printed in Chinese
/// is read by Chinese punctuation instead (R1302).
fn failures(face: &Face) -> Vec<String> {
    let text = face.text.as_str();
    let keywords = &face.keywords;
    // R988: a face printed in English that names the element glyphs (Meditative #40) is R366's
    // English shape. A face is printed in Chinese when it is punctuated in Chinese — R1302 ends
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
        out.push(
            "writes an embiggen card's Embiggened version as \"(paid N\", not \"Embiggened (N):\""
                .to_string(),
        );
    }
    // R1175000: "Paid (N):" was R366's label for the Embiggened version until issue #1175.
    if has_word_start(text, "Paid (", false) {
        out.push(
            "writes an embiggen card's Embiggened version as \"Paid (N):\", not \"Embiggened (N):\""
                .to_string(),
        );
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
    if return_to_your_hand(text) {
        out.push("writes \"to your hand\", not \"to hand\"".to_string());
    }
    let lines: Vec<&str> = if text.is_empty() {
        Vec::new()
    } else {
        text.split('\n').collect()
    };
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
            // R1445: a label that opens a double-quoted ability the card grants stays in its sentence.
            if line
                .match_indices(label)
                .any(|(index, _)| index > 0 && !opens_a_quote(&line[..index]))
            {
                out.push(format!("does not start \"{label}\" on a line of its own"));
            }
        }
    }
    out
}

// R1302 (MD-B13, ME-CN): a face printed in Chinese (Meditative #32's Radiant face, and every face of
// the Chinese table, `chinese.rs`) is R366's shape in Chinese punctuation: the keyword list is joined

/// R988: a character that punctuates a Chinese face: CJK punctuation (U+3000–U+303F) and the
pub(super) fn is_cjk_punctuation(c: char) -> bool {
    matches!(c, '\u{3000}'..='\u{303f}' | '\u{ff00}'..='\u{ffef}')
}

/// `chinese-terms.json` (R1303), the frame's words in Chinese.
pub(super) fn chinese_terms() -> Value {
    serde_json::from_str(chinese_terms_json()).expect("crates/cards/chinese-terms.json is JSON")
}

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

/// `is_tribute_or_echo_count` (a computed Echo X, R802, prints "回响X").
fn is_chinese_tribute_or_echo_count(item: &str) -> bool {
    if let Some(n) = item.strip_prefix("回响") {
        return !n.is_empty() && (n == "X" || n.bytes().all(|b| b.is_ascii_digit()));
    }
    item.strip_prefix("献祭")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

fn ends_with_a_chinese_full_stop(line: &str) -> bool {
    line.ends_with('。') || line.ends_with("。”") || line.ends_with("。」")
}

/// Every R1302 check a Chinese face fails, by name; empty when it passes them all. `keywords` are the
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

// The patch history (B4.2, R388, R646), as `scripts/patches-io.ts` read it: `crates/cards/patches/`

type Entry = IndexMap<String, Value>;

type Catalog = IndexMap<String, Entry>;

const SNAPSHOTS: &[(&str, &str)] = &[
    ("v0.1.0", include_str!("../../patches/v0.1.0.json")),
    ("v0.1.0b", include_str!("../../patches/v0.1.0b.json")),
    ("v0.1.0c", include_str!("../../patches/v0.1.0c.json")),
    ("v0.1.0d", include_str!("../../patches/v0.1.0d.json")),
    ("v0.1.1", include_str!("../../patches/v0.1.1.json")),
    ("v0.2.0", include_str!("../../patches/v0.2.0.json")),
    ("v0.2.0b", include_str!("../../patches/v0.2.0b.json")),
    ("v0.2.1", include_str!("../../patches/v0.2.1.json")),
    ("v0.2.2", include_str!("../../patches/v0.2.2.json")),
    ("v0.2.2b", include_str!("../../patches/v0.2.2b.json")),
    ("v0.2.3", include_str!("../../patches/v0.2.3.json")),
    ("v0.2.3b", include_str!("../../patches/v0.2.3b.json")),
    ("v0.2.3c", include_str!("../../patches/v0.2.3c.json")),
    ("v0.2.3d", include_str!("../../patches/v0.2.3d.json")),
    ("v0.2.3e", include_str!("../../patches/v0.2.3e.json")),
    ("v0.2.3f", include_str!("../../patches/v0.2.3f.json")),
    ("v0.2.4", include_str!("../../patches/v0.2.4.json")),
    ("v0.2.5", include_str!("../../patches/v0.2.5.json")),
    ("v0.2.5b", include_str!("../../patches/v0.2.5b.json")),
    ("v0.2.5c", include_str!("../../patches/v0.2.5c.json")),
    ("v0.2.5d", include_str!("../../patches/v0.2.5d.json")),
    ("v0.2.5e", include_str!("../../patches/v0.2.5e.json")),
];

const PATCHES_JSON: &str = include_str!("../../patches/patches.json");

fn read_snapshot(version: &str) -> Catalog {
    match SNAPSHOTS.iter().find(|(shipped, _)| *shipped == version) {
        Some((_, raw)) => serde_json::from_str(raw)
            .unwrap_or_else(|error| panic!("patches/{version}.json is not a catalog: {error}")),
        None => panic!("patches/{version}.json is not compiled into card_text.rs's SNAPSHOTS"),
    }
}

fn shipped_versions() -> Vec<String> {
    let patches: Vec<Value> = serde_json::from_str(PATCHES_JSON).expect("patches.json is a list of patches");
    patches
        .iter()
        .filter_map(|patch| patch.get("version").and_then(Value::as_str).map(str::to_string))
        .collect()
}

fn current_catalog() -> Catalog {
    serde_json::from_str(catalog_json()).expect("catalog.json is a catalog")
}

fn stringify<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("a catalog entry serializes")
}

fn face_of<'a>(entry: &'a Entry, face: &str) -> &'a Map<String, Value> {
    match entry.get(face).and_then(Value::as_object) {
        Some(object) => object,
        None => panic!("an entry's {face} face is an object"),
    }
}

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

fn without_text(card: &Entry) -> Entry {
    let mut out = card.clone();
    for face in ["base", "radiant"] {
        let mut printed = face_of(card, face).clone();
        printed.insert("text".to_string(), Value::from(""));
        out.insert(face.to_string(), Value::Object(printed));
    }
    out
}

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

fn face_field<'a>(entry: &'a Entry, face: &str, field: &str) -> Option<&'a Value> {
    face_of(entry, face).get(field)
}

fn face_text(catalog: &Catalog, id: &str, face: &str) -> String {
    catalog
        .get(id)
        .and_then(|entry| face_field(entry, face, "text"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

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
    fn r432_writes_a_specific_cost_as_the_noun_n_cost_a_price_as_the_verb_costs_n_and_an_embiggened_version_as_embiggened_n()
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
    fn r366_leads_with_the_face_s_printed_keywords_on_a_line_of_their_own_starts_each_labeled_ability_on_its_own_line_and_ends_every_other_line_with_a_full_stop()
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
    fn r1445_a_label_may_open_a_double_quoted_granted_ability_inside_a_line_and_only_the_granting_faces_quote_one()
     {
        let dummy_card = entries()[0];
        let check = |text: &str| -> Vec<String> {
            failures(&Face {
                card: dummy_card,
                face: FaceKind::Base,
                text: text.to_string(),
                keywords: Vec::new(),
            })
        };
        let inside = "does not start \"Death:\" on a line of its own";
        assert_eq!(
            check("Cry: Summon a random (1) Cost Unit. It gains \"Death: Add 1 random Book to your hand.\""),
            Vec::<String>::new()
        );
        assert_eq!(
            check("Cry: Summon a Unit. It gains \u{201c}Death: Draw 1.\u{201d}"),
            Vec::<String>::new()
        );
        assert!(
            check("Cry: Summon a Unit. Death: Draw 1.")
                .iter()
                .any(|why| why == inside)
        );
        assert!(
            check("Cry: Say \"hi.\"Death: Draw 1.")
                .iter()
                .any(|why| why == inside)
        );
        assert!(
            check("Cry: Summon a Unit. It gains 'Death: Draw 1.'")
                .iter()
                .any(|why| why == inside)
        );
        // The faces that quote a label are the two that grant a Death (R1107), Meditative+ #9's
        // Embiggened Cry (#2179) and Stinkyscape #26 ZolarKeth's granted Activate (#2657), on both faces.
        let quoting: Vec<String> = swept()
            .iter()
            .filter(|face| {
                face.text.split('\n').any(|line| {
                    LABELS.iter().any(|label| {
                        line.match_indices(label)
                            .any(|(index, _)| index > 0 && opens_a_quote(&line[..index]))
                    })
                })
            })
            .map(|face| format!("{} {}", face.card.id, face.face))
            .collect();
        assert_eq!(
            sorted(quoting),
            strings(&[
                "meditative-058 base",
                "meditative-058 radiant",
                "meditative-059 base",
                "meditative-059 radiant",
                "meditativeplus-009 base",
                "meditativeplus-009 radiant",
                "stinkyscape-026 base",
                "stinkyscape-026 radiant",
            ])
        );
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
        let after = read_snapshot("v0.2.0b");
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
        let before = read_snapshot("v0.2.0b");
        let after = read_snapshot("v0.2.1");
        let mut non_text_changed: IndexMap<String, Vec<String>> = IndexMap::new();
        let mut differing_cards: Vec<String> = Vec::new();
        for (id, current_card) in &after {
            let prior_card = before.get(id);
            assert!(prior_card.is_some(), "card {id} existed in v0.2.0b");
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
        let before = read_snapshot("v0.2.1");
        let after = read_snapshot("v0.2.2");
        let animated: IndexSet<&str> = ANIMATED_IN_V0_2_3.iter().copied().collect();
        let mut changed: Vec<String> = Vec::new();
        for (id, current_card) in &after {
            let prior_card = before.get(id);
            assert!(prior_card.is_some(), "card {id} existed in v0.2.1");
            let Some(prior_card) = prior_card else { continue };
            if stringify(prior_card) != stringify(current_card) {
                changed.push(id.clone());
            }

            if animated.contains(id.as_str()) {
                for face in ["base", "radiant"] {
                    assert!(
                        face_field(prior_card, face, "attack").is_none(),
                        "{id} {face} had no stats before v0.2.2"
                    );
                    assert!(
                        face_field(prior_card, face, "health").is_none(),
                        "{id} {face} had no stats before v0.2.2"
                    );
                    assert_eq!(
                        face_field(prior_card, face, "keywords"),
                        Some(&json!([])),
                        "{id} {face} had no keywords before v0.2.2"
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
                let mut restored = restore_face_fields(current_card, prior_card, &["text"]);
                set_or_remove(&mut restored, "loc", prior_card.get("loc"));
                assert_eq!(restored, *prior_card, "only text and loc differ on {id}");
            } else if id == "classic-052" {
                // Final Gambit's follow-up gained its R216 guard: only the script's loc moves.
                let mut restored = current_card.clone();
                set_or_remove(&mut restored, "loc", prior_card.get("loc"));
                assert_eq!(restored, *prior_card, "only loc differs on {id}");
            } else {
                assert_eq!(current_card, prior_card, "card {id} unchanged by v0.2.2");
            }
        }
        let mut expected = strings(ANIMATED_IN_V0_2_3);
        expected.extend(strings(&["classicplus-033", "classic-052"]));
        assert_eq!(sorted(changed), sorted(expected));
    }

    #[test]
    fn r366_patch_v0_2_4_aims_solarius_prime_and_appropriations_keywords_deft_duelist_and_moves_two_locs_between_v0_2_3_and_v0_2_4()
     {
        let before = read_snapshot("v0.2.2");
        // Pending until `patches ship` promotes it (R646): the current catalog until then, its snapshot after.
        let (_, after) = after_patch("v0.2.2b");
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
            assert!(prior_card.is_some(), "card {id} existed in v0.2.2");
            let Some(prior_card) = prior_card else { continue };
            if stringify(prior_card) != stringify(current_card) {
                changed.push(id.clone());
            }
        }
        // Another pending fragment's cards legitimately differ beside these five (R646), so the diff is
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
        let versions = shipped_versions();
        let at = versions.iter().position(|version| version == "v0.2.5");
        let baseline = match at {
            None => versions.last().cloned().expect("a shipped patch"),
            Some(at) => versions[at - 1].clone(),
        };
        let before = read_snapshot(&baseline);
        let after = if at.is_none() {
            current_catalog()
        } else {
            read_snapshot("v0.2.5")
        };
        // The top-level fields each balance card may move (balance patch 1, issue #88); every other
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
            assert!(prior_card.is_some(), "card {id} is not new in v0.2.5");
            let Some(prior_card) = prior_card else { continue };
            if stringify(prior_card) != stringify(current_card) {
                changed.push(id.clone());
            }
            let Some(fields) = allowed.get(id.as_str()) else {
                assert_eq!(current_card, prior_card, "card {id} unchanged by v0.2.5");
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
        let before = read_snapshot("v0.2.2");
        // Pending until `patches ship` promotes it (R646): the current catalog until then, its snapshot after.
        let (_, after) = after_patch("v0.2.3");
        let unanimated: IndexSet<&str> = ANIMATED_IN_V0_2_3.iter().copied().collect();
        let mut changed: Vec<String> = Vec::new();
        for (id, current_card) in &after {
            let prior_card = before.get(id);
            assert!(prior_card.is_some(), "card {id} existed in v0.2.2");
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
        let claimed: Vec<String> = changed
            .into_iter()
            .filter(|id| unanimated.contains(id.as_str()))
            .collect();
        assert_eq!(sorted(claimed), sorted(strings(ANIMATED_IN_V0_2_3)));
    }

    #[test]
    fn r366_patch_v0_2_7_takes_craft_a_card_s_radiant_draw_off_and_moves_plague_chalice_s_loc_between_v0_2_6_and_v0_2_7()
     {
        let before = read_snapshot("v0.2.3b");
        // Pending until `patches ship` promotes it (R646): the current catalog until then, its snapshot after.
        let (shipped, after) = after_patch("v0.2.3c");
        // The current catalog also carries every other pending patch's claims (R646), which are theirs
        // crate's tests read no files, SURFACE §3); `cargo jackioh patches check` proves them. v0.2.7
        assert!(
            shipped,
            "v0.2.3c is pending: its baseline needs the other pending fragments' claims, which `cargo jackioh patches check` reads"
        );
        let others: IndexSet<String> = IndexSet::new();
        let changed: Vec<String> = after
            .iter()
            .filter(|(id, _)| !others.contains(id.as_str()))
            .filter(|(id, card)| before.get(id.as_str()).map(stringify) != Some(stringify(card)))
            .map(|(id, _)| id.clone())
            .collect();
        assert_eq!(sorted(changed), strings(&["classic-087", "core-099"]));
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

// R1175000 (issue #1175): a card with two prices is an embiggen card. Played at the cheaper price it is
// text labels that version "Embiggened (N): …" where R366 wrote "Paid (N): …".

mod r1175000_an_embiggen_card_labels_its_embiggened_version_issue_1175 {
    use super::*;
    use jackioh_engine::CardCost;

    fn embiggened_label(card: &CardDef) -> Option<String> {
        match card.cost {
            CardCost::Embiggen { embiggen, .. } => Some(format!("Embiggened ({embiggen}):")),
            _ => None,
        }
    }

    /// `chinese.json`'s text of one face (R1303).
    fn chinese_face(table: &Value, id: &str, face: FaceKind) -> String {
        table[id][face.as_str()].as_str().unwrap_or_default().to_string()
    }

    #[test]
    fn r1175000_every_embiggen_card_starts_a_line_with_embiggened_n_its_embiggened_cost_on_both_faces_in_english_and_chinese()
     {
        let terms = chinese_terms();
        let table: Value = serde_json::from_str(chinese_json()).expect("crates/cards/chinese.json is JSON");
        let mut wrong: Vec<String> = Vec::new();
        let mut cards: IndexSet<&str> = IndexSet::new();
        for face in faces_of(&entries()) {
            let Some(label) = embiggened_label(face.card) else {
                continue;
            };
            cards.insert(face.card.id.as_str());
            if !face
                .text
                .split('\n')
                .any(|line| line.starts_with(&format!("{label} ")))
            {
                wrong.push(format!(
                    "does not start a line with {label}: {}",
                    described(&face)
                ));
            }
            if !LABELS.contains(&label.as_str()) {
                wrong.push(format!("LABELS has no {label}"));
            }
            let chinese_label = terms["labels"][label.as_str()].as_str().unwrap_or_default();
            let chinese = chinese_face(&table, &face.card.id, face.face);
            if chinese_label.is_empty() || !chinese.split('\n').any(|line| line.starts_with(chinese_label)) {
                wrong.push(format!(
                    "{} {}: the Chinese face does not start a line with {label}'s \"{chinese_label}\": {chinese}",
                    face.card.id, face.face
                ));
            }
        }
        assert_eq!(wrong, Vec::<String>::new());
        // The eight embiggen cards of issue #1175, Meditative+ #3, #4, #5, #19 and #23 (#1179, #1181),
        // and Core #83, Classic #35 and Meditative+ #9 (#2179). Every face above is held to the label, a
        // held set's too; the list pinned is the shipped sets' (R1857000: the Stinkyscape set's #79,
        // #127, #128, #129 and #154 joined at its release, #1884).
        let shipped: Vec<String> = cards
            .into_iter()
            .filter(|id| {
                CATALOG
                    .get(*id)
                    .is_some_and(|card| jackioh_engine::set_ships(card.set))
            })
            .map(str::to_string)
            .collect();
        assert_eq!(
            sorted(shipped),
            strings(&[
                "classic-035",
                "classicplus-036",
                "core-046",
                "core-056",
                "core-059",
                "core-083",
                "core-084",
                "meditative-021",
                "meditative-046",
                "meditative-081",
                "meditativeplus-003",
                "meditativeplus-004",
                "meditativeplus-005",
                "meditativeplus-009",
                "meditativeplus-019",
                "meditativeplus-023",
                "stinkyscape-079",
                "stinkyscape-127",
                "stinkyscape-128",
                "stinkyscape-129",
                "stinkyscape-154",
            ])
        );
    }

    #[test]
    fn r1175000_only_an_embiggen_card_says_embiggened_and_no_face_says_paid_n_in_english_or_chinese() {
        let table: Value = serde_json::from_str(chinese_json()).expect("crates/cards/chinese.json is JSON");
        let wrong: Vec<String> = faces_of(&entries())
            .iter()
            .filter(|face| {
                face.text.contains("Embiggened (") != embiggened_label(face.card).is_some()
                    || face.text.contains("Paid (")
                    || chinese_face(&table, &face.card.id, face.face).contains("已支付")
            })
            .map(described)
            .collect();
        assert_eq!(wrong, Vec::<String>::new());
        let dummy_card = entries()[0];
        let check = |text: &str| -> Vec<String> {
            failures(&Face {
                card: dummy_card,
                face: FaceKind::Base,
                text: text.to_string(),
                keywords: Vec::new(),
            })
        };
        assert_eq!(
            check("Aura: Draw 1.\nPaid (4): Draw 2 instead."),
            strings(&[
                "writes an embiggen card's Embiggened version as \"Paid (N):\", not \"Embiggened (N):\""
            ])
        );
        assert_eq!(
            check("Aura: Draw 1.\nEmbiggened (4): Draw 2 instead."),
            Vec::<String>::new()
        );
        assert_eq!(
            check("Aura: Draw 1. Embiggened (4): Draw 2 instead."),
            strings(&["does not start \"Embiggened (4):\" on a line of its own"])
        );
    }
}

/// R1857003: the Stinkyscape set prints five labels no other set's card does, "Activate 3:" to
/// "Activate 6:" (an Activate of that many uses a turn, R1751000) and S #79 Orca's "Embiggened (5):".
/// Each starts a line as R366's labels do, in English and in Chinese.
mod r1857003_five_labels_the_stinkyscape_set_prints {
    use super::*;

    const STINKYSCAPE_LABELS: &[(&str, &str)] = &[
        ("Activate 3:", "激活3："),
        ("Activate 4:", "激活4："),
        ("Activate 5:", "激活5："),
        ("Activate 6:", "激活6："),
        ("Embiggened (5):", "加码(5)："),
    ];

    #[test]
    fn r1857003_activate_3_to_6_and_embiggened_5_are_labels_in_both_languages() {
        let terms = chinese_terms();
        let dummy_card = entries()[0];
        let check = |text: &str| -> Vec<String> {
            failures(&Face {
                card: dummy_card,
                face: FaceKind::Base,
                text: text.to_string(),
                keywords: Vec::new(),
            })
        };
        for (label, chinese) in STINKYSCAPE_LABELS {
            assert!(LABELS.contains(label), "LABELS has no {label}");
            assert_eq!(terms["labels"][*label].as_str(), Some(*chinese), "{label}");
            assert_eq!(
                check(&format!("Aura: Draw 1.\n{label} Draw 2.")),
                Vec::<String>::new()
            );
            assert_eq!(
                check(&format!("Aura: Draw 1. {label} Draw 2.")),
                vec![format!("does not start \"{label}\" on a line of its own")],
                "{label}"
            );
            assert_eq!(
                cjk_failures(&format!("光环：抽一张牌。\n{chinese}抽两张牌。"), &[]),
                Vec::<String>::new()
            );
            assert_eq!(
                cjk_failures(&format!("光环：抽一张牌。{chinese}抽两张牌。"), &[]),
                vec![format!("does not start \"{chinese}\" on a line of its own")],
                "{chinese}"
            );
        }
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
        let lead = "嘲讽，护甲7\n亡语：抽一张牌。";
        let keywords = vec![Keyword::Taunt, Keyword::Armor { n: 7 }];
        assert_eq!(
            failures(&face("core-005", lead, keywords.clone())),
            Vec::<String>::new()
        );
        assert_eq!(
            cjk_failures("战吼和回合开始时：召唤一个单位。", &[]),
            Vec::<String>::new()
        );
        // R802's computed Echo X leads its list as an Echo count does (Meditative #5).
        assert_eq!(
            cjk_failures("回响X，吸血\n造成1点伤害。", &[Keyword::Lifesteal]),
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
        let faces: Vec<Face> = faces_of(&entries())
            .into_iter()
            .filter(|face| face.card.id == "meditative-040")
            .collect();
        assert_eq!(faces.len(), 2);
        for face in &faces {
            assert_eq!(failures(face), Vec::<String>::new(), "{}", face.text);
        }
        assert!(
            failures(&face("core-005", "抽一张牌,水。", vec![]))
                .iter()
                .any(|why| why.contains("ASCII"))
        );
    }
}

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
        assert!(!paid_then_digit("Embiggened (4): draw"));
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
/// names, as R373 kept `library`). Every entry of every set is read, the sets that have not shipped
mod r1320_players_read_nerf_and_buff_issue_543 {
    use super::*;

    fn says_degrade_or_upgrade(text: &str) -> bool {
        has_text(text, "degrad") || has_text(text, "upgrad")
    }

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
