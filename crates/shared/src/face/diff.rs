//! What a Radiant face prints that its base face does not (SPEC §10.10, R277).
//!
//! The catalog carries each Radiant face's whole text (`radiant.text`, §8's Radiant cell written out);
//! the face prints it and marks the words and numbers the base text lacks (#44 True Strike marks its 9
//! alone). The mark is a word-level LCS diff of the two public catalog texts, so it is the same wherever
//! the face is drawn. Case is ignored (#17's "Bounce" becoming "bounce" marks nothing). The separators
//! `, ; : . ( )` are tokens of their own, and a marked range never starts or ends on one, so the underline
//! reads as one phrase.
//!
//! A Chinese face (ME-CN, R1302, MD-B13) is compared character by character: Chinese puts no space
//! between words, and its punctuation `，；：。（）、` is a separator. The same diff (`word_diff`) marks
//! what a patch changed (R388), striking through the words the older text loses.
//!
//! Offsets count characters, not bytes.

/// A stretch of a text, by character offsets: `start` inclusive, `end` exclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

/// Two texts' word diff: the stretches the newer text adds, by its own offsets, and the stretches the
/// older one loses, by the older one's offsets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WordDiff {
    pub added: Vec<TextRange>,
    pub removed: Vec<TextRange>,
}

struct Token {
    text: String,
    start: usize,
    end: usize,
    word: bool,
}

const SEPARATORS: &str = ",;:.()，；：。（）、";
/// R1302: the CJK ranges whose every character is a token of its own: CJK punctuation, Extension A, the
/// Unified Ideographs, and the full-width forms.
const CJK: [(char, char); 4] = [
    ('\u{3000}', '\u{303F}'),
    ('\u{3400}', '\u{4DBF}'),
    ('\u{4E00}', '\u{9FFF}'),
    ('\u{FF00}', '\u{FFEF}'),
];
/// R102: a fused definition's text is its ingredients' texts, one per line.
const LINE_BREAK: char = '\n';

fn is_separator(c: char) -> bool {
    SEPARATORS.contains(c)
}

fn is_cjk(c: char) -> bool {
    CJK.iter().any(|(from, to)| (*from..=*to).contains(&c))
}

/// JavaScript's `\s`.
pub(super) fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}' | '\u{FEFF}'
    )
}

fn tokens_of(text: &[char], offset: usize) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let c = text[at];
        let start = at;
        if is_separator(c) || is_cjk(c) {
            at += 1;
        } else if is_space(c) {
            at += 1;
            continue;
        } else {
            while at < text.len() && !is_space(text[at]) && !is_separator(text[at]) && !is_cjk(text[at]) {
                at += 1;
            }
        }
        let word = !is_separator(c) || at - start > 1;
        tokens.push(Token {
            text: text[start..at].iter().collect(),
            start: offset + start,
            end: offset + at,
            word,
        });
    }
    tokens
}

/// Which tokens of each text are in one longest common subsequence of the two, case aside: `after` for
/// `after`'s tokens (R277's marks are the rest of them) and `before` for `before`'s (R388's struck-through
/// words are the rest of those). One alignment, so the two sides always agree.
fn common_tokens(before: &[Token], after: &[Token]) -> (Vec<bool>, Vec<bool>) {
    let a: Vec<String> = before.iter().map(|token| token.text.to_lowercase()).collect();
    let b: Vec<String> = after.iter().map(|token| token.text.to_lowercase()).collect();
    let cols = b.len() + 1;
    // lcs[i * cols + j]: the LCS length of a[i..] and b[j..], filled from the end.
    let mut lcs = vec![0_u32; (a.len() + 1) * cols];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i * cols + j] = if a[i] == b[j] {
                lcs[(i + 1) * cols + j + 1] + 1
            } else {
                lcs[(i + 1) * cols + j].max(lcs[i * cols + j + 1])
            };
        }
    }
    let mut kept_before = vec![false; a.len()];
    let mut kept_after = vec![false; b.len()];
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            kept_before[i] = true;
            kept_after[j] = true;
            i += 1;
            j += 1;
        } else if lcs[(i + 1) * cols + j] >= lcs[i * cols + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    (kept_before, kept_after)
}

