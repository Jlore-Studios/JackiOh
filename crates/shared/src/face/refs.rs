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

    /// R961, R2198004: a name inside a rules phrase names nothing ("Jade" in "Jade Counter", at the text's start
    /// too), a name that is the whole phrase names its card, and the name outside the phrase still names its own.
    #[test]
    fn r961_a_name_inside_a_rules_phrase_names_nothing_and_the_whole_phrase_itself() {
        let jade = def("m-39", "Jade");
        let fortify = def("tg-12", "Fortify");
        let mind = def("m-22", "Fortify Mind");
        assert!(found("Gain a Jade Counter.", std::slice::from_ref(&jade)).is_empty());
        assert!(found("Jade Counter: gain one.", std::slice::from_ref(&jade)).is_empty());
        assert_eq!(
            found("A Jade Counter, then Jade.", &[jade]),
            vec![(21, 25, "m-39".to_string(), false)]
        );
        assert!(found("Cast Fortify Mind.", std::slice::from_ref(&fortify)).is_empty());
        assert_eq!(
            found("Fortify Mind: draw a card.", std::slice::from_ref(&mind)),
            vec![(0, 12, "m-22".to_string(), false)]
        );
        assert_eq!(
            found("Fortify, then Fortify Mind.", &[fortify, mind]),
            vec![
                (0, 7, "tg-12".to_string(), false),
                (14, 26, "m-22".to_string(), false)
            ]
        );
    }

    /// R279: a name at the text's start, two names apart and two that touch are each a name; a hyphen joins a word, so
    /// a name after one is inside it.
    #[test]
    fn r279_names_at_the_start_apart_or_touching_are_names_and_a_hyphen_joins_a_word() {
        let ghoul = def("t-1", "Ghoul Token");
        assert_eq!(
            found("Ghoul Token arrives.", std::slice::from_ref(&ghoul)),
            vec![(0, 11, "t-1".to_string(), false)]
        );
        assert!(found("An Anti-Ghoul Token.", &[ghoul]).is_empty());
        assert_eq!(
            found(
                "Rush, then a Rush Token.",
                &[def("t-2", "Rush"), def("t-3", "Rush Token")]
            ),
            vec![
                (0, 4, "t-2".to_string(), false),
                (13, 23, "t-3".to_string(), false)
            ]
        );
        assert_eq!(
            found("Bang!!Pow", &[def("a", "Bang!"), def("b", "!Pow")]),
            vec![(0, 5, "a".to_string(), false), (5, 9, "b".to_string(), false)]
        );
    }

    /// A name that is all parenthesis has nothing before it.
    #[test]
    fn a_name_that_is_all_parenthesis_has_no_bare_name() {
        assert_eq!(bare_name("(Edition)"), "");
    }
}
