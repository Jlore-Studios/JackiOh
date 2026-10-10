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
/// 4. One script: digits, `_` and a mark that takes its letter's script go with any script;
///    otherwise every character shares one (by its `Script_Extensions`, so the kana length mark `ー`
///    goes with either kana and an Arabic vowel sign with Arabic only), except Han with Hiragana and
///    Katakana (Japanese) and Han with Hangul (Korean). A letter Unicode files under no script, not
///    even by extension (`Common`: modifier letters such as `ˈ` and `ː`), is a script of its own, so
///    it joins no other.
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

/// Unicode's `Default_Ignorable_Code_Point` ranges (DerivedCoreProperties.txt): characters a
/// renderer draws as nothing. Some of them are letters or marks by category (the Hangul fillers are
/// `Lo`, the combining grapheme joiner and the variation selectors `Mn`), so the category test alone
/// would let `Max` plus an invisible joiner stand as a second bare name beside `Max`, or a name of
/// fillers render blank. The `Cf` ones are refused by category already and are listed for
/// completeness.
const DEFAULT_IGNORABLE: &[(char, char)] = &[
    ('\u{00AD}', '\u{00AD}'),
    ('\u{034F}', '\u{034F}'),
    ('\u{061C}', '\u{061C}'),
    ('\u{115F}', '\u{1160}'),
    ('\u{17B4}', '\u{17B5}'),
    ('\u{180B}', '\u{180F}'),
    ('\u{200B}', '\u{200F}'),
    ('\u{202A}', '\u{202E}'),
    ('\u{2060}', '\u{206F}'),
    ('\u{3164}', '\u{3164}'),
    ('\u{FE00}', '\u{FE0F}'),
    ('\u{FEFF}', '\u{FEFF}'),
    ('\u{FFA0}', '\u{FFA0}'),
    ('\u{FFF0}', '\u{FFF8}'),
    ('\u{1BCA0}', '\u{1BCA3}'),
    ('\u{1D173}', '\u{1D17A}'),
    ('\u{E0000}', '\u{E0FFF}'),
];

fn is_default_ignorable(ch: char) -> bool {
    DEFAULT_IGNORABLE
        .iter()
        .any(|(first, last)| (*first..=*last).contains(&ch))
}

