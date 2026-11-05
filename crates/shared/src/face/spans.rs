//! A face's text as spans (docs/polish/6-cards.md, B10; SPEC §10.10): the one place that decides what rides
//! on a card's plain words, so every surface prints them the same way:
//! - glossary terms in bold (`Keyword`, `glossary.rs`);
//! - on a Radiant face, what its base face lacks (`marks`, R277);
//! - the names the card's `refs` link (`refs.rs`, R279) as references; a term inside a name is not bolded,
//!   since the name is the card, not the keyword ("Rush Token");
//! - in play, what its formula comes to now (`values`, R280, R372), each after its own line's label (a
//!   fused card prints its ingredients' texts one after the other, R102);
//! - in play, the numbers the view moved off their printed values (`tuned`, R386).
//!
//! A mark that crosses a name's edge is cut there, so marks and references nest. The spans come out as the
//! tree a client draws, outermost first.

use jackioh_engine::wire::{CardDef, PreviewValue};

use super::chinese::{chinese_def, radiant_word};
use super::diff::TextRange;
use super::glossary::{Token, entry, tokenize};
use super::refs::{RADIANT_WORD, RefMatch, find_refs};
use super::text::{Text, find_from};
use super::tuning::tuned_title;
use super::types::{RichText, Span};
use super::words::Words;

/// What a text is drawn with beyond its words.
#[derive(Clone, Copy, Default)]
pub struct Dressing<'a> {
    /// R279: the definitions `CardDef.refs` lists, that the text may name.
    pub refs: &'a [CardDef],
    /// R1301: the text is a Chinese face's, which names its references by their Chinese names.
    pub chinese: bool,
    /// R280: what the card's formula comes to now.
    pub values: &'a [PreviewValue],
}

struct Insert {
    at: usize,
    text: String,
    label: String,
}

struct Term {
    start: usize,
    end: usize,
    term: &'static str,
}

/// A stretch the text is wrapped in: a Radiant mark or a reference.
struct Wrap {
    start: usize,
    end: usize,
    kind: WrapKind,
}

enum WrapKind {
    Mark,
    Ref { id: String, radiant: bool },
}

/// R280, R372: what a value prints as: the word the view names it by (#93's grade letter, "C"), else its
/// number. A client never turns one into the other.
pub fn printed_value(entry: &PreviewValue) -> String {
    match &entry.display {
        Some(display) if !display.is_empty() => display.clone(),
        _ => entry.value.to_string(),
    }
}

/// R280: each value after its label's nth occurrence for the nth entry with that label. A label that occurs
/// fewer times keeps its values together: past its last occurrence they join it there, in one helper block
/// (a Radiant Lizard's Breath shows both largest piles together). A label the text never names, or an empty
/// label, falls to the end.
fn inserts_of(text: &[char], values: &[PreviewValue]) -> Vec<Insert> {
    let mut seen: Vec<(&str, usize)> = Vec::new();
    values
        .iter()
        .map(|entry| {
            let nth = match seen.iter_mut().find(|(label, _)| *label == entry.label) {
                Some((_, count)) => {
                    let nth = *count;
                    *count += 1;
                    nth
                }
                None => {
                    seen.push((entry.label.as_str(), 1));
                    0
                }
            };
            let label: Vec<char> = entry.label.chars().collect();
            let mut found: Option<usize> = None;
            let mut last: Option<usize> = None;
            if !label.is_empty() {
                let mut from = 0;
                for _ in 0..=nth {
                    found = find_from(text, &label, from);
                    match found {
                        Some(at) => {
                            last = Some(at);
                            from = at + 1;
                        }
                        None => break,
                    }
                }
            }
            let at = found.or(last);
            Insert {
                at: at.map_or(text.len(), |at| at + label.len()),
                text: printed_value(entry),
                label: entry.label.clone(),
            }
        })
        .collect()
}

fn contains(outer: (usize, usize), inner: (usize, usize)) -> bool {
    outer.0 <= inner.0 && inner.1 <= outer.1
}