/// The stretches of `tokens` the alignment did not keep: from the first unkept word to the last of a run,
/// over the spaces and separators between them, never starting or ending on a separator.
fn stretches(tokens: &[Token], kept: &[bool]) -> Vec<TextRange> {
    let mut ranges = Vec::new();
    let mut open: Option<TextRange> = None;
    for (at, token) in tokens.iter().enumerate() {
        if !token.word {
            continue;
        }
        // A separator, changed or kept, waits to see what follows it; a kept word closes the stretch.
        if kept[at] {
            if let Some(range) = open.take() {
                ranges.push(range);
            }
        } else if let Some(range) = open.as_mut() {
            range.end = token.end;
        } else {
            open = Some(TextRange {
                start: token.start,
                end: token.end,
            });
        }
    }
    ranges.extend(open);
    ranges
}

/// One line of each text against the other: what `after` adds and what `before` loses.
fn line_diff(before: &[char], after: &[char], before_offset: usize, after_offset: usize) -> WordDiff {
    let before_tokens = tokens_of(before, before_offset);
    let after_tokens = tokens_of(after, after_offset);
    let (kept_before, kept_after) = common_tokens(&before_tokens, &after_tokens);
    WordDiff {
        added: stretches(&after_tokens, &kept_after),
        removed: stretches(&before_tokens, &kept_before),
    }
}

fn lines(text: &[char]) -> Vec<&[char]> {
    text.split(|c| *c == LINE_BREAK).collect()
}

/// A word-level diff of two texts, by R277's rules, each side's stretches in order and never overlapping.
/// When both texts have the same number of lines, and more than one, each line is diffed against the line
/// of the same place (a fused text's ingredients, R102).
pub fn word_diff(before: &[char], after: &[char]) -> WordDiff {
    let before_lines = lines(before);
    let after_lines = lines(after);
    if after_lines.len() > 1 && after_lines.len() == before_lines.len() {
        let mut diff = WordDiff::default();
        let (mut before_offset, mut after_offset) = (0, 0);
        for (before_line, after_line) in before_lines.iter().zip(&after_lines) {
            let line = line_diff(before_line, after_line, before_offset, after_offset);
            diff.added.extend(line.added);
            diff.removed.extend(line.removed);
            before_offset += before_line.len() + 1;
            after_offset += after_line.len() + 1;
        }
        return diff;
    }
    line_diff(before, after, 0, 0)
}

/// R277: the stretches of `radiant` that `base` does not have.
pub fn radiant_marks(base: &[char], radiant: &[char]) -> Vec<TextRange> {
    word_diff(base, radiant).added
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(text: &str) -> Vec<char> {
        text.chars().collect()
    }

    fn marked(base: &str, radiant: &str) -> Vec<String> {
        let radiant = chars(radiant);
        radiant_marks(&chars(base), &radiant)
            .iter()
            .map(|range| radiant[range.start..range.end].iter().collect())
            .collect()
    }

    #[test]
    fn a_radiant_text_marks_what_the_base_lacks() {
        assert_eq!(marked("Charge", "Charge, First Strike"), vec!["First Strike"]);
        assert_eq!(marked("Deal 3 damage.", "Deal 9 damage."), vec!["9"]);
    }

    #[test]
    fn case_marks_nothing_and_a_mark_never_ends_on_a_separator() {
        assert!(marked("Bounce a unit.", "bounce a unit.").is_empty());
        assert_eq!(
            marked("Draw a card.", "Draw a card. Deal 2 damage."),
            vec!["Deal 2 damage"]
        );
    }

    #[test]
    fn a_chinese_text_is_compared_character_by_character() {
        assert_eq!(marked("造成3点伤害。", "造成9点伤害。"), vec!["9"]);
    }

    #[test]
    fn a_fused_text_is_compared_line_by_line() {
        assert_eq!(marked("Rush\nTaunt", "Rush\nTaunt, Charge"), vec!["Charge"]);
    }

    #[test]
    fn the_older_text_loses_what_the_newer_does_not_keep() {
        let diff = word_diff(&chars("Deal 3 damage."), &chars("Deal 4 damage."));
        assert_eq!(diff.removed, vec![TextRange { start: 5, end: 6 }]);
        assert_eq!(diff.added, vec![TextRange { start: 5, end: 6 }]);
    }
}
