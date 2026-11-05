//! B3.4, R386; R1320: face marks present Degrade and Upgrade as Nerf and Buff while verdict data keeps the
//! engine's words. Compare `tuning` and `params` with the public catalog only (SPEC §10.8; CLAUDE.md
//! rule 7). Stats and keywords use their player-facing direction, declared numbers `better` (B3.4
//! rule 5); constants set outright are neither better nor worse. `filled_text` fills a text as the
//! engine's `fill_params` does.

use indexmap::IndexMap;
use jackioh_engine::wire::{CardDef, Keyword, KeywordKind, Param, ParamBetter, Tuning};

use super::chinese::{list_separator, reader_keyword};
use super::types::{FaceTuning, TuneChange, TuneVerdict, Way};
use super::words::Words;

/// §6.3: a numbered keyword or X is better higher, except these, which are better lower.
const LESS_IS_BETTER: [&str; 1] = ["Tribute"];

/// The minus sign the catalog prints ("−3/−3"), not a hyphen.
pub const MINUS: &str = "\u{2212}";

/// A number in a face's text that the view says moved off its printed value, by character offsets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TunedRange {
    pub start: usize,
    pub end: usize,
    pub key: String,
    pub printed: i32,
    pub way: Way,
}

fn way_of(delta: i32, less_is_better: bool) -> Way {
    if (delta > 0) != less_is_better {
        Way::Better
    } else {
        Way::Worse
    }
}

fn params_of(def: Option<&CardDef>) -> &[Param] {
    def.and_then(|def| def.params.as_deref()).unwrap_or(&[])
}

/// The glyph each verdict is drawn with, so a mark never rests on color alone.
fn verdict_glyph(verdict: TuneVerdict) -> &'static str {
    match verdict {
        TuneVerdict::Upgraded => "▲",
        TuneVerdict::Degraded => "▼",
        TuneVerdict::Tuned => "◆",
    }
}

/// The word each verdict is printed with (the ribbon, the mark's title): Buffed and Nerfed (R1320).
pub fn verdict_word(verdict: TuneVerdict) -> &'static str {
    match verdict {
        TuneVerdict::Upgraded => "Buffed",
        TuneVerdict::Degraded => "Nerfed",
        TuneVerdict::Tuned => "Tuned",
    }
}

fn verdict_of(changes: &[TuneChange]) -> TuneVerdict {
    let ways: Vec<Option<Way>> = changes
        .iter()
        .map(|change| match change {
            TuneChange::Set { .. } => None,
            TuneChange::Stat { way, .. }
            | TuneChange::Keyword { way, .. }
            | TuneChange::Count { way, .. }
            | TuneChange::Number { way, .. } => Some(*way),
        })
        .collect();
    if ways.iter().all(|way| *way == Some(Way::Better)) {
        TuneVerdict::Upgraded
    } else if ways.iter().all(|way| *way == Some(Way::Worse)) {
        TuneVerdict::Degraded
    } else {
        TuneVerdict::Tuned
    }
}

/// A declared number's key in words: "drawLimit" is "Draw limit", in the reader's language.
fn key_words(words: Words, key: &str) -> String {
    let mut spaced = String::new();
    let chars: Vec<char> = key.chars().collect();
    for (at, c) in chars.iter().enumerate() {
        if at > 0
            && c.is_ascii_uppercase()
            && (chars[at - 1].is_ascii_lowercase() || chars[at - 1].is_ascii_digit())
        {
            spaced.push(' ');
        }
        spaced.extend(c.to_lowercase());
    }
    let mut capital = String::new();
    let mut rest = spaced.chars();
    if let Some(first) = rest.next() {
        capital.extend(first.to_uppercase());
    }
    capital.extend(rest);
    words.say(&capital)
}

fn signed(delta: i32) -> String {
    if delta > 0 {
        format!("+{delta}")
    } else {
        format!("{MINUS}{}", delta.abs())
    }
}

