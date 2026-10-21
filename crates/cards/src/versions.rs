//! R1371000: the two schemes a catalog version can be named by. The version retcon (#1371) renamed
//! every shipped version to what the classifier calls it, and old and new names overlap (the old
//! v0.3.4 is the new v0.2.7b, while the new v0.3.4 is the old v0.3.12), so a name alone cannot say
//! which it means. A file says: a replay, game-record or self-play file the build writes carries the
//! `versions` marker (`VERSIONS_SCHEME`, or its format's own bumped version), and one without it
//! names its versions the old way, which its reader maps once through `patches/aliases.json`
//! (`current_version_name`). Nothing the build holds is unmarked: the repository and the database
//! were renamed in place.
//!
//! This file's tests spell old names on purpose, so the retcon's tree rewrite leaves it alone
//! (`crates/tools/src/retcon.rs`, `SKIP_FILES`).

use jackioh_engine::EngineError;
use jackioh_engine::config::VERSIONS_SCHEME;
use jackioh_engine::wire::{GameRecord, parse_game_record};
use serde_json::Value;

pub use crate::history::{current_version_name, version_alias};

/// The key a marked file's object carries, holding `VERSIONS_SCHEME`.
pub const VERSIONS_KEY: &str = "versions";

/// The end of a version at `at` (`v<digits>.<digits>.<digits>` and at most one lower-case letter)
/// when one starts there and stands alone: no letter, digit, `_` or `.` before it, and no letter or
/// digit, nor a dot and a digit, after it.
fn version_at(bytes: &[u8], at: usize) -> Option<usize> {
    if bytes.get(at) != Some(&b'v') {
        return None;
    }
    if at > 0 && (bytes[at - 1].is_ascii_alphanumeric() || matches!(bytes[at - 1], b'_' | b'.')) {
        return None;
    }
    let mut end = at + 1;
    for part in 0..3 {
        let start = end;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        if end == start {
            return None;
        }
        if part < 2 {
            if bytes.get(end) != Some(&b'.') {
                return None;
            }
            end += 1;
        }
    }
    let alone = |end: usize| {
        let next = bytes.get(end);
        !next.is_some_and(u8::is_ascii_alphanumeric)
            && !(next == Some(&b'.') && bytes.get(end + 1).is_some_and(u8::is_ascii_digit))
    };
    if bytes.get(end).is_some_and(u8::is_ascii_lowercase) && alone(end + 1) {
        return Some(end + 1);
    }
    alone(end).then_some(end)
}

/// R1371000: `text` with every version that stands alone in it (a record id's `dev:<patch>:…`, a
/// seed's `learned-<version>-…`, a file name) renamed by `new_of`, each once, left to right.
pub fn rename_versions_with<'a>(text: &str, new_of: impl Fn(&str) -> Option<&'a str>) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let (mut at, mut kept) = (0, 0);
    while at < bytes.len() {
        if let Some(end) = version_at(bytes, at)
            && let Some(new) = new_of(&text[at..end])
        {
            out.push_str(&text[kept..at]);
            out.push_str(new);
            (at, kept) = (end, end);
            continue;
        }
        at += 1;
    }
    out.push_str(&text[kept..]);
    out
}

/// R1371000: an unmarked text's versions as they are named now.
pub fn current_versions_in(unmarked: &str) -> String {
    rename_versions_with(unmarked, version_alias)
}

/// R1371000: whether a file's object carries the marker.
pub fn is_marked(value: &Value) -> bool {
    value.get(VERSIONS_KEY).is_some()
}

/// R1371000: one game-record line as a file holds it, with the marker.
pub fn game_record_line(record: &GameRecord) -> String {
    let mut value = serde_json::to_value(record).unwrap_or(Value::Null);
    if let Some(object) = value.as_object_mut() {
        object.insert(VERSIONS_KEY.to_string(), Value::from(VERSIONS_SCHEME));
    }
    value.to_string()
}

/// R1371000: `parse_game_record` for a record a file holds: one without the marker has its patch
/// and the versions its id spells mapped to their names now.
pub fn parse_game_record_in_file(value: &Value) -> Result<GameRecord, EngineError> {
    let mut record = parse_game_record(value)?;
    if !is_marked(value) {
        record.patch = current_version_name(&record.patch).to_string();
        record.id = current_versions_in(&record.id);
    }
    Ok(record)
}

/// R1371000: a game-record file's lines (`parse_game_record_lines`), each read by
/// `parse_game_record_in_file`. Blank lines are skipped.
pub fn read_game_record_lines(contents: &str) -> Result<Vec<GameRecord>, EngineError> {
    let mut records = Vec::new();
    for (index, line) in contents.split('\n').enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let parsed = serde_json::from_str::<Value>(line)
            .map_err(|error| error.to_string())
            .and_then(|value| parse_game_record_in_file(&value).map_err(|error| error.message));
        match parsed {
            Ok(record) => records.push(record),
            Err(message) => return Err(EngineError::new(format!("line {}: {}", index + 1, message))),
        }
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEATS: &str = r#"{"p1":{"deck":[],"opening":[],"drawn":[],"played":[]},"p2":{"deck":[],"opening":[],"drawn":[],"played":[]}}"#;

    fn line(id: &str, patch: &str, marker: &str) -> String {
        format!(
            r#"{{"id":"{id}","source":"dev","mode":"random","patch":"{patch}","pilots":{{"p1":"ai","p2":"ai"}},"game":{{"first":"p1","winner":"p1","reason":"hero-death","turns":9,"seats":{SEATS}}}{marker}}}"#
        )
    }

    #[test]
    fn r1371000_an_unmarked_record_is_read_through_the_map_and_a_marked_one_is_not() {
        let unmarked = line("dev:v0.3.4:learned-v0.3.7b-20261018-5:3", "v0.3.4", "");
        let marked = line("dev:v0.3.4:gather:4", "v0.3.4", r#","versions":2"#);
        let records = read_game_record_lines(&format!("{unmarked}\n\n{marked}\n")).expect("the records");
        assert_eq!(records[0].patch, "v0.2.7b", "the old v0.3.4 is the new v0.2.7b");
        assert_eq!(records[0].id, "dev:v0.2.7b:learned-v0.3.1-20261018-5:3");
        assert_eq!(records[1].patch, "v0.3.4", "a marked record names the new v0.3.4");
        assert_eq!(records[1].id, "dev:v0.3.4:gather:4");
    }

    #[test]
    fn r1371000_a_record_the_build_writes_carries_the_marker() {
        let records = read_game_record_lines(&line("dev:v0.3.4:gather:4", "v0.3.4", r#","versions":2"#))
            .expect("a record");
        let written = game_record_line(&records[0]);
        let value: Value = serde_json::from_str(&written).expect("JSON");
        assert_eq!(value[VERSIONS_KEY], VERSIONS_SCHEME);
        assert_eq!(read_game_record_lines(&written).expect("read back"), records);
    }

    #[test]
    fn r1371000_a_name_the_retcon_kept_or_never_had_is_its_own() {
        assert_eq!(current_version_name("v0.1.0"), "v0.1.0");
        assert_eq!(current_version_name("v0.3.17"), "v0.4.0c");
        assert_eq!(current_version_name("v9.9.9"), "v9.9.9");
        assert_eq!(
            current_versions_in("selfplay-learned-v0.3.17-1.jsonl"),
            "selfplay-learned-v0.4.0c-1.jsonl"
        );
    }
}