/// The marks, each cut at the edge of a reference that it crosses — one it neither holds whole nor lies
/// inside — so that marks and references always nest.
fn nested_marks(marks: &[TextRange], refs: &[(usize, usize)]) -> Vec<TextRange> {
    let mut out = Vec::new();
    for mark in marks {
        let this = (mark.start, mark.end);
        let mut cuts: Vec<usize> = refs
            .iter()
            .filter(|reference| {
                reference.0 < mark.end
                    && mark.start < reference.1
                    && !contains(this, **reference)
                    && !contains(**reference, this)
            })
            .flat_map(|reference| [reference.0, reference.1])
            .filter(|point| *point > mark.start && *point < mark.end)
            .collect();
        cuts.sort_unstable();
        cuts.dedup();
        let mut from = mark.start;
        for point in cuts {
            out.push(TextRange {
                start: from,
                end: point,
            });
            from = point;
        }
        out.push(TextRange {
            start: from,
            end: mark.end,
        });
    }
    out.retain(|range| range.end > range.start);
    out
}

struct Builder<'a> {
    text: &'a [char],
    wraps: Vec<Wrap>,
    terms: Vec<Term>,
    inserts: Vec<Insert>,
    emitted: Vec<bool>,
    tuned: &'a [super::tuning::TunedRange],
    words: Words,
}

impl Builder<'_> {
    fn slice(&self, from: usize, to: usize) -> String {
        self.text[from..to].iter().collect()
    }

    /// The values owed at `point`, once each.
    fn values_at(&mut self, point: usize) -> Vec<Span> {
        let mut out = Vec::new();
        for (at, insert) in self.inserts.iter().enumerate() {
            if insert.at != point || self.emitted[at] {
                continue;
            }
            self.emitted[at] = true;
            out.push(Span::Text {
                text: " ".to_string(),
            });
            out.push(Span::Value {
                text: format!("{{{}}}", insert.text),
                value: insert.text.clone(),
                label: insert.label.clone(),
                title: self.words.say_with(
                    "{label}: currently {value}",
                    &[("label", insert.label.clone()), ("value", insert.text.clone())],
                ),
            });
        }
        out
    }

    /// `from` to `to` as text, the stretches of it a tuned number covers boxed (R386).
    fn numbers(&self, from: usize, to: usize) -> Vec<Span> {
        let mut points = vec![from, to];
        for range in self.tuned {
            for point in [range.start, range.end] {
                if point > from && point < to {
                    points.push(point);
                }
            }
        }
        points.sort_unstable();
        points.dedup();
        let mut out = Vec::new();
        for pair in points.windows(2) {
            let (start, end) = (pair[0], pair[1]);
            let text = self.slice(start, end);
            match self
                .tuned
                .iter()
                .find(|candidate| candidate.start <= start && start < candidate.end)
            {
                None => out.push(Span::Text { text }),
                Some(range) => out.push(Span::Number {
                    text,
                    key: range.key.clone(),
                    printed: range.printed,
                    way: range.way,
                    title: tuned_title(self.words, range.way, range.printed),
                }),
            }
        }
        out
    }

    /// Plain words from `start` to `end`: terms in bold unless inside a reference, values where owed.
    fn plain(&mut self, start: usize, end: usize, in_ref: bool) -> Vec<Span> {
        let mut points = vec![start, end];
        if !in_ref {
            for term in &self.terms {
                for point in [term.start, term.end] {
                    if point > start && point < end {
                        points.push(point);
                    }
                }
            }
        }
        for insert in &self.inserts {
            if insert.at > start && insert.at < end {
                points.push(insert.at);
            }
        }
        points.sort_unstable();
        points.dedup();
        let mut out = Vec::new();
        for pair in points.windows(2) {
            let (from, to) = (pair[0], pair[1]);
            let term = if in_ref {
                None
            } else {
                self.terms
                    .iter()
                    .find(|range| range.start <= from && from < range.end)
                    .map(|range| range.term)
            };
            let numbers = self.numbers(from, to);
            match term.and_then(entry) {
                None => out.extend(numbers),
                Some(entry) => out.push(Span::Keyword {
                    entry,
                    children: numbers,
                }),
            }
            if to < end {
                out.extend(self.values_at(to));
            }
        }
        out
    }

    /// `start` to `end`, with the wraps inside it drawn around their own contents.
    fn draw(&mut self, start: usize, end: usize, within: &[usize], in_ref: bool) -> Vec<Span> {
        let inside: Vec<usize> = within
            .iter()
            .copied()
            .filter(|at| self.wraps[*at].start >= start && self.wraps[*at].end <= end)
            .collect();
        // The outermost wraps here: those no other wrap holds. A mark and a reference over the very same
        // words put the reference outside.
        let holds = |outer: usize, inner: usize| {
            let (o, i) = (&self.wraps[outer], &self.wraps[inner]);
            outer != inner
                && contains((o.start, o.end), (i.start, i.end))
                && (!contains((i.start, i.end), (o.start, o.end)) || matches!(o.kind, WrapKind::Ref { .. }))
        };
        let mut top: Vec<usize> = inside
            .iter()
            .copied()
            .filter(|wrap| !inside.iter().any(|other| holds(*other, *wrap)))
            .collect();
        top.sort_by_key(|wrap| self.wraps[*wrap].start);
        let mut out = Vec::new();
        let mut at = start;
        for wrap in top {
            let (wrap_start, wrap_end) = (self.wraps[wrap].start, self.wraps[wrap].end);
            if wrap_start > at {
                out.extend(self.plain(at, wrap_start, in_ref));
                out.extend(self.values_at(wrap_start));
            }
            let rest: Vec<usize> = inside.iter().copied().filter(|other| *other != wrap).collect();
            let reference = match &self.wraps[wrap].kind {
                WrapKind::Mark => None,
                WrapKind::Ref { id, radiant } => Some((id.clone(), *radiant)),
            };
            match reference {
                None => {
                    let children = self.draw(wrap_start, wrap_end, &rest, in_ref);
                    out.push(Span::Mark { children });
                }
                Some((def_id, radiant)) => {
                    let children = self.draw(wrap_start, wrap_end, &rest, true);
                    out.push(Span::Card {
                        def_id,
                        radiant,
                        children,
                    });
                }
            }
            out.extend(self.values_at(wrap_end));
            at = wrap_end;
        }
        if at < end {
            out.extend(self.plain(at, end, in_ref));
        }
        out
    }
}

