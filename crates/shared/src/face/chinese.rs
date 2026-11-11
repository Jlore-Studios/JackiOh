//! A card's words in Simplified Chinese (ME-CN, R1300–R1303): the two sidecars beside the catalog
//! (`crates/cards/chinese.json` and `chinese-terms.json`, compiled in by `build.rs`), the way the web read
//! them before this crate did. Presentation only (CLAUDE.md rule 7): no rule reads them.
//!
//! The first table is keyed by catalog id: each card's and token's name and both faces' texts, each text
//! with exactly the `{key}`s its English face has (an English `{key|singular|plural}` is a plain `{key}`
//! there, a measure word beside it), and its preview labels. The second holds the frame's words.
//!
//! A card is drawn in Chinese where the view says it is (`CardView.chinese`, R1301), whatever the reader's
//! language, and for a reader who reads Chinese (R711000) every card is drawn from these tables, in a match
//! and in the collection alike.

use jackioh_engine::config::GLITCH_DEF_ID;
use jackioh_engine::wire::{CardDef, CardType, Keyword, KeywordKind, Tag};

use super::data::{
    CHINESE_ALL_TRIBES, CHINESE_CARDS, CHINESE_CREATED, CHINESE_GLOSSARY, CHINESE_KEYWORDS, CHINESE_RADIANT,
    CHINESE_TAGS, CHINESE_TYPES, ChineseRow, find,
};
use super::fused::{DORMANT_STARSHIP_ID, holds_dormant_starship};
use super::tuning::fill_params_text;
use super::words::Words;

/// What joins a list of keywords on a Chinese face, as the table's texts join them ("嘲讽，圣盾，复生").
pub const CHINESE_COMMA: &str = "，";

/// R102, R468: what joins a fused definition's ingredients' texts (one per line) and names.
const FUSED_LINE: &str = "\n";
const FUSED_NAME: &str = " + ";

/// Glitch (issue #170, SPEC §7) wears corrupted words wherever a lookup answers for it (`glitch.rs`).
pub fn is_glitch(def_id: &str) -> bool {
    def_id == GLITCH_DEF_ID
}

/// R711000: whether the reader's language has a card table.
pub fn reader_has_table(words: Words) -> bool {
    words.is_zh_cn()
}

/// The frame's Chinese word for a card type (R1303).
pub fn type_word(type_: CardType) -> &'static str {
    find(CHINESE_TYPES, type_.as_str())
        .copied()
        .unwrap_or(type_.as_str())
}

/// The frame's Chinese word for a tag (R1303).
pub fn tag_word(tag: Tag) -> &'static str {
    find(CHINESE_TAGS, tag.as_str()).copied().unwrap_or(tag.as_str())
}

/// The frame's Chinese word for a keyword kind (R1303).
pub fn keyword_kind_word(kind: KeywordKind) -> &'static str {
    find(CHINESE_KEYWORDS, kind.as_str())
        .copied()
        .unwrap_or(kind.as_str())
}

/// What the frame prints in place of the five tribal tags on a card with them all (R1382).
pub fn all_tribes_word() -> &'static str {
    CHINESE_ALL_TRIBES
}

/// "生成": the Created badge's text (MD-B6, R943).
pub fn created_word() -> &'static str {
    CHINESE_CREATED
}

/// The word before a card's name that points a reference at its Radiant face.
pub fn radiant_word() -> &'static str {
    CHINESE_RADIANT
}

fn entry_of(def_id: &str) -> Option<&'static ChineseRow> {
    find(CHINESE_CARDS, def_id)
}

/// R468: a fused definition's ingredients' entries, in order, or `None` when one has none.
fn ingredient_entries(def: &CardDef) -> Option<Vec<(&'static ChineseRow, bool)>> {
    let ingredients = def.ingredients.as_ref().filter(|list| !list.is_empty())?;
    ingredients
        .iter()
        .map(|ingredient| entry_of(&ingredient.def_id).map(|entry| (entry, ingredient.radiant == Some(true))))
        .collect()
}

/// R3045000: the Dormant Starship's name in Chinese, for a fusion that holds its hull: a Starship keeps the
/// hull's name whatever was fused into it, as the English face does (`model.rs`); `None` for any other definition.
fn hull_name_of(def: &CardDef) -> Option<&'static str> {
    if holds_dormant_starship(def) {
        entry_of(DORMANT_STARSHIP_ID).map(|entry| entry.name)
    } else {
        None
    }
}

