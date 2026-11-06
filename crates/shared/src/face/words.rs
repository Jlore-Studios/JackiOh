//! The reader's words: `t()`, as the web's `i18n.ts` did it for a face. The English stays the key. A
//! language is a dictionary compiled in (`data/zh-CN.json`, English sentence to words); English needs none.
//! The engine's sentences arrive complete from Rust (a quest's goal, an aura), so a key holding `{name}`
//! tokens also matches a finished sentence, each value looked up in turn.

use super::data::{ZH_CN, ZH_CN_PATTERNS, find};
use super::types::Locale;

/// A reader's language, and the lookups that read it.
#[derive(Clone, Copy, Debug)]
pub struct Words {
    pub locale: Locale,
}

/// A `{name}` token's value for [`Words::say_with`].
pub type Param<'a> = (&'a str, String);

impl Words {
    pub fn new(locale: Locale) -> Words {
        Words { locale }
    }

    /// Whether the reader reads Simplified Chinese: a card table is drawn for them (R711000).
    pub fn is_zh_cn(self) -> bool {
        self.locale == Locale::ZhCn
    }

    /// R711000: whether a card is drawn from the Chinese table: the view flagged it, or the reader reads Chinese.
    pub fn drawn_in_chinese(self, flagged: bool) -> bool {
        flagged || self.is_zh_cn()
    }

    fn entry(self, english: &str) -> Option<&'static str> {
        match self.locale {
            Locale::En => None,
            Locale::ZhCn => find(ZH_CN, english).copied(),
        }
    }

    /// The reader's words for this exact English sentence; the English when the language has none.
    pub fn say(self, english: &str) -> String {
        if let Some(found) = self.entry(english) {
            return found.to_string();
        }
        if self.locale == Locale::En {
            return english.to_string();
        }
        self.matched(english).unwrap_or_else(|| english.to_string())
    }

    /// `say` with `{name}`s filled from `params`; the English, filled the same way, when the language has none.
    pub fn say_with(self, english: &str, params: &[Param<'_>]) -> String {
        match self.entry(english) {
            Some(found) => fill(found, params),
            None => fill(english, params),
        }
    }

    /// When a `{name}` key matches a complete English sentence, each value is looked up in turn, as the
    /// engine nests its sentences; a value is shorter than the sentence it came from, so the lookup ends.
    fn matched(self, english: &str) -> Option<String> {
        let text: Vec<char> = english.chars().collect();
        for key in ZH_CN_PATTERNS {
            let pattern = Pattern::of(key);
            let Some(values) = pattern.matches(&text) else {
                continue;
            };
            let entry = find(ZH_CN, key)?;
            let values: Vec<Param<'_>> = values
                .into_iter()
                .map(|(name, value)| (name, self.say(&value)))
                .collect();
            return Some(fill(entry, &values));
        }
        None
    }
}

