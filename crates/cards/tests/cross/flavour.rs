//! R660: the flavour sidecar (`flavour.json`, the client's `apps/web/src/cards/flavour.ts`) against the
//! catalog. Its keys are catalog ids and nothing else, every card and token has a line, and each entry
//! is plain words under its caps. Whether a line speaks rules words is the client's test
//! (apps/web/src/cards/flavour.test.ts), beside the list the voice lines answer to (issue #115).
//!
//! Port of `packages/cards/test/flavour.test.ts`.

use indexmap::IndexMap;
use jackioh_cards::{CATALOG, flavour_json};
use serde_json::{Map, Value};

/// A flavour line's longest length, in characters (UTF-16 code units, as JS counts them): one or two
/// short lines under the rules. The client's `FLAVOUR_MAX_CHARS` (`packages/cards/src/flavour.ts`,
/// which moves to the web, SURFACE §7.4); the sidecar's contract, held here at the same number.
const FLAVOUR_MAX_CHARS: usize = 120;

/// An artist credit's longest length, in characters. The client's `ARTIST_MAX_CHARS`.
const ARTIST_MAX_CHARS: usize = 60;

const FIELDS: &[&str] = &["flavour", "artist"];

/// Every card's flavour line and artist, by catalog id (TS `FLAVOUR`), each entry kept as raw JSON so
/// an unknown field or a value that is not a string is seen, not refused by a typed parse.
fn flavour() -> IndexMap<String, Map<String, Value>> {
    serde_json::from_str(flavour_json()).expect("crates/cards/flavour.json is an object of objects")
}

/// A string the sidecar may hold: trimmed, non-empty, on one line, and no longer than `cap`.
fn problem_with(value: &Value, cap: usize) -> Option<String> {
    let Some(value) = value.as_str() else {
        return Some("is not a string".to_string());
    };
    if value.trim() != value {
        return Some("has space at an end".to_string());
    }
    if value.is_empty() {
        return Some("is empty".to_string());
    }
    if value.contains('\r') || value.contains('\n') {
        return Some("spans lines".to_string());
    }
    // JS `value.length`: UTF-16 code units.
    let length = value.encode_utf16().count();
    if length > cap {
        return Some(format!("is {length} characters, over {cap}"));
    }
    None
}

mod r660_the_flavour_sidecar {
    use super::*;

    #[test]
    fn r660_every_key_is_a_catalog_card_or_token() {
        let strangers: Vec<String> = flavour()
            .keys()
            .filter(|id| !CATALOG.contains_key(id.as_str()))
            .cloned()
            .collect();
        assert_eq!(strangers, Vec::<String>::new());
    }

    #[test]
    fn r660_every_card_and_token_has_a_flavour_line() {
        let sidecar = flavour();
        let without: Vec<String> = CATALOG
            .keys()
            .filter(|id| {
                sidecar
                    .get(id.as_str())
                    .and_then(|entry| entry.get("flavour"))
                    .is_none()
            })
            .cloned()
            .collect();
        assert_eq!(without, Vec::<String>::new());
    }

    #[test]
    fn r660_an_entry_carries_only_a_flavour_line_and_an_artist_each_plain_words_under_its_cap() {
        let mut problems: Vec<String> = Vec::new();
        for (id, entry) in flavour() {
            for field in entry.keys() {
                if !FIELDS.contains(&field.as_str()) {
                    problems.push(format!("{id}: unknown field \"{field}\""));
                }
            }
            if let Some(line) = entry.get("flavour")
                && let Some(problem) = problem_with(line, FLAVOUR_MAX_CHARS)
            {
                problems.push(format!("{id}: flavour {problem}"));
            }
            if let Some(artist) = entry.get("artist")
                && let Some(problem) = problem_with(artist, ARTIST_MAX_CHARS)
            {
                problems.push(format!("{id}: artist {problem}"));
            }
        }
        assert_eq!(problems, Vec::<String>::new());
    }

    #[test]
    fn r660_the_caps_hold_the_check_a_line_one_character_over_is_refused() {
        assert_eq!(
            problem_with(&Value::from("x".repeat(FLAVOUR_MAX_CHARS)), FLAVOUR_MAX_CHARS),
            None
        );
        let over = problem_with(&Value::from("x".repeat(FLAVOUR_MAX_CHARS + 1)), FLAVOUR_MAX_CHARS);
        assert!(over.is_some_and(|p| p.contains("over")));
        let padded = problem_with(&Value::from(" padded"), FLAVOUR_MAX_CHARS);
        assert!(padded.is_some_and(|p| p.contains("space")));
        let lines = problem_with(&Value::from("two\nlines"), FLAVOUR_MAX_CHARS);
        assert!(lines.is_some_and(|p| p.contains("lines")));
        let empty = problem_with(&Value::from(""), FLAVOUR_MAX_CHARS);
        assert!(empty.is_some_and(|p| p.contains("empty")));
        let number = problem_with(&Value::from(7), FLAVOUR_MAX_CHARS);
        assert!(number.is_some_and(|p| p.contains("string")));
    }
}