/// One change in a player's words: "+2 Attack", "Gained Rush", "Damage 2 → 3".
pub fn change_words(words: Words, change: &TuneChange) -> String {
    match change {
        TuneChange::Stat { stat, delta, .. } => {
            let english = if stat == "attack" {
                "{delta} Attack"
            } else {
                "{delta} Health"
            };
            words.say_with(english, &[("delta", signed(*delta))])
        }
        TuneChange::Keyword { keyword, added, .. } => {
            let english = if *added {
                "Gained {keyword}"
            } else {
                "Lost {keyword}"
            };
            words.say_with(english, &[("keyword", reader_keyword(words, keyword))])
        }
        TuneChange::Count { key, delta, .. } => format!("{} {}", reader_keyword(words, key), signed(*delta)),
        TuneChange::Number {
            key, printed, value, ..
        } => format!("{} {printed} → {value}", key_words(words, key)),
        TuneChange::Set { key, value } => words.say_with(
            "{label} set to {value}",
            &[("label", key_words(words, key)), ("value", value.to_string())],
        ),
    }
}

/// B3.4, R386: what changed on a card, from its view's `tuning` and `params` against its printed face.
/// `None` when nothing did: no tuning, and every declared number at its printed value.
pub fn face_tuning(
    words: Words,
    def: Option<&CardDef>,
    radiant: bool,
    tuning: Option<&Tuning>,
    values: Option<&IndexMap<String, i32>>,
) -> Option<FaceTuning> {
    let mut changes: Vec<TuneChange> = Vec::new();
    let (mut attack, mut health) = (None, None);

    for (stat, delta) in [
        ("attack", tuning.and_then(|t| t.attack)),
        ("health", tuning.and_then(|t| t.health)),
    ] {
        let delta = delta.unwrap_or(0);
        if delta == 0 {
            continue;
        }
        let way = way_of(delta, false);
        if stat == "attack" {
            attack = Some(way);
        } else {
            health = Some(way);
        }
        changes.push(TuneChange::Stat {
            stat: stat.to_string(),
            delta,
            way,
        });
    }

    let added: Vec<Keyword> = tuning.and_then(|t| t.add_keywords.clone()).unwrap_or_default();
    let removed: Vec<KeywordKind> = tuning.and_then(|t| t.remove_keywords.clone()).unwrap_or_default();
    for keyword in &added {
        changes.push(TuneChange::Keyword {
            keyword: jackioh_engine::wire::keyword_key(keyword),
            added: true,
            way: Way::Better,
        });
    }
    for kind in &removed {
        changes.push(TuneChange::Keyword {
            keyword: kind.as_str().to_string(),
            added: false,
            way: Way::Worse,
        });
    }

    for (key, delta) in tuning.and_then(|t| t.x.as_ref()).into_iter().flatten() {
        if *delta == 0 {
            continue;
        }
        changes.push(TuneChange::Count {
            key: key.clone(),
            delta: *delta,
            way: way_of(*delta, LESS_IS_BETTER.contains(&key.as_str())),
        });
    }

    let declared = params_of(def);
    for param in declared {
        let Some(value) = values.and_then(|values| values.get(&param.key)).copied() else {
            continue;
        };
        let printed = if radiant { param.radiant } else { param.base };
        if value == printed {
            continue;
        }
        changes.push(TuneChange::Number {
            key: param.key.clone(),
            printed,
            value,
            way: way_of(value - printed, param.better == ParamBetter::Down),
        });
    }

    for (key, value) in tuning.and_then(|t| t.set.as_ref()).into_iter().flatten() {
        if declared.iter().any(|param| &param.key == key) {
            continue;
        }
        changes.push(TuneChange::Set {
            key: key.clone(),
            value: *value,
        });
    }

    if changes.is_empty() {
        return None;
    }
    let verdict = verdict_of(&changes);
    let change_words: Vec<String> = changes
        .iter()
        .map(|change| self::change_words(words, change))
        .collect();
    let summary = words.say_with(
        "{verdict}: {changes}",
        &[
            ("verdict", words.say(verdict_word(verdict))),
            ("changes", change_words.join(list_separator(words))),
        ],
    );
    Some(FaceTuning {
        verdict,
        glyph: verdict_glyph(verdict).to_string(),
        word: words.say(verdict_word(verdict)),
        changes,
        change_words,
        summary,
        attack,
        health,
        added,
        removed,
    })
}

