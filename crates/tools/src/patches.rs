//! `cargo jackioh patches`: the card patch history (B4.2, R388, R646, R650), with several patches in
//! flight at once: pending fragments, the check that proves them, and the promotion that ships them,
//! in ship order. The files live under `crates/cards/patches/` beside `crates/cards/catalog.json`.
//! Surface contract: docs/v0.3.0/SURFACE.md §12.
//!
//! ```text
//! cargo jackioh patches <version> [date] "<title>" \
//!   [--source "<issue, PR, commits>"] [--notes "<what changed>"] [--cards <id,...>]
//! cargo jackioh patches check
//! cargo jackioh patches ship
//! ```
//!
//! - A branch changes `catalog.json` and adds one fragment, `pending/<version>.json`: `{ version,
//!   title, sources, notes, cards }`. `cards` lists the catalog ids the patch creates, changes or
//!   removes (`--cards` says them; otherwise they are diffed from the working catalog against the
//!   newest shipped snapshot); `version` is a bare patch number or a micro `vA.B.Y` (R650). Branches
//!   never edit `patches.json`, the snapshots, `index.json` or `shipped.json`. The optional `date`
//!   is checked and not stored: promotion dates the patch by the UTC date of the commit that added
//!   the fragment.
//! - `check` fails naming the card when a catalog entry differs from the newest shipped snapshot
//!   without exactly one fragment claiming it, when a claimed card does not differ, when a fragment's
//!   version is neither a bare patch number nor a micro `vA.B.Y`, when a micro cannot be named after
//!   the newest patch, when a fragment claims no card, or when its title, sources or notes is empty
//!   (they become the shipped patch's, R388). CI runs it beside `catalog check`.
//! - `ship` promotes every fragment on main, oldest first-parent commit that added one first: it
//!   appends the patch (its version, a micro named after the then-newest patch (R650), or
//!   `<version>b`, `c`, … when that name already shipped), snapshots `catalog.json` as that commit
//!   left it, records `{ version, commit, blob }` in `shipped.json`, deletes the fragment,
//!   regenerates the derived files and bumps `CATALOG_VERSION`. Running it twice is running it once.
//!   It needs full history (`fetch-depth: 0`) and writes nothing unless each adding commit's catalog
//!   changed exactly the cards its fragment claims and the last one is `catalog.json` as it stands.
//!   `.github/workflows/patches-ship.yml` runs it after every merge that touches `pending/`.
//!
//! There is no clock here (CLAUDE.md rule 4): dates come from the git history, and the UTC
//! conversion below is arithmetic. One file holds the modules `naming`, `patch`, `patches_io` and
//! `versions` (R650's micro versions) and the command itself, each keeping its module (SURFACE §4.2),
//! with their tests at the bottom; `js` comes first. Some functions have no caller in the binary,
//! only in the tests (`naming`'s convention, `versions_at_sites` and the paths `patches_io` names).
#![allow(dead_code)]

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, anyhow, bail};
use indexmap::{IndexMap, IndexSet};

use self::patch::bump_sites;
use self::patches_io::{
    Catalog, CheckFragmentsArgs, NamedFragment, PatchEntry, PendingFragment, ShippedEntry, check_fragments,
    differing_ids, git_blob_hash, is_fragment_version, next_ship_name, read_fragments, read_patches,
    read_shipped, read_snapshot, rebuild_derived, remove_file, snapshot_path, write_json,
};
use self::versions::resolve_version;
use jackioh_engine::wire::{SetName, set_ships};

/// The repository root, from this crate's own manifest directory (`crates/tools`), resolved at compile time.
pub(crate) fn repo_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.ancestors().nth(2).unwrap_or(manifest).to_path_buf()
}

/* js */

/// JSON and string helpers: an object that keeps its key order, a JSON parser and printer, number
/// formatting and the few regular-expression classes the tools match with (no regex crate, SURFACE §2).
pub(crate) mod js {
    use std::fmt;

    use indexmap::IndexMap;
    use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
    use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

    /// Up to here a double is an exact integer and prints without a fraction.
    const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

    /// A JSON object, keys in the order the text wrote them.
    pub type Object = IndexMap<String, Json>;

    /// A JSON value. serde_json's own `Value` sorts object keys (the workspace has no `preserve_order`,
    /// SURFACE §2), but the patch history compares catalog entries byte for byte, key order included
    /// (`differingIds`, `sameCatalog`), so objects here keep insertion order; a repeated key keeps its
    /// first place and last value. Numbers are doubles. `==` ignores key order; `stringify` equality
    /// does not.
    #[derive(Clone, Debug, PartialEq)]
    pub enum Json {
        Null,
        Bool(bool),
        Number(f64),
        String(String),
        Array(Vec<Json>),
        Object(Object),
    }

    impl Json {
        pub fn as_str(&self) -> Option<&str> {
            match self {
                Json::String(text) => Some(text),
                _ => None,
            }
        }

        pub fn as_f64(&self) -> Option<f64> {
            match self {
                Json::Number(n) => Some(*n),
                _ => None,
            }
        }

        pub fn as_array(&self) -> Option<&Vec<Json>> {
            match self {
                Json::Array(items) => Some(items),
                _ => None,
            }
        }

        pub fn as_object(&self) -> Option<&Object> {
            match self {
                Json::Object(object) => Some(object),
                _ => None,
            }
        }

        pub fn is_integer(&self) -> bool {
            matches!(self, Json::Number(n) if n.is_finite() && n.fract() == 0.0)
        }
    }

    impl From<&str> for Json {
        fn from(text: &str) -> Json {
            Json::String(text.to_string())
        }
    }

    impl From<String> for Json {
        fn from(text: String) -> Json {
            Json::String(text)
        }
    }

    impl From<i32> for Json {
        fn from(n: i32) -> Json {
            Json::Number(f64::from(n))
        }
    }

    impl From<bool> for Json {
        fn from(b: bool) -> Json {
            Json::Bool(b)
        }
    }

    impl From<Object> for Json {
        fn from(object: Object) -> Json {
            Json::Object(object)
        }
    }

    impl From<Vec<Json>> for Json {
        fn from(items: Vec<Json>) -> Json {
            Json::Array(items)
        }
    }

    struct JsonVisitor;

    impl<'de> Visitor<'de> for JsonVisitor {
        type Value = Json;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("any JSON value")
        }

        fn visit_bool<E: de::Error>(self, v: bool) -> Result<Json, E> {
            Ok(Json::Bool(v))
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Json, E> {
            Ok(Json::Number(v as f64))
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Json, E> {
            Ok(Json::Number(v as f64))
        }

        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Json, E> {
            Ok(Json::Number(v))
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Json, E> {
            Ok(Json::String(v.to_string()))
        }

        fn visit_string<E: de::Error>(self, v: String) -> Result<Json, E> {
            Ok(Json::String(v))
        }

        fn visit_unit<E: de::Error>(self) -> Result<Json, E> {
            Ok(Json::Null)
        }

        fn visit_none<E: de::Error>(self) -> Result<Json, E> {
            Ok(Json::Null)
        }

        fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Json, D::Error> {
            Json::deserialize(deserializer)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
            let mut items = Vec::new();
            while let Some(item) = seq.next_element::<Json>()? {
                items.push(item);
            }
            Ok(Json::Array(items))
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
            let mut object = Object::new();
            while let Some((key, value)) = map.next_entry::<String, Json>()? {
                object.insert(key, value);
            }
            Ok(Json::Object(object))
        }
    }

    impl<'de> Deserialize<'de> for Json {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Json, D::Error> {
            deserializer.deserialize_any(JsonVisitor)
        }
    }

    impl Serialize for Json {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            match self {
                Json::Null => serializer.serialize_unit(),
                Json::Bool(b) => serializer.serialize_bool(*b),
                // `JSON.stringify(NaN)` is `null`; an integral double prints with no fraction.
                Json::Number(n) if !n.is_finite() => serializer.serialize_unit(),
                Json::Number(n) if n.fract() == 0.0 && n.abs() <= MAX_SAFE_INTEGER => {
                    serializer.serialize_i64(*n as i64)
                }
                Json::Number(n) => serializer.serialize_f64(*n),
                Json::String(text) => serializer.serialize_str(text),
                Json::Array(items) => {
                    let mut seq = serializer.serialize_seq(Some(items.len()))?;
                    for item in items {
                        seq.serialize_element(item)?;
                    }
                    seq.end()
                }
                Json::Object(object) => {
                    let mut map = serializer.serialize_map(Some(object.len()))?;
                    for (key, value) in object {
                        map.serialize_entry(key, value)?;
                    }
                    map.end()
                }
            }
        }
    }

    pub fn parse(text: &str) -> serde_json::Result<Json> {
        serde_json::from_str(text)
    }

    /// Compact JSON for anything serde writes in field order (fields serialise in declaration order).
    pub fn stringify<T: serde::Serialize + ?Sized>(value: &T) -> String {
        serde_json::to_string(value).unwrap_or_else(|error| panic!("JSON.stringify: {error}"))
    }

    /// Pretty JSON in the two-space form.
    pub fn stringify_pretty<T: serde::Serialize + ?Sized>(value: &T) -> String {
        serde_json::to_string_pretty(value).unwrap_or_else(|error| panic!("JSON.stringify: {error}"))
    }

    /// Integers print as integers; a fraction as Rust's shortest round-trip form, which equals JS's for
    /// every number the catalog holds (JS uses exponents below 1e-6 and from 1e21, Rust never does).
    pub fn number_string(n: f64) -> String {
        if n.is_nan() {
            return "NaN".to_string();
        }
        if n.is_infinite() {
            return if n > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
        }
        if n.fract() == 0.0 && n.abs() <= MAX_SAFE_INTEGER {
            return (n as i64).to_string();
        }
        format!("{n}")
    }

    pub fn to_js_string(value: Option<&Json>) -> String {
        match value {
            None => "undefined".to_string(),
            Some(Json::Null) => "null".to_string(),
            Some(Json::Bool(b)) => b.to_string(),
            Some(Json::Number(n)) => number_string(*n),
            Some(Json::String(text)) => text.clone(),
            Some(Json::Array(items)) => join(items, ","),
            Some(Json::Object(_)) => "[object Object]".to_string(),
        }
    }

    /// Each item as `String(item)`, `null` as the empty string.
    pub fn join(items: &[Json], separator: &str) -> String {
        items
            .iter()
            .map(|item| match item {
                Json::Null => String::new(),
                other => to_js_string(Some(other)),
            })
            .collect::<Vec<_>>()
            .join(separator)
    }

    /// A JavaScript line terminator: what `.` never matches and what `^` and `$` stand next to under
    /// the `m` flag.
    pub fn is_line_terminator(c: char) -> bool {
        matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
    }

    /// `^\d+$`: one or more ASCII digits and nothing else (JS's `\d` is ASCII).
    pub fn is_digits(text: &str) -> bool {
        !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
    }

    /// One match of `@jackioh/shared`'s `PARAM_PLACEHOLDER`
    /// (`/\{([A-Za-z][A-Za-z0-9]*)(?:\|([^|{}]*)\|([^|{}]*))?\}/g`): its byte span, its key and, for
    /// the agreeing form `{key|singular|plural}` (R482), both wordings.
    pub struct Placeholder<'t> {
        pub start: usize,
        pub end: usize,
        pub key: &'t str,
        pub words: Option<(&'t str, &'t str)>,
    }

    /// `PARAM_PLACEHOLDER` anchored at `start` (which holds `{`). The key class excludes `|` and `}`,
    /// so the key is maximal and needs no backtracking; the agreeing group's two classes exclude
    /// `|{}`.
    fn placeholder_at(text: &str, start: usize) -> Option<Placeholder<'_>> {
        let bytes = text.as_bytes();
        let mut at = start + 1;
        if !bytes.get(at).is_some_and(u8::is_ascii_alphabetic) {
            return None;
        }
        let key_start = at;
        at += 1;
        while bytes.get(at).is_some_and(u8::is_ascii_alphanumeric) {
            at += 1;
        }
        let key = &text[key_start..at];
        match bytes.get(at) {
            Some(b'}') => Some(Placeholder {
                start,
                end: at + 1,
                key,
                words: None,
            }),
            Some(b'|') => {
                let word_end = |from: usize| {
                    let mut to = from;
                    while bytes.get(to).is_some_and(|b| !matches!(b, b'|' | b'{' | b'}')) {
                        to += 1;
                    }
                    to
                };
                let one_start = at + 1;
                let one_end = word_end(one_start);
                if bytes.get(one_end) != Some(&b'|') {
                    return None;
                }
                let many_start = one_end + 1;
                let many_end = word_end(many_start);
                if bytes.get(many_end) != Some(&b'}') {
                    return None;
                }
                Some(Placeholder {
                    start,
                    end: many_end + 1,
                    key,
                    words: Some((&text[one_start..one_end], &text[many_start..many_end])),
                })
            }
            _ => None,
        }
    }

    /// Every non-overlapping match of `PARAM_PLACEHOLDER`, left to right, as `matchAll` and a global
    /// `replace` find them.
    pub fn placeholder_spans(text: &str) -> Vec<Placeholder<'_>> {
        let mut out = Vec::new();
        let mut at = 0;
        while let Some(offset) = text[at..].find('{') {
            let start = at + offset;
            match placeholder_at(text, start) {
                Some(found) => {
                    at = found.end;
                    out.push(found);
                }
                None => at = start + 1,
            }
        }
        out
    }
}

/* naming */

/// The catalog-id <-> filename convention for card scripts and card tests (SPEC §10.9; set folders,
/// B2.2). Pure string functions, no I/O and no catalog import. The Rust card files follow SURFACE
/// §4.1's own convention (`c001_big_d_fender.rs`), which `crates/cards/build.rs` reads by each file's
/// `ID`; these are the rules of the older file names.
///
/// The convention, by example (the catalog id is the key of `catalog.json`, SPEC §5):
///
/// ```text
///   core-001             "Big D-fender"         src/scripts/001-big-d-fender.ts
///   core-051-1           "KY's Empty Notebook"  src/scripts/051-1-kys-empty-notebook.ts
///   core-t-rush          "Rush Token"           src/scripts/t-rush.ts
///   classic-043          "Plague Nuke"          src/scripts/classic/043-plague-nuke.ts
///   classicplus-012-1    "Devour"               src/scripts/classic-plus/012-1-devour.ts
///   classicplus-t-ai-01  "Helpful Assistant"    src/scripts/classic-plus/t-ai-01-helpful-assistant.ts
/// ```
///
/// and the same path under `test/` with `.test.ts`. So a file's path is its set's folder (none for
/// Core) and a basename `<prefix>-<slug>`, where the prefix is the id minus its set segment, except
/// for Core's named tokens of SPEC §7 — the four shared ones, The Coin and the Ghoul Token — which
/// are filed under the bare prefix (`t-rush`, `t-coin`). An index repeats across sets, so the folder,
/// never the basename alone, says which set a file belongs to.
pub mod naming {
    use std::cmp::Ordering;

    use super::js::is_digits;

    /// Every shipped id is `<set>-<prefix>`, the set segment holding no hyphen (`classicplus`, B2.2).
    const SET_SEGMENT_SEPARATOR: char = '-';

