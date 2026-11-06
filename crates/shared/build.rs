//! The face model's data, compiled in (CLAUDE.md rule 4: data reaches a pure crate only at compile
//! time; `crates/shared/README.md`, host contract 6). It reads the JSON beside this file and the cards'
//! Chinese tables and writes `$OUT_DIR/face_data.rs`: immutable sorted tables the face model looks
//! things up in by binary search, so a call parses no JSON and the crate keeps no static of its own
//! state or cache.
//!
//! - `data/glossary.json`: the keyword glossary, in the order its tokenizer sorts (`glossary.rs`);
//! - `data/power-words.json`: the thirteen powers of #98 Heroic Power (`power.rs`);
//! - `data/short-forms.json`: the faces that print a short form (`short.rs`);
//! - `data/chaos-names.json`: the effects of the three Call to Chaos cards, the clause the engine names to
//!   the words a reel and the log show (`log/chaos.rs`; the web's `fx/chaos.ts` reads the same file);
//! - `data/zh-CN.json`: the face, glossary and log strings in Simplified Chinese, English sentence to words
//!   (`words.rs`), the keys with `{name}` tokens listed again in matching order;
//! - `../cards/chinese.json` and `../cards/chinese-terms.json`: the Chinese card and frame tables
//!   (ME-CN, R1300 to R1303), which `crates/cards` writes from `card-text.yaml` (`chinese.rs`).
//!
//! The crate's `clippy.toml` bans file and environment reads; a build script runs at compile time,
//! which is where data may reach a pure crate, as `crates/cards/build.rs` does.
#![allow(clippy::disallowed_methods)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn read(path: &Path) -> Value {
    println!("cargo:rerun-if-changed={}", path.display());
    let text = std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{} is not JSON: {error}", path.display()))
}

fn text(value: &Value) -> &str {
    value.as_str().expect("a string")
}

/// A Rust string literal for `value`.
fn lit(value: &str) -> String {
    format!("{value:?}")
}

fn object(value: &Value) -> &serde_json::Map<String, Value> {
    value.as_object().expect("an object")
}

fn list(value: &Value) -> &Vec<Value> {
    value.as_array().expect("an array")
}

fn strings(value: &Value) -> String {
    let items: Vec<String> = list(value).iter().map(|item| lit(text(item))).collect();
    format!("&[{}]", items.join(", "))
}

/// `(key, value)` rows of an object of strings, sorted by key (byte order, as `[T]::binary_search_by` reads them).
fn pairs(value: &Value) -> String {
    let mut rows: Vec<(&String, &Value)> = object(value).iter().collect();
    rows.sort_by(|a, b| a.0.cmp(b.0));
    let rows: Vec<String> = rows
        .iter()
        .map(|(key, word)| format!("({}, {})", lit(key), lit(text(word))))
        .collect();
    format!("&[{}]", rows.join(",\n    "))
}