/// A moved number's tooltip: "Better than printed: 2" (the number bare, since "(2)" reads as a cost, R432).
pub fn tuned_title(words: Words, way: Way, printed: i32) -> String {
    let english = if way == Way::Better {
        "Better than printed: {printed}"
    } else {
        "Worse than printed: {printed}"
    };
    words.say_with(english, &[("printed", printed.to_string())])
}

/// One `{key}` or `{key|singular|plural}` in a text: its byte range, the key and the agreeing words.
struct Placeholder<'t> {
    start: usize,
    end: usize,
    key: &'t str,
    words: Option<(&'t str, &'t str)>,
}

/// Every placeholder of `text`, left to right and never overlapping (B3.4 rule 5, R482).
fn placeholders(text: &str) -> Vec<Placeholder<'_>> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(offset) = text[at..].find('{') {
        let start = at + offset;
        at = start + 1;
        if !bytes.get(start + 1).is_some_and(u8::is_ascii_alphabetic) {
            continue;
        }
        let mut to = start + 2;
        while bytes.get(to).is_some_and(u8::is_ascii_alphanumeric) {
            to += 1;
        }
        let key = &text[start + 1..to];
        match bytes.get(to) {
            Some(b'}') => {
                found.push(Placeholder {
                    start,
                    end: to + 1,
                    key,
                    words: None,
                });
                at = to + 1;
            }
            Some(b'|') => {
                let word = |from: usize| {
                    let mut end = from;
                    while bytes.get(end).is_some_and(|b| !matches!(b, b'|' | b'{' | b'}')) {
                        end += 1;
                    }
                    end
                };
                let one_end = word(to + 1);
                if bytes.get(one_end) != Some(&b'|') {
                    continue;
                }
                let many_end = word(one_end + 1);
                if bytes.get(many_end) != Some(&b'}') {
                    continue;
                }
                found.push(Placeholder {
                    start,
                    end: many_end + 1,
                    key,
                    words: Some((&text[to + 1..one_end], &text[one_end + 1..many_end])),
                });
                at = many_end + 1;
            }
            _ => {}
        }
    }
    found
}

/// A text with its `{key}`s filled in as `fillParams` fills them, and where each number the view moved off
/// its printed value stands in the result (the digits alone), by character offsets. `values` is
/// `CardView.params`.
pub fn filled_text_of(
    def: &CardDef,
    text: &str,
    radiant: bool,
    values: Option<&IndexMap<String, i32>>,
) -> (String, Vec<TunedRange>) {
    let params = match def.params.as_deref() {
        Some(params) if !params.is_empty() => params,
        _ => return (text.to_string(), Vec::new()),
    };
    let mut out = String::new();
    let mut length = 0;
    let mut tuned = Vec::new();
    let mut from = 0;
    for found in placeholders(text) {
        let before = &text[from..found.start];
        out.push_str(before);
        length += before.chars().count();
        from = found.end;
        let Some(param) = params.iter().find(|candidate| candidate.key == found.key) else {
            let whole = &text[found.start..found.end];
            out.push_str(whole);
            length += whole.chars().count();
            continue;
        };
        let printed = if radiant { param.radiant } else { param.base };
        let value = values
            .and_then(|values| values.get(found.key))
            .copied()
            .unwrap_or(printed);
        let digits = value.to_string();
        if value != printed {
            tuned.push(TunedRange {
                start: length,
                end: length + digits.len(),
                key: found.key.to_string(),
                printed,
                way: way_of(value - printed, param.better == ParamBetter::Down),
            });
        }
        out.push_str(&digits);
        length += digits.len();
        if let Some((one, many)) = found.words {
            let word = if value == 1 { one } else { many };
            out.push(' ');
            out.push_str(word);
            length += 1 + word.chars().count();
        }
    }
    out.push_str(&text[from..]);
    (out, tuned)
}

