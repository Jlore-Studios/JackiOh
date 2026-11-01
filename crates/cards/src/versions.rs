//! R1371000: the two schemes a catalog version can be named by. The version retcon (#1371) renamed
//! every shipped version to what the classifier calls it, and old and new names overlap (the old
//! v0.3.4 is the new v0.2.7b, while the new v0.3.4 is the old v0.3.12), so a name alone cannot say
//! which it means. A file says: a replay, game-record or self-play file the build writes carries the
//! `versions` marker (`VERSIONS_SCHEME`, or its format's own bumped version), and one without it
//! names its versions the old way, which its reader maps once through `patches/aliases.json`
//! (`current_version_name`). Nothing the build holds is unmarked: the repository and the database
//! were renamed in place.
//!
//! R2532000: the v0.5 retcon (#2532) renamed the v0.5 line a second time, so a scheme is now a
//! number: an unmarked file's is 1, a marked file's the `versions` it carries, and today's
//! `VERSIONS_SCHEME`. A file of an older scheme has its names read through `patches/aliases.json`
//! (`version_name_from`), once.
//!
//! This file's tests spell old names on purpose, so the retcon's tree rewrite leaves it alone
//! (`crates/tools/src/retcon.rs`, `SKIP_FILES`).

use jackioh_engine::EngineError;
use jackioh_engine::config::VERSIONS_SCHEME;
use jackioh_engine::wire::{GameRecord, parse_game_record};
use serde_json::Value;

pub use crate::history::{current_version_name, version_alias, version_alias_from, version_name_from};

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

/// R2532000: the scheme a file's object names its versions by: the `versions` it carries, else 1,
/// the names from before any retcon.
pub fn scheme_of(value: &Value) -> u32 {
    value
        .get(VERSIONS_KEY)
        .and_then(Value::as_u64)
        .and_then(|scheme| u32::try_from(scheme).ok())
        .unwrap_or(1)
}

/// R2532000: a text of a file named by `scheme`, its versions as they are named now.
pub fn versions_now_in(scheme: u32, text: &str) -> String {
    rename_versions_with(text, |name| version_alias_from(scheme, name))
}

/// R1371000: one game-record line as a file holds it, with the marker.
pub fn game_record_line(record: &GameRecord) -> String {
    let mut value = serde_json::to_value(record).unwrap_or(Value::Null);
    if let Some(object) = value.as_object_mut() {
        object.insert(VERSIONS_KEY.to_string(), Value::from(VERSIONS_SCHEME));
    }
    value.to_string()
}

/// R1371000: `parse_game_record` for a record a file holds: one of an older scheme (R2532000), an
/// unmarked one among them, has its patch and the versions its id spells mapped to their names now.
pub fn parse_game_record_in_file(value: &Value) -> Result<GameRecord, EngineError> {
    let mut record = parse_game_record(value)?;
    let scheme = scheme_of(value);
    if scheme < VERSIONS_SCHEME {
        record.patch = version_name_from(scheme, &record.patch).to_string();
        record.id = versions_now_in(scheme, &record.id);
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
            versions_now_in(1, "selfplay-learned-v0.3.17-1.jsonl"),
            "selfplay-learned-v0.4.0c-1.jsonl"
        );
    }

    // R2532000: a record marked 2 names the v0.5 line as it was before the v0.5 retcon, and is read
    // through `aliases.json`'s `marked`; one marked 3 is today's; an unmarked one is scheme 1's.
    #[test]
    fn r2532000_a_record_marked_2_names_the_v0_5_line_as_it_was() {
        let two = line("dev:v0.5.11:gather:1", "v0.5.11", r#","versions":2"#);
        let three = line("dev:v0.5.7:gather:2", "v0.5.7", r#","versions":3"#);
        let one = line("dev:v0.3.17b:gather:3", "v0.3.17b", "");
        let records = read_game_record_lines(&format!("{two}\n{three}\n{one}\n")).expect("the records");
        assert_eq!(records[0].patch, "v0.5.7", "the old v0.5.11 is the new v0.5.7");
        assert_eq!(records[0].id, "dev:v0.5.7:gather:1");
        assert_eq!(
            records[1].patch, "v0.5.7",
            "a record marked 3 names today's v0.5.7"
        );
        assert_eq!(records[2].patch, "v0.4.0d");
        assert_eq!(
            version_name_from(2, "v0.5.7"),
            "v0.5.5",
            "the old v0.5.7 is Training Grounds' release"
        );
        assert_eq!(
            version_name_from(2, "v0.4.0"),
            "v0.4.0",
            "the retcon kept every earlier name"
        );
        assert_eq!(version_name_from(VERSIONS_SCHEME, "v0.5.11"), "v0.5.11");
        assert_eq!(
            versions_now_in(2, "selfplay-learned-v0.5.4-1.jsonl"),
            "selfplay-learned-v0.5.3-1.jsonl"
        );
    }
}