fn main() {
    let here = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let data = here.join("data");
    let cards = here.join("../cards");
    let mut out = String::new();

    // The glossary.
    let glossary = read(&data.join("glossary.json"));
    let mut rows = Vec::new();
    for entry in list(&glossary["entries"]) {
        let row = match entry.get("row") {
            Some(row) => format!("Some({})", lit(text(row))),
            None => "None".to_string(),
        };
        rows.push(format!(
            "GlossaryRow {{ id: {}, label: {}, rule: {}, section: {}, row: {}, aliases: {} }}",
            lit(text(&entry["id"])),
            lit(text(&entry["label"])),
            lit(text(&entry["rule"])),
            lit(text(&entry["section"])),
            row,
            strings(&entry["aliases"]),
        ));
    }
    writeln!(
        out,
        "pub static GLOSSARY: &[GlossaryRow] = &[\n    {}\n];",
        rows.join(",\n    ")
    )
    .unwrap();
    writeln!(
        out,
        "pub static PLAIN_SPELLINGS: &[&str] = {};",
        strings(&glossary["plain"])
    )
    .unwrap();
    // The spellings the tokenizer matches, the longest first (`glossary.rs`): every term's label and
    // aliases, in the glossary's order, then the plain spellings, which match like terms but print plain
    // (R910000). The sort is stable, so equal lengths keep that order.
    let mut patterns: Vec<(&str, Option<&str>)> = Vec::new();
    for entry in list(&glossary["entries"]) {
        patterns.push((text(&entry["label"]), Some(text(&entry["id"]))));
        for alias in list(&entry["aliases"]) {
            patterns.push((text(alias), Some(text(&entry["id"]))));
        }
    }
    for spelling in list(&glossary["plain"]) {
        patterns.push((text(spelling), None));
    }
    patterns.sort_by_key(|(spelling, _)| std::cmp::Reverse(spelling.chars().count()));
    let rows: Vec<String> = patterns
        .iter()
        .map(|(spelling, term)| {
            format!(
                "({}, {})",
                lit(spelling),
                term.map_or("None".to_string(), |id| format!("Some({})", lit(id)))
            )
        })
        .collect();
    writeln!(
        out,
        "pub static PATTERNS: &[(&str, Option<&str>)] = &[\n    {}\n];",
        rows.join(",\n    ")
    )
    .unwrap();

    // The powers of #98.
    let powers = read(&data.join("power-words.json"));
    let mut rows = Vec::new();
    for (name, entry) in object(&powers) {
        rows.push(format!(
            "PowerRow {{ name: {}, x: {}, title: {}, radiant_title: {}, base: {}, radiant: {} }}",
            lit(name),
            entry["x"].as_i64().expect("x"),
            lit(text(&entry["title"])),
            lit(text(&entry["radiantTitle"])),
            lit(text(&entry["base"])),
            lit(text(&entry["radiant"])),
        ));
    }
    writeln!(
        out,
        "pub static POWERS: &[PowerRow] = &[\n    {}\n];",
        rows.join(",\n    ")
    )
    .unwrap();

    // The Call to Chaos cards' effect names, by edition and clause.
    let chaos = read(&data.join("chaos-names.json"));
    let mut editions: Vec<(&String, &Value)> = object(&chaos).iter().collect();
    editions.sort_by(|a, b| a.0.cmp(b.0));
    let rows: Vec<String> = editions
        .iter()
        .map(|(id, names)| format!("({}, {})", lit(id), pairs(names)))
        .collect();
    writeln!(
        out,
        "pub static CHAOS_NAMES: &[(&str, &[(&str, &str)])] = &[\n    {}\n];",
        rows.join(",\n    ")
    )
    .unwrap();

    // The short forms, by definition id.
    let short = read(&data.join("short-forms.json"));
    let form = |value: Option<&Value>| match value {
        None => "None".to_string(),
        Some(Value::Number(n)) => format!("Some(ShortForm::Lines({}))", n.as_u64().expect("a line count")),
        Some(Value::String(s)) if s == "colon" => "Some(ShortForm::Colon)".to_string(),
        Some(Value::String(s)) if s == "clamp" => "Some(ShortForm::Clamp)".to_string(),
        Some(other) => panic!("not a short form: {other}"),
    };
    let mut forms: Vec<(&String, &Value)> = object(&short["forms"]).iter().collect();
    forms.sort_by(|a, b| a.0.cmp(b.0));
    let rows: Vec<String> = forms
        .iter()
        .map(|(id, entry)| {
            format!(
                "({}, ({}, {}))",
                lit(id),
                form(entry.get("base")),
                form(entry.get("radiant"))
            )
        })
        .collect();
    writeln!(
        out,
        "pub static SHORT_FORMS: &[(&str, ShortPair)] = &[\n    {}\n];",
        rows.join(",\n    ")
    )
    .unwrap();

    // The Simplified Chinese dictionary, and the keys that also match a finished sentence, most
    // specific first (`words.rs`).
    let zh = read(&data.join("zh-CN.json"));
    writeln!(out, "pub static ZH_CN: &[(&str, &str)] = {};", pairs(&zh)).unwrap();
    let mut keys: Vec<&String> = object(&zh).keys().filter(|key| patterned(key)).collect();
    // Longest literal first; the sort is stable over the keys' byte order, so a tie is still settled the same way.
    keys.sort_by_key(|key| std::cmp::Reverse(literal_len(key)));
    writeln!(
        out,
        "pub static ZH_CN_PATTERNS: &[&str] = &[{}];",
        keys.iter().map(|key| lit(key)).collect::<Vec<_>>().join(", ")
    )
    .unwrap();

    // The Chinese cards (ME-CN).
    let table = read(&cards.join("chinese.json"));
    let mut rows = Vec::new();
    let mut ids: Vec<(&String, &Value)> = object(&table)
        .iter()
        .filter(|(id, _)| id.as_str() != "//")
        .collect();
    ids.sort_by(|a, b| a.0.cmp(b.0));
    for (id, entry) in ids {
        let previews = match entry.get("previews") {
            Some(previews) => {
                let rows: Vec<String> = object(previews)
                    .iter()
                    .map(|(en, zh)| format!("({}, {})", lit(en), lit(text(zh))))
                    .collect();
                format!("&[{}]", rows.join(", "))
            }
            None => "&[]".to_string(),
        };
        rows.push(format!(
            "({}, ChineseRow {{ name: {}, base: {}, radiant: {}, previews: {} }})",
            lit(id),
            lit(text(&entry["name"])),
            lit(text(&entry["base"])),
            lit(text(&entry["radiant"])),
            previews,
        ));
    }
    writeln!(
        out,
        "pub static CHINESE_CARDS: &[(&str, ChineseRow)] = &[\n    {}\n];",
        rows.join(",\n    ")
    )
    .unwrap();

    // The Chinese frame's words.
    let terms = read(&cards.join("chinese-terms.json"));
    for (name, field) in [
        ("TYPES", "types"),
        ("TAGS", "tags"),
        ("RARITIES", "rarities"),
        ("KEYWORDS", "keywords"),
        ("LABELS", "labels"),
    ] {
        writeln!(
            out,
            "pub static CHINESE_{name}: &[(&str, &str)] = {};",
            pairs(&terms[field])
        )
        .unwrap();
    }
    writeln!(
        out,
        "pub const CHINESE_RADIANT: &str = {};",
        lit(text(&terms["radiant"]))
    )
    .unwrap();
    writeln!(
        out,
        "pub const CHINESE_CREATED: &str = {};",
        lit(text(&terms["created"]))
    )
    .unwrap();
    writeln!(
        out,
        "pub const CHINESE_ALL_TRIBES: &str = {};",
        lit(text(&terms["allTribes"]))
    )
    .unwrap();
    let mut rows: Vec<(&String, &Value)> = object(&terms["glossary"]).iter().collect();
    rows.sort_by(|a, b| a.0.cmp(b.0));
    let rows: Vec<String> = rows
        .iter()
        .map(|(id, entry)| {
            format!(
                "({}, ({}, {}))",
                lit(id),
                lit(text(&entry["label"])),
                lit(text(&entry["rule"]))
            )
        })
        .collect();
    writeln!(
        out,
        "pub static CHINESE_GLOSSARY: &[(&str, (&str, &str))] = &[\n    {}\n];",
        rows.join(",\n    ")
    )
    .unwrap();

    let target = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets it")).join("face_data.rs");
    std::fs::write(&target, out).unwrap_or_else(|error| panic!("{}: {error}", target.display()));
    println!("cargo:rerun-if-changed=build.rs");
}

