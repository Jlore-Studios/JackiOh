//! Short rules text (issues #814, #1017): what a face prints when its whole text would not fit its rules
//! box. The art never gives way: the face prints a short form, or its text cut at the box's last line with
//! "…", and the whole text stands on the left of its hover preview, in the touch sheet and in the detail
//! view. #1017 cut every catalog face under the limit but these, whose text is a list no box holds or a line
//! over after the cut. The forms (`data/short-forms.json`): a number, its first n lines with "…" in place of
//! the last full stop (#98 Heroic Power's thirteen powers; faces whose last line is a reminder or flavour);
//! "colon", up to its first colon (the Call to Chaos editions, whose list must stay whole in the catalog,
//! R436); "clamp", the whole text, cut by the box at its last line (#1563, #1704, #1828).
//!
//! Only a face printing its catalog text takes a short form (in play #98 prints its rolled power and a Call
//! to Chaos "???", `power.rs`). In play the Summit prints only its header and the next Ascent line (#814,
//! R280); a face in play past the preview's limit clamps rather than shrinking its art, which the client's
//! layout decides. Offsets count characters.

use jackioh_engine::wire::PreviewValue;

use super::data::{SHORT_FORMS, ShortForm, find};
use super::diff::is_space;

/// Meditative #8 Reach the Summit.
pub const SUMMIT_DEF_ID: &str = "meditative-008";

pub const ASCENT_LABEL: &str = "Ascent level";

/// What ends a short form: the text goes on in the extended description.
const MORE: char = '…';

/// A colon, English or Chinese (R1302), for the "colon" form.
fn is_colon(c: char) -> bool {
    matches!(c, ':' | '：')
}

fn is_last_stop(c: char) -> bool {
    matches!(c, '.' | ':' | '。' | '：')
}

/// `cut` ending in "…", which replaces its last full stop or colon.
fn trail_off(cut: &[char]) -> Vec<char> {
    let mut end = cut.len();
    while end > 0 && is_space(cut[end - 1]) {
        end -= 1;
    }
    if end > 0 && is_last_stop(cut[end - 1]) {
        end -= 1;
    }
    let mut out = cut[..end].to_vec();
    out.push(MORE);
    out
}

/// The short form of a card's face, or `None` when the face prints its whole text.
pub fn short_form_of(def_id: &str, radiant: bool) -> Option<ShortForm> {
    let (base, on_radiant) = find(SHORT_FORMS, def_id)?;
    if radiant { *on_radiant } else { *base }
}

/// The words a number or "colon" short form prints, ending in "…". `None` when there is nothing to cut and
/// for "clamp", which prints the whole text.
pub fn cut_text(full: &[char], form: ShortForm) -> Option<Vec<char>> {
    match form {
        ShortForm::Clamp => None,
        ShortForm::Colon => {
            let at = full.iter().position(|c| is_colon(*c))?;
            if at + 1 == full.len() {
                None
            } else {
                Some(trail_off(&full[..=at]))
            }
        }
        ShortForm::Lines(count) => {
            let lines: Vec<&[char]> = full.split(|c| *c == '\n').collect();
            if lines.len() <= count {
                return None;
            }
            let mut kept: Vec<char> = Vec::new();
            for (at, line) in lines[..count].iter().enumerate() {
                if at > 0 {
                    kept.push('\n');
                }
                kept.extend(*line);
            }
            Some(trail_off(&kept))
        }
    }
}

/// The Summit's Ascent level as the view gives it (`CardView.preview`, R280). `None` for every other card and
/// wherever the view carries no preview (the collection).
pub fn ascent_level_of(values: &[PreviewValue]) -> Option<i32> {
    values
        .iter()
        .find(|entry| entry.label == ASCENT_LABEL)
        .map(|entry| entry.value)
}

fn trimmed(line: &[char]) -> &[char] {
    let from = line.iter().position(|c| !is_space(*c)).unwrap_or(line.len());
    let to = line.iter().rposition(|c| !is_space(*c)).map_or(from, |at| at + 1);
    &line[from..to]
}

/// `^Ascent (\d+):` on a trimmed line: the level it names.
fn ascent_line(line: &[char]) -> Option<i32> {
    let line = trimmed(line);
    let rest = line.strip_prefix(&['A', 's', 'c', 'e', 'n', 't', ' '][..])?;
    let digits = rest.iter().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 || rest.get(digits) != Some(&':') {
        return None;
    }
    rest[..digits].iter().collect::<String>().parse().ok()
}

/// What a Summit face in play prints for `level`: the header (every line before the first "Ascent N:" line)
/// and only the level's own line. `None` when there is nothing to shorten to (a translated face parses no
/// Ascent lines).
pub fn summit_short_text(def_id: &str, full: &[char], level: Option<i32>) -> Option<Vec<char>> {
    if def_id != SUMMIT_DEF_ID {
        return None;
    }
    let level = level?;
    let lines: Vec<&[char]> = full.split(|c| *c == '\n').collect();
    let first = lines.iter().position(|line| ascent_line(line).is_some())?;
    let next = lines.iter().find(|line| ascent_line(line) == Some(level))?;
    let mut out: Vec<char> = Vec::new();
    for (at, line) in lines[..first]
        .iter()
        .copied()
        .chain(std::iter::once(*next))
        .enumerate()
    {
        if at > 0 {
            out.push('\n');
        }
        out.extend(line);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(text: &str) -> Vec<char> {
        text.chars().collect()
    }

    fn string(cut: Option<Vec<char>>) -> Option<String> {
        cut.map(|chars| chars.into_iter().collect())
    }

    #[test]
    fn the_first_lines_end_in_an_ellipsis_in_place_of_the_last_stop() {
        assert_eq!(
            string(cut_text(&chars("One.\nTwo.\nThree."), ShortForm::Lines(2))),
            Some("One.\nTwo…".to_string())
        );
        assert_eq!(string(cut_text(&chars("One.\nTwo."), ShortForm::Lines(2))), None);
    }

    #[test]
    fn colon_cuts_at_the_first_colon_unless_it_ends_the_text() {
        assert_eq!(
            string(cut_text(&chars("Choose one: a, b, c."), ShortForm::Colon)),
            Some("Choose one…".to_string())
        );
        assert_eq!(string(cut_text(&chars("Cry:"), ShortForm::Colon)), None);
        assert_eq!(
            string(cut_text(&chars("战吼：造成伤害。"), ShortForm::Colon)),
            Some("战吼…".to_string())
        );
    }

    #[test]
    fn clamp_cuts_nothing() {
        assert_eq!(cut_text(&chars("Long text"), ShortForm::Clamp), None);
    }

    #[test]
    fn the_summit_prints_its_header_and_the_levels_own_line() {
        let full = chars("Header.\nAscent 1: One.\nAscent 2: Two.\nAscent 3: Three.");
        assert_eq!(
            string(summit_short_text(SUMMIT_DEF_ID, &full, Some(2))),
            Some("Header.\nAscent 2: Two.".to_string())
        );
        assert_eq!(summit_short_text(SUMMIT_DEF_ID, &full, Some(9)), None);
        assert_eq!(summit_short_text("core-001", &full, Some(2)), None);
        assert_eq!(summit_short_text(SUMMIT_DEF_ID, &full, None), None);
    }

    #[test]
    fn a_card_has_a_form_per_face() {
        assert_eq!(short_form_of("core-098", false), Some(ShortForm::Lines(2)));
        assert_eq!(short_form_of("classic-023", true), None);
        assert_eq!(short_form_of("classic-023", false), Some(ShortForm::Clamp));
        assert_eq!(short_form_of("core-001", false), None);
    }
}
