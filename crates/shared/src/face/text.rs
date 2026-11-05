//! A face's text as the model works it out: the words, the stretches of them a Radiant face adds to its
//! base face's (R277), and the numbers the view moved off their printed values (R386), by character
//! offsets. Every later step (a copier's text, a Quest's stage, a short form) moves these together with
//! the words; the spans a client reads are built from them last (`spans.rs`).

use super::diff::TextRange;
use super::tuning::TunedRange;

/// The words of a face, with the marks and tuned numbers that stand in them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Text {
    pub full: Vec<char>,
    pub marks: Vec<TextRange>,
    pub tuned: Vec<TunedRange>,
}

impl Text {
    pub fn plain(words: &str) -> Text {
        Text {
            full: words.chars().collect(),
            marks: Vec::new(),
            tuned: Vec::new(),
        }
    }

    pub fn string(&self) -> String {
        self.full.iter().collect()
    }

    pub fn len(&self) -> usize {
        self.full.len()
    }

    pub fn is_empty(&self) -> bool {
        self.full.is_empty()
    }
}

/// `text` shifted by `by` characters (negative moves it toward the start).
fn shifted(at: usize, by: isize) -> usize {
    at.saturating_add_signed(by)
}

pub fn shift(range: TextRange, by: isize) -> TextRange {
    TextRange {
        start: shifted(range.start, by),
        end: shifted(range.end, by),
    }
}

/// Where `needle` first stands in `hay` at or after `from`.
pub fn find_from(hay: &[char], needle: &[char], from: usize) -> Option<usize> {
    if needle.is_empty() {
        return (from <= hay.len()).then_some(from);
    }
    if hay.len() < needle.len() {
        return None;
    }
    (from..=hay.len() - needle.len()).find(|at| hay[*at..*at + needle.len()] == *needle)
}

/// B5 E14, R511: a copier's text in play (Classic #57 Echo): the copied text replaces its own copying
/// sentence, whatever else its face prints stays; a face without that sentence prints the copied text alone.
/// `sentence` is the copier's own base text with its printed numbers. Only the copied text's tuned numbers
/// carry over: the copier's own words are no longer there to be read.
pub fn copied_text(own: &Text, sentence: &[char], copied: &Text) -> Text {
    let at = if sentence.is_empty() {
        None
    } else {
        find_from(&own.full, sentence, 0)
    };
    let Some(at) = at else { return copied.clone() };
    let after = at + sentence.len();
    let by = |offset: isize| move |range: &TextRange| shift(*range, offset);
    let mut full: Vec<char> = own.full[..at].to_vec();
    full.extend(&copied.full);
    full.extend(&own.full[after..]);
    let moved = copied.full.len() as isize - sentence.len() as isize;
    let mut marks: Vec<TextRange> = own
        .marks
        .iter()
        .filter(|range| range.end <= at)
        .copied()
        .collect();
    marks.extend(copied.marks.iter().map(by(at as isize)));
    marks.extend(
        own.marks
            .iter()
            .filter(|range| range.start >= after)
            .map(by(moved)),
    );
    let tuned = copied
        .tuned
        .iter()
        .map(|range| TunedRange {
            start: shifted(range.start, at as isize),
            end: shifted(range.end, at as isize),
            ..range.clone()
        })
        .collect();
    Text { full, marks, tuned }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(text: &str) -> Vec<char> {
        text.chars().collect()
    }

    #[test]
    fn a_copiers_sentence_is_replaced_and_the_marks_around_it_move_with_their_words() {
        let own = Text {
            full: chars("Echo: copy. Draw a card."),
            marks: vec![TextRange { start: 13, end: 23 }],
            tuned: Vec::new(),
        };
        let copied = Text {
            full: chars("Deal 3 damage to everything"),
            marks: vec![TextRange { start: 5, end: 6 }],
            tuned: Vec::new(),
        };
        let got = copied_text(&own, &chars("copy."), &copied);
        assert_eq!(got.string(), "Echo: Deal 3 damage to everything Draw a card.");
        assert_eq!(
            got.marks,
            vec![TextRange { start: 11, end: 12 }, TextRange { start: 35, end: 45 }]
        );
    }

    #[test]
    fn a_face_without_the_sentence_prints_the_copied_text_alone() {
        let own = Text::plain("Something else");
        let copied = Text::plain("Deal 3 damage");
        assert_eq!(
            copied_text(&own, &chars("copy."), &copied).string(),
            "Deal 3 damage"
        );
    }
}
