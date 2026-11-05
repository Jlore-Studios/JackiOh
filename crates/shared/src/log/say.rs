//! A log line's words as spans: a sentence in the reader's language with its `{name}`s filled from parts,
//! the way the web's `t(english, params)` filled them (`i18n.ts`), and the capital the log's own words open a
//! line with. A part is plain words, or spans (a glossary term), so a line can carry the term it is about.

use crate::face::types::{RichText, Span};
use crate::face::words::Words;

/// What fills a `{name}` of a sentence.
pub enum Part {
    /// Plain words (a name, a seat, a number).
    Text(String),
    /// Spans that stand there as they are (a glossary term).
    Spans(Vec<Span>),
}

impl From<&str> for Part {
    fn from(text: &str) -> Part {
        Part::Text(text.to_string())
    }
}

impl From<String> for Part {
    fn from(text: String) -> Part {
        Part::Text(text)
    }
}

impl From<i32> for Part {
    fn from(number: i32) -> Part {
        Part::Text(number.to_string())
    }
}

impl From<usize> for Part {
    fn from(number: usize) -> Part {
        Part::Text(number.to_string())
    }
}

impl From<Vec<Span>> for Part {
    fn from(spans: Vec<Span>) -> Part {
        Part::Spans(spans)
    }
}

/// A sentence as the line's words: `words` gives the reader's sentence for `english` (the English when the
/// language has none), and each `{name}` of it stands for its part. A token with no part stays as written, as
/// `i18n.ts`'s `fill` left it.
pub fn sentence(words: Words, english: &str, parts: &[(&str, Part)]) -> Vec<Span> {
    let template: Vec<char> = words.say_with(english, &[]).chars().collect();
    let mut out: Vec<Span> = Vec::new();
    let mut plain = String::new();
    let mut at = 0;
    while at < template.len() {
        if template[at] == '{' {
            let mut to = at + 1;
            while to < template.len() && (template[to].is_ascii_alphanumeric() || template[to] == '_') {
                to += 1;
            }
            if to > at + 1 && template.get(to) == Some(&'}') {
                let name: String = template[at + 1..to].iter().collect();
                if let Some((_, part)) = parts.iter().find(|(known, _)| *known == name) {
                    match part {
                        Part::Text(text) => plain.push_str(text),
                        Part::Spans(spans) => {
                            flush(&mut out, &mut plain);
                            out.extend(spans.iter().cloned());
                        }
                    }
                    at = to + 1;
                    continue;
                }
            }
        }
        plain.push(template[at]);
        at += 1;
    }
    flush(&mut out, &mut plain);
    out
}

fn flush(out: &mut Vec<Span>, plain: &mut String) {
    if !plain.is_empty() {
        out.push(Span::Text {
            text: std::mem::take(plain),
        });
    }
}

/// The words of `spans`, as a reader reads them.
pub fn plain_words(spans: &[Span]) -> String {
    spans
        .iter()
        .map(|span| match span {
            Span::Text { text } | Span::Number { text, .. } | Span::Value { text, .. } => text.clone(),
            Span::Keyword { children, .. } | Span::Card { children, .. } | Span::Mark { children } => {
                plain_words(children)
            }
        })
        .collect()
}

/// The log's own words ("your hero", "a unit") open a line with a capital; a card's name is as printed.
const OWN_WORDS: [&str; 4] = ["a", "an", "your", "the"];

fn opens_with_own_words(text: &str) -> bool {
    OWN_WORDS.iter().any(|word| {
        text.strip_prefix(word)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
    })
}

/// `text` with its first letter capitalised when it opens with one of the log's own words.
pub fn capitalised(text: &str) -> String {
    if !opens_with_own_words(text) {
        return text.to_string();
    }
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

/// `spans` with the first letter of the line capitalised when it opens with one of the log's own words.
pub fn capitalised_spans(mut spans: Vec<Span>) -> Vec<Span> {
    if opens_with_own_words(&plain_words(&spans)) {
        capitalise_first(&mut spans);
    }
    spans
}

/// Upper-cases the first letter of the first span that has one.
fn capitalise_first(spans: &mut [Span]) -> bool {
    for span in spans {
        match span {
            Span::Text { text } | Span::Number { text, .. } | Span::Value { text, .. } => {
                if let Some(first) = text.chars().next() {
                    *text = format!("{}{}", first.to_ascii_uppercase(), &text[first.len_utf8()..]);
                    return true;
                }
            }
            Span::Keyword { children, .. } | Span::Card { children, .. } | Span::Mark { children } => {
                if capitalise_first(children) {
                    return true;
                }
            }
        }
    }
    false
}

/// A line's spans as the rich text a client draws.
pub fn rich(spans: Vec<Span>) -> RichText {
    RichText {
        full: plain_words(&spans),
        spans,
        unlinked: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::types::Locale;

    #[test]
    fn a_sentence_fills_its_tokens_in_the_readers_words() {
        let parts = [
            ("seat", Part::from("You")),
            ("card", Part::from("Pointmaster")),
            ("cost", Part::from(3)),
        ];
        let spans = sentence(Words::new(Locale::En), "{seat} played {card} for {cost}", &parts);
        assert_eq!(plain_words(&spans), "You played Pointmaster for 3");
        let zh = sentence(
            Words::new(Locale::ZhCn),
            "{seat} played {card} for {cost}",
            &parts,
        );
        assert_eq!(plain_words(&zh), "You打出了Pointmaster，花费3");
    }

    #[test]
    fn a_token_with_no_part_stays_as_written() {
        let spans = sentence(
            Words::new(Locale::En),
            "{seat} did {what}",
            &[("seat", Part::from("You"))],
        );
        assert_eq!(plain_words(&spans), "You did {what}");
    }

    #[test]
    fn a_term_stands_as_its_own_span() {
        let term = vec![Span::Text { text: "Taunt".into() }];
        let spans = sentence(
            Words::new(Locale::En),
            "{target} gained {keyword}",
            &[("target", Part::from("It")), ("keyword", Part::from(term))],
        );
        assert_eq!(spans.len(), 2);
        assert_eq!(plain_words(&spans), "It gained Taunt");
    }

    #[test]
    fn only_the_logs_own_words_take_a_capital() {
        assert_eq!(capitalised("your hero"), "Your hero");
        assert_eq!(capitalised("the opponent's lane 1"), "The opponent's lane 1");
        assert_eq!(capitalised("a unit"), "A unit");
        assert_eq!(capitalised("an effect"), "An effect");
        assert_eq!(capitalised("a"), "A");
        assert_eq!(capitalised("another card"), "another card");
        assert_eq!(capitalised("theatre"), "theatre");
        assert_eq!(capitalised("Pointmaster"), "Pointmaster");
        let spans = capitalised_spans(vec![
            Span::Text { text: "your".into() },
            Span::Text { text: " hero".into() },
        ]);
        assert_eq!(plain_words(&spans), "Your hero");
    }
}
