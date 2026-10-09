//! Usernames (SPEC §9.4, R1432–R1436): the checks a proposed name passes, the form it is stored in,
//! the key two names clash on, and the light filter. No I/O: the store keeps the names
//! (`profiles.username_base`, `username_key`, `username_tag`, migration 0028) and the routes are
//! `api/username.rs`.
//!
//! A username is a base name plus, when the base is taken, a numeric tag: `Max`, `Max#1`, `Max#2`.
//! The server is the only judge of a name (§9.1, CLAUDE.md rule 7): the client proposes one, asks
//! for a preview and renders what comes back. A username is only a display label; every table,
//! request and log names a player by profile id (R1436).
//!
//! - [`normalize_username`] checks a proposed base name (R1432, R1433) and answers its stored form,
//!   NFKC, so fullwidth `ＭＡＸ` is stored and previewed as `MAX`.
//! - [`username_key`] is the NFKC case fold two names clash on (R1434): `Max`, `max` and `ＭＡＸ` are
//!   one name, and so are `Straße` and `STRASSE`. Both stores call it, and Postgres collation never
//!   decides.
//! - [`render_username`] and [`parse_username`] go between `(base, tag)` and `Max#3`.

use caseless::default_case_fold_str;
use unicode_normalization::UnicodeNormalization;
use unicode_properties::{GeneralCategory, GeneralCategoryGroup, UnicodeGeneralCategory};
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;

use crate::config::{USERNAME_INPUT_MAX_CHARS, USERNAME_MAX_LENGTH, USERNAME_MAX_MARKS, USERNAME_MIN_LENGTH};

/// Why a proposed base name is refused (R1432, R1433). Each has one plain sentence for the player
/// and a code the client can tell apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsernameRefusal {
    /// A character that is not a letter, a decimal digit, `_` or a combining mark after a letter;
    /// anything outside the Basic Multilingual Plane; or more than `USERNAME_MAX_MARKS` marks in a
    /// row.
    Characters,
    /// Letters from two scripts that no language writes together.
    MixedScripts,
    /// Fewer than `USERNAME_MIN_LENGTH` user-perceived characters.
    TooShort,
    /// More than `USERNAME_MAX_LENGTH` user-perceived characters.
    TooLong,
    /// The light filter (R1433). Its sentence never names the word that matched.
    NotAllowed,
}

impl UsernameRefusal {
    /// The refusal as the client reads it (`reason` in a preview).
    pub fn code(self) -> &'static str {
        match self {
            UsernameRefusal::Characters => "characters",
            UsernameRefusal::MixedScripts => "mixed_scripts",
            UsernameRefusal::TooShort => "too_short",
            UsernameRefusal::TooLong => "too_long",
            UsernameRefusal::NotAllowed => "not_allowed",
        }
    }

    /// The one sentence the player is shown.
    pub fn message(self) -> String {
        match self {
            UsernameRefusal::Characters => {
                "Use letters, digits and _ only: no spaces, punctuation, symbols or emoji.".to_string()
            }
            UsernameRefusal::MixedScripts => "Write your name in one alphabet, not a mix.".to_string(),
            UsernameRefusal::TooShort => format!("Use at least {USERNAME_MIN_LENGTH} characters."),
            UsernameRefusal::TooLong => format!("Use at most {USERNAME_MAX_LENGTH} characters."),
            UsernameRefusal::NotAllowed => "That name isn't allowed.".to_string(),
        }
    }
}