/// Neighbouring runs of plain words are one run.
fn merged(spans: Vec<Span>) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    for span in spans {
        let span = match span {
            Span::Keyword { entry, children } => Span::Keyword {
                entry,
                children: merged(children),
            },
            Span::Card {
                def_id,
                radiant,
                children,
            } => Span::Card {
                def_id,
                radiant,
                children: merged(children),
            },
            Span::Mark { children } => Span::Mark {
                children: merged(children),
            },
            other => other,
        };
        if let (Some(Span::Text { text: before }), Span::Text { text }) = (out.last_mut(), &span) {
            before.push_str(text);
            continue;
        }
        out.push(span);
    }
    out
}

/// `text` as spans, dressed as `dressing` says. `terms` turns the glossary's words into `Keyword` spans.
pub fn spans_of(text: &Text, dressing: &Dressing<'_>, terms: bool, words: Words) -> Vec<Span> {
    let chars = &text.full;
    // R279, R1301: a Chinese face names its references by their Chinese names.
    let ref_matches: Vec<RefMatch> = if dressing.refs.is_empty() {
        Vec::new()
    } else if dressing.chinese {
        let converted: Vec<CardDef> = dressing.refs.iter().map(chinese_def).collect();
        find_refs(chars, &converted, radiant_word())
    } else {
        find_refs(chars, dressing.refs, RADIANT_WORD)
    };
    let ref_spans: Vec<(usize, usize)> = ref_matches.iter().map(|found| (found.start, found.end)).collect();
    let mut wraps: Vec<Wrap> = ref_matches
        .into_iter()
        .map(|found| Wrap {
            start: found.start,
            end: found.end,
            kind: WrapKind::Ref {
                id: found.id,
                radiant: found.radiant,
            },
        })
        .collect();
    wraps.extend(
        nested_marks(&text.marks, &ref_spans)
            .into_iter()
            .map(|range| Wrap {
                start: range.start,
                end: range.end,
                kind: WrapKind::Mark,
            }),
    );

    let mut term_ranges = Vec::new();
    if terms {
        let mut at = 0;
        for token in tokenize(chars) {
            match token {
                Token::Text(plain) => at += plain.chars().count(),
                Token::Term { text, term } => {
                    let length = text.chars().count();
                    term_ranges.push(Term {
                        start: at,
                        end: at + length,
                        term,
                    });
                    at += length;
                }
            }
        }
    }

    let inserts = inserts_of(chars, dressing.values);
    let emitted = vec![false; inserts.len()];
    let within: Vec<usize> = (0..wraps.len()).collect();
    let mut builder = Builder {
        text: chars,
        wraps,
        terms: term_ranges,
        inserts,
        emitted,
        tuned: &text.tuned,
        words,
    };
    let mut spans = builder.draw(0, chars.len(), &within, false);
    spans.extend(builder.values_at(chars.len()));
    merged(spans)
}