/// `text` with each `{name}` token the params name replaced by its value, the others left as they are.
pub fn fill(text: &str, params: &[Param<'_>]) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut at = 0;
    while at < chars.len() {
        if chars[at] == '{' {
            let mut to = at + 1;
            while to < chars.len() && (chars[to].is_ascii_alphanumeric() || chars[to] == '_') {
                to += 1;
            }
            if to > at + 1 && chars.get(to) == Some(&'}') {
                let name: String = chars[at + 1..to].iter().collect();
                match params.iter().find(|(known, _)| *known == name) {
                    Some((_, value)) => out.push_str(value),
                    None => out.extend(&chars[at..=to]),
                }
                at = to + 1;
                continue;
            }
        }
        out.push(chars[at]);
        at += 1;
    }
    out
}

/// A dictionary key as a pattern: the literal stretches of the key around its tokens. A token in
/// parentheses stands for one or more characters that are not parentheses, any other for one or more of
/// anything, each as few as lets the rest match; the whole sentence must match.
struct Pattern<'a> {
    literals: Vec<Vec<char>>,
    tokens: Vec<(&'a str, bool)>,
}

impl<'a> Pattern<'a> {
    fn of(key: &'a str) -> Pattern<'a> {
        let bytes = key.as_bytes();
        let mut literals = Vec::new();
        let mut tokens = Vec::new();
        let mut last = 0;
        let mut at = 0;
        while at < bytes.len() {
            if bytes[at] == b'{' {
                let mut to = at + 1;
                while to < bytes.len() && (bytes[to].is_ascii_alphanumeric() || bytes[to] == b'_') {
                    to += 1;
                }
                if to > at + 1 && bytes.get(to) == Some(&b'}') {
                    literals.push(key[last..at].chars().collect());
                    let enclosed = key[..at].ends_with('(') && key[to + 1..].starts_with(')');
                    tokens.push((&key[at + 1..to], enclosed));
                    last = to + 1;
                    at = to + 1;
                    continue;
                }
            }
            at += 1;
        }
        literals.push(key[last..].chars().collect());
        Pattern { literals, tokens }
    }

    /// What each token stands for in `text`, or `None` when the sentence is not this key's.
    fn matches(&self, text: &[char]) -> Option<Vec<(&'a str, String)>> {
        let first = self.literals.first()?;
        if !text.starts_with(first) {
            return None;
        }
        let mut found = Vec::with_capacity(self.tokens.len());
        self.go(text, 0, first.len(), &mut found).then_some(found)
    }

    /// Tokens `..i` are matched and the text is read up to `at`.
    fn go(&self, text: &[char], i: usize, at: usize, found: &mut Vec<(&'a str, String)>) -> bool {
        let Some(&(name, enclosed)) = self.tokens.get(i) else {
            return at == text.len();
        };
        let literal = &self.literals[i + 1];
        for end in at + 1..=text.len() {
            if enclosed && matches!(text[end - 1], '(' | ')') {
                return false;
            }
            if !text[end..].starts_with(literal) {
                continue;
            }
            found.push((name, text[at..end].iter().collect()));
            if self.go(text, i + 1, end + literal.len(), found) {
                return true;
            }
            found.pop();
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_tokens_and_keeps_the_unknown() {
        assert_eq!(
            fill("{a} of {b} and {c}", &[("a", "1".into()), ("b", "2".into())]),
            "1 of 2 and {c}"
        );
        assert_eq!(fill("no tokens {", &[]), "no tokens {");
    }

    #[test]
    fn a_pattern_matches_lazily_and_anchored() {
        let pattern = Pattern::of("Stage {n} of {of}, {name}");
        let text: Vec<char> = "Stage 2 of 3, Comradery, Ltd".chars().collect();
        let values = pattern.matches(&text).expect("matches");
        assert_eq!(
            values,
            vec![
                ("n", "2".to_string()),
                ("of", "3".to_string()),
                ("name", "Comradery, Ltd".to_string())
            ]
        );
        assert!(pattern.matches(&"Stage".chars().collect::<Vec<_>>()).is_none());
    }

    #[test]
    fn a_token_in_parentheses_takes_no_parenthesis() {
        let pattern = Pattern::of("costs ({cost}) now");
        assert!(
            pattern
                .matches(&"costs (3) now".chars().collect::<Vec<_>>())
                .is_some()
        );
        assert!(
            pattern
                .matches(&"costs ((3)) now".chars().collect::<Vec<_>>())
                .is_none()
        );
    }

    #[test]
    fn english_reads_as_itself_and_chinese_looks_the_sentence_up() {
        assert_eq!(Words::new(Locale::En).say("Cast on draw"), "Cast on draw");
        assert_eq!(Words::new(Locale::ZhCn).say("Cast on draw"), "抽到时施放");
        assert_eq!(
            Words::new(Locale::ZhCn).say("A sentence nobody wrote"),
            "A sentence nobody wrote"
        );
        assert_eq!(
            Words::new(Locale::ZhCn).say_with("Brittle {n}: crumbles at 0", &[("n", "3".into())]),
            "易碎3：降至0时碎裂"
        );
        assert_eq!(
            Words::new(Locale::En).say_with("Brittle {n}: crumbles at 0", &[("n", "3".into())]),
            "Brittle 3: crumbles at 0"
        );
    }
}