/// R1432, R1433: a proposed base name's stored form, NFKC, or why it is refused. The checks run in
/// this order, so a name with a bad character is refused for that before its length is counted:
///
/// 1. The input is trimmed. More than `USERNAME_INPUT_MAX_CHARS` characters is too long, refused
///    before anything else reads it; a character outside the Basic Multilingual Plane is refused.
/// 2. NFKC.
/// 3. Every character is a letter (`L*`), a decimal digit (`Nd`), `_`, or a combining mark (`M*`)
///    that follows a letter, at most `USERNAME_MAX_MARKS` in a row, and in the BMP.
/// 4. One script: digits, `_` and anything Common or Inherited go with any script; otherwise every
///    character shares one, except Han with Hiragana and Katakana (Japanese) and Han with Hangul
///    (Korean).
/// 5. `USERNAME_MIN_LENGTH` to `USERNAME_MAX_LENGTH` extended grapheme clusters.
/// 6. The light filter.
pub fn normalize_username(raw: &str) -> Result<String, UsernameRefusal> {
    let trimmed = raw.trim();
    if trimmed.chars().count() > USERNAME_INPUT_MAX_CHARS {
        return Err(UsernameRefusal::TooLong);
    }
    // R1432: every character in the Basic Multilingual Plane, as typed. Checked before NFKC too,
    // which would otherwise turn the supplementary plane's mathematical letters (`𝐌𝐚𝐱`) into the
    // BMP's.
    if trimmed.chars().any(|ch| !in_bmp(ch)) {
        return Err(UsernameRefusal::Characters);
    }
    let name: String = trimmed.nfkc().collect();
    check_characters(&name)?;
    check_scripts(&name)?;
    let length = name.graphemes(true).count();
    if length < USERNAME_MIN_LENGTH {
        return Err(UsernameRefusal::TooShort);
    }
    if length > USERNAME_MAX_LENGTH {
        return Err(UsernameRefusal::TooLong);
    }
    if is_blocked(&name) {
        return Err(UsernameRefusal::NotAllowed);
    }
    Ok(name)
}

/// R1434: the key two base names clash on, the NFKC form of the full case fold of the stored
/// (NFKC) form: `Max`, `max` and `MAX` give `max`; `Straße` and `STRASSE` give `strasse`.
pub fn username_key(display: &str) -> String {
    default_case_fold_str(display).nfkc().collect()
}

/// A username as it is shown: `Max`, or `Max#3` when it carries a tag.
pub fn render_username(base: &str, tag: Option<i64>) -> String {
    match tag {
        Some(tag) => format!("{base}#{tag}"),
        None => base.to_string(),
    }
}

/// A username as shown, split back into its base and tag: `Max#3` is `("Max", Some(3))`, `Max` is
/// `("Max", None)`. None when the part after the last `#` is not a whole number of at least 1. The
/// base is not checked here; a base never holds a `#` (R1432), so the last one starts the tag.
pub fn parse_username(shown: &str) -> Option<(&str, Option<i64>)> {
    match shown.rsplit_once('#') {
        None => Some((shown, None)),
        Some((base, tag)) => {
            if tag.is_empty() || !tag.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            let tag: i64 = tag.parse().ok()?;
            (tag >= 1).then_some((base, Some(tag)))
        }
    }
}

fn in_bmp(ch: char) -> bool {
    u32::from(ch) <= 0xFFFF
}

fn is_letter(ch: char) -> bool {
    ch.general_category_group() == GeneralCategoryGroup::Letter
}

fn is_mark(ch: char) -> bool {
    ch.general_category_group() == GeneralCategoryGroup::Mark
}

fn is_digit(ch: char) -> bool {
    ch.general_category() == GeneralCategory::DecimalNumber
}

/// R1432's characters, over the NFKC form.
fn check_characters(name: &str) -> Result<(), UsernameRefusal> {
    // How many marks in a row follow the last letter; None while no letter leads the run.
    let mut marks: Option<usize> = None;
    for ch in name.chars() {
        if !in_bmp(ch) {
            return Err(UsernameRefusal::Characters);
        }
        if is_mark(ch) {
            match marks {
                Some(run) if run < USERNAME_MAX_MARKS => marks = Some(run + 1),
                _ => return Err(UsernameRefusal::Characters),
            }
        } else if is_letter(ch) {
            marks = Some(0);
        } else if is_digit(ch) || ch == '_' {
            marks = None;
        } else {
            return Err(UsernameRefusal::Characters);
        }
    }
    Ok(())
}

/// R1432's one script per name. Digits go with any script, as the issue's rule says, whichever
/// script Unicode files them under (Arabic-Indic digits are Arabic).
fn check_scripts(name: &str) -> Result<(), UsernameRefusal> {
    let mut scripts: Vec<Script> = Vec::new();
    for ch in name.chars() {
        if is_digit(ch) {
            continue;
        }
        let script = ch.script();
        if matches!(script, Script::Common | Script::Inherited | Script::Unknown) {
            continue;
        }
        if !scripts.contains(&script) {
            scripts.push(script);
        }
    }
    const JAPANESE: &[Script] = &[Script::Han, Script::Hiragana, Script::Katakana];
    const KOREAN: &[Script] = &[Script::Han, Script::Hangul];
    let within = |allowed: &[Script]| scripts.iter().all(|script| allowed.contains(script));
    if scripts.len() <= 1 || within(JAPANESE) || within(KOREAN) {
        Ok(())
    } else {
        Err(UsernameRefusal::MixedScripts)
    }
}