/// `text` as rich text: its words and its spans.
pub fn rich(text: &Text, dressing: &Dressing<'_>, words: Words) -> RichText {
    let spans = spans_of(text, dressing, true, words);
    let unlinked = names_a_card(&spans).then(|| {
        let bare = Dressing {
            refs: &[],
            ..*dressing
        };
        spans_of(text, &bare, true, words)
    });
    RichText {
        full: text.string(),
        spans,
        unlinked,
    }
}

/// Whether any span of `spans` is a reference to a card.
fn names_a_card(spans: &[Span]) -> bool {
    spans.iter().any(|span| match span {
        Span::Card { .. } => true,
        Span::Keyword { children, .. } | Span::Mark { children } => names_a_card(children),
        _ => false,
    })
}

/// Plain words from the engine or a card, with only the glossary's terms in bold.
pub fn rich_words(words_text: &str, words: Words) -> RichText {
    let text = Text::plain(words_text);
    RichText {
        full: words_text.to_string(),
        spans: spans_of(&text, &Dressing::default(), true, words),
        unlinked: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::tuning::TunedRange;
    use crate::face::types::{Locale, Way};

    fn en() -> Words {
        Words::new(Locale::En)
    }

    fn text(span: &Span) -> String {
        match span {
            Span::Text { text } | Span::Number { text, .. } | Span::Value { text, .. } => text.clone(),
            Span::Keyword { children, .. } | Span::Card { children, .. } | Span::Mark { children } => {
                children.iter().map(self::text).collect()
            }
        }
    }

    #[test]
    fn terms_are_bold_and_the_rest_plain() {
        let spans = rich_words("Cry: Deal 3 damage.", en()).spans;
        assert_eq!(spans.len(), 2);
        assert!(matches!(&spans[0], Span::Keyword { entry, .. } if entry.id == "Cry"));
        assert_eq!(text(&spans[1]), " Deal 3 damage.");
    }

    #[test]
    fn a_mark_wraps_the_words_a_radiant_face_adds_and_a_number_the_view_moved() {
        let mut t = Text::plain("Deal 9 damage to Rush");
        t.marks = vec![TextRange { start: 5, end: 6 }];
        t.tuned = vec![TunedRange {
            start: 5,
            end: 6,
            key: "dmg".into(),
            printed: 3,
            way: Way::Better,
        }];
        let spans = spans_of(&t, &Dressing::default(), true, en());
        assert!(
            matches!(&spans[1], Span::Mark { children } if matches!(&children[0], Span::Number { printed: 3, way: Way::Better, .. }))
        );
        assert_eq!(
            spans.iter().map(text).collect::<String>(),
            "Deal 9 damage to Rush"
        );
    }

    #[test]
    fn a_value_prints_after_its_label_and_a_missing_label_falls_to_the_end() {
        let values = [
            PreviewValue {
                label: "damage".into(),
                value: 4,
                display: None,
                ids: None,
            },
            PreviewValue {
                label: "nowhere".into(),
                value: 2,
                display: Some("C".into()),
                ids: None,
            },
        ];
        let t = Text::plain("Deal damage twice");
        let spans = spans_of(
            &t,
            &Dressing {
                values: &values,
                ..Dressing::default()
            },
            true,
            en(),
        );
        let flat: String = spans.iter().map(text).collect();
        assert_eq!(flat, "Deal damage {4} twice {C}");
    }
}
