//! Where a card's text names another card (SPEC §10.10, R279). The catalog lists the cards and tokens a
//! card's text names (`CardDef.refs`), and `crates/cards/tests/cross/references.rs` proves the list with
//! one naming rule, which this module reads the same way. Presentation only (CLAUDE.md rule 7): the faces a
//! reference shows are printed catalog faces, public by §5.1.

use jackioh_engine::wire::CardDef;

use super::text::find_from;

/// A name in a text that points at a card: [start, end) in characters, the card's id, and whether the
/// name points at its Radiant face.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefMatch {
    pub start: usize,
    pub end: usize,
    pub id: String,
    pub radiant: bool,
}

const PLURAL: char = 's';
/// The word before a name that points it at the Radiant face, in English.
pub const RADIANT_WORD: &str = "Radiant ";

/// R903: one named alias — "Coin" names The Coin, which M #27's texts call by the short name.
fn aliases_of(name: &str) -> &'static [&'static str] {
    if name == "The Coin" { &["Coin"] } else { &[] }
}

/// Rules phrases that hold a card's name without naming it (R961): "Jade Counter" holds "Jade" (Meditative
/// #39.2), which names nothing inside it; "Fortify Mind" (M #22.1) holds "Fortify" (Training Grounds #12)
/// and still names itself (R2198004).
const RULES_PHRASES: [&str; 2] = ["Jade Counter", "Fortify Mind"];

fn is_word_char(c: Option<&char>) -> bool {
    c.is_some_and(|c| c.is_ascii_alphanumeric() || *c == '\'' || *c == '-')
}

/// A card's name before its trailing parenthesis, full-width in Chinese (R1301): `/\s*[(（].*[)）]\s*$/` taken out.
fn bare_name(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let Some(last) = chars.iter().rposition(|c| !c.is_whitespace()) else {
        return name.to_string();
    };
    if !matches!(chars[last], ')' | '）') {
        return name.to_string();
    }
    let Some(open) =
        (0..last).find(|at| matches!(chars[*at], '(' | '（') && !chars[*at..last].contains(&'\n'))
    else {
        return name.to_string();
    };
    let mut from = open;
    while from > 0 && chars[from - 1].is_whitespace() {
        from -= 1;
    }
    chars[..from].iter().collect()
}

/// A card's names in a text: its name, and its name before a parenthesis (full-width in Chinese, R1301).
pub fn names_of(def: &CardDef) -> Vec<String> {
    let bare = bare_name(&def.name);
    let mut out = if bare == def.name {
        vec![def.name.clone()]
    } else {
        vec![def.name.clone(), bare]
    };
    out.extend(aliases_of(&def.name).iter().map(|alias| (*alias).to_string()));
    out
}

/// Whether the match at [start, end) lies inside a rules phrase, which reads as the phrase's word. A match
/// that is the whole phrase is not inside it (R2198004).
fn inside_rules_phrase(text: &[char], start: usize, end: usize) -> bool {
    RULES_PHRASES.iter().any(|phrase| {
        let phrase: Vec<char> = phrase.chars().collect();
        let mut from = 0;
        while let Some(at) = find_from(text, &phrase, from) {
            if at <= start && end <= at + phrase.len() && !(at == start && end == at + phrase.len()) {
                return true;
            }
            from = at + 1;
        }
        false
    })
}

/// Every whole-word occurrence of `name` (plural included) in `text`, as [start, end) pairs.
fn occurrences(text: &[char], name: &[char]) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = find_from(text, name, from) {
        let mut end = at + name.len();
        if text.get(end) == Some(&PLURAL) {
            end += 1;
        }
        if !(at > 0 && is_word_char(text.get(at - 1)))
            && !is_word_char(text.get(end))
            && !inside_rules_phrase(text, at, end)
        {
            found.push((at, end));
        }
        from = at + 1;
    }
    found
}

/// The names in `text` that point at one of `refs`, in text order and never overlapping: where two names
/// start at the same place the longer wins, and a name inside another name's stretch is not a name of its
/// own. `radiant_word` is the word before a name that points it at the Radiant face: a Chinese text
/// (R1301) writes "光辉" right before the name, with no space.
pub fn find_refs(text: &[char], refs: &[CardDef], radiant_word: &str) -> Vec<RefMatch> {
    let radiant_word: Vec<char> = radiant_word.chars().collect();
    let mut candidates: Vec<RefMatch> = Vec::new();
    for def in refs {
        for name in names_of(def) {
            let name: Vec<char> = name.chars().collect();
            for (start, end) in occurrences(text, &name) {
                let from = start.saturating_sub(radiant_word.len());
                candidates.push(RefMatch {
                    start,
                    end,
                    id: def.id.clone(),
                    radiant: text[from..start] == radiant_word[..],
                });
            }
        }
    }
    candidates.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));
    let mut kept: Vec<RefMatch> = Vec::new();
    let mut reach = 0;
    let mut reached = false;
    for candidate in candidates {
        if reached && candidate.start < reach {
            continue;
        }
        reach = candidate.end;
        reached = true;
        kept.push(candidate);
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(id: &str, name: &str) -> CardDef {
        serde_json::from_value(serde_json::json!({
            "id": id, "index": "1", "name": name, "set": "Core", "type": "Unit", "tags": [], "rarity": "Common", "token": false,
            "cost": 1, "base": { "keywords": [], "text": "" }, "radiant": { "keywords": [], "text": "" }
        }))
        .expect("a definition")
    }

    fn found(text: &str, refs: &[CardDef]) -> Vec<(usize, usize, String, bool)> {
        let text: Vec<char> = text.chars().collect();
        find_refs(&text, refs, RADIANT_WORD)
            .into_iter()
            .map(|m| (m.start, m.end, m.id, m.radiant))
            .collect()
    }

    #[test]
    fn a_name_links_alone_or_plural_and_never_inside_a_word() {
        let ghoul = def("t-1", "Ghoul Token");
        assert_eq!(
            found("Summon a Ghoul Token.", std::slice::from_ref(&ghoul)),
            vec![(9, 20, "t-1".to_string(), false)]
        );
        assert_eq!(
            found("Summon two Ghoul Tokens.", std::slice::from_ref(&ghoul)),
            vec![(11, 23, "t-1".to_string(), false)]
        );
        assert!(found("Summon a XGhoul Token.", &[ghoul]).is_empty());
    }

    #[test]
    fn a_radiant_word_before_a_name_points_at_the_radiant_face() {
        let ghoul = def("t-1", "Ghoul Token");
        assert_eq!(
            found("Summon a Radiant Ghoul Token.", &[ghoul]),
            vec![(17, 28, "t-1".to_string(), true)]
        );
    }

    #[test]
    fn the_longer_name_wins_where_two_start_together() {
        let rush = def("t-2", "Rush");
        let token = def("t-3", "Rush Token");
        assert_eq!(
            found("A Rush Token.", &[rush, token]),
            vec![(2, 12, "t-3".to_string(), false)]
        );
    }

    #[test]
    fn a_name_before_a_parenthesis_is_a_name_too() {
        assert_eq!(
            names_of(&def("t-4", "Call to Chaos (Edition)")),
            vec!["Call to Chaos (Edition)", "Call to Chaos"]
        );
        assert_eq!(names_of(&def("t-5", "The Coin")), vec!["The Coin", "Coin"]);
    }
}
