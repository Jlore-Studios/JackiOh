//! The face model's data as the build script compiled it in (`build.rs`): immutable tables, looked up by
//! binary search where they are sorted.

/// One term of the keyword glossary (SPEC §6), in English; `chinese.rs` gives it its Chinese words.
pub struct GlossaryRow {
    pub id: &'static str,
    pub label: &'static str,
    pub rule: &'static str,
    pub section: &'static str,
    pub row: Option<&'static str>,
    pub aliases: &'static [&'static str],
}

/// One of the thirteen powers of #98 Heroic Power (§8 #98, R752).
pub struct PowerRow {
    pub name: &'static str,
    pub x: i32,
    pub title: &'static str,
    pub radiant_title: &'static str,
    pub base: &'static str,
    pub radiant: &'static str,
}

/// How a face's text shortens when its box cannot hold it (`short.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortForm {
    /// The first n lines, "…" in place of the last full stop.
    Lines(usize),
    /// Up to the first colon.
    Colon,
    /// The whole text, cut by the box at its last line.
    Clamp,
}

/// A card in Simplified Chinese (ME-CN, R1303): its name, both faces' texts and its preview labels.
pub struct ChineseRow {
    pub name: &'static str,
    pub base: &'static str,
    pub radiant: &'static str,
    pub previews: &'static [(&'static str, &'static str)],
}

/// A card's short forms: on its base face, and on its Radiant face.
pub type ShortPair = (Option<ShortForm>, Option<ShortForm>);

include!(concat!(env!("OUT_DIR"), "/face_data.rs"));

/// The value of `key` in a table sorted by key.
pub fn find<'a, T>(table: &'a [(&'static str, T)], key: &str) -> Option<&'a T> {
    table
        .binary_search_by(|(candidate, _)| (*candidate).cmp(key))
        .ok()
        .map(|at| &table[at].1)
}