/// The `{name}` tokens of `key` (`{` word characters `}`) as (start, end) byte ranges.
fn tokens(key: &str) -> Vec<(usize, usize)> {
    let bytes = key.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'{' {
            let mut to = at + 1;
            while to < bytes.len() && (bytes[to].is_ascii_alphanumeric() || bytes[to] == b'_') {
                to += 1;
            }
            if to > at + 1 && bytes.get(to) == Some(&b'}') {
                found.push((at, to + 1));
                at = to + 1;
                continue;
            }
        }
        at += 1;
    }
    found
}

/// A dictionary key's length with its tokens taken out: how specific its pattern is.
fn literal_len(key: &str) -> usize {
    let mut length = key.chars().count();
    for (from, to) in tokens(key) {
        length -= key[from..to].chars().count();
    }
    length
}

/// Whether a key may match a finished sentence (`i18n.ts`'s `patternsOf`): it holds tokens, and at least
/// two words of its own, or every token stands in parentheses and it has one.
fn patterned(key: &str) -> bool {
    let tokens = tokens(key);
    if tokens.is_empty() {
        return false;
    }
    let mut stripped = String::new();
    let mut last = 0;
    let mut enclosed = true;
    for (from, to) in &tokens {
        stripped.push_str(&key[last..*from]);
        stripped.push(' ');
        last = *to;
        enclosed &= key[..*from].ends_with('(') && key[*to..].starts_with(')');
    }
    stripped.push_str(&key[last..]);
    let words = stripped
        .split(|c: char| !c.is_alphabetic())
        .filter(|word| !word.is_empty())
        .count();
    words >= 2 || (enclosed && words > 0)
}