/// R1432's characters, over the NFKC form. An invisible character is refused whatever its
/// category says.
fn check_characters(name: &str) -> Result<(), UsernameRefusal> {
    // How many marks in a row follow the last letter; None while no letter leads the run.
    let mut marks: Option<usize> = None;
    for ch in name.chars() {
        if !in_bmp(ch) || is_default_ignorable(ch) {
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

/// What a name may be written in under R1432's one script per name: one of Unicode's scripts, or
/// one of the two mixes a language needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Writing {
    Script(Script),
    /// Han with Hiragana and Katakana.
    Japanese,
    /// Han with Hangul.
    Korean,
}

/// The writings a character may stand in, or None when it goes with any: a digit, whichever script
/// Unicode files it under (Arabic-Indic digits are Arabic), `_`, and a mark that takes the script
/// of the letter before it (`Inherited`, with no extension naming scripts). A mark whose extension
/// names its scripts goes with those alone, so an Arabic vowel sign on a Latin letter is a mix. A
/// character Unicode files under no script, not even by extension (`Common`), is a writing of its
/// own: the issue lets only digits and `_` go with any script.
fn writings(ch: char) -> Option<Vec<Writing>> {
    if is_digit(ch) || ch == '_' {
        return None;
    }
    let extension = ch.script_extension();
    if extension.is_inherited() {
        return None;
    }
    if extension.is_common() {
        return Some(vec![Writing::Script(Script::Common)]);
    }
    let mut out = Vec::new();
    for script in extension.iter() {
        out.push(Writing::Script(script));
        let mixes: &[Writing] = match script {
            Script::Han => &[Writing::Japanese, Writing::Korean],
            Script::Hiragana | Script::Katakana => &[Writing::Japanese],
            Script::Hangul => &[Writing::Korean],
            _ => &[],
        };
        for mix in mixes {
            if !out.contains(mix) {
                out.push(*mix);
            }
        }
    }
    Some(out)
}

/// R1432's one script per name: some writing every character may stand in. An unassigned
/// character stands in none, though the character check has refused it already.
fn check_scripts(name: &str) -> Result<(), UsernameRefusal> {
    let mut shared: Option<Vec<Writing>> = None;
    for own in name.chars().filter_map(writings) {
        shared = Some(match shared {
            None => own,
            Some(mut kept) => {
                kept.retain(|writing| own.contains(writing));
                kept
            }
        });
    }
    match shared {
        Some(kept) if kept.is_empty() => Err(UsernameRefusal::MixedScripts),
        _ => Ok(()),
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
/// `fück` reads `fuck`), underscores dropped, the common digit substitutions read back as the
/// letters they stand for, and the Latin look-alikes read as the letters they imitate.
fn fold(text: &str) -> String {
    default_case_fold_str(text)
        .nfd()
        .filter(|ch| !is_mark(*ch) && *ch != '_')
        .map(|ch| substitute(ch).or_else(|| look_alike(ch)).unwrap_or(ch))
        .collect()
}

/// R1433: the Latin letters that fancy-text tools pass off as plain ones and that neither NFKC nor
/// the case fold reads back, the small capitals and the dotless `ı` and `ȷ`, as the letter each
/// imitates, so `ɴɪɢɢᴇʀ` and `Bıtch` meet the lists as their plain spellings do. Only the filter
/// reads them so; the name keeps them.
fn look_alike(ch: char) -> Option<char> {
    let plain = match ch {
        'ᴀ' => 'a',
        'ʙ' => 'b',
        'ᴄ' => 'c',
        'ᴅ' => 'd',
        'ᴇ' => 'e',
        'ꜰ' => 'f',
        'ɢ' => 'g',
        'ʜ' => 'h',
        'ɪ' | 'ı' => 'i',
        'ᴊ' | 'ȷ' => 'j',
        'ᴋ' => 'k',
        'ʟ' => 'l',
        'ᴍ' => 'm',
        'ɴ' => 'n',
        'ᴏ' => 'o',
        'ᴘ' => 'p',
        'ꞯ' => 'q',
        'ʀ' => 'r',
        'ꜱ' => 's',
        'ᴛ' => 't',
        'ᴜ' => 'u',
        'ᴠ' => 'v',
        'ᴡ' => 'w',
        'ʏ' => 'y',
        'ᴢ' => 'z',
        _ => return None,
    };
    Some(plain)
}

/// R1433's common digit substitutions: the letter a digit stands for, if it stands for one.
fn substitute(ch: char) -> Option<char> {
    match ch {
        '0' => Some('o'),
        '1' => Some('i'),
        '3' => Some('e'),
        '4' => Some('a'),
        '5' => Some('s'),
        '7' => Some('t'),
        _ => None,
    }
}

/// Which substitution digits [`read_back_digits`] reads back as letters. A digit at the edge of a
/// word may be a letter (`a55`) or a number after the word (`Dick1`), so the filter reads a name
/// both ways.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReadBack {
    /// Every one in a run that holds a letter: `a55` reads `ass`, `5h1t` reads `shit`.
    All,
    /// Only one with a letter on both sides of it in its run: `d1ck` reads `dick`, and the `1` of
    /// `Dick1` stays a digit, so `Dick` splits off as a token of its own.
    Inner,
}

/// R1433: an NFKC name with its substitution digits read back as letters where they sit in a run of
/// letters (all of them, or only the inner ones: [`ReadBack`]), before it is split into tokens. A
/// run is the letters, marks and substitution digits between underscores and other digits; a run
/// with no letter in it keeps its digits, so they still split off as a token of digits alone and the
/// `455` of `Bob_455` is never read as `ass`. `Bob455` reads as the one token `Bobass`. A letter read
/// back takes the case of the nearest letter before it in the run, or else after it, so it never
/// makes a camelCase split of its own: `C0CK` reads `COCK`, not `CoCK`.
fn read_back_digits(name: &str, digits: ReadBack) -> String {
    fn flush(run: &mut Vec<char>, out: &mut String, digits: ReadBack) {
        if !run.iter().copied().any(is_letter) {
            out.extend(run.drain(..));
            return;
        }
        for at in 0..run.len() {
            let ch = run[at];
            let Some(letter) = substitute(ch) else {
                out.push(ch);
                continue;
            };
            let before = run[..at].iter().rev().copied().find(|ch| is_letter(*ch));
            let after = run[at + 1..].iter().copied().find(|ch| is_letter(*ch));
            if digits == ReadBack::Inner && (before.is_none() || after.is_none()) {
                out.push(ch);
            } else if before.or(after).is_some_and(char::is_uppercase) {
                out.push(letter.to_ascii_uppercase());
            } else {
                out.push(letter);
            }
        }
        run.clear();
    }
    let mut out = String::with_capacity(name.len());
    let mut run: Vec<char> = Vec::new();
    for ch in name.chars() {
        if is_letter(ch) || is_mark(ch) || substitute(ch).is_some() {
            run.push(ch);
        } else {
            flush(&mut run, &mut out, digits);
            out.push(ch);
        }
    }
    flush(&mut run, &mut out, digits);
    out
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
                last_kind != kind || (kind == Kind::Letter && last_ch.is_lowercase() && ch.is_uppercase())
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

/// R1433: whether the light filter refuses an NFKC name. The token list is matched against the
/// tokens of the name with its digits read back both ways ([`read_back_digits`], [`ReadBack`]), so
/// `a55`, `d1ck` and `Dick1` are each refused and `Tit4n` is not. A token without a letter is never
/// matched against it: its digits would read back as letters (`455` as `ass`) and refuse an
/// innocent `Bob_455`.
fn is_blocked(name: &str) -> bool {
    let folded = fold(name);
    if words(BLOCKLIST_ANYWHERE).any(|word| folded.contains(word)) {
        return true;
    }
    [ReadBack::All, ReadBack::Inner]
        .into_iter()
        .flat_map(|digits| tokens(&read_back_digits(name, digits)))
        .filter(|token| token.chars().any(is_letter))
        .map(|token| fold(&token))
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
    fn r1433_reads_substitution_digits_back_only_in_a_run_with_letters() {
        let all = |name| read_back_digits(name, ReadBack::All);
        assert_eq!(all("a55"), "ass");
        assert_eq!(all("BigA55"), "BigASS");
        assert_eq!(all("5h1t"), "shit");
        assert_eq!(all("5H1T"), "SHIT");
        assert_eq!(all("C0CK_7"), "COCK_7");
        assert_eq!(all("Tit4n"), "Titan");
        assert_eq!(all("Bob455"), "Bobass");
        assert_eq!(all("Bob_455"), "Bob_455");
        assert_eq!(all("Ass99"), "Ass99");
        assert_eq!(all("a5s9"), "ass9");
        assert_eq!(all("Dick1"), "Dicki");
    }

    #[test]
    fn r1433_reads_only_inner_substitution_digits_back_the_other_way() {
        let inner = |name| read_back_digits(name, ReadBack::Inner);
        assert_eq!(inner("d1ck"), "dick");
        assert_eq!(inner("Tit4n"), "Titan");
        assert_eq!(inner("Dick1"), "Dick1");
        assert_eq!(inner("1Dick"), "1Dick");
        assert_eq!(inner("D1ck1"), "DIck1");
        assert_eq!(inner("a55"), "a55");
        assert_eq!(inner("Bob_455"), "Bob_455");
        assert_eq!(tokens(&inner("Dick1")), vec!["Dick", "1"]);
    }

    #[test]
    fn r1433_folds_small_capitals_and_dotless_letters_to_the_letters_they_imitate() {
        assert_eq!(fold("ꜰᴜᴄᴋ"), "fuck");
        assert_eq!(fold("Bıtch"), "bitch");
        assert_eq!(fold("ᴅɪᴄᴋ"), "dick");
        assert_eq!(fold("ȷoe"), "joe");
    }

    #[test]
    fn r1432_a_name_shares_one_writing_by_script_extensions() {
        // The kana length mark is Common by script but kana by extension.
        assert!(check_scripts("ラーメン").is_ok());
        assert!(check_scripts("らーめん").is_ok());
        assert!(check_scripts("東京ラーメン").is_ok());
        // The Arabic tatweel goes with Arabic.
        assert!(check_scripts("محـمد").is_ok());
        // A modifier letter of no script, even by extension, joins no other; one whose extension
        // names Latin (the modifier apostrophe) goes with Latin.
        assert_eq!(check_scripts("Max\u{02C8}"), Err(UsernameRefusal::MixedScripts));
        assert_eq!(
            check_scripts("ラーメン\u{02D0}"),
            Err(UsernameRefusal::MixedScripts)
        );
        assert!(check_scripts("Max\u{02BC}").is_ok());
        // A mark goes with the scripts its extension names: Arabic vowel signs on Arabic, not on Latin.
        assert!(check_scripts("مُحَمَّد").is_ok());
        assert_eq!(check_scripts("Ma\u{064E}x"), Err(UsernameRefusal::MixedScripts));
        // Digits, `_` and inherited marks go with any.
        assert!(check_scripts("Zoe\u{308}_99").is_ok());
        assert!(check_scripts("محمد_٣").is_ok());
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
