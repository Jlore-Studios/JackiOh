//! The keyword glossary, and rules text as tokens: plain text, and glossary terms the face prints in
//! bold (docs/polish/6-cards.md, Surface B "Tokenizer"; R2098000 for the tooltips).
//!
//! Every `rule` is reviewed data in a player's words (#133, `data/glossary.json`); each entry names its
//! spec `section` (§6.1 unit keywords, §6.2 triggers, §6.3 verbs, §5.2 Radiant) and, if the spec row is
//! spelled differently, its `row`. `label` is what the tokenizer matches, case-sensitively; `aliases` are
//! other spellings.
//!
//! Matching is case-sensitive and longest-first over every label and alias, and over the plain
//! spellings, which match like terms but print plain (R910000: "your exile" is the pile), only at a word
//! boundary (so "Locking" and "taunt" stay plain). A match extends over a following ` <digits|X|♾️>` and
//! then a `:`, so "Cry:", "Armor 2", "Combo 2:" and "Activate ♾️:" (R384) are each one term.

use jackioh_engine::wire::{Enchantment, KeywordKind};

use super::data::{CHINESE_GLOSSARY, GLOSSARY, GlossaryRow, PATTERNS, find};
use super::types::{FaceTuning, GlossaryEntry, TuneChange, Way};

/// The row of the glossary a term names, in English.
pub fn row_of(id: &str) -> Option<&'static GlossaryRow> {
    GLOSSARY.iter().find(|row| row.id == id)
}

fn entry_of(row: &GlossaryRow) -> GlossaryEntry {
    GlossaryEntry {
        id: row.id.to_string(),
        label: row.label.to_string(),
        rule: row.rule.to_string(),
        section: row.section.to_string(),
        row: row.row.map(str::to_string),
    }
}

/// The glossary entry for a term, in English.
pub fn entry(id: &str) -> Option<GlossaryEntry> {
    row_of(id).map(entry_of)
}

/// ME-CN, R1301: the entry with its label and rule in Chinese (R1303's table), or as it is where the table has none.
pub fn chinese_entry(entry: GlossaryEntry) -> GlossaryEntry {
    match find(CHINESE_GLOSSARY, &entry.id) {
        Some((label, rule)) => GlossaryEntry {
            label: (*label).to_string(),
            rule: (*rule).to_string(),
            ..entry
        },
        None => entry,
    }
}

/// A piece of rules text: plain words, or a glossary term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Text(String),
    Term { text: String, term: &'static str },
}

fn is_word_char(c: Option<&char>) -> bool {
    c.is_some_and(|c| c.is_ascii_alphanumeric())
}

/// ` 3`, ` 10`, ` X` or ` ♾️` (Activate's "any number of times", with or without its emoji selector),
/// ending at a word boundary: how many characters of `rest` it takes.
fn number_suffix(rest: &[char]) -> Option<usize> {
    if rest.first() != Some(&' ') {
        return None;
    }
    let body = &rest[1..];
    let digits = body.iter().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 {
        return (!is_word_char(body.get(digits))).then_some(1 + digits);
    }
    match body.first() {
        Some('X') => (!is_word_char(body.get(1))).then_some(2),
        Some('\u{267E}') => {
            if body.get(1) == Some(&'\u{FE0F}') && !is_word_char(body.get(2)) {
                Some(3)
            } else {
                (!is_word_char(body.get(1))).then_some(2)
            }
        }
        _ => None,
    }
}

/// How many characters `spelling` takes at `at`, if `text` spells it there.
fn spelled_at(text: &[char], at: usize, spelling: &str) -> Option<usize> {
    let mut i = at;
    for c in spelling.chars() {
        if text.get(i) != Some(&c) {
            return None;
        }
        i += 1;
    }
    Some(i - at)
}

/// The match at `start`: a term, or a plain spelling (`term` None) that the caller prints as text.
fn match_at(text: &[char], start: usize) -> Option<(usize, Option<&'static str>)> {
    if start > 0 && is_word_char(text.get(start - 1)) {
        return None;
    }
    for (spelling, term) in PATTERNS {
        let Some(length) = spelled_at(text, start, spelling) else {
            continue;
        };
        let mut end = start + length;
        if is_word_char(text.get(end)) {
            continue;
        }
        let Some(term) = term else {
            return Some((end, None));
        };
        if let Some(taken) = number_suffix(&text[end..]) {
            end += taken;
        }
        if text.get(end) == Some(&':') {
            end += 1;
        }
        return Some((end, Some(term)));
    }
    None
}

/// `text` as tokens: plain words and terms, the terms in bold.
pub fn tokenize(text: &[char]) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut plain = String::new();
    let mut index = 0;
    while index < text.len() {
        match match_at(text, index) {
            None => {
                plain.push(text[index]);
                index += 1;
            }
            Some((end, None)) => {
                plain.extend(&text[index..end]);
                index = end;
            }
            Some((end, Some(term))) => {
                if !plain.is_empty() {
                    tokens.push(Token::Text(std::mem::take(&mut plain)));
                }
                tokens.push(Token::Term {
                    text: text[index..end].iter().collect(),
                    term,
                });
                index = end;
            }
        }
    }
    if !plain.is_empty() {
        tokens.push(Token::Text(plain));
    }
    tokens
}