/// R1301: `def` with its name and both faces' texts in Chinese. A fused one's (R468) are its ingredients'
/// joined as the engine joins the English (R469: one fused on its Radiant face puts that face in the base
/// text too), but a fused Dormant Starship's name is the hull's (R3045000). Any other with no entry, and
/// Glitch, comes back as given.
pub fn chinese_def(def: &CardDef) -> CardDef {
    if is_glitch(&def.id) {
        return def.clone();
    }
    let mut out = def.clone();
    if let Some(own) = entry_of(&def.id) {
        out.name = own.name.to_string();
        out.base.text = own.base.to_string();
        out.radiant.text = own.radiant.to_string();
        return out;
    }
    let hull = hull_name_of(def);
    let Some(parts) = ingredient_entries(def) else {
        if let Some(hull) = hull {
            out.name = hull.to_string();
        }
        return out;
    };
    out.name = match hull {
        Some(hull) => hull.to_string(),
        None => parts
            .iter()
            .map(|(entry, _)| entry.name)
            .collect::<Vec<_>>()
            .join(FUSED_NAME),
    };
    out.base.text = parts
        .iter()
        .map(|(entry, radiant)| if *radiant { entry.radiant } else { entry.base })
        .collect::<Vec<_>>()
        .join(FUSED_LINE);
    out.radiant.text = parts
        .iter()
        .map(|(entry, _)| entry.radiant)
        .collect::<Vec<_>>()
        .join(FUSED_LINE);
    out
}

/// R1301: a keyword as a Chinese face prints it: its word, with its number stuck to it ("护甲7").
pub fn chinese_keyword(keyword: &Keyword) -> String {
    let word = keyword_kind_word(keyword.kind());
    match keyword.n() {
        Some(n) => format!("{word}{n}"),
        None => word.to_string(),
    }
}

/// R280, R1301: a preview label in Chinese: of the card's templates (its own or its ingredients'), filled as
/// its face is, the one whose English fill is `label` gives its Chinese template, filled the same way. A
/// label no template fills comes back as it is.
pub fn chinese_preview_label(
    def: &CardDef,
    radiant: bool,
    label: &str,
    values: Option<&indexmap::IndexMap<String, i32>>,
) -> String {
    let fill = |template: &str| fill_params_text(def, template, radiant, values);
    let ids = std::iter::once(def.id.as_str()).chain(
        def.ingredients
            .iter()
            .flatten()
            .map(|ingredient| ingredient.def_id.as_str()),
    );
    for id in ids {
        for (english, chinese) in entry_of(id).map_or(&[][..], |entry| entry.previews) {
            if fill(english) == label {
                return fill(chinese);
            }
        }
    }
    label.to_string()
}

/// R711000: a keyword or glossary term in a tuning change's English ("Armor 2") as the reader reads it
/// ("护甲2"); as given when the reader's language has no table or the text names no term.
pub fn reader_keyword(words: Words, text: &str) -> String {
    if !reader_has_table(words) {
        return text.to_string();
    }
    // The glossary's labels, then the keywords' words over them: the longest id that is the text or starts it.
    let mut best: Option<(&str, &str)> = None;
    let candidates = CHINESE_GLOSSARY
        .iter()
        .map(|(id, (label, _))| (*id, *label))
        .filter(|(id, _)| find(CHINESE_KEYWORDS, id).is_none())
        .chain(CHINESE_KEYWORDS.iter().map(|(id, word)| (*id, *word)));
    for (id, word) in candidates {
        let named = text == id || text.strip_prefix(id).is_some_and(|rest| rest.starts_with(' '));
        if named && best.is_none_or(|(known, _)| id.chars().count() > known.chars().count()) {
            best = Some((id, word));
        }
    }
    match best {
        Some((id, word)) => format!("{word}{}", text[id.len()..].trim()),
        None => text.to_string(),
    }
}

/// R711000: what joins a list of phrases in the reader's language: "; ", or "；" for a reader of Chinese.
pub fn list_separator(words: Words) -> &'static str {
    if words.is_zh_cn() { "；" } else { "; " }
}

/// R1301: the Chinese name of the card `def_id` names: its entry's, or, given the definition, a fused
/// card's (`chinese_def`, a fused Dormant Starship's the hull's, R3045000); else `english`, as given.
pub fn chinese_name(def_id: &str, english: &str, def: Option<&CardDef>) -> String {
    if is_glitch(def_id) {
        return english.to_string();
    }
    if let Some(own) = entry_of(def_id) {
        return own.name.to_string();
    }
    match def {
        Some(def) if ingredient_entries(def).is_some() || hull_name_of(def).is_some() => {
            chinese_def(def).name
        }
        _ => english.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::types::Locale;

    #[test]
    fn a_keyword_sticks_its_number_to_its_word() {
        assert_eq!(chinese_keyword(&Keyword::Armor { n: 7 }), "护甲7");
        assert_eq!(chinese_keyword(&Keyword::Taunt), "嘲讽");
    }

    #[test]
    fn a_reader_of_chinese_reads_a_keyword_in_a_change_in_chinese() {
        let zh = Words::new(Locale::ZhCn);
        assert_eq!(reader_keyword(zh, "Armor 2"), "护甲2");
        assert_eq!(reader_keyword(zh, "Taunt"), "嘲讽");
        assert_eq!(reader_keyword(zh, "Nothing"), "Nothing");
        assert_eq!(reader_keyword(Words::new(Locale::En), "Armor 2"), "Armor 2");
    }
}