// ---------------------------------------------------------------------------------------------
// The light filter (R1433)
// ---------------------------------------------------------------------------------------------

/// Words refused wherever they appear (slurs and the strongest profanity).
const BLOCKLIST_ANYWHERE: &str = include_str!("blocklist_anywhere.txt");
/// Words refused only as a whole token, since they occur inside innocent words.
const BLOCKLIST_TOKEN: &str = include_str!("blocklist_token.txt");

/// One list's words: a lowercase word per line, `#` comments and blank lines skipped.
fn words(list: &'static str) -> impl Iterator<Item = &'static str> {
    list.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

/// R1433's folding before a match: the full case fold, NFD with every combining mark dropped (so
/// `fück` reads `fuck`), underscores dropped, and the common digit substitutions read back as the
/// letters they stand for.
fn fold(text: &str) -> String {
    default_case_fold_str(text)
        .nfd()
        .filter(|ch| !is_mark(*ch) && *ch != '_')
        .map(|ch| match ch {
            '0' => 'o',
            '1' => 'i',
            '3' => 'e',
            '4' => 'a',
            '5' => 's',
            '7' => 't',
            other => other,
        })
        .collect()
}

/// R1433's tokens of an NFKC name: split on `_`, on every change between a letter and a digit, and
/// where a lowercase letter is followed by an uppercase one (`BigAss` is `Big`, `Ass`). A mark
/// stays with the letter before it.
fn tokens(name: &str) -> Vec<String> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Kind {
        Letter,
        Digit,
    }
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut last: Option<(Kind, char)> = None;
    for ch in name.chars() {
        if ch == '_' {
            out.push(std::mem::take(&mut current));
            last = None;
            continue;
        }
        if is_mark(ch) {
            current.push(ch);
            continue;
        }
        let kind = if is_digit(ch) { Kind::Digit } else { Kind::Letter };
        let split = match last {
            None => false,
            Some((last_kind, last_ch)) => {
                last_kind != kind
                    || (kind == Kind::Letter && last_ch.is_lowercase() && ch.is_uppercase())
            }
        };
        if split {
            out.push(std::mem::take(&mut current));
        }
        current.push(ch);
        last = Some((kind, ch));
    }
    out.push(current);
    out.retain(|token| !token.is_empty());
    out
}

/// R1433: whether the light filter refuses an NFKC name. A token without a letter is never matched
/// against the token list: its digits would read back as letters (`455` as `ass`) and refuse an
/// innocent `Bob455`.
fn is_blocked(name: &str) -> bool {
    let folded = fold(name);
    if words(BLOCKLIST_ANYWHERE).any(|word| folded.contains(word)) {
        return true;
    }
    tokens(name)
        .iter()
        .filter(|token| token.chars().any(is_letter))
        .map(|token| fold(token))
        .any(|token| words(BLOCKLIST_TOKEN).any(|word| token == word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r1433_splits_tokens_on_underscores_digits_and_camel_case() {
        assert_eq!(tokens("BigAss"), vec!["Big", "Ass"]);
        assert_eq!(tokens("my_ass"), vec!["my", "ass"]);
        assert_eq!(tokens("Ass99"), vec!["Ass", "99"]);
        assert_eq!(tokens("Scunthorpe"), vec!["Scunthorpe"]);
        assert_eq!(tokens("MAX__x"), vec!["MAX", "x"]);
    }

    #[test]
    fn r1432_parses_and_renders_a_tag() {
        assert_eq!(parse_username("Max#3"), Some(("Max", Some(3))));
        assert_eq!(parse_username("Max"), Some(("Max", None)));
        assert_eq!(parse_username("Max#0"), None);
        assert_eq!(parse_username("Max#"), None);
        assert_eq!(parse_username("Max#-1"), None);
        assert_eq!(parse_username("Max#+1"), None);
        assert_eq!(render_username("Max", Some(3)), "Max#3");
        assert_eq!(render_username("Max", None), "Max");
    }
}