    /// A shipped set's id segment.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum SetSegment {
        Core,
        Classic,
        Classicplus,
    }

    impl SetSegment {
        /// The segment as ids write it.
        pub fn as_str(self) -> &'static str {
            match self {
                SetSegment::Core => "core",
                SetSegment::Classic => "classic",
                SetSegment::Classicplus => "classicplus",
            }
        }
    }

    /// The shipped sets' id segments and the folder each one's scripts and tests live in, in catalog order.
    pub const SET_FOLDERS: &[(SetSegment, &str)] = &[
        (SetSegment::Core, ""),
        (SetSegment::Classic, "classic"),
        (SetSegment::Classicplus, "classic-plus"),
    ];

    const SET_SEGMENTS: &[SetSegment] = &[SetSegment::Core, SetSegment::Classic, SetSegment::Classicplus];

    fn folder_for(segment: SetSegment) -> &'static str {
        SET_FOLDERS
            .iter()
            .find(|(each, _)| *each == segment)
            .map_or("", |(_, folder)| *folder)
    }

    /// The set folders that are not the package root, for a tool that walks them.
    pub const SET_SUBFOLDERS: &[&str] = &["classic", "classic-plus"];

    /// `classic-043` -> `classic`; `None` for an id whose set segment names no shipped set.
    pub fn set_segment_of(id: &str) -> Option<SetSegment> {
        let cut = id.find(SET_SEGMENT_SEPARATOR)?;
        if cut == 0 {
            return None;
        }
        let segment = &id[..cut];
        SET_SEGMENTS.iter().copied().find(|each| each.as_str() == segment)
    }

    /// The folder a catalog id's script and test live in: `""` for Core, `classic`, `classic-plus`.
    pub fn folder_of(id: &str) -> &'static str {
        set_segment_of(id).map_or("", folder_for)
    }

    /// The set segment a folder holds (`""` -> `core`), or `None` for a folder that holds none.
    pub fn segment_of_folder(folder: &str) -> Option<SetSegment> {
        SET_SEGMENTS
            .iter()
            .copied()
            .find(|segment| folder_for(*segment) == folder)
    }

    /// Catalog order of the sets (Core, Classic, Classic+), for sorting; an unknown set sorts last.
    pub fn set_rank(id: Option<&str>) -> usize {
        match id.and_then(set_segment_of) {
            None => SET_SEGMENTS.len(),
            Some(segment) => SET_SEGMENTS
                .iter()
                .position(|each| *each == segment)
                .unwrap_or(SET_SEGMENTS.len()),
        }
    }

    /// The filename prefix of a catalog id: the id minus its leading set segment (`core-051-1` ->
    /// `051-1`). Panics on a string that is no catalog id; every caller hands it catalog ids.
    pub fn slug_prefix_of(id: &str) -> &str {
        match id.find(SET_SEGMENT_SEPARATOR) {
            Some(cut) if cut > 0 && cut != id.len() - 1 => &id[cut + 1..],
            _ => panic!("\"{id}\" is not a catalog id (expected \"<set>-<prefix>\", e.g. \"core-043\")"),
        }
    }

    /// `051-1` and `t-rush` are token prefixes; `051` is a card prefix. Nothing nests deeper.
    pub fn is_token_prefix(prefix: &str) -> bool {
        prefix.starts_with("t-")
            || prefix
                .split_once('-')
                .is_some_and(|(main, sub)| is_digits(main) && is_digits(sub))
    }

    /// A shared token's prefix (`t-rush`, `t-ai-01`): a token no one card defines. Core's (SPEC §7's four
    /// shared ones, The Coin and the Ghoul Token) are filed under the bare prefix; Classic+'s AI
    /// generated cards carry their slug like any card.
    pub fn is_shared_token_prefix(prefix: &str) -> bool {
        prefix.starts_with("t-")
    }

    /// A card name as a filename slug: lowercase, apostrophes dropped, every other run of
    /// non-alphanumerics collapsed to one `-`: "KY's Empty Notebook" -> `kys-empty-notebook` (the
    /// apostrophe vanishes rather than becoming a dash), "4-mana 7/7" -> `4-mana-7-7`.
    pub fn slugify(name: &str) -> String {
        let lowered: String = name
            .to_lowercase()
            .chars()
            .filter(|c| !matches!(c, '\'' | '\u{2018}' | '\u{2019}'))
            .collect();
        let mut out = String::new();
        let mut in_run = false;
        for c in lowered.chars() {
            if c.is_ascii_lowercase() || c.is_ascii_digit() {
                out.push(c);
                in_run = false;
            } else if !in_run {
                out.push('-');
                in_run = true;
            }
        }
        out.trim_matches('-').to_string()
    }

    /// The canonical basename (no extension, no folder) for a catalog entry.
    pub fn expected_basename(id: &str, name: &str) -> String {
        let prefix = slug_prefix_of(id);
        // Core's SPEC §7 shared tokens are one word already ("Rush Token" under `t-rush`), so the
        // prefix is the whole filename; anything else carries its slug.
        if is_shared_token_prefix(prefix) && set_segment_of(id) == Some(SetSegment::Core) {
            return prefix.to_string();
        }
        format!("{prefix}-{}", slugify(name))
    }

    /// The path under `src/scripts/` or `test/`, without an extension: `classic/043-plague-nuke`.
    pub fn expected_rel_path(id: &str, name: &str) -> String {
        let folder = folder_of(id);
        let basename = expected_basename(id, name);
        if folder.is_empty() {
            basename
        } else {
            format!("{folder}/{basename}")
        }
    }

    /// `001-big-d-fender.ts` -> `001-big-d-fender`; also strips `.test.ts`.
    pub fn basename_of(filename: &str) -> &str {
        let stripped = filename.strip_suffix(".test.ts").unwrap_or(filename);
        stripped.strip_suffix(".ts").unwrap_or(stripped)
    }

    /// Whether `basename` (no extension) is the file of catalog id `id`, in that id's own set folder.
    ///
    /// The slug is not checked: the prefix decides which card a file belongs to (`expected_basename`
    /// reports a misspelt slug). The hard case is the prefix boundary: `051-1-kys-empty-notebook` belongs
    /// to `core-051-1` and must NOT be read as a slug of `core-051`.
    ///
    /// With `all_ids` (every catalog id) the longest id prefix of that set the basename carries wins.
    /// Without it the rule is the heuristic "a lone digit segment right after a card prefix is a token
    /// sub-index", right for every shipped id except #25 "4-mana 7/7" (slug `025-4-mana-7-7`); pass
    /// `all_ids` when the exact answer matters.
    pub fn matches_card(basename: &str, id: &str, all_ids: Option<&[String]>) -> bool {
        if let Some(all_ids) = all_ids {
            return resolve_basename(basename, all_ids, folder_of(id)).as_deref() == Some(id);
        }

        let prefix = slug_prefix_of(id);
        if basename == prefix {
            return true; // the slugless form: `t-rush.ts`
        }
        let Some(rest) = basename
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_prefix('-'))
        else {
            return false;
        };
        if rest.is_empty() {
            return false;
        }
        if is_token_prefix(prefix) {
            return true; // a token id has no sub-token, so the rest is a slug
        }
        let mut chars = rest.chars();
        let lone_digit =
            chars.next().is_some_and(|c| c.is_ascii_digit()) && matches!(chars.next(), None | Some('-'));
        !lone_digit
    }

    /// Which catalog id a basename in `folder` (`""` is Core's) belongs to, by longest prefix among that
    /// folder's set: `051-1-kys-empty-notebook` matches `051` and `051-1`, and the longer one owns the
    /// file. `None` means the filename names no card of that set, or the folder holds no set.
    pub fn resolve_basename(basename: &str, all_ids: &[String], folder: &str) -> Option<String> {
        let segment = segment_of_folder(folder)?;
        let mut best: Option<&String> = None;
        let mut best_length: Option<usize> = None;
        for id in all_ids {
            if set_segment_of(id) != Some(segment) {
                continue;
            }
            let prefix = slug_prefix_of(id);
            if basename != prefix && !basename.starts_with(&format!("{prefix}-")) {
                continue;
            }
            if best_length.is_none_or(|length| prefix.len() > length) {
                best = Some(id);
                best_length = Some(prefix.len());
            }
        }
        best.cloned()
    }

    /// A path's folder and basename.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct RelPath<'a> {
        pub folder: &'a str,
        pub basename: &'a str,
    }

    /// `classic/043-plague-nuke` -> its folder and basename; a Core path has the folder `""`.
    pub fn split_rel_path(rel_path: &str) -> RelPath<'_> {
        match rel_path.rfind('/') {
            None => RelPath {
                folder: "",
                basename: rel_path,
            },
            Some(cut) => RelPath {
                folder: &rel_path[..cut],
                basename: &rel_path[cut + 1..],
            },
        }
    }

    /// Which catalog id a path under `src/scripts/` or `test/` (no extension) belongs to.
    pub fn resolve_rel_path(rel_path: &str, all_ids: &[String]) -> Option<String> {
        let RelPath { folder, basename } = split_rel_path(rel_path);
        resolve_basename(basename, all_ids, folder)
    }

    /// A filename prefix as a sortable number, mirroring the engine's index ranking: `001` -> 1,
    /// `051-1` -> 51.1 (so a card-defined token sorts straight after its card), `t-rush` -> +Infinity
    /// (shared tokens sort last within their set).
    pub fn prefix_rank(prefix: &str) -> f64 {
        let (main, sub) = match prefix.split_once('-') {
            None => (prefix, None),
            Some((main, sub)) => (main, Some(sub)),
        };
        if !is_digits(main) || sub.is_some_and(|sub| !is_digits(sub)) {
            return f64::INFINITY;
        }
        let text = match sub {
            None => main.to_string(),
            Some(sub) => format!("{main}.{sub}"),
        };
        let rank = text.parse::<f64>().unwrap_or(f64::INFINITY);
        if rank.is_finite() { rank } else { f64::INFINITY }
    }

    /// A sort key: `[named-no-card tier, set, §5 index rank, path]`.
    pub type SortKey = (usize, usize, f64, String);

    /// The deterministic ordering every tool uses: catalog order — set (Core, Classic, Classic+),
    /// then SPEC §5 index ascending, card-defined tokens after their card, shared tokens last within
    /// the set — and anything that names no catalog card after all of it; then the path, so the
    /// order never depends on the order the filesystem listed a directory.
    pub fn sort_key(path: &str, id: Option<&str>) -> SortKey {
        let tier = usize::from(id.is_none());
        let set = set_rank(id);
        let rank = id.map_or(f64::INFINITY, |id| prefix_rank(slug_prefix_of(id)));
        (tier, set, rank, path.to_string())
    }

    /// Compares two `sort_key`s.
    pub fn compare_sort_keys(a: &SortKey, b: &SortKey) -> Ordering {
        if a.0 != b.0 {
            return a.0.cmp(&b.0);
        }
        if a.1 != b.1 {
            return a.1.cmp(&b.1);
        }
        if a.2 != b.2 {
            // Infinity - Infinity is NaN, so compare, don't subtract
            return if a.2 < b.2 {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        if a.3 == b.3 {
            return Ordering::Equal;
        }
        if a.3 < b.3 {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }

    /// A legal, unique JS identifier for the namespace import of a script file: paths are unique
    /// within `src/scripts/`, the folder is part of the alias (`classic/043-…` -> `mclassic_043_…`),
    /// and the `m` guard keeps `001-…` from starting an identifier with a digit.
    pub fn module_alias_of(rel_path: &str) -> String {
        let mut out = String::from("m");
        let mut in_run = false;
        for c in rel_path.chars() {
            if c.is_ascii_alphanumeric() {
                out.push(c);
                in_run = false;
            } else if !in_run {
                out.push('_');
                in_run = true;
            }
        }
        out
    }
}

/* patch */

/// Every file that carries the catalog version (B4.2, R388, R646), and how to rewrite it there.
/// `patches ship`'s promotion bumps them to the newest shipped patch; the patch tests hold all of them
/// to it. A deployment then reseeds: the server's `release` stamps every `cards` row and
/// `app.settings.catalog_version` with the new version.
///
/// A micro `vA.B.Y` (R650) keeps its `Y` in the fragment until promotion names it (`versions`).
/// Branches add a fragment under `patches/pending/` rather than writing the history (R646).
///
/// Every binary compiles the catalog version in from `crates/cards/patches/patches.json` (SURFACE
/// §11.3). What still carries the string is the server's `.env.example`, `render.yaml` (whose value
/// the server checks against its own at boot) and, when the web build names it,
/// `apps/web/.env.production`'s `VITE_CATALOG_VERSION`.
pub mod patch {
    use std::fs;
    use std::path::Path;

    use anyhow::{Context as _, bail};

    use super::js::is_line_terminator;
    use jackioh_engine::wire::is_js_space;

    /// How a site's pattern finds the version.
    #[derive(Clone, Copy, Debug)]
    pub enum SitePattern {
        /// The first line that starts with the key (`CATALOG_VERSION=`); the key is kept.
        Line(&'static str),
        /// render.yaml's env entry: `- key: CATALOG_VERSION`, then `value: <version>`.
        RenderValue,
    }

    /// One file that carries the catalog version. `optional` sites are rewritten when they carry
    /// it and skipped when they do not (the web's `.env.production` names it only if a build reads
    /// it); every other site must carry it.
    #[derive(Clone, Copy, Debug)]
    pub struct VersionSite {
        pub file: &'static str,
        pub pattern: SitePattern,
        pub optional: bool,
    }

    pub const VERSION_SITES: &[VersionSite] = &[
        VersionSite {
            file: "crates/server/.env.example",
            pattern: SitePattern::Line("CATALOG_VERSION="),
            optional: false,
        },
        VersionSite {
            file: "render.yaml",
            pattern: SitePattern::RenderValue,
            optional: false,
        },
        VersionSite {
            file: "apps/web/.env.production",
            pattern: SitePattern::Line("VITE_CATALOG_VERSION="),
            optional: true,
        },
    ];

    /// One match of a site's pattern: its byte span, and where the version starts inside it (everything before is kept).
    struct SiteMatch {
        start: usize,
        value_start: usize,
        end: usize,
    }

    const RENDER_KEY: &str = "- key: CATALOG_VERSION\n";
    const RENDER_VALUE: &str = "value: ";

    impl SitePattern {
        fn locate(self, text: &str) -> Option<SiteMatch> {
            match self {
                SitePattern::Line(key) => locate_line(text, key),
                SitePattern::RenderValue => locate_render_value(text),
            }
        }
    }

    /// The first line holding `key` at its start (the text's start or just after a line terminator), up to its terminator.
    fn locate_line(text: &str, key: &str) -> Option<SiteMatch> {
        let mut starts = vec![0];
        for (at, c) in text.char_indices() {
            if is_line_terminator(c) {
                starts.push(at + c.len_utf8());
            }
        }
        starts
            .into_iter()
            .find(|start| text[*start..].starts_with(key))
            .map(|start| {
                let value_start = start + key.len();
                let end = text[value_start..]
                    .char_indices()
                    .find(|(_, c)| is_line_terminator(*c))
                    .map_or(text.len(), |(at, _)| value_start + at);
                SiteMatch {
                    start,
                    value_start,
                    end,
                }
            })
    }

    /// `/(- key: CATALOG_VERSION\n\s+value: )\S+/`. `\s+` is greedy and `value: ` opens with a
    /// letter, so the only run that can work is the whole run of white space.
    fn locate_render_value(text: &str) -> Option<SiteMatch> {
        for (start, _) in text.match_indices(RENDER_KEY) {
            let after_key = start + RENDER_KEY.len();
            let spaces = text[after_key..]
                .char_indices()
                .find(|(_, c)| !is_js_space(*c))
                .map_or(text.len() - after_key, |(at, _)| at);
            if spaces == 0 {
                continue;
            }
            let label = after_key + spaces;
            if !text[label..].starts_with(RENDER_VALUE) {
                continue;
            }
            let value_start = label + RENDER_VALUE.len();
            let value_length = text[value_start..]
                .char_indices()
                .find(|(_, c)| is_js_space(*c))
                .map_or(text.len() - value_start, |(at, _)| at);
            if value_length == 0 {
                continue;
            }
            return Some(SiteMatch {
                start,
                value_start,
                end: value_start + value_length,
            });
        }
        None
    }

    fn non_space_length(text: &str) -> usize {
        text.char_indices()
            .find(|(_, c)| is_js_space(*c))
            .map_or(text.len(), |(at, _)| at)
    }

    /// The version a site's match carries: the first alternative (`"v"`, `=v` or `value: v`) matching at the leftmost position.
    fn version_in(matched: &str) -> Option<String> {
        for (at, _) in matched.char_indices() {
            let rest = &matched[at..];
            if let Some(inner) = rest.strip_prefix('"')
                && let Some(length) = inner.find('"')
                && length > 0
            {
                return Some(inner[..length].to_string());
            }
            if let Some(inner) = rest.strip_prefix('=')
                && non_space_length(inner) > 0
            {
                return Some(inner[..non_space_length(inner)].to_string());
            }
            if let Some(inner) = rest.strip_prefix(RENDER_VALUE)
                && non_space_length(inner) > 0
            {
                return Some(inner[..non_space_length(inner)].to_string());
            }
        }
        None
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct SiteVersion {
        pub file: &'static str,
        pub version: Option<String>,
    }

    /// The version each site carries now, or `None` where the pattern finds none. An optional site
    /// that is missing or carries no version is left out.
    pub fn versions_at_sites(repo_root: &Path) -> anyhow::Result<Vec<SiteVersion>> {
        let mut out = Vec::new();
        for site in VERSION_SITES {
            let path = repo_root.join(site.file);
            let text = match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(_) if site.optional => continue,
                Err(error) => return Err(error).with_context(|| format!("{}: cannot be read", site.file)),
            };
            let found = site.pattern.locate(&text);
            if found.is_none() && site.optional {
                continue;
            }
            let version = found.and_then(|found| version_in(&text[found.start..found.end]));
            out.push(SiteVersion {
                file: site.file,
                version,
            });
        }
        Ok(out)
    }

    /// Rewrites `CATALOG_VERSION` everywhere the string lives (promotion bumps it, R646). Every site
    /// is read and matched before the first one is written, so a missing site leaves all of them
    /// alone.
    pub fn bump_sites(version: &str, repo_root: &Path) -> anyhow::Result<()> {
        let mut rewritten = Vec::new();
        for site in VERSION_SITES {
            let path = repo_root.join(site.file);
            let text = match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(_) if site.optional => continue,
                Err(error) => return Err(error).with_context(|| format!("{}: cannot be read", site.file)),
            };
            let Some(found) = site.pattern.locate(&text) else {
                if site.optional {
                    continue;
                }
                bail!("{}: no CATALOG_VERSION to rewrite", site.file);
            };
            let mut next = String::with_capacity(text.len() + version.len());
            next.push_str(&text[..found.value_start]);
            next.push_str(version);
            next.push_str(&text[found.end..]);
            rewritten.push((path, next));
        }
        for (path, text) in rewritten {
            fs::write(&path, text).with_context(|| format!("{} cannot be written", path.display()))?;
        }
        Ok(())
    }
}

/* patches_io */

/// The card patch history on disk (B4.2, R388, R646): `crates/cards/patches/`.
///
/// ```text
///   patches.json       every shipped patch in ship order: { version, date, title, source, notes, changes }
///   <version>.json     the whole catalog as that patch left it (a snapshot, not a diff)
///   index.json         GENERATED: for each card id, the versions in which it was added or changed
///   shipped.json       every shipped patch's { version, commit, blob }: the commit that shipped it
///                      and its snapshot file's git blob hash, so the workflow writes data, not source
///   pending/           one fragment per patch being built: { version, title, sources, notes, cards }
/// ```
///
/// The order of patches is `patches.json`'s order and nothing else: a version is an opaque string,
/// compared for equality only and never parsed or sorted (R105, R388). `changes` and `index.json`
/// are derived from the snapshots by `rebuild_derived()`. Shipped snapshots are never amended:
/// branches add a fragment under `pending/` and `patches ship` promotes it after it merges (R646).
/// I/O lives in the tools (CLAUDE.md rule 4); the patch tests read the same files through these.
pub mod patches_io {
    use std::fs;
    use std::path::{Path, PathBuf};

    use anyhow::{Context as _, anyhow, bail};
    use indexmap::{IndexMap, IndexSet};
    use serde::{Deserialize, Serialize};

    use super::js::{self, Json, Object};

    /// `crates/cards/patches/`.
    pub fn patches_dir() -> PathBuf {
        super::repo_root().join("crates/cards/patches")
    }

    pub fn patches_json() -> PathBuf {
        patches_dir().join("patches.json")
    }

    pub fn index_json() -> PathBuf {
        patches_dir().join("index.json")
    }

    /// Pending fragments: one file per patch being built, never edited after it ships (R646).
    pub fn pending_dir() -> PathBuf {
        patches_dir().join("pending")
    }

    /// Every shipped patch's provenance: the commit that shipped it and its snapshot's blob (R646).
    pub fn shipped_json() -> PathBuf {
        patches_dir().join("shipped.json")
    }

    /// A catalog as a snapshot holds it: entries by id, each entry's fields in the file's order.
    pub type Catalog = IndexMap<String, Object>;

    #[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
    #[serde(rename_all = "camelCase")]
    pub enum ChangeKind {
        Added,
        Removed,
        Changed,
    }

    /// One card's line in a patch: added, changed (with the fields that moved) or removed. A struct,
    /// not a tagged enum, so it writes `{ id, name, kind, fields }` in the order `patches.json`
    /// carries (serde writes an internal tag first). `fields` is present exactly on `changed` lines.
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct PatchChange {
        pub id: String,
        pub name: String,
        pub kind: ChangeKind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub fields: Option<Vec<String>>,
    }

    impl PatchChange {
        pub fn added(id: &str, name: &str) -> PatchChange {
            PatchChange {
                id: id.to_string(),
                name: name.to_string(),
                kind: ChangeKind::Added,
                fields: None,
            }
        }

        pub fn removed(id: &str, name: &str) -> PatchChange {
            PatchChange {
                id: id.to_string(),
                name: name.to_string(),
                kind: ChangeKind::Removed,
                fields: None,
            }
        }

        pub fn changed(id: &str, name: &str, fields: Vec<String>) -> PatchChange {
            PatchChange {
                id: id.to_string(),
                name: name.to_string(),
                kind: ChangeKind::Changed,
                fields: Some(fields),
            }
        }

        /// The fields a `changed` line lists; none for the other kinds.
        pub fn fields(&self) -> &[String] {
            self.fields.as_deref().unwrap_or(&[])
        }
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct PatchEntry {
        /// The catalog version this patch set, e.g. "v0.2.0" (R388). Opaque (R105).
        pub version: String,
        /// The day it shipped, YYYY-MM-DD.
        pub date: String,
        pub title: String,
        /// Where it came from: the issue, the PR, the commits.
        pub source: String,
        /// What the patch did, in the designer's and the players' words.
        pub notes: String,
        /// The first-parent commits that shipped this patch; new promotions carry one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub commits: Option<Vec<String>>,
        /// Whether the entry was rebuilt from historical catalog data instead of promoted.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub reconstructed: Option<bool>,
        /// GENERATED from the snapshots: every card the patch added, changed or removed, in catalog
        /// order.
        #[serde(default)]
        pub changes: Vec<PatchChange>,
    }

    /// `/^[A-Za-z0-9][A-Za-z0-9._-]*$/`: a name a snapshot file may carry.
    fn is_snapshot_name(version: &str) -> bool {
        let mut bytes = version.bytes();
        bytes.next().is_some_and(|b| b.is_ascii_alphanumeric())
            && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    }

    pub fn snapshot_path(version: &str, dir: &Path) -> anyhow::Result<PathBuf> {
        if !is_snapshot_name(version) {
            bail!("\"{version}\" is not a patch version (letters, digits, \".\", \"_\" and \"-\")");
        }
        Ok(dir.join(format!("{version}.json")))
    }

    pub fn read_patches(path: &Path) -> anyhow::Result<Vec<PatchEntry>> {
        let text = fs::read_to_string(path).with_context(|| format!("{} cannot be read", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("{} is not a patch list", path.display()))
    }

    pub fn read_snapshot(version: &str, dir: &Path) -> anyhow::Result<Catalog> {
        let path = snapshot_path(version, dir)?;
        let text = fs::read_to_string(&path).with_context(|| format!("{} cannot be read", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("{} is not a catalog", path.display()))
    }

    /// Writes pretty JSON and a newline through a temporary file, so a reader never sees half a file.
    pub fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> anyhow::Result<()> {
        let mut temp = path.as_os_str().to_owned();
        temp.push(format!(".{}.tmp", std::process::id()));
        let temp = PathBuf::from(temp);
        fs::write(&temp, format!("{}\n", js::stringify_pretty(value)))
            .with_context(|| format!("{} cannot be written", temp.display()))?;
        fs::rename(&temp, path).with_context(|| format!("{} cannot be written", path.display()))
    }

    /// Whether two values stringify equal, a missing value being equal only to another.
    fn same(a: Option<&Json>, b: Option<&Json>) -> bool {
        a.map(js::stringify) == b.map(js::stringify)
    }

    /// Fills a raw entry's face text from the entry's printed values: `{key|singular|plural}` takes the
    /// singular wording at 1 and the plural at any other value; unknown keys are left as written. A
    /// private copy (the engine's `fill_params` takes a typed `CardDef`, which a snapshot entry, or a
    /// test's partial one, need not deserialise into).
    fn fill_params(def: &Object, face: &str) -> String {
        let text = def
            .get(face)
            .and_then(Json::as_object)
            .and_then(|face| face.get("text"))
            .and_then(Json::as_str)
            .unwrap_or_default();
        let params = match def.get("params").and_then(Json::as_array) {
            Some(params) if !params.is_empty() => params,
            _ => return text.to_string(),
        };
        let mut out = String::with_capacity(text.len());
        let mut at = 0;
        for placeholder in js::placeholder_spans(text) {
            out.push_str(&text[at..placeholder.start]);
            let param = params.iter().find(|param| {
                param
                    .as_object()
                    .and_then(|param| param.get("key"))
                    .and_then(Json::as_str)
                    == Some(placeholder.key)
            });
            match param {
                None => out.push_str(&text[placeholder.start..placeholder.end]),
                Some(param) => {
                    let value = param.as_object().and_then(|param| param.get(face));
                    out.push_str(&js::to_js_string(value));
                    if let Some((one, many)) = placeholder.words {
                        out.push(' ');
                        out.push_str(if value == Some(&Json::Number(1.0)) {
                            one
                        } else {
                            many
                        });
                    }
                }
            }
            at = placeholder.end;
        }
        out.push_str(&text[at..]);
        out
    }

    /// The fields of one entry that differ, one level into its faces ("base.text", "cost"). A face's
    /// text is compared as it prints, its `{key}` numbers filled in (`fillParams`, as the
    /// patch-notes diff does): a patch that moves only a param's value still rewords the faces that
    /// print it, so the face counts as changed.
    pub fn changed_fields(before: &Object, after: &Object) -> Vec<String> {
        let mut fields = Vec::new();
        let keys: IndexSet<&String> = before.keys().chain(after.keys()).collect();
        for key in keys {
            let face = matches!(key.as_str(), "base" | "radiant");
            let face_a = before.get(key).and_then(Json::as_object);
            let face_b = after.get(key).and_then(Json::as_object);
            if let (true, Some(face_a), Some(face_b)) = (face, face_a, face_b) {
                let inner: IndexSet<&String> = face_a.keys().chain(face_b.keys()).collect();
                for field in inner {
                    let (was, now) = (face_a.get(field), face_b.get(field));
                    if field == "text"
                        && was.and_then(Json::as_str).is_some()
                        && now.and_then(Json::as_str).is_some()
                    {
                        if fill_params(before, key) != fill_params(after, key) {
                            fields.push(format!("{key}.{field}"));
                        }
                    } else if !same(was, now) {
                        fields.push(format!("{key}.{field}"));
                    }
                }
            } else if !same(before.get(key), after.get(key)) {
                fields.push(key.clone());
            }
        }
        fields
    }

    /// What `after` changed against `before`, in `after`'s order, removals last.
    pub fn diff_catalogs(before: Option<&Catalog>, after: &Catalog) -> Vec<PatchChange> {
        let name_of = |def: Option<&Object>, id: &str| -> String {
            def.and_then(|def| def.get("name"))
                .and_then(Json::as_str)
                .unwrap_or(id)
                .to_string()
        };
        let mut out = Vec::new();
        for (id, def) in after {
            match before.and_then(|before| before.get(id)) {
                None => out.push(PatchChange::added(id, &name_of(Some(def), id))),
                Some(prev) => {
                    let fields = changed_fields(prev, def);
                    if !fields.is_empty() {
                        out.push(PatchChange::changed(id, &name_of(Some(def), id), fields));
                    }
                }
            }
        }
        if let Some(before) = before {
            for (id, def) in before {
                if !after.contains_key(id) {
                    out.push(PatchChange::removed(id, &name_of(Some(def), id)));
                }
            }
        }
        out
    }

    /// For each card id, the versions in which it was added or changed, in patch order.
    pub fn build_index(patches: &[PatchEntry]) -> IndexMap<String, Vec<String>> {
        let mut index: IndexMap<String, Vec<String>> = IndexMap::new();
        for patch in patches {
            for change in &patch.changes {
                if change.kind == ChangeKind::Removed {
                    continue;
                }
                index
                    .entry(change.id.clone())
                    .or_default()
                    .push(patch.version.clone());
            }
        }
        index
    }

    /// Recomputes every patch's `changes` from the snapshots, in patches.json's order, and rewrites
    /// `index.json`. Returns the patches as written.
    pub fn rebuild_derived(dir: &Path) -> anyhow::Result<Vec<PatchEntry>> {
        let patches = read_patches(&dir.join("patches.json"))?;
        let mut previous: Option<Catalog> = None;
        let mut next = Vec::with_capacity(patches.len());
        for patch in patches {
            let snapshot = read_snapshot(&patch.version, dir)?;
            let changes = diff_catalogs(previous.as_ref(), &snapshot);
            previous = Some(snapshot);
            next.push(PatchEntry { changes, ..patch });
        }
        write_json(&dir.join("patches.json"), &next)?;
        write_json(&dir.join("index.json"), &build_index(&next))?;
        Ok(next)
    }

    // Pending fragments and the shipped list (R646): several card patches are built on separate branches
    // at once, so branches never edit `patches.json`, the snapshots or the shipped list. A branch adds one
    // fragment claiming the catalog ids its patch touches; `patches check` proves the claims and `patches
    // ship` promotes each fragment in ship order after it merges.

    /// A patch not yet shipped: the designer's label, what it does, and the catalog ids it touches.
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct PendingFragment {
        /// The designer's label, a bare patch number ("v0.2.5") or a micro "vA.B.Y" named at
        /// promotion (R650), never a revision ("v0.2.0b").
        pub version: String,
        pub title: String,
        /// Where the patch came from: the issue, the PR, the commits. One string, as `source` below.
        pub sources: String,
        /// What the patch does, in the designer's and the players' words.
        pub notes: String,
        /// The catalog ids the patch creates, changes or removes, in catalog order.
        pub cards: Vec<String>,
    }

    /// One shipped patch's provenance: the commit that shipped it and its snapshot's blob.
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct ShippedEntry {
        /// The patch's version as `patches.json` lists it, revisions ("v0.2.0b") included.
        pub version: String,
        /// The first-parent commit on main that shipped it (for old patches, the one that holds them).
        pub commit: String,
        /// The git blob hash (`git hash-object`) of its snapshot file's bytes.
        pub blob: String,
    }

    /// A fragment's version is a bare patch number or a micro `vA.B.Y` (R646, R650): "v0.2.5" or
    /// "v0.2.Y", never a revision ("v0.2.0b") or a placeholder ("v0.2.X": the designer picks the X
    /// before the patch is made).
    pub fn is_fragment_version(version: &str) -> bool {
        super::versions::split_version(version).is_some_and(|(_, _, tail)| tail == "Y" || js::is_digits(tail))
    }

    pub fn pending_path(version: &str, dir: &Path) -> anyhow::Result<PathBuf> {
        if !is_fragment_version(version) {
            bail!(
                "\"{version}\" is not a pending fragment (a bare patch number like \"v0.2.5\", or a micro \"v0.2.Y\")"
            );
        }
        Ok(dir.join(format!("{version}.json")))
    }

    fn is_fragment(value: &Json) -> bool {
        let Json::Object(fragment) = value else {
            return false;
        };
        let string = |key: &str| matches!(fragment.get(key), Some(Json::String(_)));
        string("version")
            && string("title")
            && string("sources")
            && string("notes")
            && matches!(fragment.get("cards"), Some(Json::Array(cards)) if cards.iter().all(|id| matches!(id, Json::String(_))))
    }

    /// One fragment file: its name under `pending/` and what it holds.
    #[derive(Clone, Debug, PartialEq)]
    pub struct NamedFragment {
        pub name: String,
        pub fragment: PendingFragment,
    }

    /// Every pending fragment by file name, or [] when `pending/` does not exist yet.
    pub fn read_fragments(dir: &Path) -> anyhow::Result<Vec<NamedFragment>> {
        let Ok(listing) = fs::read_dir(dir) else {
            return Ok(Vec::new());
        };
        let mut names: Vec<String> = listing
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        let mut out = Vec::new();
        for name in names {
            if !name.ends_with(".json") || name.starts_with('.') {
                continue;
            }
            let path = dir.join(&name);
            let text = fs::read_to_string(&path).with_context(|| format!("pending/{name} cannot be read"))?;
            let value = js::parse(&text).with_context(|| format!("pending/{name} is not JSON"))?;
            if !is_fragment(&value) {
                bail!("pending/{name} is not a fragment ({{ version, title, sources, notes, cards }})");
            }
            let fragment: PendingFragment =
                serde_json::from_str(&text).with_context(|| format!("pending/{name} is not a fragment"))?;
            out.push(NamedFragment { name, fragment });
        }
        Ok(out)
    }

    fn is_shipped_entry(value: &Json) -> bool {
        let Json::Object(entry) = value else {
            return false;
        };
        let string = |key: &str| matches!(entry.get(key), Some(Json::String(_)));
        string("version") && string("commit") && string("blob")
    }

    pub fn read_shipped(path: &Path) -> anyhow::Result<Vec<ShippedEntry>> {
        let text = fs::read_to_string(path).with_context(|| format!("{} cannot be read", path.display()))?;
        let entries = js::parse(&text).with_context(|| format!("{} is not JSON", path.display()))?;
        let well_formed = matches!(&entries, Json::Array(entries) if entries.iter().all(is_shipped_entry));
        if !well_formed {
            bail!("shipped.json is not a list of {{ version, commit, blob }}");
        }
        serde_json::from_str(&text)
            .map_err(|_| anyhow!("shipped.json is not a list of {{ version, commit, blob }}"))
    }

    /// Removes a file that is there, and never complains about one that is not.
    pub fn remove_file(path: &Path) {
        // Already gone: promotion deletes each fragment once, and a second `ship` finds none.
        let _ = fs::remove_file(path);
    }

    /// SHA-1 (FIPS 180-4) in lower-case hex: the tools take no hash crate (SURFACE §2), and git names a blob by this digest.
    fn sha1_hex(data: &[u8]) -> String {
        let mut state: [u32; 5] = [0x6745_2301, 0xEFCD_AB89, 0x98BA_DCFE, 0x1032_5476, 0xC3D2_E1F0];
        let bit_length = (data.len() as u64).wrapping_mul(8);
        let mut message = data.to_vec();
        message.push(0x80);
        while message.len() % 64 != 56 {
            message.push(0);
        }
        message.extend_from_slice(&bit_length.to_be_bytes());
        for block in message.chunks_exact(64) {
            let mut w = [0u32; 80];
            for (i, word) in block.chunks_exact(4).enumerate() {
                w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
            }
            for i in 16..80 {
                w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
            }
            let [mut a, mut b, mut c, mut d, mut e] = state;
            for (i, word) in w.iter().enumerate() {
                let (f, k) = match i {
                    0..=19 => ((b & c) | (!b & d), 0x5A82_7999_u32),
                    20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                    40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                    _ => (b ^ c ^ d, 0xCA62_C1D6),
                };
                let next = a
                    .rotate_left(5)
                    .wrapping_add(f)
                    .wrapping_add(e)
                    .wrapping_add(k)
                    .wrapping_add(*word);
                e = d;
                d = c;
                c = b.rotate_left(30);
                b = a;
                a = next;
            }
            for (slot, value) in state.iter_mut().zip([a, b, c, d, e]) {
                *slot = slot.wrapping_add(value);
            }
        }
        state.iter().map(|word| format!("{word:08x}")).collect()
    }

    /// The git blob hash of a file's text: the sha1 of `blob <bytes>\0<text>`, as `git hash-object`
    /// prints it. `shipped.json` carries one per snapshot, so a rewritten snapshot fails the proof.
    pub fn git_blob_hash(text: &str) -> String {
        let mut data = format!("blob {}\0", text.len()).into_bytes();
        data.extend_from_slice(text.as_bytes());
        sha1_hex(&data)
    }

    /// The name a fragment ships under (R646): its own version, unless that version already shipped,
    /// in which case the next revision letter — the first revision of v0.2.0 is "v0.2.0b", then
    /// "v0.2.0c" (`docs/issues-and-patches.md`). A shipped version never reopens.
    pub fn next_ship_name(taken: &IndexSet<String>, version: &str) -> anyhow::Result<String> {
        if !taken.contains(version) {
            return Ok(version.to_string());
        }
        for letter in 'b'..='z' {
            let revision = format!("{version}{letter}");
            if !taken.contains(&revision) {
                return Ok(revision);
            }
        }
        bail!("no free revision letter for \"{version}\" (b through z are all shipped)")
    }

    /// The ids whose entry is not the same bytes in both catalogs (added, removed, or any field moved,
    /// key order and a reworded `{key}` included), in `after`'s order, removals last. This is what a
    /// fragment claims: `diff_catalogs` reports changes the way players read them, but every entry that
    /// would make the next snapshot differ must belong to a patch. `same_catalog` agrees with this one.
    pub fn differing_ids(before: &Catalog, after: &Catalog) -> Vec<String> {
        let mut out: Vec<String> = after
            .iter()
            .filter(|(id, def)| before.get(*id).map(js::stringify) != Some(js::stringify(*def)))
            .map(|(id, _)| id.clone())
            .collect();
        for id in before.keys() {
            if !after.contains_key(id) {
                out.push(id.clone());
            }
        }
        out
    }

    /// Two catalogs hold the same entries, key order aside.
    pub fn same_catalog(a: &Catalog, b: &Catalog) -> bool {
        if a.len() != b.len() {
            return false;
        }
        a.iter().all(|(id, def)| {
            b.get(id)
                .is_some_and(|other| js::stringify(def) == js::stringify(other))
        })
    }

    /// The working catalog with every pending-claimed entry reverted to the newest shipped
    /// snapshot: entries the snapshots hold come back, created ones go away. With no fragments it
    /// is the catalog as it stands.
    pub fn revert_pending(catalog: &Catalog, newest: &Catalog, claimed: &IndexSet<String>) -> Catalog {
        let mut out = Catalog::new();
        for (id, def) in catalog {
            if !claimed.contains(id) {
                out.insert(id.clone(), def.clone());
            }
        }
        for (id, def) in newest {
            if claimed.contains(id) {
                out.insert(id.clone(), def.clone());
            }
        }
        out
    }

    /// The input of `check_fragments`: the fragment files, the working catalog and the newest shipped snapshot.
    pub struct CheckFragmentsArgs<'a> {
        pub files: &'a [NamedFragment],
        pub catalog: &'a Catalog,
        pub newest: &'a Catalog,
    }

    /// Every way the pending fragments disagree with the newest shipped snapshot, each naming the card
    /// (and the fragment) at fault, or [] when the tree is shippable; the rules are `patches check`'s
    /// (module header). Pure: `patches check` reads the files and prints what this returns.
    pub fn check_fragments(args: &CheckFragmentsArgs<'_>) -> Vec<String> {
        let mut problems = Vec::new();
        for NamedFragment { name, fragment } in args.files {
            if !is_fragment_version(&fragment.version) {
                problems.push(format!(
                    "pending/{name} names version \"{}\", not a fragment version (a bare patch number ^v\\d+\\.\\d+\\.\\d+$ or a micro vA.B.Y)",
                    fragment.version
                ));
            }
            if *name != format!("{}.json", fragment.version) {
                problems.push(format!(
                    "pending/{name} holds version \"{}\", not the version its file names",
                    fragment.version
                ));
            }
            // The fragment's fields become the shipped patch's, which must carry all three (R388).
            for (field, value) in [
                ("title", &fragment.title),
                ("sources", &fragment.sources),
                ("notes", &fragment.notes),
            ] {
                if value.trim().is_empty() {
                    problems.push(format!(
                        "pending/{name} has an empty {field}, but a shipped patch needs one"
                    ));
                }
            }
            // A patch is a catalog change; one with none would bump the version over nothing (R650).
            if fragment.cards.is_empty() {
                problems.push(format!(
                    "pending/{name} claims no cards, but a patch ships a catalog change"
                ));
            }
        }
        let differed: IndexSet<String> = differing_ids(args.newest, args.catalog).into_iter().collect();
        let mut claimants: IndexMap<String, Vec<String>> = IndexMap::new();
        for NamedFragment { fragment, .. } in args.files {
            for id in &fragment.cards {
                let versions = claimants.entry(id.clone()).or_default();
                if !versions.contains(&fragment.version) {
                    versions.push(fragment.version.clone());
                }
            }
        }
        let mut sorted_differed: Vec<&String> = differed.iter().collect();
        sorted_differed.sort();
        for id in sorted_differed {
            let versions = claimants.get(id).map_or(&[][..], Vec::as_slice);
            if versions.is_empty() {
                problems.push(format!(
                    "\"{id}\" differs from the newest shipped snapshot but no pending fragment claims it"
                ));
            } else if versions.len() > 1 {
                let named: Vec<String> = versions.iter().map(|version| format!("\"{version}\"")).collect();
                problems.push(format!(
                    "\"{id}\" is claimed by {}, but one card ships in one patch",
                    named.join(" and ")
                ));
            }
        }
        let mut sorted_claimants: Vec<(&String, &Vec<String>)> = claimants.iter().collect();
        sorted_claimants.sort_by(|a, b| a.0.cmp(b.0));
        for (id, versions) in sorted_claimants {
            if !differed.contains(id) {
                problems.push(format!(
                    "\"{id}\" is claimed by \"{}\" but identical to the newest shipped snapshot",
                    versions[0]
                ));
            }
        }
        problems
    }
}

/* versions */

/// A patch's version from the name an issue gave it (docs/issues-and-patches.md, Version numbers; R650).
///
/// - `vA.B.Y` is a **micro patch**: it ships as the newest version in `patches.json` (the last
///   entry, R388's order) with the next letter after it, so a micro patch made after v0.2.5 is
///   v0.2.5b, and one after v0.2.7c is v0.2.7d. The newest version must be an `A.B` one: a `v0.2.Y`
///   cannot follow a v0.3.1.
/// - Any other name is the designer's and is used as given (`vA.B.X` must be replaced first).
///
/// This is the one place a version string is read, and only here, when a patch ships (a fragment
/// keeps its `Y` until `patches ship` promotes it, R646): R105 still holds everywhere else, where
/// a version is compared for equality and never parsed or ordered.
pub mod versions {
    use anyhow::bail;

    use super::js::is_digits;

    /// `^v(\d+)\.(\d+)\.` and the rest: a version's A, B and what follows the second dot.
    pub(crate) fn split_version(text: &str) -> Option<(&str, &str, &str)> {
        let rest = text.strip_prefix('v')?;
        let (a, rest) = rest.split_once('.')?;
        let (b, tail) = rest.split_once('.')?;
        (is_digits(a) && is_digits(b)).then_some((a, b, tail))
    }

    /// `MICRO = /^v(\d+)\.(\d+)\.Y$/`: a micro patch's A and B.
    fn micro_parts(text: &str) -> Option<(&str, &str)> {
        split_version(text).and_then(|(a, b, tail)| (tail == "Y").then_some((a, b)))
    }

    /// `PLACEHOLDER = /^v\d+\.\d+\.X$/`.
    fn is_placeholder(text: &str) -> bool {
        split_version(text).is_some_and(|(_, _, tail)| tail == "X")
    }

    /// `SHIPPED = /^v(\d+)\.(\d+)\.(\d+)([a-z]?)$/`: the newest version: `vA.B.C`, optionally
    /// followed by one letter (b … z). Its A, B, C and the letter (`""` for none).
    fn shipped_parts(text: &str) -> Option<(&str, &str, &str, &str)> {
        let (a, b, tail) = split_version(text)?;
        let cut = if tail.as_bytes().last().is_some_and(u8::is_ascii_lowercase) {
            tail.len() - 1
        } else {
            tail.len()
        };
        let (c, letter) = tail.split_at(cut);
        is_digits(c).then_some((a, b, c, letter))
    }

    pub fn resolve_version(asked: &str, shipped: &[String]) -> anyhow::Result<String> {
        if is_placeholder(asked) {
            bail!("{asked}: the designer picks the X before the patch is made");
        }
        let Some((micro_a, micro_b)) = micro_parts(asked) else {
            return Ok(asked.to_string());
        };
        let newest = shipped.last();
        let parts = newest.and_then(|newest| shipped_parts(newest));
        let (Some(newest), Some((a, b, c, letter))) = (newest, parts) else {
            bail!(
                "{asked}: the newest version ({}) has no vA.B.C form to follow",
                newest.map_or("none", String::as_str)
            );
        };
        if a != micro_a || b != micro_b {
            bail!("{asked}: the newest version is {newest}, not a v{micro_a}.{micro_b} one");
        }
        if letter == "z" {
            bail!("{asked}: {newest} has no letter left after z");
        }
        let next = match letter.as_bytes().first() {
            None => 'b',
            Some(byte) => char::from(byte + 1),
        };
        Ok(format!("v{a}.{b}.{c}{next}"))
    }
}

/* patches (the command) */

const CATALOG_REL: &str = "crates/cards/catalog.json";
const PATCHES_REL: &str = "crates/cards/patches";

/// Every patches path under one root, so tests promote a fixture repo instead of this one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatchPaths {
    pub dir: PathBuf,
    pub patches_json: PathBuf,
    pub shipped_json: PathBuf,
    pub pending_dir: PathBuf,
    pub catalog: PathBuf,
}

/// Every patches path under one root, so tests promote a fixture repo instead of this one.
pub fn patch_paths(repo_root: &Path) -> PatchPaths {
    let dir = repo_root.join(PATCHES_REL);
    PatchPaths {
        patches_json: dir.join("patches.json"),
        shipped_json: dir.join("shipped.json"),
        pending_dir: dir.join("pending"),
        catalog: repo_root.join(CATALOG_REL),
        dir,
    }
}

/// `catalog.json` as a patch sees it: its shipped view (R1420).
fn read_catalog(path: &Path) -> anyhow::Result<Catalog> {
    let text = fs::read_to_string(path).with_context(|| format!("{} cannot be read", path.display()))?;
    let catalog: Catalog =
        serde_json::from_str(&text).with_context(|| format!("{} is not a catalog", path.display()))?;
    Ok(shipped_view(catalog))
}

/// R1420: whether a catalog entry's set ships (`SHIPPED_SETS`). An entry whose `set` is no set the
/// engine knows is kept, for `catalog check` to name.
fn entry_ships(entry: &js::Object) -> bool {
    let Some(set) = entry.get("set").and_then(js::Json::as_str) else {
        return true;
    };
    match SetName::ALL.iter().find(|known| known.as_str() == set) {
        Some(known) => set_ships(*known),
        None => true,
    }
}

/// R1420: the catalog a patch ships, every entry of a set that ships, in the file's order. A set the
/// catalog holds before it ships is in no snapshot and no fragment claims its cards; the patch that
/// adds it to `SHIPPED_SETS` claims every one of them at once.
pub fn shipped_view(catalog: Catalog) -> Catalog {
    catalog
        .into_iter()
        .filter(|(_, entry)| entry_ships(entry))
        .collect()
}

/// R1420: `catalog.json`'s text as a snapshot holds it. The text itself when every entry ships, so a
/// history of shipped sets alone is the file byte for byte; otherwise the shipped view written as
/// the catalog is (two-space JSON and the file's closing newline).
pub fn shipped_text(raw: &str) -> anyhow::Result<String> {
    let catalog: Catalog = serde_json::from_str(raw).context("the catalog is not a catalog")?;
    if catalog.values().all(entry_ships) {
        return Ok(raw.to_string());
    }
    let mut text = js::stringify_pretty(&shipped_view(catalog));
    if raw.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

/// The arguments of `write_fragment`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FragmentArgs {
    pub version: String,
    pub title: String,
    pub date: Option<String>,
    /// `--source`: where the patch came from. Stored on the fragment as `sources`.
    pub sources: Option<String>,
    pub notes: Option<String>,
    pub cards: Option<Vec<String>>,
}

/// Writes (or updates) the pending fragment for a patch being built. `cards` defaults to every
/// catalog id that differs from the newest shipped snapshot, in catalog order.
pub fn write_fragment(repo_root: &Path, args: &FragmentArgs) -> anyhow::Result<PendingFragment> {
    let paths = patch_paths(repo_root);
    let patches = read_patches(&paths.patches_json)?;
    let Some(newest) = patches.last() else {
        bail!("patches.json holds no shipped patch");
    };
    let catalog = read_catalog(&paths.catalog)?;
    let newest_snapshot = read_snapshot(&newest.version, &paths.dir)?;
    let cards = match &args.cards {
        Some(cards) => cards.clone(),
        None => differing_ids(&newest_snapshot, &catalog),
    };
    let fragment = PendingFragment {
        version: args.version.clone(),
        title: args.title.clone(),
        sources: args.sources.clone().unwrap_or_default(),
        notes: args.notes.clone().unwrap_or_default(),
        cards,
    };
    fs::create_dir_all(&paths.pending_dir)
        .with_context(|| format!("{} cannot be created", paths.pending_dir.display()))?;
    write_json(
        &patches_io::pending_path(&args.version, &paths.pending_dir)?,
        &fragment,
    )?;
    Ok(fragment)
}

/// Every way the tree is not shippable, each naming the card (and the fragment) at fault, or []
/// when `ship` would go through. Pure files in, strings out: `main` prints and exits on these.
pub fn check_patches(repo_root: &Path) -> anyhow::Result<Vec<String>> {
    let paths = patch_paths(repo_root);
    let patches = read_patches(&paths.patches_json)?;
    let Some(newest) = patches.last() else {
        bail!("patches.json holds no shipped patch");
    };
    let files = read_fragments(&paths.pending_dir)?;
    let catalog = read_catalog(&paths.catalog)?;
    let newest_snapshot = read_snapshot(&newest.version, &paths.dir)?;
    let mut problems = check_fragments(&CheckFragmentsArgs {
        files: &files,
        catalog: &catalog,
        newest: &newest_snapshot,
    });
    // A micro `vA.B.Y` is named after the newest patch when it ships (R650): fail now when no
    // patch it could follow is there, instead of failing mid-promotion.
    let versions: Vec<String> = patches.iter().map(|patch| patch.version.clone()).collect();
    for NamedFragment { name, fragment } in &files {
        if !is_fragment_version(&fragment.version) {
            continue;
        }
        if let Err(error) = resolve_version(&fragment.version, &versions) {
            problems.push(format!("pending/{name} cannot ship: {error}"));
        }
    }
    Ok(problems)
}

/// `git <args>` in `repo_root`, its stdout as text; a failing git is an error carrying its stderr.
pub(crate) fn git(repo_root: &Path, args: &[&str], extra_env: &[(&str, &str)]) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .envs(extra_env.iter().copied())
        .output()
        .with_context(|| format!("git {} could not run", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "git {} failed ({}): {}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout)
        .with_context(|| format!("git {} printed text that is not UTF-8", args.join(" ")))
}

/// The first-parent commit on main that added a file, or "" when history does not hold it.
pub fn adding_commit(repo_root: &Path, rel_path: &str) -> anyhow::Result<String> {
    let log = git(
        repo_root,
        &[
            "log",
            "--first-parent",
            "--format=%H",
            "--diff-filter=A",
            "--",
            rel_path,
        ],
        &[],
    )?;
    Ok(log
        .split('\n')
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .to_string())
}

/// A unix timestamp as a UTC YYYY-MM-DD. Arithmetic over days (Howard Hinnant's civil_from_days),
/// never the clock: the timestamp comes from the git history, not from now.
pub fn utc_date_of(unix_seconds: i64) -> String {
    let days = unix_seconds.div_euclid(86400) + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let full_year = if month <= 2 { year + 1 } else { year };
    format!("{full_year}-{month:02}-{day:02}")
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShipResult {
    pub shipped: Vec<String>,
}

/// One fragment on its way to shipping: its file, the commit that added it, and where that commit
/// sits on the first-parent line (0 is the newest).
struct Queued {
    name: String,
    fragment: PendingFragment,
    commit: String,
    at: usize,
}

/// A queued fragment with the name it ships under.
struct Planned {
    name: String,
    fragment: PendingFragment,
    commit: String,
    version: String,
}

/// A planned fragment with its commit's catalog, proved to change exactly the cards it claims.
struct Proved {
    plan: Planned,
    raw: String,
}

/// Promotes every pending fragment to a shipped patch, in the order of the first-parent commit that
/// added it (ship order, R646). The snapshot is `catalog.json` as that commit left it: the squash
/// commit, or the merge commit, a pull request lands on main as. Idempotent: with no fragments it
/// changes nothing.
///
/// Nothing is written until every fragment has been named and proved: each adding commit's catalog
/// must differ from the patch before it on exactly the cards its fragment claims, and the last one
/// must be the catalog as it stands. A claimed card changed again after its fragment merged, or a
/// fragment edited to claim more, fails here with the files untouched.
pub fn ship_patches(repo_root: &Path) -> anyhow::Result<ShipResult> {
    let paths = patch_paths(repo_root);
    let files = read_fragments(&paths.pending_dir)?;
    if files.is_empty() {
        return Ok(ShipResult { shipped: Vec::new() });
    }

    let problems = check_patches(repo_root)?;
    if !problems.is_empty() {
        bail!("cannot ship pending fragments:\n{}", problems.join("\n"));
    }

    let order: IndexMap<String, usize> = git(repo_root, &["log", "--first-parent", "--format=%H"], &[])?
        .split('\n')
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .enumerate()
        .map(|(index, sha)| (sha.to_string(), index))
        .collect();
    let mut queued = Vec::with_capacity(files.len());
    for NamedFragment { name, fragment } in files {
        let rel = format!("{PATCHES_REL}/pending/{name}");
        let commit = adding_commit(repo_root, &rel)?;
        if commit.is_empty() {
            bail!(
                "pending/{name} was not added on this history's first-parent line (ship needs full history)"
            );
        }
        let Some(&at) = order.get(&commit) else {
            bail!("pending/{name} was added by {commit}, which is not on this history's first-parent line");
        };
        queued.push(Queued {
            name,
            fragment,
            commit,
            at,
        });
    }
    queued.sort_by_key(|entry| std::cmp::Reverse(entry.at));
    // A commit's catalog is one snapshot, so it can ship one patch: two fragments added together
    // would have to split one diff between them.
    for pair in queued.windows(2) {
        let (before, after) = (&pair[0], &pair[1]);
        if before.commit == after.commit {
            bail!(
                "pending/{} and pending/{} were added by one commit, {}, which can ship one patch",
                before.name,
                after.name,
                after.commit
            );
        }
    }

    let mut patches = read_patches(&paths.patches_json)?;
    let mut taken: IndexSet<String> = patches.iter().map(|patch| patch.version.clone()).collect();
    let mut shipped = read_shipped(&paths.shipped_json)?;
    // Every shipped name in order, plus what this run already named: a micro `vA.B.Y` fragment is
    // named after the then-newest one (R650). Every name is computed before anything is written, so
    // an unnameable fragment fails before the first write, never mid-promotion.
    let mut sequence: Vec<String> = patches.iter().map(|patch| patch.version.clone()).collect();
    let mut planned = Vec::with_capacity(queued.len());
    for Queued {
        name,
        fragment,
        commit,
        ..
    } in queued
    {
        let version = next_ship_name(&taken, &resolve_version(&fragment.version, &sequence)?)?;
        taken.insert(version.clone());
        sequence.push(version.clone());
        planned.push(Planned {
            name,
            fragment,
            commit,
            version,
        });
    }
    let Some(newest) = patches.last() else {
        bail!("patches.json holds no shipped patch");
    };
    let mut previous = read_snapshot(&newest.version, &paths.dir)?;
    let mut proved = Vec::with_capacity(planned.len());
    for plan in planned {
        let full = git(
            repo_root,
            &["show", &format!("{}:{CATALOG_REL}", plan.commit)],
            &[],
        )?;
        let raw = shipped_text(&full)
            .with_context(|| format!("{CATALOG_REL} at {} is not a catalog", plan.commit))?;
        let snapshot: Catalog = serde_json::from_str(&raw)
            .with_context(|| format!("{CATALOG_REL} at {} is not a catalog", plan.commit))?;
        let changed = differing_ids(&previous, &snapshot);
        let claimed: IndexSet<&String> = plan.fragment.cards.iter().collect();
        if changed.len() != claimed.len() || !changed.iter().all(|id| claimed.contains(id)) {
            bail!(
                "pending/{}: the catalog {} left changes {} but the fragment claims {}; nothing was shipped",
                plan.name,
                plan.commit,
                js::stringify(&changed),
                js::stringify(&plan.fragment.cards)
            );
        }
        previous = snapshot;
        proved.push(Proved { plan, raw });
    }
    let Some(last) = proved.last() else {
        bail!("no pending fragment to ship");
    };
    let current = fs::read_to_string(&paths.catalog)
        .with_context(|| format!("{} cannot be read", paths.catalog.display()))?;
    let current =
        shipped_text(&current).with_context(|| format!("{} is not a catalog", paths.catalog.display()))?;
    if last.raw != current {
        bail!(
            "catalog.json changed after {} added pending/{}, so the newest snapshot would not be catalog.json; nothing was shipped",
            last.plan.commit,
            last.plan.name
        );
    }

    // The last patch shipped is the newest one, so the version moves first: a version site that is
    // missing stops the promotion before any history is written.
    bump_sites(&last.plan.version, repo_root)?;
    let mut names = Vec::with_capacity(proved.len());
    for Proved { plan, raw } in proved {
        let Planned {
            name,
            fragment,
            commit,
            version,
        } = plan;
        let path = snapshot_path(&version, &paths.dir)?;
        fs::write(&path, &raw).with_context(|| format!("{} cannot be written", path.display()))?;
        let timestamp = git(repo_root, &["log", "-1", "--format=%ct", &commit], &[])?;
        let seconds: i64 = timestamp
            .trim()
            .parse()
            .with_context(|| format!("git printed {timestamp:?} as the time of {commit}"))?;
        patches.push(PatchEntry {
            version: version.clone(),
            date: utc_date_of(seconds),
            title: fragment.title,
            source: fragment.sources,
            notes: fragment.notes,
            commits: Some(vec![commit.clone()]),
            reconstructed: Some(false),
            changes: Vec::new(),
        });
        let written =
            fs::read_to_string(&path).with_context(|| format!("{} cannot be read", path.display()))?;
        shipped.push(ShippedEntry {
            version: version.clone(),
            commit,
            blob: git_blob_hash(&written),
        });
        remove_file(&paths.pending_dir.join(&name));
        names.push(version);
    }
    write_json(&paths.patches_json, &patches)?;
    write_json(&paths.shipped_json, &shipped)?;
    rebuild_derived(&paths.dir)?;
    Ok(ShipResult { shipped: names })
}

/// `/^\d{4}-\d{2}-\d{2}$/`.
fn is_iso_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                *b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
}

const FRAGMENT_USAGE: &str =
    "usage: patches <version> [date] \"<title>\" [--source …] [--notes …] [--cards <id,...>]";

fn parse_fragment_args(argv: &[String]) -> anyhow::Result<FragmentArgs> {
    let mut positional: Vec<&str> = Vec::new();
    let mut flags: IndexMap<&str, &str> = IndexMap::new();
    let mut i = 0;
    while i < argv.len() {
        let arg = argv[i].as_str();
        if let Some(flag) = arg.strip_prefix("--") {
            let Some(value) = argv.get(i + 1) else {
                bail!("{arg} needs a value");
            };
            flags.insert(flag, value.as_str());
            i += 2;
        } else {
            positional.push(arg);
            i += 1;
        }
    }
    let (Some(&version), Some(&second)) = (positional.first(), positional.get(1)) else {
        bail!(FRAGMENT_USAGE);
    };
    if positional.len() > 3 {
        bail!(FRAGMENT_USAGE);
    }
    if !is_fragment_version(version) {
        bail!(
            "\"{version}\" is not a fragment version (a bare patch number ^v\\d+\\.\\d+\\.\\d+$ or a micro vA.B.Y)"
        );
    }
    // Promotion dates the patch by its merge commit, so a given date is validated and not stored.
    let (title, date) = match positional.get(2) {
        None => (second, flags.get("date").copied()),
        Some(&third) => (third, Some(second)),
    };
    if let Some(date) = date
        && !is_iso_date(date)
    {
        bail!("\"{date}\" is not a YYYY-MM-DD date");
    }
    Ok(FragmentArgs {
        version: version.to_string(),
        title: title.to_string(),
        date: date.map(str::to_string),
        sources: flags.get("source").map(|source| (*source).to_string()),
        notes: flags.get("notes").map(|notes| (*notes).to_string()),
        cards: flags.get("cards").map(|cards| {
            cards
                .split(',')
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(str::to_string)
                .collect()
        }),
    })
}

/// `patches check`'s failure: each problem on its own line, outside the `patches:` prefix every other error carries.
#[derive(Debug)]
struct CheckFailed(Vec<String>);

impl fmt::Display for CheckFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lines: Vec<String> = self
            .0
            .iter()
            .map(|problem| format!("patches check: {problem}"))
            .collect();
        f.write_str(&lines.join("\n"))
    }
}

impl std::error::Error for CheckFailed {}

fn main(repo_root: &Path, argv: &[String]) -> anyhow::Result<()> {
    let command = argv.first().map(String::as_str);
    if command == Some("check") {
        let problems = check_patches(repo_root)?;
        if !problems.is_empty() {
            return Err(anyhow::Error::new(CheckFailed(problems)));
        }
        println!("patches check: every catalog change is claimed by exactly one pending fragment");
        return Ok(());
    }
    if command == Some("ship") {
        let result = ship_patches(repo_root)?;
        if result.shipped.is_empty() {
            println!("patches ship: no pending fragments, nothing changed");
        } else {
            for version in &result.shipped {
                println!("patches ship: shipped {version}");
            }
        }
        return Ok(());
    }
    if command.is_none() {
        bail!(
            "usage: patches <version> [date] \"<title>\" [--source …] [--notes …] [--cards <id,...>] | patches check | patches ship"
        );
    }
    let args = parse_fragment_args(argv)?;
    let fragment = write_fragment(repo_root, &args)?;
    let listed = if fragment.cards.is_empty() {
        " (the catalog matches the newest snapshot)".to_string()
    } else {
        format!(" ({})", fragment.cards.join(", "))
    };
    println!(
        "fragment {}: claims {} card(s){listed}",
        fragment.version,
        fragment.cards.len()
    );
    Ok(())
}

/// `cargo jackioh patches …` (SURFACE §12). The arguments go to `patches.ts`'s own parser as it read
/// them, so `check`, `ship` and the fragment form share one positional list.
#[derive(clap::Args)]
pub struct Args {
    /// `<version> [date] "<title>" [--source …] [--notes …] [--cards <id,...>]`, or `check`, or `ship`.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, value_name = "ARGS")]
    pub argv: Vec<String>,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    main(&repo_root(), &args.argv).map_err(|error| {
        if error.is::<CheckFailed>() {
            error
        } else {
            anyhow!("patches: {error:#}")
        }
    })
}

/* tests */

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::js::{Json, Object};

    /// A directory under the system temp dir, removed when dropped.
    pub(super) struct TempDir(PathBuf);

    impl TempDir {
        pub(super) fn new(prefix: &str) -> TempDir {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("{prefix}{}-{n}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("a temp dir");
            TempDir(path)
        }

        pub(super) fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// An object literal, keys in the order written.
    pub(super) fn object(pairs: Vec<(&str, Json)>) -> Object {
        pairs
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect()
    }

    pub(super) fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_string()).collect()
    }

    mod naming_test {
        use super::strings;
        use crate::patches::naming::*;

        #[test]
        fn slugify_reads_every_name_as_its_comment_says() {
            for (name, slug) in [
                ("Big D-fender", "big-d-fender"),
                ("KY's Empty Notebook", "kys-empty-notebook"),
                ("CN-Virus", "cn-virus"),
                ("/fullsend", "fullsend"),
                ("Call to Chaos (Core Edition)", "call-to-chaos-core-edition"),
                ("\"Miss\" Mrow", "miss-mrow"),
                ("4-mana 7/7", "4-mana-7-7"),
                ("Forever&", "forever"),
                ("BOOM! Big Max", "boom-big-max"),
            ] {
                assert_eq!(slugify(name), slug, "{name}");
            }
        }

        #[test]
        fn expected_rel_path_files_each_set_in_its_folder_and_cores_shared_tokens_bare() {
            assert_eq!(expected_rel_path("core-001", "Big D-fender"), "001-big-d-fender");
            assert_eq!(
                expected_rel_path("core-051-1", "KY's Empty Notebook"),
                "051-1-kys-empty-notebook"
            );
            assert_eq!(expected_rel_path("core-t-rush", "Rush Token"), "t-rush");
            assert_eq!(
                expected_rel_path("classic-043", "Plague Nuke"),
                "classic/043-plague-nuke"
            );
            assert_eq!(
                expected_rel_path("classicplus-012-1", "Devour"),
                "classic-plus/012-1-devour"
            );
            assert_eq!(
                expected_rel_path("classicplus-t-ai-01", "Helpful Assistant"),
                "classic-plus/t-ai-01-helpful-assistant"
            );
        }

        #[test]
        fn matches_card_keeps_the_token_sub_index_off_its_card() {
            assert!(!matches_card("051-1-kys-empty-notebook", "core-051", None));
            assert!(matches_card("051-1-kys-empty-notebook", "core-051-1", None));
            assert!(matches_card("t-rush", "core-t-rush", None));
            let ids = strings(&["core-051", "core-051-1", "core-025"]);
            assert_eq!(
                resolve_basename("051-1-kys-empty-notebook", &ids, "").as_deref(),
                Some("core-051-1")
            );
            // #25's slug opens with a digit segment: only the exact answer gets it right.
            assert!(!matches_card("025-4-mana-7-7", "core-025", None));
            assert!(matches_card("025-4-mana-7-7", "core-025", Some(ids.as_slice())));
            assert_eq!(
                resolve_rel_path("classic/043-plague-nuke", &strings(&["classic-043", "core-043"]))
                    .as_deref(),
                Some("classic-043")
            );
        }

        #[test]
        fn set_subfolders_are_the_folders_that_are_not_the_root() {
            let derived: Vec<&str> = SET_FOLDERS
                .iter()
                .map(|(_, folder)| *folder)
                .filter(|folder| !folder.is_empty())
                .collect();
            assert_eq!(derived, SET_SUBFOLDERS);
        }

        #[test]
        fn sort_key_orders_by_set_then_index_tokens_after_their_card_and_shared_tokens_last() {
            assert_eq!(prefix_rank("001"), 1.0);
            assert_eq!(prefix_rank("051-1"), 51.1);
            assert_eq!(prefix_rank("t-rush"), f64::INFINITY);
            let mut keys = [
                sort_key("x", None),
                sort_key("t-rush", Some("core-t-rush")),
                sort_key("classic/001-a", Some("classic-001")),
                sort_key("051-1-b", Some("core-051-1")),
                sort_key("051-a", Some("core-051")),
            ];
            keys.sort_by(compare_sort_keys);
            let order: Vec<&str> = keys.iter().map(|key| key.3.as_str()).collect();
            assert_eq!(order, ["051-a", "051-1-b", "t-rush", "classic/001-a", "x"]);
            assert_eq!(
                module_alias_of("classic/043-plague-nuke"),
                "mclassic_043_plague_nuke"
            );
            assert_eq!(basename_of("001-big-d-fender.test.ts"), "001-big-d-fender");
        }
    }

    // R650: a micro patch's version (`vA.B.Y`) is the newest shipped version plus the next letter.
    mod versions_test {
        mod resolve_version {
            use crate::patches::tests::strings;
            use crate::patches::versions::resolve_version;

            fn refused(asked: &str, shipped: &[&str]) -> String {
                format!(
                    "{:#}",
                    resolve_version(asked, &strings(shipped)).expect_err("refused")
                )
            }

            #[test]
            fn r650_names_a_micro_patch_after_the_newest_version_with_the_next_letter() {
                assert_eq!(
                    resolve_version("v0.2.Y", &strings(&["v0.1.1", "v0.2.0", "v0.2.5"])).unwrap(),
                    "v0.2.5b"
                );
                assert_eq!(
                    resolve_version("v0.2.Y", &strings(&["v0.2.5", "v0.2.7c"])).unwrap(),
                    "v0.2.7d"
                );
                // The newest is the last entry in patches.json, whatever its name (R388), never the largest.
                assert_eq!(
                    resolve_version("v0.2.Y", &strings(&["v0.2.9", "v0.2.4"])).unwrap(),
                    "v0.2.4b"
                );
            }

            #[test]
            fn leaves_a_version_the_designer_named_as_it_is() {
                assert_eq!(
                    resolve_version("v0.2.6", &strings(&["v0.2.5"])).unwrap(),
                    "v0.2.6"
                );
                assert_eq!(
                    resolve_version("v0.2.5b", &strings(&["v0.2.5"])).unwrap(),
                    "v0.2.5b"
                );
            }

            #[test]
            fn refuses_what_it_cannot_name() {
                assert!(refused("v0.2.X", &["v0.2.5"]).contains("designer picks the X"));
                assert!(refused("v0.2.Y", &["v0.3.1"]).contains("not a v0.2 one"));
                assert!(refused("v0.2.Y", &[]).contains("none"));
                assert!(refused("v0.2.Y", &["core-1"]).contains("no vA.B.C form"));
                assert!(refused("v0.2.Y", &["v0.2.5z"]).contains("no letter left"));
            }
        }
    }

    // R646: several card patches are built at once, so branches add a pending fragment under
    // `patches/pending/` instead of editing the history. The pure rules are proved here on fixtures;
    // the promotion is proved below on a throwaway git repo that replays the acceptance scenario
    // (v0.2.5 landing before v0.2.0).
    mod patches_ship_test {
        use indexmap::IndexSet;

        use super::{object, strings};
        use crate::patches::js::{Json, Object};
        use crate::patches::patches_io::*;

        pub(super) fn card(id: &str, cost: i32) -> Object {
            object(vec![
                ("id", Json::from(id)),
                ("name", Json::from(format!("Card {id}"))),
                ("cost", Json::from(cost)),
            ])
        }

        /// A catalog of these cards, keyed by id, in this order.
        pub(super) fn catalog_of(cards: Vec<Object>) -> Catalog {
            cards
                .into_iter()
                .map(|card| (card["id"].as_str().expect("an id").to_string(), card))
                .collect()
        }

        fn named(
            name: &str,
            version: &str,
            title: &str,
            sources: &str,
            notes: &str,
            cards: &[&str],
        ) -> NamedFragment {
            NamedFragment {
                name: name.to_string(),
                fragment: PendingFragment {
                    version: version.to_string(),
                    title: title.to_string(),
                    sources: sources.to_string(),
                    notes: notes.to_string(),
                    cards: strings(cards),
                },
            }
        }

        fn set(ids: &[&str]) -> IndexSet<String> {
            ids.iter().map(|id| (*id).to_string()).collect()
        }

        fn check(files: &[NamedFragment], catalog: &Catalog, newest: &Catalog) -> Vec<String> {
            check_fragments(&CheckFragmentsArgs {
                files,
                catalog,
                newest,
            })
        }

        mod r646_pending_fragments_and_the_check_that_proves_them {
            use super::*;
            use crate::patches::utc_date_of;

            #[test]
            fn r646_claims_every_catalog_change_exactly_once_and_only_changes() {
                let newest = catalog_of(vec![card("aaa", 1), card("bbb", 1)]);
                let files = vec![named("v0.2.5.json", "v0.2.5", "t", "s", "n", &["aaa"])];
                assert_eq!(
                    check(&files, &catalog_of(vec![card("aaa", 2), card("bbb", 1)]), &newest),
                    Vec::<String>::new()
                );
                assert_eq!(
                    check(&[], &catalog_of(vec![card("aaa", 2), card("bbb", 1)]), &newest),
                    strings(&[
                        "\"aaa\" differs from the newest shipped snapshot but no pending fragment claims it"
                    ])
                );
                assert_eq!(
                    check(&[], &catalog_of(vec![card("aaa", 1)]), &newest),
                    strings(&[
                        "\"bbb\" differs from the newest shipped snapshot but no pending fragment claims it"
                    ])
                );
                let mut twice = files.clone();
                twice.push(named("v0.2.0.json", "v0.2.0", "t", "s", "n", &["aaa"]));
                assert_eq!(
                    check(&twice, &catalog_of(vec![card("aaa", 2), card("bbb", 1)]), &newest),
                    strings(&[
                        "\"aaa\" is claimed by \"v0.2.5\" and \"v0.2.0\", but one card ships in one patch"
                    ])
                );
                assert_eq!(
                    check(&files, &catalog_of(vec![card("aaa", 1), card("bbb", 1)]), &newest),
                    strings(&[
                        "\"aaa\" is claimed by \"v0.2.5\" but identical to the newest shipped snapshot"
                    ])
                );
            }

            #[test]
            fn r646_holds_every_fragment_to_a_bare_patch_number_matching_its_file() {
                let newest = catalog_of(vec![card("aaa", 1)]);
                let catalog = catalog_of(vec![card("aaa", 1)]);
                let bad = vec![named("v9.json", "9.9", "t", "s", "n", &[])];
                assert!(check(&bad, &catalog, &newest).join("\n").contains("\"9.9\""));
                let renamed = vec![named("v0.2.5.json", "v0.2.0", "t", "s", "n", &[])];
                assert!(
                    check(&renamed, &catalog, &newest)
                        .join("\n")
                        .contains("its file names")
                );
            }

            #[test]
            fn r646_lets_a_fragment_hold_a_micro_va_b_y_named_at_promotion_but_never_a_va_b_x() {
                let newest = catalog_of(vec![card("aaa", 1)]);
                let catalog = catalog_of(vec![card("aaa", 2)]);
                let micro = vec![named("v0.2.Y.json", "v0.2.Y", "t", "s", "n", &["aaa"])];
                assert_eq!(check(&micro, &catalog, &newest), Vec::<String>::new());
                let placeholder = vec![named("v0.2.X.json", "v0.2.X", "t", "s", "n", &["aaa"])];
                assert!(
                    check(&placeholder, &catalog, &newest)
                        .join("\n")
                        .contains("v0.2.X")
                );
            }

            #[test]
            fn r646_holds_a_fragment_to_what_a_shipped_patch_carries_a_title_a_source_and_notes() {
                let newest = catalog_of(vec![card("aaa", 1)]);
                let empty = vec![named("v0.2.5.json", "v0.2.5", "", " ", "", &["aaa"])];
                assert_eq!(
                    check(&empty, &catalog_of(vec![card("aaa", 2)]), &newest),
                    strings(&[
                        "pending/v0.2.5.json has an empty title, but a shipped patch needs one",
                        "pending/v0.2.5.json has an empty sources, but a shipped patch needs one",
                        "pending/v0.2.5.json has an empty notes, but a shipped patch needs one",
                    ])
                );
            }

            #[test]
            fn r646_refuses_a_fragment_that_claims_no_card_a_patch_is_a_catalog_change() {
                let newest = catalog_of(vec![card("aaa", 1)]);
                let none = vec![named("v0.2.5.json", "v0.2.5", "t", "s", "n", &[])];
                assert_eq!(
                    check(&none, &newest, &newest),
                    strings(&["pending/v0.2.5.json claims no cards, but a patch ships a catalog change"])
                );
            }

            #[test]
            fn r646_counts_an_entry_whose_bytes_moved_as_changed_key_order_included_so_every_catalog_has_a_legal_state()
             {
                let newest = catalog_of(vec![object(vec![
                    ("id", Json::from("aaa")),
                    ("name", Json::from("Card aaa")),
                    ("cost", Json::from(1)),
                ])]);
                let reordered = catalog_of(vec![object(vec![
                    ("cost", Json::from(1)),
                    ("id", Json::from("aaa")),
                    ("name", Json::from("Card aaa")),
                ])]);
                let files = vec![named("v0.2.5.json", "v0.2.5", "t", "s", "n", &["aaa"])];
                assert_eq!(differing_ids(&newest, &reordered), strings(&["aaa"]));
                assert_eq!(
                    check(&[], &reordered, &newest),
                    strings(&[
                        "\"aaa\" differs from the newest shipped snapshot but no pending fragment claims it"
                    ])
                );
                assert_eq!(check(&files, &reordered, &newest), Vec::<String>::new());
                assert!(same_catalog(
                    &revert_pending(&reordered, &newest, &set(&["aaa"])),
                    &newest
                ));
            }

            #[test]
            fn r646_reverts_the_catalog_to_the_newest_snapshot_on_exactly_the_claimed_cards() {
                let newest = catalog_of(vec![card("aaa", 1), card("bbb", 1)]);
                let catalog = catalog_of(vec![card("aaa", 2), card("ccc", 1)]);
                let reverted = revert_pending(&catalog, &newest, &set(&["aaa", "bbb", "ccc"]));
                assert!(same_catalog(&reverted, &newest));
                assert!(!same_catalog(&catalog, &newest));
                assert!(!same_catalog(
                    &revert_pending(&catalog, &newest, &set(&[])),
                    &newest
                ));
                assert!(same_catalog(&newest, &newest));
            }

            #[test]
            fn r646_ships_a_taken_version_as_the_next_revision_letter_never_by_reopening_it() {
                assert_eq!(next_ship_name(&set(&[]), "v0.2.0").unwrap(), "v0.2.0");
                assert_eq!(next_ship_name(&set(&["v0.2.0"]), "v0.2.0").unwrap(), "v0.2.0b");
                assert_eq!(
                    next_ship_name(&set(&["v0.2.0", "v0.2.0b"]), "v0.2.0").unwrap(),
                    "v0.2.0c"
                );
                assert_eq!(next_ship_name(&set(&["v0.2.5"]), "v0.2.0").unwrap(), "v0.2.0");
            }

            #[test]
            fn r646_dates_a_patch_by_its_commits_utc_day_never_the_local_one() {
                assert_eq!(utc_date_of(0), "1970-01-01");
                // 2026-10-01T12:00:00Z.
                assert_eq!(utc_date_of(1_790_856_000), "2026-10-01");
                // 2026-10-01T00:30:00+02:00 is still 2026-09-30 in UTC.
                assert_eq!(utc_date_of(1_790_807_400), "2026-09-30");
            }

            #[test]
            fn r646_hashes_a_snapshots_bytes_the_way_git_does() {
                assert_eq!(
                    git_blob_hash("test\n"),
                    "9daeafb9864cf43055ae93beb0afd6c7d144bfa4"
                );
                assert_eq!(git_blob_hash(""), "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
            }
        }

        mod r646_promotion_in_ship_order {
            use std::fs;
            use std::path::Path;
            use std::process::Command;

            use indexmap::IndexMap;
            use serde::Serialize;

            use super::*;
            use crate::patches::js;
            use crate::patches::tests::TempDir;
            use crate::patches::{FragmentArgs, ShipResult, check_patches, ship_patches, write_fragment};

            const IDENT: &[(&str, &str)] = &[
                ("GIT_AUTHOR_NAME", "night bot"),
                ("GIT_AUTHOR_EMAIL", "bot@example.invalid"),
                ("GIT_COMMITTER_NAME", "night bot"),
                ("GIT_COMMITTER_EMAIL", "bot@example.invalid"),
                ("GIT_CONFIG_COUNT", "1"),
                ("GIT_CONFIG_KEY_0", "commit.gpgsign"),
                ("GIT_CONFIG_VALUE_0", "false"),
            ];

            fn git(root: &Path, args: &[&str], date: Option<&str>) -> String {
                let mut command = Command::new("git");
                command.args(args).current_dir(root).envs(IDENT.iter().copied());
                if let Some(date) = date {
                    command
                        .env("GIT_AUTHOR_DATE", date)
                        .env("GIT_COMMITTER_DATE", date);
                }
                let output = command.output().expect("git runs");
                assert!(
                    output.status.success(),
                    "git {args:?} failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                String::from_utf8(output.stdout).expect("git prints text")
            }

            fn write_files(root: &Path, files: &[(&str, String)]) {
                for (rel, text) in files {
                    let path = root.join(rel);
                    fs::create_dir_all(path.parent().expect("a parent")).expect("a folder");
                    fs::write(&path, text).expect("a fixture file");
                }
            }

            fn json<T: Serialize + ?Sized>(value: &T) -> String {
                format!("{}\n", js::stringify_pretty(value))
            }

            fn cost_of(snapshot: &Catalog, id: &str) -> Option<f64> {
                snapshot
                    .get(id)
                    .and_then(|def| def.get("cost"))
                    .and_then(Json::as_f64)
            }

            fn read(path: &Path) -> String {
                fs::read_to_string(path).expect("a file")
            }

            fn fragment_args(
                version: &str,
                title: &str,
                sources: &str,
                notes: &str,
                cards: Option<&[&str]>,
            ) -> FragmentArgs {
                FragmentArgs {
                    version: version.to_string(),
                    title: title.to_string(),
                    sources: Some(sources.to_string()),
                    notes: Some(notes.to_string()),
                    cards: cards.map(strings),
                    ..FragmentArgs::default()
                }
            }

            fn base_patch(notes: &str, changes: Vec<PatchChange>) -> PatchEntry {
                PatchEntry {
                    version: "v0.1.1".to_string(),
                    date: "2026-09-27".to_string(),
                    title: "base".to_string(),
                    source: "test".to_string(),
                    notes: notes.to_string(),
                    commits: None,
                    reconstructed: None,
                    changes,
                }
            }

            fn unshipped(version: &str) -> Vec<ShippedEntry> {
                vec![ShippedEntry {
                    version: version.to_string(),
                    commit: "0".repeat(40),
                    blob: "0".repeat(40),
                }]
            }

            fn index_of(pairs: Vec<(&str, Vec<&str>)>) -> IndexMap<String, Vec<String>> {
                pairs
                    .into_iter()
                    .map(|(id, versions)| (id.to_string(), strings(&versions)))
                    .collect()
            }

            /// The version sites as a fixture tree carries them at `version`.
            fn version_sites(version: &str) -> Vec<(&'static str, String)> {
                vec![
                    (
                        "crates/server/.env.example",
                        format!("CATALOG_VERSION={version}\n"),
                    ),
                    (
                        "render.yaml",
                        format!("x:\n- key: CATALOG_VERSION\n  value: {version}\n"),
                    ),
                ]
            }

            fn shipped_of(names: &[&str]) -> ShipResult {
                ShipResult {
                    shipped: strings(names),
                }
            }

            fn in_set(mut card: Object, set: &str) -> Object {
                card.insert("set".to_string(), Json::from(set));
                card
            }

            // R1420: a set the catalog holds before it ships is no patch's: no fragment claims its
            // cards, `patches check` passes with them unclaimed, and a promotion snapshots the catalog
            // without them, while catalog.json keeps them.
            #[test]
            fn r1420_ships_the_catalog_without_the_cards_of_a_set_that_has_not_shipped() {
                let temp = TempDir::new("jackioh-ship-unshipped-");
                let root = temp.path();
                let dir = root.join("crates/cards/patches");
                let base = catalog_of(vec![
                    in_set(card("aaa", 1), "Core"),
                    in_set(card("bbb", 1), "Core"),
                ]);
                git(root, &["init", "-q", "-b", "main"], None);
                let mut files = vec![
                    ("crates/cards/catalog.json", json(&base)),
                    (
                        "crates/cards/patches/patches.json",
                        json(&[base_patch(
                            "base notes",
                            vec![
                                PatchChange::added("aaa", "Card aaa"),
                                PatchChange::added("bbb", "Card bbb"),
                            ],
                        )]),
                    ),
                    ("crates/cards/patches/v0.1.1.json", json(&base)),
                    (
                        "crates/cards/patches/index.json",
                        json(&index_of(vec![("aaa", vec!["v0.1.1"]), ("bbb", vec!["v0.1.1"])])),
                    ),
                    ("crates/cards/patches/shipped.json", json(&unshipped("v0.1.1"))),
                ];
                files.extend(version_sites("v0.1.1"));
                write_files(root, &files);
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "base"],
                    Some("2026-10-08T12:00:00+00:00"),
                );

                // A batch adds a Meditative card: nothing to claim, and the check passes.
                let mut with_card = base.clone();
                with_card.insert("med".to_string(), in_set(card("med", 2), "Meditative"));
                write_files(root, &[("crates/cards/catalog.json", json(&with_card))]);
                assert_eq!(check_patches(root).unwrap(), Vec::<String>::new());
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "a card of a set being built"],
                    Some("2026-10-08T13:00:00+00:00"),
                );

                // A shipped card changes beside it: the fragment claims that card alone.
                let mut changed = with_card.clone();
                changed.insert("aaa".to_string(), in_set(card("aaa", 3), "Core"));
                write_files(root, &[("crates/cards/catalog.json", json(&changed))]);
                let written = write_fragment(
                    root,
                    &fragment_args("v0.2.0", "Patch v0.2.0", "issue #496", "aaa", None),
                )
                .unwrap();
                assert_eq!(written.cards, strings(&["aaa"]));
                assert_eq!(check_patches(root).unwrap(), Vec::<String>::new());
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "v0.2.0"],
                    Some("2026-10-08T14:00:00+00:00"),
                );

                assert_eq!(ship_patches(root).unwrap(), shipped_of(&["v0.2.0"]));
                let snapshot: Catalog = serde_json::from_str(&read(&dir.join("v0.2.0.json"))).unwrap();
                assert_eq!(snapshot.keys().collect::<Vec<_>>(), vec!["aaa", "bbb"]);
                assert_eq!(cost_of(&snapshot, "aaa"), Some(3.0));
                let catalog: Catalog =
                    serde_json::from_str(&read(&root.join("crates/cards/catalog.json"))).unwrap();
                assert!(
                    catalog.contains_key("med"),
                    "catalog.json keeps the card being built"
                );
            }

            #[test]
            fn r1420_the_shipped_text_is_the_file_itself_when_every_entry_ships() {
                let shipped = json(&catalog_of(vec![in_set(card("aaa", 1), "Core"), card("bbb", 2)]));
                assert_eq!(crate::patches::shipped_text(&shipped).unwrap(), shipped);
                let mixed = json(&catalog_of(vec![
                    in_set(card("aaa", 1), "Core"),
                    in_set(card("med", 1), "Meditative"),
                    card("bbb", 2),
                ]));
                let view = crate::patches::shipped_text(&mixed).unwrap();
                assert_eq!(
                    view,
                    json(&catalog_of(vec![in_set(card("aaa", 1), "Core"), card("bbb", 2)]))
                );
            }

            // The issue's acceptance scenario on a three-card catalog: branch A adds fragment v0.2.5
            // changing card aaa, branch B adds fragment v0.2.0 changing card bbb. A merges, then B
            // merges with no conflict under pending/, and promotion ships v0.2.5 before v0.2.0
            // whatever the names say. A later fragment named v0.2.0 ships as v0.2.0b, then v0.2.0c.
            #[test]
            fn r646_ships_pending_fragments_oldest_merge_first_snapshots_each_merges_catalog_and_letters_revisions()
             {
                let temp = TempDir::new("jackioh-ship-");
                let root = temp.path();
                let dir = root.join("crates/cards/patches");
                let base = catalog_of(vec![card("aaa", 1), card("bbb", 1), card("ccc", 1)]);
                git(root, &["init", "-q", "-b", "main"], None);
                let mut files = vec![
                    ("crates/cards/catalog.json", json(&base)),
                    (
                        "crates/cards/patches/patches.json",
                        json(&[base_patch(
                            "base notes",
                            vec![
                                PatchChange::added("aaa", "Card aaa"),
                                PatchChange::added("bbb", "Card bbb"),
                                PatchChange::added("ccc", "Card ccc"),
                            ],
                        )]),
                    ),
                    ("crates/cards/patches/v0.1.1.json", json(&base)),
                    (
                        "crates/cards/patches/index.json",
                        json(&index_of(vec![
                            ("aaa", vec!["v0.1.1"]),
                            ("bbb", vec!["v0.1.1"]),
                            ("ccc", vec!["v0.1.1"]),
                        ])),
                    ),
                    ("crates/cards/patches/shipped.json", json(&unshipped("v0.1.1"))),
                    // The optional site: rewritten when the web build names the version.
                    (
                        "apps/web/.env.production",
                        "VITE_SERVER_HTTP_URL=x\nVITE_CATALOG_VERSION=v0.1.1\n".to_string(),
                    ),
                ];
                files.extend(version_sites("v0.1.1"));
                write_files(root, &files);
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "base"],
                    Some("2026-09-27T12:00:00+00:00"),
                );
                let c0 = git(root, &["rev-parse", "HEAD"], None).trim().to_string();
                write_files(
                    root,
                    &[(
                        "crates/cards/patches/shipped.json",
                        json(&[ShippedEntry {
                            version: "v0.1.1".to_string(),
                            commit: c0,
                            blob: git_blob_hash(&read(&dir.join("v0.1.1.json"))),
                        }]),
                    )],
                );

                let mut after_a = base.clone();
                after_a.insert("aaa".to_string(), card("aaa", 2));
                write_files(root, &[("crates/cards/catalog.json", json(&after_a))]);
                let written = write_fragment(
                    root,
                    &fragment_args("v0.2.5", "Patch v0.2.5", "issue #48", "X", None),
                )
                .unwrap();
                assert_eq!(written.cards, strings(&["aaa"]));
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "fragment v0.2.5"],
                    Some("2026-10-01T12:00:00+00:00"),
                );
                let c1 = git(root, &["rev-parse", "HEAD"], None).trim().to_string();

                let mut after_b = after_a.clone();
                after_b.insert("bbb".to_string(), card("bbb", 3));
                write_files(root, &[("crates/cards/catalog.json", json(&after_b))]);
                write_fragment(
                    root,
                    &fragment_args("v0.2.0", "Patch v0.2.0", "issue #40", "Y", Some(&["bbb"][..])),
                )
                .unwrap();
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "fragment v0.2.0"],
                    Some("2026-10-02T12:00:00+00:00"),
                );
                let c2 = git(root, &["rev-parse", "HEAD"], None).trim().to_string();

                assert_eq!(check_patches(root).unwrap(), Vec::<String>::new());
                assert_eq!(ship_patches(root).unwrap(), shipped_of(&["v0.2.5", "v0.2.0"]));

                let patches = read_patches(&dir.join("patches.json")).unwrap();
                let versions: Vec<&str> = patches.iter().map(|patch| patch.version.as_str()).collect();
                assert_eq!(versions, ["v0.1.1", "v0.2.5", "v0.2.0"]);
                let v25 = patches
                    .iter()
                    .find(|patch| patch.version == "v0.2.5")
                    .expect("v0.2.5");
                let v20 = patches
                    .iter()
                    .find(|patch| patch.version == "v0.2.0")
                    .expect("v0.2.0");
                assert_eq!(
                    (
                        v25.date.as_str(),
                        v25.title.as_str(),
                        v25.source.as_str(),
                        v25.notes.as_str()
                    ),
                    ("2026-10-01", "Patch v0.2.5", "issue #48", "X")
                );
                assert_eq!(
                    (v25.commits.clone(), v25.reconstructed),
                    (Some(vec![c1.clone()]), Some(false))
                );
                assert_eq!(v20.date, "2026-10-02");
                // Each snapshot is the catalog as its merge left it: v0.2.0 carries v0.2.5's change.
                assert_eq!(cost_of(&read_snapshot("v0.2.5", &dir).unwrap(), "aaa"), Some(2.0));
                assert_eq!(cost_of(&read_snapshot("v0.2.5", &dir).unwrap(), "bbb"), Some(1.0));
                assert_eq!(cost_of(&read_snapshot("v0.2.0", &dir).unwrap(), "aaa"), Some(2.0));
                assert_eq!(cost_of(&read_snapshot("v0.2.0", &dir).unwrap(), "bbb"), Some(3.0));
                // …so v0.2.0's card-by-card changes list only its own card.
                assert_eq!(
                    v20.changes,
                    vec![PatchChange::changed("bbb", "Card bbb", strings(&["cost"]))]
                );
                let index: IndexMap<String, Vec<String>> =
                    serde_json::from_str(&read(&dir.join("index.json"))).unwrap();
                assert_eq!(
                    index,
                    index_of(vec![
                        ("aaa", vec!["v0.1.1", "v0.2.5"]),
                        ("bbb", vec!["v0.1.1", "v0.2.0"]),
                        ("ccc", vec!["v0.1.1"]),
                    ])
                );
                assert_eq!(
                    read_fragments(&dir.join("pending")).unwrap(),
                    Vec::<NamedFragment>::new()
                );
                let shipped = read_shipped(&dir.join("shipped.json")).unwrap();
                let shipped_versions: Vec<&str> =
                    shipped.iter().map(|entry| entry.version.as_str()).collect();
                assert_eq!(shipped_versions, ["v0.1.1", "v0.2.5", "v0.2.0"]);
                assert_eq!(
                    shipped[1],
                    ShippedEntry {
                        version: "v0.2.5".to_string(),
                        commit: c1.clone(),
                        blob: git_blob_hash(&read(&dir.join("v0.2.5.json"))),
                    }
                );
                assert_eq!(shipped[2].commit, c2);
                assert!(read(&root.join("crates/server/.env.example")).contains("CATALOG_VERSION=v0.2.0"));
                assert!(read(&root.join("render.yaml")).contains("value: v0.2.0"));
                assert!(read(&root.join("apps/web/.env.production")).contains("VITE_CATALOG_VERSION=v0.2.0"));
                let working: Catalog =
                    serde_json::from_str(&read(&root.join("crates/cards/catalog.json"))).unwrap();
                assert_eq!(working, after_b);
                assert_eq!(check_patches(root).unwrap(), Vec::<String>::new());

                let patches_bytes = read(&dir.join("patches.json"));
                assert_eq!(ship_patches(root).unwrap(), shipped_of(&[]));
                assert_eq!(read(&dir.join("patches.json")), patches_bytes);

                // The promotion is committed, as the workflow's pull request would commit it: the
                // next fragment under a shipped name is added again, not modified.
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "ship v0.2.5 and v0.2.0"],
                    Some("2026-10-02T12:00:00+00:00"),
                );

                // A later fragment under a shipped name ships as the next revision letter, appended last.
                let mut after_c = after_b.clone();
                after_c.insert("ccc".to_string(), card("ccc", 5));
                write_files(root, &[("crates/cards/catalog.json", json(&after_c))]);
                write_fragment(root, &fragment_args("v0.2.0", "follow-up", "issue", "Z", None)).unwrap();
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "fragment v0.2.0 again"],
                    Some("2026-10-03T12:00:00+00:00"),
                );
                assert_eq!(ship_patches(root).unwrap(), shipped_of(&["v0.2.0b"]));
                let patches = read_patches(&dir.join("patches.json")).unwrap();
                let versions: Vec<&str> = patches.iter().map(|patch| patch.version.as_str()).collect();
                assert_eq!(versions, ["v0.1.1", "v0.2.5", "v0.2.0", "v0.2.0b"]);
                let v20b = patches
                    .iter()
                    .find(|patch| patch.version == "v0.2.0b")
                    .expect("v0.2.0b");
                assert_eq!(v20b.date, "2026-10-03");
                assert_eq!(
                    cost_of(&read_snapshot("v0.2.0b", &dir).unwrap(), "ccc"),
                    Some(5.0)
                );
                assert_eq!(
                    v20b.changes,
                    vec![PatchChange::changed("ccc", "Card ccc", strings(&["cost"]))]
                );
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "ship v0.2.0b"],
                    Some("2026-10-03T12:00:00+00:00"),
                );

                let mut after_d = after_c.clone();
                after_d.insert("ccc".to_string(), card("ccc", 6));
                write_files(root, &[("crates/cards/catalog.json", json(&after_d))]);
                write_fragment(
                    root,
                    &fragment_args("v0.2.0", "another follow-up", "issue", "Z2", None),
                )
                .unwrap();
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "fragment v0.2.0 a third time"],
                    Some("2026-10-04T12:00:00+00:00"),
                );
                assert_eq!(ship_patches(root).unwrap(), shipped_of(&["v0.2.0c"]));
                let versions: Vec<String> = read_patches(&dir.join("patches.json"))
                    .unwrap()
                    .into_iter()
                    .map(|patch| patch.version)
                    .collect();
                assert_eq!(
                    versions,
                    strings(&["v0.1.1", "v0.2.5", "v0.2.0", "v0.2.0b", "v0.2.0c"])
                );
            }

            // A micro `vA.B.Y` fragment keeps its Y until promotion names it after the then-newest
            // patch (R650): the first ships as v0.1.1b, the next as v0.1.1c, each snapshot carrying
            // only its card.
            #[test]
            fn r646_names_a_micro_va_b_y_fragment_after_the_then_newest_patch_when_it_ships() {
                let temp = TempDir::new("jackioh-ship-micro-");
                let root = temp.path();
                let dir = root.join("crates/cards/patches");
                let base = catalog_of(vec![card("aaa", 1), card("bbb", 1)]);
                git(root, &["init", "-q", "-b", "main"], None);
                let mut files = vec![
                    ("crates/cards/catalog.json", json(&base)),
                    (
                        "crates/cards/patches/patches.json",
                        json(&[base_patch(
                            "base notes",
                            vec![
                                PatchChange::added("aaa", "Card aaa"),
                                PatchChange::added("bbb", "Card bbb"),
                            ],
                        )]),
                    ),
                    ("crates/cards/patches/v0.1.1.json", json(&base)),
                    (
                        "crates/cards/patches/index.json",
                        json(&index_of(vec![("aaa", vec!["v0.1.1"]), ("bbb", vec!["v0.1.1"])])),
                    ),
                    ("crates/cards/patches/shipped.json", json(&unshipped("v0.1.1"))),
                ];
                files.extend(version_sites("v0.1.1"));
                write_files(root, &files);
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "base"],
                    Some("2026-09-27T12:00:00+00:00"),
                );

                let mut micro = base.clone();
                micro.insert("aaa".to_string(), card("aaa", 2));
                write_files(root, &[("crates/cards/catalog.json", json(&micro))]);
                assert_eq!(
                    write_fragment(root, &fragment_args("v0.1.Y", "micro", "issue", "m", None))
                        .unwrap()
                        .cards,
                    strings(&["aaa"])
                );
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "fragment v0.1.Y"],
                    Some("2026-10-05T12:00:00+00:00"),
                );
                assert_eq!(check_patches(root).unwrap(), Vec::<String>::new());
                assert_eq!(ship_patches(root).unwrap(), shipped_of(&["v0.1.1b"]));
                let patches = read_patches(&dir.join("patches.json")).unwrap();
                let first = patches
                    .iter()
                    .find(|patch| patch.version == "v0.1.1b")
                    .expect("v0.1.1b");
                assert_eq!(
                    first.changes,
                    vec![PatchChange::changed("aaa", "Card aaa", strings(&["cost"]))]
                );
                assert_eq!(
                    cost_of(&read_snapshot("v0.1.1b", &dir).unwrap(), "aaa"),
                    Some(2.0)
                );
                assert_eq!(
                    cost_of(&read_snapshot("v0.1.1b", &dir).unwrap(), "bbb"),
                    Some(1.0)
                );
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "ship v0.1.1b"],
                    Some("2026-10-05T12:00:00+00:00"),
                );

                let mut again = micro.clone();
                again.insert("bbb".to_string(), card("bbb", 4));
                write_files(root, &[("crates/cards/catalog.json", json(&again))]);
                write_fragment(root, &fragment_args("v0.1.Y", "micro again", "issue", "m2", None)).unwrap();
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "fragment v0.1.Y again"],
                    Some("2026-10-06T12:00:00+00:00"),
                );
                assert_eq!(ship_patches(root).unwrap(), shipped_of(&["v0.1.1c"]));
                let versions: Vec<String> = read_patches(&dir.join("patches.json"))
                    .unwrap()
                    .into_iter()
                    .map(|patch| patch.version)
                    .collect();
                assert_eq!(versions, strings(&["v0.1.1", "v0.1.1b", "v0.1.1c"]));
            }

            #[test]
            fn r646_fails_check_when_a_micro_cannot_be_named_after_the_newest_patch() {
                let temp = TempDir::new("jackioh-check-micro-");
                let root = temp.path();
                let base = catalog_of(vec![card("aaa", 1)]);
                let mut patch = base_patch("n", Vec::new());
                patch.version = "v0.3.0".to_string();
                write_files(
                    root,
                    &[
                        (
                            "crates/cards/catalog.json",
                            json(&catalog_of(vec![card("aaa", 2)])),
                        ),
                        ("crates/cards/patches/patches.json", json(&[patch])),
                        ("crates/cards/patches/v0.3.0.json", json(&base)),
                        ("crates/cards/patches/index.json", json(&index_of(Vec::new()))),
                        (
                            "crates/cards/patches/shipped.json",
                            json(&Vec::<ShippedEntry>::new()),
                        ),
                    ],
                );
                write_fragment(root, &fragment_args("v0.2.Y", "t", "s", "n", None)).unwrap();
                assert!(check_patches(root).unwrap().join("\n").contains("cannot ship"));
            }

            /// A repo whose only patch is v0.1.1 over `base`, committed, with its provenance recorded.
            fn shipped_repo(prefix: &str, base: &Catalog) -> TempDir {
                let temp = TempDir::new(prefix);
                let root = temp.path();
                git(root, &["init", "-q", "-b", "main"], None);
                let mut files = vec![
                    ("crates/cards/catalog.json", json(base)),
                    (
                        "crates/cards/patches/patches.json",
                        json(&[base_patch("n", Vec::new())]),
                    ),
                    ("crates/cards/patches/v0.1.1.json", json(base)),
                    ("crates/cards/patches/index.json", json(&index_of(Vec::new()))),
                    ("crates/cards/patches/shipped.json", json(&unshipped("v0.1.1"))),
                ];
                files.extend(version_sites("v0.1.1"));
                write_files(root, &files);
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "base"],
                    Some("2026-09-27T12:00:00+00:00"),
                );
                temp
            }

            #[test]
            fn r646_ships_a_fragment_a_merge_commit_brought_in_snapshotting_the_catalog_that_merge_left() {
                let base = catalog_of(vec![card("aaa", 1), card("bbb", 1)]);
                let temp = shipped_repo("jackioh-ship-merge-", &base);
                let root = temp.path();
                let dir = root.join("crates/cards/patches");
                git(root, &["checkout", "-q", "-b", "patch"], None);
                let mut changed = base.clone();
                changed.insert("aaa".to_string(), card("aaa", 2));
                write_files(root, &[("crates/cards/catalog.json", json(&changed))]);
                write_fragment(root, &fragment_args("v0.2.5", "t", "s", "n", None)).unwrap();
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "fragment v0.2.5"],
                    Some("2026-10-01T12:00:00+00:00"),
                );
                git(root, &["checkout", "-q", "main"], None);
                write_files(root, &[("README.md", "unrelated\n".to_string())]);
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "meanwhile on main"],
                    Some("2026-10-01T13:00:00+00:00"),
                );
                git(
                    root,
                    &["merge", "-q", "--no-ff", "patch", "-m", "Merge pull request"],
                    Some("2026-10-02T12:00:00+00:00"),
                );
                let merge = git(root, &["rev-parse", "HEAD"], None).trim().to_string();
                assert_eq!(ship_patches(root).unwrap(), shipped_of(&["v0.2.5"]));
                let patches = read_patches(&dir.join("patches.json")).unwrap();
                let v25 = patches
                    .iter()
                    .find(|patch| patch.version == "v0.2.5")
                    .expect("v0.2.5");
                assert_eq!(
                    (v25.date.as_str(), v25.commits.clone()),
                    ("2026-10-02", Some(vec![merge]))
                );
                assert_eq!(cost_of(&read_snapshot("v0.2.5", &dir).unwrap(), "aaa"), Some(2.0));
            }

            #[test]
            fn r646_writes_nothing_when_a_claimed_card_changed_again_after_its_fragment_merged() {
                let base = catalog_of(vec![card("aaa", 1), card("bbb", 1)]);
                let temp = shipped_repo("jackioh-ship-late-", &base);
                let root = temp.path();
                let with = |cards: Vec<Object>| {
                    let mut catalog = base.clone();
                    for card in cards {
                        catalog.insert(card["id"].as_str().expect("an id").to_string(), card);
                    }
                    catalog
                };
                write_files(
                    root,
                    &[("crates/cards/catalog.json", json(&with(vec![card("aaa", 2)])))],
                );
                write_fragment(root, &fragment_args("v0.2.5", "t", "s", "n", None)).unwrap();
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "fragment v0.2.5"],
                    Some("2026-10-01T12:00:00+00:00"),
                );
                // A later merge moves the claimed card again without touching the fragment: check
                // passes, but no snapshot of the fragment's commit is today's catalog.
                write_files(
                    root,
                    &[("crates/cards/catalog.json", json(&with(vec![card("aaa", 3)])))],
                );
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "aaa again"],
                    Some("2026-10-02T12:00:00+00:00"),
                );
                assert_eq!(check_patches(root).unwrap(), Vec::<String>::new());
                let refused = format!("{:#}", ship_patches(root).expect_err("refused"));
                assert!(refused.contains("catalog.json changed after"), "{refused}");
                assert_eq!(git(root, &["status", "--porcelain"], None), "");

                // A fragment edited to claim a card its commit did not change is refused the same way.
                write_files(
                    root,
                    &[(
                        "crates/cards/catalog.json",
                        json(&with(vec![card("aaa", 3), card("bbb", 2)])),
                    )],
                );
                write_fragment(root, &fragment_args("v0.2.5", "t", "s", "n", None)).unwrap();
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "fragment v0.2.5 claims bbb too"],
                    Some("2026-10-03T12:00:00+00:00"),
                );
                assert_eq!(check_patches(root).unwrap(), Vec::<String>::new());
                let refused = format!("{:#}", ship_patches(root).expect_err("refused"));
                assert!(
                    refused.contains("but the fragment claims [\"aaa\",\"bbb\"]"),
                    "{refused}"
                );
                assert_eq!(git(root, &["status", "--porcelain"], None), "");
            }

            #[test]
            fn r646_writes_nothing_when_a_version_site_is_missing() {
                let base = catalog_of(vec![card("aaa", 1)]);
                let temp = shipped_repo("jackioh-ship-site-", &base);
                let root = temp.path();
                write_files(
                    root,
                    &[
                        (
                            "crates/cards/catalog.json",
                            json(&catalog_of(vec![card("aaa", 2)])),
                        ),
                        ("render.yaml", "x: {}\n".to_string()),
                    ],
                );
                write_fragment(root, &fragment_args("v0.2.5", "t", "s", "n", None)).unwrap();
                git(root, &["add", "-A"], None);
                git(
                    root,
                    &["commit", "-q", "-m", "fragment v0.2.5"],
                    Some("2026-10-01T12:00:00+00:00"),
                );
                let refused = format!("{:#}", ship_patches(root).expect_err("refused"));
                assert!(refused.contains("render.yaml: no CATALOG_VERSION"), "{refused}");
                assert_eq!(git(root, &["status", "--porcelain"], None), "");
            }

            #[test]
            fn r646_fails_check_on_the_wiring_not_only_on_fixtures_an_unclaimed_change_in_a_real_tree() {
                let temp = TempDir::new("jackioh-check-");
                let root = temp.path();
                let base = catalog_of(vec![card("aaa", 1)]);
                write_files(
                    root,
                    &[
                        (
                            "crates/cards/catalog.json",
                            json(&catalog_of(vec![card("aaa", 2)])),
                        ),
                        (
                            "crates/cards/patches/patches.json",
                            json(&[base_patch("n", Vec::new())]),
                        ),
                        ("crates/cards/patches/v0.1.1.json", json(&base)),
                        ("crates/cards/patches/index.json", json(&index_of(Vec::new()))),
                        (
                            "crates/cards/patches/shipped.json",
                            json(&Vec::<ShippedEntry>::new()),
                        ),
                    ],
                );
                assert_eq!(
                    check_patches(root).unwrap(),
                    strings(&[
                        "\"aaa\" differs from the newest shipped snapshot but no pending fragment claims it"
                    ])
                );
                write_fragment(root, &fragment_args("v0.2.5", "t", "s", "n", None)).unwrap();
                assert_eq!(check_patches(root).unwrap(), Vec::<String>::new());
            }
        }
    }

    // R388 (B4.2): card patches are data. `crates/cards/patches/patches.json` lists every shipped
    // patch in ship order, each `<version>.json` is the whole catalog as that patch left it,
    // `index.json` says in which versions each card changed, and `shipped.json` carries each patch's
    // shipping commit and snapshot blob. `CATALOG_VERSION` is the newest patch's version, everywhere
    // the string lives. A version is opaque (R105): its order is patches.json's, never a comparison
    // of strings.
    //
    // Several patches are built at once (R646): while a fragment is pending, the catalog differs from
    // the newest snapshot on exactly the claimed cards, and `patches ship` promotes each fragment
    // after it merges. The table the brief checked on 2026-09-30 for the history before v0.2.0 is
    // asserted below, card by card where it names cards.
    mod patches_test {
        use indexmap::{IndexMap, IndexSet};

        use super::{object, strings};
        use crate::patches::js::{Json, Object};
        use crate::patches::patches_io::*;

        fn patches() -> Vec<PatchEntry> {
            read_patches(&patches_json()).expect("crates/cards/patches/patches.json")
        }

        fn versions(patches: &[PatchEntry]) -> Vec<String> {
            patches.iter().map(|patch| patch.version.clone()).collect()
        }

        fn changes_of(patches: &[PatchEntry], version: &str) -> Vec<PatchChange> {
            patches
                .iter()
                .find(|patch| patch.version == version)
                .map(|patch| patch.changes.clone())
                .unwrap_or_default()
        }

        fn ids_of(patches: &[PatchEntry], version: &str, kind: ChangeKind) -> Vec<String> {
            changes_of(patches, version)
                .into_iter()
                .filter(|change| change.kind == kind)
                .map(|change| change.id)
                .collect()
        }

        /// The catalog the cards crate compiles in, each entry's fields in order, as a patch sees it: its shipped view (R1420).
        fn catalog() -> Catalog {
            super::super::shipped_view(
                serde_json::from_str(jackioh_cards::catalog_json()).expect("crates/cards/catalog.json"),
            )
        }

        fn snapshot(version: &str) -> Catalog {
            read_snapshot(version, &patches_dir()).unwrap_or_else(|error| panic!("{version}: {error:#}"))
        }

        fn fragments() -> Vec<NamedFragment> {
            read_fragments(&pending_dir()).expect("crates/cards/patches/pending")
        }

        fn sorted(mut items: Vec<String>) -> Vec<String> {
            items.sort();
            items
        }

        /// A changed line's fields, sorted; none for any other line.
        fn fields_of(changes: &[PatchChange], id: &str) -> Vec<String> {
            changes
                .iter()
                .find(|change| change.id == id)
                .filter(|change| change.kind == ChangeKind::Changed)
                .map_or_else(Vec::new, |change| sorted(change.fields().to_vec()))
        }

        fn is_hex40(text: &str) -> bool {
            text.len() == 40
                && text
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }

        mod r388_card_patch_history_b4_2 {
            use std::fs;

            use super::*;
            use crate::patches::patch::versions_at_sites;
            use crate::patches::tests::TempDir;
            use crate::patches::{is_iso_date, repo_root};

            #[test]
            fn r388_lists_every_patch_once_in_the_order_they_were_made_each_with_its_snapshot() {
                let patches = patches();
                let versions = versions(&patches);
                // Promotions only ever append (R646), so the history the file holds today can grow
                // past this prefix but never move it: assert the prefix, plus the newest patch main
                // shipped.
                assert_eq!(
                    versions[..8],
                    strings(&[
                        "v0.1.0", "v0.1.0b", "v0.1.0c", "v0.1.0d", "v0.1.1", "v0.2.0", "v0.2.1", "v0.2.2"
                    ])
                );
                assert!(versions.iter().any(|version| version == "v0.2.3"));
                assert_eq!(versions.iter().collect::<IndexSet<_>>().len(), versions.len());
                for patch in &patches {
                    let path = snapshot_path(&patch.version, &patches_dir()).unwrap();
                    assert!(path.exists(), "{}.json", patch.version);
                    assert!(is_iso_date(&patch.date), "{}", patch.version);
                    assert!(!patch.title.is_empty(), "{}", patch.version);
                    assert!(!patch.source.is_empty(), "{}", patch.version);
                    assert!(!patch.notes.is_empty(), "{}", patch.version);
                }
            }

            #[test]
            fn r388_makes_the_catalog_version_the_newest_patch_and_catalog_json_its_snapshot_apart_from_pending_fragments_r646()
             {
                let versions = versions(&patches());
                // Never a literal: `patches ship` moves the newest patch, and its pull request cannot
                // edit tests.
                assert_eq!(Some(&jackioh_cards::CATALOG_VERSION.to_string()), versions.last());
                let snapshot = snapshot(jackioh_cards::CATALOG_VERSION);
                // Pending fragments hold the catalog ahead of the newest snapshot on exactly their
                // claimed cards (R646): reverted to the snapshot, the catalog is the snapshot.
                let claimed: IndexSet<String> = fragments()
                    .into_iter()
                    .flat_map(|named| named.fragment.cards)
                    .collect();
                let catalog = catalog();
                let reverted = revert_pending(&catalog, &snapshot, &claimed);
                assert!(
                    same_catalog(&reverted, &snapshot),
                    "catalog.json with every pending-claimed entry reverted is the newest snapshot: claim the difference with `cargo jackioh patches <version> \"<title>\"`"
                );
                if claimed.is_empty() {
                    let snapshot_ids: Vec<&String> = snapshot.keys().collect();
                    let catalog_ids: Vec<&String> = catalog.keys().collect();
                    assert_eq!(snapshot_ids, catalog_ids);
                }
            }

            #[test]
            fn r646_lists_every_shipped_patch_once_in_shipped_json_with_the_commit_that_shipped_it_and_its_snapshots_blob()
             {
                let shipped = read_shipped(&shipped_json()).unwrap();
                let shipped_versions: Vec<String> =
                    shipped.iter().map(|entry| entry.version.clone()).collect();
                assert_eq!(shipped_versions, versions(&patches()));
                assert_eq!(
                    shipped_versions.iter().collect::<IndexSet<_>>().len(),
                    shipped.len()
                );
                for entry in &shipped {
                    assert!(is_hex40(&entry.commit), "{} commit", entry.version);
                    assert!(is_hex40(&entry.blob), "{} blob", entry.version);
                    // The blob is the snapshot file's bytes as git hashes them, so a rewritten
                    // snapshot fails.
                    let bytes =
                        fs::read_to_string(snapshot_path(&entry.version, &patches_dir()).unwrap()).unwrap();
                    assert_eq!(entry.blob, git_blob_hash(&bytes), "{}.json", entry.version);
                }
            }

            // The Rust binaries compile the version in from patches.json (SURFACE §11.3), so no source site carries it.
            #[test]
            fn r388_bumps_the_version_everywhere_the_string_lives_the_servers_env_example_and_render_yaml() {
                let sites = versions_at_sites(&repo_root()).unwrap();
                assert!(sites.iter().any(|site| site.file == "render.yaml"));
                assert!(sites.iter().any(|site| site.file == "crates/server/.env.example"));
                for site in sites {
                    assert_eq!(
                        site.version.as_deref(),
                        Some(jackioh_cards::CATALOG_VERSION),
                        "{}",
                        site.file
                    );
                }
            }

            // The Docker image needs no start command: the server compiles the version in from
            // patches.json and serves it whatever `CATALOG_VERSION` says (SURFACE §11.3), and `cargo
            // jackioh catalog-version` prints the same newest patch for the workflows that read it.
            #[test]
            fn r388_serves_the_catalog_version_from_the_patch_list_so_a_stale_dashboard_value_is_never_served()
             {
                // The version a deploy serves and stamps is the newest patch, which is CATALOG_VERSION.
                assert_eq!(
                    crate::catalog::newest_version(&patches_json()).unwrap(),
                    jackioh_cards::CATALOG_VERSION
                );

                // render.yaml leaves the start to the image, so no command can override the version.
                let render = fs::read_to_string(repo_root().join("render.yaml")).unwrap();
                assert!(
                    !render
                        .lines()
                        .any(|line| line.trim_start().starts_with("startCommand:"))
                );

                // A patch list that names no newest version stops the chain with nothing printed.
                let temp = TempDir::new("catalog-version-");
                let empty = temp.path().join("patches.json");
                fs::write(&empty, "[]").unwrap();
                let refused = format!(
                    "{:#}",
                    crate::catalog::newest_version(&empty).expect_err("refused")
                );
                assert!(refused.contains("names no newest version"), "{refused}");
            }

            #[test]
            fn r388_derives_each_patchs_card_by_card_changes_and_the_per_card_index_from_the_snapshots_alone()
            {
                let patches = patches();
                let mut previous: Option<Catalog> = None;
                for patch in &patches {
                    let snapshot = snapshot(&patch.version);
                    assert_eq!(
                        patch.changes,
                        diff_catalogs(previous.as_ref(), &snapshot),
                        "{} changes",
                        patch.version
                    );
                    previous = Some(snapshot);
                }
                let index: IndexMap<String, Vec<String>> =
                    serde_json::from_str(&fs::read_to_string(index_json()).unwrap()).unwrap();
                assert_eq!(index, build_index(&patches));
            }

            #[test]
            fn r388_rebuilds_the_history_the_brief_checked_v0_1_0_to_v0_1_1_from_git() {
                let patches = patches();
                // v0.1.0: the initial commit, Core as first built, 100 cards and 9 tokens.
                assert_eq!(ids_of(&patches, "v0.1.0", ChangeKind::Added).len(), 109);
                assert_eq!(changes_of(&patches, "v0.1.0").len(), 109);
                // v0.1.0b: #3, #68's name, #81 (twice), the Rush, Sheep, Felinor and Bread Tokens'
                // Radiant faces.
                assert_eq!(
                    ids_of(&patches, "v0.1.0b", ChangeKind::Changed),
                    strings(&[
                        "core-003",
                        "core-068",
                        "core-081",
                        "core-t-rush",
                        "core-t-sheep",
                        "core-t-felinor",
                        "core-t-bread"
                    ])
                );
                // v0.1.0c: #95's text; The Coin added.
                assert_eq!(
                    ids_of(&patches, "v0.1.0c", ChangeKind::Changed),
                    strings(&["core-095"])
                );
                assert_eq!(
                    ids_of(&patches, "v0.1.0c", ChangeKind::Added),
                    strings(&["core-t-coin"])
                );
                // v0.1.0d: the Radiant pass, 99 entries.
                assert_eq!(ids_of(&patches, "v0.1.0d", ChangeKind::Changed).len(), 99);
                // v0.1.1: the Ghoul Token added and 105 entries changed.
                assert_eq!(
                    ids_of(&patches, "v0.1.1", ChangeKind::Added),
                    strings(&["core-t-ghoul"])
                );
                assert_eq!(ids_of(&patches, "v0.1.1", ChangeKind::Changed).len(), 105);
                // Nothing has ever been removed.
                let removed: Vec<&PatchChange> = patches
                    .iter()
                    .flat_map(|patch| &patch.changes)
                    .filter(|change| change.kind == ChangeKind::Removed)
                    .collect();
                assert!(removed.is_empty(), "{removed:?}");
            }

            #[test]
            fn r388_records_patch_v0_2_0_the_two_new_sets_added_and_the_core_patches_of_issue_40() {
                let patches = patches();
                assert_eq!(ids_of(&patches, "v0.2.0", ChangeKind::Added).len(), 206);
                let before = snapshot("v0.1.1");
                let after = snapshot("v0.2.0");
                let cost =
                    |snapshot: &Catalog, id: &str| snapshot.get(id).and_then(|def| def.get("cost")).cloned();
                let ids: Vec<String> = [16, 17, 34, 43, 49, 65, 88]
                    .iter()
                    .map(|n| format!("core-0{n:02}"))
                    .collect();
                let costs = |snapshot: &Catalog| -> Vec<Option<Json>> {
                    ids.iter().map(|id| cost(snapshot, id)).collect()
                };
                let numbers = |list: &[i32]| -> Vec<Option<Json>> {
                    list.iter().map(|n| Some(Json::from(*n))).collect()
                };
                assert_eq!(costs(&before), numbers(&[2, 3, 3, 3, 3, 2, 3]));
                assert_eq!(costs(&after), numbers(&[3, 4, 4, 4, 4, 1, 4]));
                let changes = changes_of(&patches, "v0.2.0");
                let changed = changes
                    .iter()
                    .find(|change| change.id == "core-016")
                    .expect("core-016 changed");
                assert!(changed.fields().iter().any(|field| field == "cost"));
            }

            #[test]
            fn r388_records_patch_v0_2_3_eighteen_field_spells_animated_and_ivory_towers_text_issue_113() {
                let patches = patches();
                assert_eq!(
                    ids_of(&patches, "v0.2.3", ChangeKind::Added),
                    Vec::<String>::new()
                );
                assert_eq!(
                    ids_of(&patches, "v0.2.3", ChangeKind::Changed),
                    strings(&[
                        "core-014",
                        "core-033",
                        "core-038",
                        "core-065",
                        "core-073",
                        "classic-004",
                        "classic-007",
                        "classic-052",
                        "classic-062",
                        "classic-064",
                        "classic-087",
                        "classicplus-007",
                        "classicplus-012-5",
                        "classicplus-012-7",
                        "classicplus-031",
                        "classicplus-033",
                        "classicplus-061",
                        "classicplus-063",
                        "classicplus-070",
                        "classicplus-078",
                    ])
                );
                let after = snapshot("v0.2.3");
                let base_of = |id: &str| {
                    after
                        .get(id)
                        .and_then(|def| def.get("base"))
                        .and_then(Json::as_object)
                };
                let keyword = base_of("core-073")
                    .and_then(|face| face.get("keywords"))
                    .and_then(Json::as_array)
                    .and_then(|keywords| keywords.first())
                    .and_then(Json::as_object)
                    .and_then(|keyword| keyword.get("kind"))
                    .and_then(Json::as_str);
                assert_eq!(keyword, Some("Animated"));
                assert_eq!(
                    base_of("classicplus-033")
                        .and_then(|face| face.get("text"))
                        .and_then(Json::as_str),
                    Some("The first Unit you stack onto this is fused into it.")
                );
                // Final Gambit's follow-up gained its R216 guard: the script's lines move, nothing
                // printed does.
                let changes = changes_of(&patches, "v0.2.3");
                let gambit = changes
                    .iter()
                    .find(|change| change.id == "classic-052")
                    .expect("classic-052");
                assert_eq!(
                    (gambit.kind, gambit.fields()),
                    (ChangeKind::Changed, &strings(&["loc"])[..])
                );
            }

            #[test]
            fn r388_records_patch_v0_2_4_aimed_random_casts_and_the_deft_keyword_issue_181_pending_or_shipped_r646()
             {
                // Pending until `patches ship` promotes it, then shipped: either way the patch is
                // these five cards' changes against v0.2.3's snapshot.
                let five = strings(&[
                    "core-045",
                    "classic-003",
                    "classicplus-010",
                    "classicplus-038-1",
                    "classicplus-040",
                ]);
                let patches = patches();
                let shipped = versions(&patches).iter().any(|version| version == "v0.2.4");
                let pending = fragments()
                    .into_iter()
                    .find(|named| named.fragment.version == "v0.2.4")
                    .map(|named| named.fragment);
                assert!(shipped || pending.is_some(), "v0.2.4 is pending or shipped");
                if !shipped {
                    assert_eq!(
                        pending.as_ref().map(|fragment| fragment.cards.clone()),
                        Some(five.clone())
                    );
                }
                let changes: Vec<PatchChange> = if shipped {
                    changes_of(&patches, "v0.2.4")
                } else {
                    diff_catalogs(Some(&snapshot("v0.2.3")), &catalog())
                        .into_iter()
                        .filter(|change| five.contains(&change.id))
                        .collect()
                };
                assert!(changes.iter().all(|change| change.kind == ChangeKind::Changed));
                assert_eq!(
                    changes.iter().map(|change| change.id.clone()).collect::<Vec<_>>(),
                    five
                );
                assert_eq!(
                    fields_of(&changes, "core-045"),
                    strings(&[
                        "base.keywords",
                        "base.text",
                        "loc",
                        "radiant.keywords",
                        "radiant.text"
                    ])
                );
                assert_eq!(fields_of(&changes, "classic-003"), strings(&["loc"]));
                assert_eq!(fields_of(&changes, "classicplus-010"), strings(&["loc"]));
                assert_eq!(
                    fields_of(&changes, "classicplus-038-1"),
                    strings(&["base.text", "radiant.text"])
                );
                assert_eq!(
                    fields_of(&changes, "classicplus-040"),
                    strings(&["base.text", "radiant.text"])
                );
            }

            #[test]
            fn r388_records_patch_v0_2_6_book_of_wildfire_becomes_a_different_book_issue_271() {
                // Shipped by `patches ship` (R646): Classic #55 changed against v0.2.5's snapshot.
                let patches = patches();
                assert_eq!(
                    ids_of(&patches, "v0.2.6", ChangeKind::Added),
                    Vec::<String>::new()
                );
                assert_eq!(
                    ids_of(&patches, "v0.2.6", ChangeKind::Removed),
                    Vec::<String>::new()
                );
                assert_eq!(
                    ids_of(&patches, "v0.2.6", ChangeKind::Changed),
                    strings(&["classic-055"])
                );
                assert_eq!(
                    fields_of(&changes_of(&patches, "v0.2.6"), "classic-055"),
                    strings(&["base.text", "loc", "radiant.text"])
                );
            }

            #[test]
            fn r388_records_patch_v0_2_10_classic_and_classic_balance_patch_1_issue_88() {
                // Numbered v0.2.10, the next number (R743). Pending, the fragment claims the balance
                // cards and the catalog differs from the newest shipped snapshot on exactly those;
                // shipped, `patches ship` has recorded the same cards against the patch before it. The
                // test holds on both sides of the promotion, which cannot edit it.
                let patches = patches();
                let versions = versions(&patches);
                let fragment = fragments()
                    .into_iter()
                    .find(|named| named.fragment.version == "v0.2.10" && named.fragment.sources == "#88")
                    .map(|named| named.fragment);
                let at = versions.iter().position(|version| version == "v0.2.10");
                let before = snapshot(match at {
                    None => versions.last().expect("a newest patch"),
                    Some(at) => &versions[at - 1],
                });
                let after = if fragment.is_some() {
                    catalog()
                } else {
                    snapshot("v0.2.10")
                };
                let claimed = match &fragment {
                    Some(fragment) => fragment.cards.clone(),
                    None => ids_of(&patches, "v0.2.10", ChangeKind::Changed),
                };
                if fragment.is_none() {
                    assert_eq!(
                        ids_of(&patches, "v0.2.10", ChangeKind::Added),
                        Vec::<String>::new()
                    );
                    assert_eq!(
                        ids_of(&patches, "v0.2.10", ChangeKind::Removed),
                        Vec::<String>::new()
                    );
                }
                let changes: Vec<PatchChange> = diff_catalogs(Some(&before), &after)
                    .into_iter()
                    .filter(|change| claimed.contains(&change.id))
                    .collect();
                assert!(changes.iter().all(|change| change.kind == ChangeKind::Changed));
                assert_eq!(
                    changes.iter().map(|change| change.id.clone()).collect::<Vec<_>>(),
                    claimed
                );
                assert_eq!(claimed.len(), 62);
            }

            #[test]
            fn r388_records_patch_v0_2_5_animated_removed_from_eighteen_field_spells_issue_218() {
                // The eighteen, in catalog order — the order a fragment's `cards` and a patch's
                // `changes` use.
                let unanimated = strings(&[
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
                ]);
                let catalog = catalog();
                let face = catalog
                    .get("core-073")
                    .and_then(|def| def.get("base"))
                    .and_then(Json::as_object)
                    .expect("core-073");
                assert_eq!(face.get("attack"), None);
                assert_eq!(face.get("keywords"), Some(&Json::Array(Vec::new())));
                // Pending, the fragment is the patch's whole record (R646); shipped, `patches ship`
                // has promoted it to the list with a snapshot of this catalog. The test holds on both
                // sides of the promotion, which cannot edit it.
                let patches = patches();
                match fragments()
                    .into_iter()
                    .find(|named| named.fragment.version == "v0.2.5")
                {
                    Some(named) => assert_eq!(named.fragment.cards, unanimated),
                    None => {
                        assert_eq!(
                            ids_of(&patches, "v0.2.5", ChangeKind::Added),
                            Vec::<String>::new()
                        );
                        assert_eq!(
                            ids_of(&patches, "v0.2.5", ChangeKind::Removed),
                            Vec::<String>::new()
                        );
                        assert_eq!(ids_of(&patches, "v0.2.5", ChangeKind::Changed), unanimated);
                    }
                }
            }

            #[test]
            fn r388_records_patch_v0_2_1_card_text_pass_issue_45() {
                let patches = patches();
                assert_eq!(ids_of(&patches, "v0.2.1", ChangeKind::Added).len(), 0);
                assert_eq!(ids_of(&patches, "v0.2.1", ChangeKind::Removed).len(), 0);
                assert_eq!(ids_of(&patches, "v0.2.1", ChangeKind::Changed).len(), 22);
                assert!(changes_of(&patches, "v0.2.1").iter().all(|change| {
                    change.kind == ChangeKind::Changed
                        && change
                            .fields()
                            .iter()
                            .all(|field| field == "base.text" || field == "radiant.text")
                }));
            }

            #[test]
            fn r388_records_patch_v0_2_2_small_set_of_mechanics_changes_issue_149() {
                let patches = patches();
                assert_eq!(ids_of(&patches, "v0.2.2", ChangeKind::Added).len(), 0);
                assert_eq!(ids_of(&patches, "v0.2.2", ChangeKind::Removed).len(), 0);
                assert_eq!(ids_of(&patches, "v0.2.2", ChangeKind::Changed).len(), 22);
                let changes = changes_of(&patches, "v0.2.2");
                let plague: Vec<String> = ids_of(&patches, "v0.2.2", ChangeKind::Changed)
                    .into_iter()
                    .filter(|id| fields_of(&changes, id).iter().any(|field| field == "tags"))
                    .collect();
                assert_eq!(plague.len(), 17);
                assert!(
                    plague
                        .iter()
                        .all(|id| fields_of(&changes, id) == strings(&["tags"]))
                );
                // The mechanics: Exile's threshold, Joro's Spell-only text, Blade Storm's Whirlwind
                // cast and refs, Adaptive Growth's numbers, Chaos Machine's other-card text. A face
                // counts as changed when its printed words move, even when only a param's value
                // moved (Exile's base face, Adaptive Growth's Radiant face).
                assert_eq!(
                    fields_of(&changes, "classic-010"),
                    strings(&["base.text", "params"])
                );
                assert_eq!(
                    fields_of(&changes, "classic-033"),
                    strings(&["base.text", "radiant.text"])
                );
                assert_eq!(
                    fields_of(&changes, "classicplus-032-3"),
                    strings(&["base.text", "refs"])
                );
                assert_eq!(
                    fields_of(&changes, "classicplus-050"),
                    strings(&["base.text", "loc", "params", "radiant.text"])
                );
                assert_eq!(
                    fields_of(&changes, "classicplus-070"),
                    strings(&["base.text", "loc", "radiant.text"])
                );
                let before = snapshot("v0.2.1");
                let after = snapshot("v0.2.2");
                let param = |snapshot: &Catalog, id: &str, key: &str| -> Option<(Option<f64>, Option<f64>)> {
                    snapshot
                        .get(id)
                        .and_then(|def| def.get("params"))
                        .and_then(Json::as_array)?
                        .iter()
                        .filter_map(Json::as_object)
                        .find(|param| param.get("key").and_then(Json::as_str) == Some(key))
                        .map(|param| {
                            (
                                param.get("base").and_then(Json::as_f64),
                                param.get("radiant").and_then(Json::as_f64),
                            )
                        })
                };
                assert_eq!(
                    param(&before, "classic-010", "threshold"),
                    Some((Some(1.0), Some(3.0)))
                );
                assert_eq!(
                    param(&after, "classic-010", "threshold"),
                    Some((Some(2.0), Some(3.0)))
                );
                assert_eq!(
                    param(&before, "classicplus-050", "debuff"),
                    Some((Some(4.0), Some(4.0)))
                );
                assert_eq!(
                    param(&after, "classicplus-050", "debuff"),
                    Some((Some(2.0), Some(3.0)))
                );
            }

            #[test]
            fn r388_changed_fields_compares_a_faces_text_as_it_prints_its_params_filled_in() {
                let face = |threshold: i32, tail: &str| -> Object {
                    object(vec![
                        (
                            "params",
                            Json::Array(vec![Json::Object(object(vec![
                                ("key", Json::from("threshold")),
                                ("base", Json::from(threshold)),
                                ("radiant", Json::from(3)),
                            ]))]),
                        ),
                        (
                            "base",
                            Json::Object(object(vec![
                                ("keywords", Json::Array(Vec::new())),
                                (
                                    "text",
                                    Json::from(format!(
                                        "Counter a ({{threshold}}) Cost or less card. {tail}"
                                    )),
                                ),
                            ])),
                        ),
                        (
                            "radiant",
                            Json::Object(object(vec![
                                ("keywords", Json::Array(Vec::new())),
                                ("text", Json::from("No placeholder here.")),
                            ])),
                        ),
                    ])
                };
                assert_eq!(
                    changed_fields(&face(1, ""), &face(2, "")),
                    strings(&["params", "base.text"])
                );
                assert_eq!(
                    changed_fields(&face(1, "Draw 1."), &face(1, "Draw 2.")),
                    strings(&["base.text"])
                );
                assert_eq!(changed_fields(&face(1, ""), &face(1, "")), Vec::<String>::new());
            }

            #[test]
            fn r388_indexes_each_card_by_the_versions_that_added_or_changed_it() {
                let index = build_index(&patches());
                let first = |id: &str| {
                    index
                        .get(id)
                        .and_then(|versions| versions.first())
                        .map(String::as_str)
                };
                assert_eq!(first("core-t-coin"), Some("v0.1.0c"));
                assert_eq!(first("core-t-ghoul"), Some("v0.1.1"));
                assert_eq!(first("classic-001"), Some("v0.2.0"));
                assert!(
                    index
                        .get("core-016")
                        .is_some_and(|versions| versions.iter().any(|v| v == "v0.2.0"))
                );
                // Every catalog entry was added by some patch, or is claimed by a pending fragment —
                // which is not a shipped patch yet, so the index does not name it (R646).
                let claimed: IndexSet<String> = fragments()
                    .into_iter()
                    .flat_map(|named| named.fragment.cards)
                    .collect();
                let catalog = catalog();
                let unindexed: Vec<&String> = catalog
                    .keys()
                    .filter(|id| !index.contains_key(*id) && !claimed.contains(*id))
                    .collect();
                assert!(unindexed.is_empty(), "{unindexed:?}");
            }
        }

        // R743: the card patches are numbered in order, a normal patch taking the next number and a
        // micro patch the next letter. Commit messages keep the names they were written with, which the
        // shipping commits below tie to the new ones.
        mod r743_the_card_patches_numbered_in_order_issue_290 {
            use super::*;

            /// Each renamed patch: its former name, its shipping commit and its source.
            const RENAMED: &[(&str, &str, &str, &str)] = &[
                (
                    "v0.2.1",
                    "v0.2.4",
                    "9dcb65e481717a49bfc750b58ea67a11a1852f39",
                    "#45",
                ),
                (
                    "v0.2.2",
                    "v0.2.5",
                    "9e030abd6e3f846fde1f3e5003518d76ad512043",
                    "#149",
                ),
                (
                    "v0.2.3",
                    "v0.2.10",
                    "d26ab17eb8515eb35b9c3ca214c3343cb4fb1a8d",
                    "Issue #113",
                ),
                (
                    "v0.2.4",
                    "v0.2.11",
                    "7f3458e41b6c54bf5d35e52f45c4b5c124c3844d",
                    "#181, #206",
                ),
                (
                    "v0.2.5",
                    "v0.2.12",
                    "1be5a28e08c3b6ef94b95b3845c908adaa86d14e",
                    "Issue #218",
                ),
                (
                    "v0.2.6",
                    "v0.2.13",
                    "b26b1b9dde3f723528e60739caf5aee359a16c20",
                    "#271",
                ),
                (
                    "v0.2.7",
                    "v0.2.14",
                    "533b4e2db58eaae47b0edd0eee6da6d7e12439f3",
                    "#126",
                ),
                (
                    "v0.2.7b",
                    "v0.2.14b",
                    "80e7960ccaef1a8c0bb5c463eadacd8fd402d858",
                    "#260",
                ),
                (
                    "v0.2.8",
                    "v0.2.16",
                    "509e9a302ab71542a4e141fe7231c9743722bdba",
                    "#170",
                ),
                (
                    "v0.2.8b",
                    "v0.2.16b",
                    "16724d26939a1a03779523f11461b41c57871ccb",
                    "#323",
                ),
                (
                    "v0.2.9",
                    "v0.2.17",
                    "97a00bd613bd3adfaafdc4d1580f5d895d2b5e22",
                    "#44",
                ),
            ];

            /// A number the renumbering retired, as a word: `/\bv0\.2\.(1[0-4]b?|16b?|17)\b/u`.
            fn names_a_retired_number(text: &str) -> bool {
                const TAILS: &[&str] = &[
                    "10b", "10", "11b", "11", "12b", "12", "13b", "13", "14b", "14", "16b", "16", "17",
                ];
                let word = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
                text.match_indices("v0.2.").any(|(at, prefix)| {
                    let rest = &text[at + prefix.len()..];
                    !word(text[..at].chars().next_back())
                        && TAILS
                            .iter()
                            .any(|tail| rest.starts_with(tail) && !word(rest[tail.len()..].chars().next()))
                })
            }

            fn patch<'a>(patches: &'a [PatchEntry], version: &str) -> Option<&'a PatchEntry> {
                patches.iter().find(|patch| patch.version == version)
            }

            #[test]
            fn r743_lists_every_card_patch_after_v0_2_0_under_the_next_number_or_the_next_letter_for_a_micro_patch()
             {
                let versions = versions(&patches());
                // The shipped prefix (R646): promotions append after it and never move it.
                assert_eq!(
                    versions[..17],
                    strings(&[
                        "v0.1.0", "v0.1.0b", "v0.1.0c", "v0.1.0d", "v0.1.1", "v0.2.0", "v0.2.1", "v0.2.2",
                        "v0.2.3", "v0.2.4", "v0.2.5", "v0.2.6", "v0.2.7", "v0.2.7b", "v0.2.8", "v0.2.8b",
                        "v0.2.9",
                    ])
                );
                let renamed: Vec<String> = RENAMED
                    .iter()
                    .map(|(version, ..)| (*version).to_string())
                    .collect();
                assert_eq!(versions[6..17], renamed[..]);
            }

            #[test]
            fn r743_keeps_each_renamed_patchs_shipping_commit_source_and_snapshot_and_no_old_name_anywhere_in_the_history()
             {
                let patches = patches();
                let versions = versions(&patches);
                let shipped = read_shipped(&shipped_json()).unwrap();
                for (version, was, commit, source) in RENAMED {
                    assert_eq!(
                        shipped
                            .iter()
                            .find(|entry| entry.version == *version)
                            .map(|entry| entry.commit.as_str()),
                        Some(*commit),
                        "{version}"
                    );
                    assert_eq!(
                        patch(&patches, version).map(|patch| patch.source.as_str()),
                        Some(*source),
                        "{version}"
                    );
                    if let Some(commits) = patch(&patches, version).and_then(|patch| patch.commits.clone()) {
                        assert_eq!(commits, strings(&[*commit]), "{version}");
                    }
                    // The old name has no snapshot, unless a later patch has taken it in order
                    // (v0.2.4, v0.2.5).
                    if !versions.iter().any(|each| each == was) {
                        assert!(
                            !snapshot_path(was, &patches_dir()).unwrap().exists(),
                            "{was}.json"
                        );
                    }
                }
                // Their titles and notes name no number the renumbering retired.
                for (version, ..) in RENAMED {
                    let patch = patch(&patches, version);
                    let text = format!(
                        "{} {}",
                        patch.map_or("", |patch| patch.title.as_str()),
                        patch.map_or("", |patch| patch.notes.as_str())
                    );
                    assert!(!names_a_retired_number(&text), "{version}: {text}");
                }
            }

            #[test]
            fn r743_titles_each_renamed_normal_patch_with_its_new_number() {
                let patches = patches();
                let title_of = |version: &str| patch(&patches, version).map(|patch| patch.title.clone());
                let title = |text: &str| Some(text.to_string());
                assert_eq!(title_of("v0.2.1"), title("Patch v0.2.1: card text pass"));
                assert_eq!(
                    title_of("v0.2.3"),
                    title("Patch v0.2.3: Animated pass on Field Spells, Ivory Tower fuses")
                );
                assert_eq!(
                    title_of("v0.2.4"),
                    title("Patch v0.2.4: aimed random casts and the Deft keyword")
                );
                assert_eq!(
                    title_of("v0.2.5"),
                    title("Patch v0.2.5: undo the v0.2.3 animated additions")
                );
                assert_eq!(title_of("v0.2.7"), title("Patch v0.2.7: More card patches"));
                assert_eq!(title_of("v0.2.8"), title("Patch v0.2.8: Easter egg, Glitch"));
                assert_eq!(title_of("v0.2.9"), title("Patch v0.2.9: rarity pass"));
                // The two whose titles named no number keep them, and the micro patches keep their Y
                // (R650).
                assert_eq!(title_of("v0.2.2"), title("Small set of mechanics changes"));
                assert_eq!(
                    title_of("v0.2.6"),
                    title("Book of Wildfire becomes a different Book at the end of your turn")
                );
                assert_eq!(
                    title_of("v0.2.7b"),
                    title("Patch v0.2.Y: the yellow condition glow for ten Core cards")
                );
                assert_eq!(title_of("v0.2.8b"), title("Patch v0.2.Y: Buff Gary the Gambler"));
            }

            #[test]
            fn r743_has_issue_170s_glitch_shipped_as_v0_2_16_as_v0_2_8_the_patch_that_added_the_token() {
                let patches = patches();
                assert_eq!(
                    ids_of(&patches, "v0.2.8", ChangeKind::Added),
                    strings(&["classic-t-glitch"])
                );
                assert_eq!(
                    build_index(&patches)
                        .get("classic-t-glitch")
                        .and_then(|versions| versions.first())
                        .map(String::as_str),
                    Some("v0.2.8")
                );
            }
        }

        // R375: what the two builds of the history agreed on is held here: the versions before v0.2.0,
        // their order, and the cards they hold.
        mod r375_issue_39s_versions_of_the_patch_history {
            use super::*;

            #[test]
            fn r375_keeps_v0_1_0_v0_1_0b_v0_1_0c_v0_1_0d_and_v0_1_1_in_that_order_before_v0_2_0() {
                let versions = versions(&patches());
                let cut = versions
                    .iter()
                    .position(|version| version == "v0.2.0")
                    .expect("v0.2.0");
                assert_eq!(
                    versions[..cut],
                    strings(&["v0.1.0", "v0.1.0b", "v0.1.0c", "v0.1.0d", "v0.1.1"])
                );
            }

            #[test]
            fn r375_has_v0_1_0_as_100_cards_and_9_tokens_and_the_coin_first_in_v0_1_0c() {
                let first = snapshot("v0.1.0");
                let is_token = |def: &Object| {
                    def.get("token") == Some(&Json::Bool(true))
                        || def
                            .get("tags")
                            .and_then(Json::as_array)
                            .is_some_and(|tags| tags.iter().any(|tag| tag.as_str() == Some("Token")))
                };
                let tokens = first.values().filter(|def| is_token(def)).count();
                assert_eq!(first.len() - tokens, 100);
                assert_eq!(tokens, 9);
                assert!(!snapshot("v0.1.0").contains_key("core-t-coin"));
                assert!(snapshot("v0.1.0c").contains_key("core-t-coin"));
            }
        }
    }
}