/// A face's text with its `{key}` numbers filled in (B3.4 rule 5): printed values, or `values` in play.
pub fn filled_text(
    def: &CardDef,
    radiant: bool,
    values: Option<&IndexMap<String, i32>>,
) -> (String, Vec<TunedRange>) {
    let text = if radiant {
        &def.radiant.text
    } else {
        &def.base.text
    };
    filled_text_of(def, text, radiant, values)
}

/// `fillParams` over a text of its own (a Chinese template, a copier's sentence): `def`'s declared numbers.
pub fn fill_params_text(
    def: &CardDef,
    text: &str,
    radiant: bool,
    values: Option<&IndexMap<String, i32>>,
) -> String {
    filled_text_of(def, text, radiant, values).0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::types::Locale;

    fn def(text: &str) -> CardDef {
        let mut def: CardDef = serde_json::from_str(
            r#"{"id":"t-1","index":"1","name":"T","set":"Core","type":"Spell","tags":[],"rarity":"Common","token":false,"cost":1,
                "params":[{"key":"damage","base":2,"radiant":4,"better":"up"},{"key":"draw","base":1,"radiant":2,"better":"up"}],
                "base":{"keywords":[],"text":""},"radiant":{"keywords":[],"text":""}}"#,
        )
        .expect("a definition");
        def.base.text = text.to_string();
        def
    }

    fn values(pairs: &[(&str, i32)]) -> IndexMap<String, i32> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), *value))
            .collect()
    }

    #[test]
    fn placeholders_fill_agree_and_mark_what_moved() {
        let def = def("Deal {damage} damage. Draw {draw|card|cards}.");
        let (text, tuned) = filled_text(&def, false, Some(&values(&[("damage", 3), ("draw", 2)])));
        assert_eq!(text, "Deal 3 damage. Draw 2 cards.");
        assert_eq!(
            tuned
                .iter()
                .map(|range| (range.start, range.end, range.key.as_str(), range.way))
                .collect::<Vec<_>>(),
            vec![(5, 6, "damage", Way::Better), (20, 21, "draw", Way::Better),]
        );
        let (printed, none) = filled_text(&def, false, None);
        assert_eq!(printed, "Deal 2 damage. Draw 1 card.");
        assert!(none.is_empty());
    }

    #[test]
    fn a_text_with_no_declared_numbers_is_left_as_written() {
        let mut plain = def("Deal {damage}.");
        plain.params = None;
        assert_eq!(filled_text(&plain, false, None).0, "Deal {damage}.");
    }

    #[test]
    fn a_tuning_is_judged_by_the_direction_that_is_better() {
        let words = Words::new(Locale::En);
        let tuning = Tuning {
            attack: Some(2),
            x: Some(values(&[("Tribute", 1)])),
            ..Tuning::default()
        };
        let got = face_tuning(words, None, false, Some(&tuning), None).expect("a tuning");
        assert_eq!(got.verdict, TuneVerdict::Tuned);
        assert_eq!(got.change_words, vec!["+2 Attack", "Tribute +1"]);
        assert_eq!(got.summary, "Tuned: +2 Attack; Tribute +1");
    }

    #[test]
    fn a_moved_number_reads_printed_to_now() {
        let words = Words::new(Locale::ZhCn);
        let def = def("");
        let got =
            face_tuning(words, Some(&def), false, None, Some(&values(&[("damage", 1)]))).expect("a tuning");
        assert_eq!(got.verdict, TuneVerdict::Degraded);
        assert_eq!(
            got.change_words,
            vec!["Damage 2 → 1".replace("Damage", &words.say("Damage"))]
        );
    }
}