/// Distinct terms in order of first appearance.
pub fn terms_in(text: &[char]) -> Vec<&'static str> {
    let mut seen: Vec<&'static str> = Vec::new();
    for token in tokenize(text) {
        if let Token::Term { term, .. } = token
            && !seen.contains(&term)
        {
            seen.push(term);
        }
    }
    seen
}

/// The terms a face in play's states name, so its badges are explained beside it: a Brittle count (R385), a
/// Buff's or a Nerf's changes and the keywords they added (R386, R1320), Cast on draw riding the card (E39),
/// and a card standing as a Unit (R383).
pub fn state_terms(
    brittle: Option<i32>,
    tuning: Option<&FaceTuning>,
    enchantments: &[Enchantment],
    animated: Option<Option<i32>>,
) -> Vec<String> {
    let mut ids = Vec::new();
    if brittle.is_some() {
        ids.push("Brittle".to_string());
    }
    if let Some(tuning) = tuning {
        let moved = |way: Way| {
            tuning
                .changes
                .iter()
                .any(|change| !matches!(change, TuneChange::Set { .. }) && way_of(change) == Some(way))
        };
        if moved(Way::Better) {
            ids.push("Buff".to_string());
        }
        if moved(Way::Worse) {
            ids.push("Nerf".to_string());
        }
        for keyword in &tuning.added {
            ids.push(keyword.kind().as_str().to_string());
        }
    }
    if enchantments
        .iter()
        .any(|entry| matches!(entry, Enchantment::CastOnDraw))
    {
        ids.push("Cast on draw".to_string());
    }
    if let Some(home) = animated {
        ids.push(
            if home.is_none() {
                "Animated"
            } else {
                "Animated on your turn"
            }
            .to_string(),
        );
    }
    ids
}

fn way_of(change: &TuneChange) -> Option<Way> {
    match change {
        TuneChange::Stat { way, .. }
        | TuneChange::Keyword { way, .. }
        | TuneChange::Count { way, .. }
        | TuneChange::Number { way, .. } => Some(*way),
        TuneChange::Set { .. } => None,
    }
}

/// The entries a face names: its text's terms, then its keywords, then its states'. ME-CN, R1301: a Chinese
/// face's terms are found in its English text, and each entry reads in Chinese (R1303's table).
pub fn glossary_for(
    text: &[char],
    keywords: &[KeywordKind],
    states: &[String],
    in_chinese: bool,
) -> Vec<GlossaryEntry> {
    let mut ids: Vec<String> = Vec::new();
    let mut add = |id: &str| {
        if !ids.iter().any(|known| known == id) {
            ids.push(id.to_string());
        }
    };
    for id in terms_in(text) {
        add(id);
    }
    for kind in keywords {
        add(kind.as_str());
    }
    for id in states {
        add(id);
    }
    ids.iter()
        .filter_map(|id| entry(id))
        .map(|entry| if in_chinese { chinese_entry(entry) } else { entry })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str) -> Vec<Token> {
        tokenize(&text.chars().collect::<Vec<_>>())
    }

    fn term(text: &str, term: &'static str) -> Token {
        Token::Term {
            text: text.to_string(),
            term,
        }
    }

    #[test]
    fn a_term_takes_its_number_and_its_colon() {
        assert_eq!(
            tokens("Cry: Armor 2"),
            vec![
                term("Cry:", "Cry"),
                Token::Text(" ".into()),
                term("Armor 2", "Armor")
            ]
        );
        assert_eq!(
            tokens("Combo 2: draw"),
            vec![term("Combo 2:", "Combo"), Token::Text(" draw".into())]
        );
        assert_eq!(
            tokens("Activate \u{267E}\u{FE0F}: x"),
            vec![
                term("Activate \u{267E}\u{FE0F}:", "Activate"),
                Token::Text(" x".into())
            ]
        );
    }

    #[test]
    fn a_term_inside_a_word_stays_plain() {
        assert_eq!(tokens("Locking taunt"), vec![Token::Text("Locking taunt".into())]);
    }

    #[test]
    fn the_exile_pile_is_plain_and_the_verb_bold() {
        assert_eq!(tokens("from exile"), vec![Token::Text("from exile".into())]);
        assert_eq!(
            tokens("Exile a unit"),
            vec![term("Exile", "Exile"), Token::Text(" a unit".into())]
        );
    }

    #[test]
    fn the_terms_of_a_text_are_listed_once_in_order() {
        assert_eq!(
            terms_in(&"Rush, Taunt. Rush".chars().collect::<Vec<_>>()),
            vec!["Rush", "Taunt"]
        );
    }
}
