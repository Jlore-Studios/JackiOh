//! R5289000: the card data JSON laid out a record per line. `layout(json, depth)` writes the JSON text
//! `json` so that each container nested less than `depth` deep opens and closes on lines of its own, with
//! each of its members on a line of its own (`"key": value`, two spaces a level), when it is the root
//! and not empty, or when it holds an object or an array; any other container, every record at `depth`
//! among them, is written compact on its member's line. So `"unset": [...]` in a delta, which holds only
//! strings, stays on one line. The text ends with a newline.
//!
//! Only white space outside strings changes, so the value and key order are kept: every number,
//! escape and key is copied as the input wrote it. `apps/web/scripts/json-layout.mjs` is its copy for
//! the voice manifest, which a JavaScript script writes; its test holds the two to the same bytes.
//! `crates/tools/tests/export_card_text.rs` includes this file by `#[path]`, so it holds `layout` and
//! its private helpers alone.

const INDENT: &str = "  ";

fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\n' | '\r' | '\t')
}

fn is_mark(c: char) -> bool {
    matches!(c, '{' | '}' | '[' | ']' | ',' | ':')
}

fn opens(token: &str) -> bool {
    matches!(token, "{" | "[")
}

fn closes(token: &str) -> bool {
    matches!(token, "}" | "]")
}

/// Each mark, each string whole (its escapes kept), each other literal; white space dropped.
fn tokens(json: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut chars = json.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        if is_space(c) {
            continue;
        }
        let mut end = start + c.len_utf8();
        if c == '"' {
            let mut escaped = false;
            for (at, d) in chars.by_ref() {
                end = at + d.len_utf8();
                if escaped {
                    escaped = false;
                } else if d == '\\' {
                    escaped = true;
                } else if d == '"' {
                    break;
                }
            }
        } else if !is_mark(c) {
            while let Some(&(at, d)) = chars.peek() {
                if is_space(d) || is_mark(d) || d == '"' {
                    break;
                }
                end = at + d.len_utf8();
                chars.next();
            }
        }
        tokens.push(&json[start..end]);
    }
    tokens
}

fn new_line(out: &mut String, level: usize) {
    out.push('\n');
    out.push_str(&INDENT.repeat(level));
}

/// `json` laid out a record per line, its records `depth` levels down (the module's rule).
pub fn layout(json: &str, depth: usize) -> String {
    let list = tokens(json);
    let mut out = String::with_capacity(json.len());
    // For each container open at this token, whether it is laid out a member per line.
    let mut open: Vec<bool> = Vec::new();
    for (at, token) in list.iter().enumerate() {
        let rest = &list[at + 1..];
        if opens(token) {
            let wraps = open.len() < depth
                && if open.is_empty() {
                    rest.first().is_some_and(|next| !closes(next))
                } else {
                    rest.iter()
                        .find(|next| opens(next) || closes(next))
                        .is_some_and(|next| opens(next))
                };
            out.push_str(token);
            open.push(wraps);
            if wraps {
                new_line(&mut out, open.len());
            }
        } else if closes(token) {
            if open.pop() == Some(true) {
                new_line(&mut out, open.len());
            }
            out.push_str(token);
        } else {
            out.push_str(token);
            if open.last() == Some(&true) {
                match *token {
                    "," => new_line(&mut out, open.len()),
                    ":" => out.push(' '),
                    _ => {}
                }
            }
        }
    }
    out.push('\n');
    out
}
