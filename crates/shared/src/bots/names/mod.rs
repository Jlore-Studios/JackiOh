//! A new ladder bot's username, drawn as people pick theirs (R1454): a register by its
//! share (`BOT_NAME_SHARE_*`, `BOT_NAME_LUIS_SHARE`), then shapes by the register's weights
//! (`BOT_NAME_SHAPES_*`), filled from the word lists beside this module. A name the shared
//! filter or the bots' own `never.txt` refuses is redrawn, up to `BOT_NAME_DRAW_ATTEMPTS` times,
//! before the bot keeps its `Player#n`.
//!
//! A politics word is never drawn beside an edgy or crude one: each register draws only from
//! its own lists, and the politics register's shapes never meet another register's. No shape
//! writes a `Real`, `Official` or `The_Real` prefix, so no bot poses as a person (R1454).
//!
//! What a username may be is the accounts' rule (R1432), not this module's: the host judges each name
//! drawn (`draw_username`'s `accepts`), as it judges a person's, and `never_lists` is the bots' own
//! list for it to hold the name to.

use jackioh_engine::Rng;

use crate::bots::int_in;
use crate::config::{
    BOT_DEFAULT_USERNAME_SHARE, BOT_NAME_BIRTH_YEAR_MAX, BOT_NAME_BIRTH_YEAR_MIN, BOT_NAME_CHINESE_NUMBERS,
    BOT_NAME_CHINESE_YEAR_SHARE, BOT_NAME_COMPOUND_LETTERS, BOT_NAME_DRAW_ATTEMPTS,
    BOT_NAME_FILLER_NUMBER_SHARE, BOT_NAME_FILLER_NUMBERS, BOT_NAME_LUIS_SHARE, BOT_NAME_SHAPES_CHINESE_HAN,
    BOT_NAME_SHAPES_CHINESE_PINYIN, BOT_NAME_SHAPES_CRUDE, BOT_NAME_SHAPES_EDGY, BOT_NAME_SHAPES_GAMER,
    BOT_NAME_SHAPES_GIVEN, BOT_NAME_SHAPES_NONSENSE, BOT_NAME_SHAPES_PLAIN, BOT_NAME_SHAPES_POLITICS,
    BOT_NAME_SHAPES_REFERENCE, BOT_NAME_SHARE_ANIME, BOT_NAME_SHARE_CHINESE_HAN,
    BOT_NAME_SHARE_CHINESE_PINYIN, BOT_NAME_SHARE_CRUDE, BOT_NAME_SHARE_EDGY, BOT_NAME_SHARE_GAMER,
    BOT_NAME_SHARE_GIVEN, BOT_NAME_SHARE_NONSENSE, BOT_NAME_SHARE_OTHER_REFS, BOT_NAME_SHARE_PLAIN,
    BOT_NAME_SHARE_POLITICS, BOT_NAME_STRETCH_MAX, BOT_NAME_STRETCH_MIN, BOT_NAME_SYLLABLES_MAX,
    BOT_NAME_SYLLABLES_MIN, BOT_NAME_TWO_DIGIT_MAX, BOT_NAME_TWO_DIGIT_MIN, BOT_NAME_YEAR_SHARE,
    BotNameShape,
};

/// R1454: word lists for bot username registers.
const ADJECTIVES: &str = include_str!("adjectives.txt");
const NOUNS: &str = include_str!("nouns.txt");
const NONSENSE: &str = include_str!("nonsense.txt");
const GAMER: &str = include_str!("gamer.txt");
const EDGY: &str = include_str!("edgy.txt");
const CRUDE: &str = include_str!("crude.txt");
const ANIME: &str = include_str!("anime.txt");
const REFS: &str = include_str!("refs.txt");
const POLITICS: &str = include_str!("politics.txt");
const GIVEN: &str = include_str!("given.txt");
const CHINESE_HAN: &str = include_str!("chinese_han.txt");
const CHINESE_SURNAMES: &str = include_str!("chinese_surnames.txt");
const CHINESE_PINYIN: &str = include_str!("chinese_pinyin.txt");
/// R1454: bot-specific exclusion list on top of shared filter.
const NEVER: &str = include_str!("never.txt");
/// R1454: common given name drawn by `BOT_NAME_LUIS_SHARE`.
const LUIS: &str = "Luis";

/// R1454: the onsets of a nonsense syllable (no `n`, `j`, `g` or `k`: slur soundalikes).
const ONSETS: &[&str] = &[
    "b", "bl", "br", "d", "dr", "f", "fl", "gl", "gr", "kr", "l", "m", "p", "pl", "qu", "r", "s", "sk",
    "skr", "sn", "sp", "t", "tr", "v", "w", "z",
];
const VOWELS: &[&str] = &["a", "e", "i", "o", "u", "oo", "ee", "ai", "ou"];
/// R1454: the codas of a nonsense draw, two of them empty.
const CODAS: &[&str] = &[
    "", "", "rp", "nk", "x", "ng", "b", "dle", "mp", "zz", "rb", "rf", "m",
];
const INITIALS: &[&str] = &[
    "A", "B", "C", "D", "E", "F", "G", "H", "J", "L", "M", "N", "P", "R", "S", "T", "V", "Z",
];
const ENDINGS: &[&str] = &["ii", "zz", "_"];
const PREFIXES: &[&str] = &["The", "El", "Its", "Mr", "Lil"];
const SUFFIXES: &[&str] = &["TV", "YT", "Gaming", "Plays", "HD", "Live"];
const FAN_SUFFIXES: &[&str] = &["Main", "Fan", "Stan", "Kun", "4Life", "Enjoyer"];
const FAN_PREFIXES: &[&str] = &["Not", "Team"];

/// R1454: which word register a username is drawn from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Register {
    Plain,
    Nonsense,
    Gamer,
    Edgy,
    Anime,
    OtherRefs,
    Crude,
    Politics,
    Given,
    Luis,
    ChineseHan,
    ChinesePinyin,
}

impl Register {
    /// R1454: the lists a register fills its shapes from: the pair's first word, then every
    /// other word.
    fn lists(self) -> (&'static str, &'static str) {
        match self {
            Register::Plain => (ADJECTIVES, NOUNS),
            Register::Gamer => (NOUNS, GAMER),
            Register::Luis => (GIVEN, LUIS),
            Register::ChinesePinyin => (CHINESE_SURNAMES, CHINESE_PINYIN),
            Register::Nonsense => (NONSENSE, NONSENSE),
            Register::Edgy => (EDGY, EDGY),
            Register::Anime => (ANIME, ANIME),
            Register::OtherRefs => (REFS, REFS),
            Register::Crude => (CRUDE, CRUDE),
            Register::Politics => (POLITICS, POLITICS),
            Register::Given => (GIVEN, GIVEN),
            Register::ChineseHan => (CHINESE_HAN, CHINESE_HAN),
        }
    }

    /// R1454: the shape table a register draws from.
    fn shapes(self) -> &'static [(BotNameShape, f64)] {
        match self {
            Register::Plain => BOT_NAME_SHAPES_PLAIN,
            Register::Nonsense => BOT_NAME_SHAPES_NONSENSE,
            Register::Gamer => BOT_NAME_SHAPES_GAMER,
            Register::Edgy => BOT_NAME_SHAPES_EDGY,
            Register::Anime | Register::OtherRefs => BOT_NAME_SHAPES_REFERENCE,
            Register::Crude => BOT_NAME_SHAPES_CRUDE,
            Register::Politics => BOT_NAME_SHAPES_POLITICS,
            Register::Given | Register::Luis => BOT_NAME_SHAPES_GIVEN,
            Register::ChineseHan => BOT_NAME_SHAPES_CHINESE_HAN,
            Register::ChinesePinyin => BOT_NAME_SHAPES_CHINESE_PINYIN,
        }
    }
}

/// R1454: weighted registers for named bots.
const REGISTERS: [(Register, f64); 12] = [
    (Register::Plain, BOT_NAME_SHARE_PLAIN),
    (Register::Nonsense, BOT_NAME_SHARE_NONSENSE),
    (Register::Gamer, BOT_NAME_SHARE_GAMER),
    (Register::Edgy, BOT_NAME_SHARE_EDGY),
    (Register::Anime, BOT_NAME_SHARE_ANIME),
    (Register::OtherRefs, BOT_NAME_SHARE_OTHER_REFS),
    (Register::Crude, BOT_NAME_SHARE_CRUDE),
    (Register::Politics, BOT_NAME_SHARE_POLITICS),
    (Register::Given, BOT_NAME_SHARE_GIVEN),
    (
        Register::Luis,
        BOT_NAME_LUIS_SHARE / (1.0 - BOT_DEFAULT_USERNAME_SHARE),
    ),
    (Register::ChineseHan, BOT_NAME_SHARE_CHINESE_HAN),
    (Register::ChinesePinyin, BOT_NAME_SHARE_CHINESE_PINYIN),
];

fn entries(list: &'static str) -> Vec<&'static str> {
    list.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// R1454: weighted random selection from `table`.
fn weighted<T: Copy>(rng: &mut Rng, table: &[(T, f64)]) -> Option<T> {
    let total: f64 = table.iter().map(|(_, weight)| weight).sum();
    let mut draw = rng.next() * total;
    for (item, weight) in table {
        if draw < *weight {
            return Some(*item);
        }
        draw -= weight;
    }
    table.last().map(|(item, _)| *item)
}

fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn year(rng: &mut Rng) -> i64 {
    int_in(rng, BOT_NAME_BIRTH_YEAR_MIN, BOT_NAME_BIRTH_YEAR_MAX)
}

/// R1454: drawn number: birth year, filler, or two-digit number.
fn number(rng: &mut Rng, register: Register) -> String {
    let draw = rng.next();
    if matches!(register, Register::ChineseHan | Register::ChinesePinyin) {
        if draw < BOT_NAME_CHINESE_YEAR_SHARE {
            return year(rng).to_string();
        }
        return rng
            .pick(BOT_NAME_CHINESE_NUMBERS)
            .copied()
            .unwrap_or(666)
            .to_string();
    }
    if draw < BOT_NAME_YEAR_SHARE {
        year(rng).to_string()
    } else if draw < BOT_NAME_YEAR_SHARE + BOT_NAME_FILLER_NUMBER_SHARE {
        rng.pick(BOT_NAME_FILLER_NUMBERS)
            .copied()
            .unwrap_or(69)
            .to_string()
    } else {
        int_in(rng, BOT_NAME_TWO_DIGIT_MIN, BOT_NAME_TWO_DIGIT_MAX).to_string()
    }
}

fn leet(word: &str) -> String {
    word.chars()
        .map(|ch| match ch {
            'a' | 'A' => '4',
            'e' | 'E' => '3',
            'o' | 'O' => '0',
            'i' | 'I' => '1',
            _ => ch,
        })
        .collect()
}

/// R1454: lowercases word and stretches vowel or last character.
fn stretched(rng: &mut Rng, word: &str) -> String {
    let lower = word.to_lowercase();
    let extra = int_in(rng, BOT_NAME_STRETCH_MIN, BOT_NAME_STRETCH_MAX);
    let mut at: Option<(usize, char)> = None;
    for (index, ch) in lower.char_indices() {
        if matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u') {
            at = Some((index, ch));
        }
    }
    let Some((index, ch)) = at.or_else(|| lower.char_indices().last()) else {
        return lower;
    };
    let mut out = String::with_capacity(lower.len() + extra as usize);
    out.push_str(&lower[..index]);
    for _ in 0..=extra {
        out.push(ch);
    }
    out.push_str(&lower[index + ch.len_utf8()..]);
    out
}

/// R1454: Spanish diminutive suffix.
fn diminutive(word: &str) -> String {
    if let Some(stripped) = word.strip_suffix('a').or_else(|| word.strip_suffix('A')) {
        format!("{stripped}ita")
    } else if let Some(stripped) = word.strip_suffix('o').or_else(|| word.strip_suffix('O')) {
        format!("{stripped}ito")
    } else {
        format!("{word}ito")
    }
}

/// R1454: nonsense syllables: onset-vowel pairs and one coda.
fn syllables(rng: &mut Rng) -> String {
    let count = int_in(rng, BOT_NAME_SYLLABLES_MIN, BOT_NAME_SYLLABLES_MAX);
    let mut out = String::new();
    for _ in 0..count {
        out.push_str(rng.pick(ONSETS).copied().unwrap_or("b"));
        out.push_str(rng.pick(VOWELS).copied().unwrap_or("a"));
    }
    out.push_str(rng.pick(CODAS).copied().unwrap_or(""));
    out
}

/// R1454: renders a name in `shape` drawn from `register`'s word lists.
fn render(rng: &mut Rng, register: Register, shape: BotNameShape) -> Option<String> {
    let (first, second) = register.lists();
    let firsts = entries(first);
    let seconds = entries(second);
    let lead = rng.pick(&firsts).copied()?;
    let word = rng.pick(&seconds).copied()?;
    let name = match shape {
        BotNameShape::Bare => word.to_string(),
        BotNameShape::Lower => word.to_lowercase(),
        BotNameShape::Camel => format!("{}{}", capitalized(lead), capitalized(word)),
        BotNameShape::LowerPair => format!("{}{}", lead.to_lowercase(), word.to_lowercase()),
        BotNameShape::SnakePair => {
            format!("{}_{}", lead.to_lowercase(), word.to_lowercase())
        }
        BotNameShape::LowerPairDigits => format!(
            "{}{}{}",
            lead.to_lowercase(),
            word.to_lowercase(),
            number(rng, register)
        ),
        BotNameShape::Digits => format!("{word}{}", number(rng, register)),
        BotNameShape::LowerDigits => {
            format!("{}{}", word.to_lowercase(), number(rng, register))
        }
        BotNameShape::SnakeDigits => format!("{}_{}", word, number(rng, register)),
        BotNameShape::Xx => {
            if rng.coin() {
                format!("xX{}Xx", word.to_lowercase())
            } else {
                format!("xX_{word}_Xx")
            }
        }
        BotNameShape::Prefix => format!(
            "{}{}",
            rng.pick(PREFIXES).copied().unwrap_or("The"),
            capitalized(word)
        ),
        BotNameShape::FanPrefix => format!(
            "{}{}",
            rng.pick(FAN_PREFIXES).copied().unwrap_or("Not"),
            capitalized(word)
        ),
        BotNameShape::Suffix => {
            format!("{word}{}", rng.pick(SUFFIXES).copied().unwrap_or("TV"))
        }
        BotNameShape::FanSuffix => {
            format!("{word}{}", rng.pick(FAN_SUFFIXES).copied().unwrap_or("Main"))
        }
        BotNameShape::Leet => leet(word),
        BotNameShape::Stretch => stretched(rng, word),
        BotNameShape::Syllables => syllables(rng),
        BotNameShape::Doubled => {
            if rng.coin() {
                format!("{word}{}", word.chars().last()?)
            } else {
                format!("{word}{}", rng.pick(ENDINGS).copied().unwrap_or("ii"))
            }
        }
        BotNameShape::Initial => {
            if rng.coin() {
                format!("{word}_{}", rng.pick(INITIALS).copied().unwrap_or("M"))
            } else {
                format!("{word}{}", rng.pick(INITIALS).copied().unwrap_or("M"))
            }
        }
        BotNameShape::Diminutive => diminutive(word),
        BotNameShape::Gamer => {
            let base = if rng.coin() {
                diminutive(word)
            } else {
                word.to_string()
            };
            if rng.coin() {
                format!("{base}Gamer{}", year(rng))
            } else {
                format!("{base}Gamer")
            }
        }
        BotNameShape::Compound => {
            let head: String = lead.chars().take(BOT_NAME_COMPOUND_LETTERS).collect();
            format!("{word}{}", capitalized(&head))
        }
    };
    Some(name)
}

/// R1454: the bots' own exclusion list (`never.txt`, R1433) as the two lists the account filter reads: the
/// words refused anywhere in a name, and the ones refused as a whole token (written `=word`).
pub fn never_lists() -> (Vec<&'static str>, Vec<&'static str>) {
    let mut anywhere: Vec<&'static str> = Vec::new();
    let mut token: Vec<&'static str> = Vec::new();
    for word in entries(NEVER) {
        if let Some(stripped) = word.strip_prefix('=') {
            token.push(stripped);
        } else {
            anywhere.push(word);
        }
    }
    (anywhere, token)
}

/// R1454: draws a username `accepts` takes, or `None` to keep `Player#n`. `accepts` is the host's own
/// judgement of a drawn name: it answers the name as the account will hold it (normalized as a person's
/// is, R1432) when it passes that filter and `never_lists`, else `None`, and the draw tries again, up to
/// `BOT_NAME_DRAW_ATTEMPTS` times.
pub fn draw_username(rng: &mut Rng, accepts: &mut dyn FnMut(&str) -> Option<String>) -> Option<String> {
    let register = weighted(rng, &REGISTERS)?;
    for _ in 0..BOT_NAME_DRAW_ATTEMPTS {
        let shape = weighted(rng, register.shapes())?;
        let Some(name) = render(rng, register, shape) else {
            continue;
        };
        if let Some(base) = accepts(&name) {
            return Some(base);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Draws enough that each share below is read within a few hundredths.
    const DRAWS: usize = 6_000;
    /// How far a share read over `DRAWS` draws may sit from the share it should be.
    const SPREAD: f64 = 0.03;

    fn share(count: usize) -> f64 {
        count as f64 / DRAWS as f64
    }

    #[track_caller]
    fn near(actual: f64, expected: f64, what: &str) {
        assert!(
            (actual - expected).abs() <= SPREAD,
            "{what}: {actual:.4}, not {expected:.4}"
        );
    }

    /// R1454: each register fills its shapes from its own lists and draws its own shape table.
    #[test]
    fn r1454_each_register_draws_from_its_own_lists_and_shapes() {
        let registers = [
            (Register::Plain, (ADJECTIVES, NOUNS), BOT_NAME_SHAPES_PLAIN),
            (Register::Gamer, (NOUNS, GAMER), BOT_NAME_SHAPES_GAMER),
            (Register::Luis, (GIVEN, LUIS), BOT_NAME_SHAPES_GIVEN),
            (
                Register::ChinesePinyin,
                (CHINESE_SURNAMES, CHINESE_PINYIN),
                BOT_NAME_SHAPES_CHINESE_PINYIN,
            ),
            (Register::Nonsense, (NONSENSE, NONSENSE), BOT_NAME_SHAPES_NONSENSE),
            (Register::Edgy, (EDGY, EDGY), BOT_NAME_SHAPES_EDGY),
            (Register::Anime, (ANIME, ANIME), BOT_NAME_SHAPES_REFERENCE),
            (Register::OtherRefs, (REFS, REFS), BOT_NAME_SHAPES_REFERENCE),
            (Register::Crude, (CRUDE, CRUDE), BOT_NAME_SHAPES_CRUDE),
            (Register::Politics, (POLITICS, POLITICS), BOT_NAME_SHAPES_POLITICS),
            (Register::Given, (GIVEN, GIVEN), BOT_NAME_SHAPES_GIVEN),
            (
                Register::ChineseHan,
                (CHINESE_HAN, CHINESE_HAN),
                BOT_NAME_SHAPES_CHINESE_HAN,
            ),
        ];
        for (register, lists, shapes) in registers {
            assert_eq!(register.lists(), lists, "{register:?}");
            assert_eq!(register.shapes(), shapes, "{register:?}");
            assert!(!shapes.is_empty(), "{register:?}");
        }
    }

    /// R1454: Luis is BOT_NAME_LUIS_SHARE of every bot, those that keep their `Player#n` included, so
    /// among the named ones it weighs that share over the named ones' share.
    #[test]
    fn r1454_luis_is_its_share_of_every_bot() {
        let luis = REGISTERS
            .iter()
            .find(|(register, _)| *register == Register::Luis)
            .map(|(_, weight)| *weight)
            .expect("Luis is a register");
        assert!((luis * (1.0 - BOT_DEFAULT_USERNAME_SHARE) - BOT_NAME_LUIS_SHARE).abs() < 1e-12);
    }

    /// R1454: a list's entries are its lines, trimmed, without blank lines or `#` comments.
    #[test]
    fn r1454_a_lists_entries_are_its_trimmed_lines_but_blanks_and_comments() {
        assert_eq!(
            entries("  # a comment\nfoo\n\n   bar  \n#\nbaz"),
            ["foo", "bar", "baz"]
        );
    }

    /// R1454: a weighted draw takes each item at its weight's share of the total.
    #[test]
    fn r1454_a_weighted_draw_takes_each_item_at_its_share() {
        let table = [(0usize, 1.0), (1, 1.0), (2, 1.0)];
        let mut rng = Rng::new("r1454-weighted", 0);
        let mut counts = [0usize; 3];
        for _ in 0..DRAWS {
            counts[weighted(&mut rng, &table).expect("a table of weight draws")] += 1;
        }
        for (item, count) in counts.into_iter().enumerate() {
            near(share(count), 1.0 / 3.0, &format!("item {item}"));
        }
    }

    /// R1454: the words a name is built of.
    #[test]
    fn r1454_a_names_words_are_shaped_as_the_shape_says() {
        assert_eq!(capitalized("tempo"), "Tempo");
        assert_eq!(capitalized(""), "");
        assert_eq!(leet("aEoIx-Banana"), "4301x-B4n4n4");
        assert_eq!(diminutive("Rosa"), "Rosita");
        assert_eq!(diminutive("Pedro"), "Pedrito");
        assert_eq!(diminutive("Luis"), "Luisito");
    }

    /// R1454: a stretched word is lowercased with its last vowel, or its last letter when it has none,
    /// held BOT_NAME_STRETCH_MIN to BOT_NAME_STRETCH_MAX more times, the rest of the word kept.
    #[test]
    fn r1454_a_stretched_word_holds_its_last_vowel_and_keeps_the_rest() {
        let mut rng = Rng::new("r1454-stretch", 0);
        let held = |name: &str, before: &str, ch: char, after: &str| -> i64 {
            let middle = name
                .strip_prefix(before)
                .and_then(|rest| rest.strip_suffix(after))
                .unwrap_or_else(|| panic!("{name} is not {before}…{after}"));
            assert!(middle.chars().all(|c| c == ch), "{name}");
            i64::try_from(middle.chars().count()).expect("a short word") - 1
        };
        for _ in 0..50 {
            for (word, before, ch, after) in [
                ("Bark", "b", 'a', "rk"),
                ("Tempo", "temp", 'o', ""),
                ("Zzz", "zz", 'z', ""),
            ] {
                let extra = held(&stretched(&mut rng, word), before, ch, after);
                assert!(
                    (BOT_NAME_STRETCH_MIN..=BOT_NAME_STRETCH_MAX).contains(&extra),
                    "{word}: {extra}"
                );
            }
        }
    }

    /// Whether `name` is `pairs` onset-vowel syllables and one coda.
    fn syllable_shaped(name: &str, pairs: i64) -> bool {
        if pairs == 0 {
            return CODAS.contains(&name);
        }
        ONSETS.iter().any(|onset| {
            name.strip_prefix(onset).is_some_and(|rest| {
                VOWELS.iter().any(|vowel| {
                    rest.strip_prefix(vowel)
                        .is_some_and(|rest| syllable_shaped(rest, pairs - 1))
                })
            })
        })
    }

    /// R1454: a nonsense name is BOT_NAME_SYLLABLES_MIN to BOT_NAME_SYLLABLES_MAX onset-vowel pairs
    /// and one coda.
    #[test]
    fn r1454_a_nonsense_name_is_its_syllables_and_a_coda() {
        let mut rng = Rng::new("r1454-syllables", 0);
        for _ in 0..200 {
            let name = syllables(&mut rng);
            assert!(
                (BOT_NAME_SYLLABLES_MIN..=BOT_NAME_SYLLABLES_MAX).any(|pairs| syllable_shaped(&name, pairs)),
                "{name}"
            );
        }
    }

    /// R1454: a name's number is a birth year at BOT_NAME_YEAR_SHARE, a filler at
    /// BOT_NAME_FILLER_NUMBER_SHARE and two digits for the rest; a Chinese register's, a year at
    /// BOT_NAME_CHINESE_YEAR_SHARE and one of its own numbers for the rest.
    #[test]
    fn r1454_a_names_number_is_a_year_a_filler_or_two_digits_at_their_shares() {
        let years = BOT_NAME_BIRTH_YEAR_MIN..=BOT_NAME_BIRTH_YEAR_MAX;
        let two_digits = BOT_NAME_TWO_DIGIT_MIN..=BOT_NAME_TWO_DIGIT_MAX;
        let mut rng = Rng::new("r1454-number", 0);
        let (mut year_count, mut filler_count, mut short_count) = (0, 0, 0);
        for _ in 0..DRAWS {
            let n: i64 = number(&mut rng, Register::Plain).parse().expect("a number");
            if years.contains(&n) {
                assert!(!BOT_NAME_FILLER_NUMBERS.contains(&n), "{n}");
                year_count += 1;
            } else if two_digits.contains(&n) {
                short_count += 1;
            } else {
                assert!(BOT_NAME_FILLER_NUMBERS.contains(&n), "{n}");
                filler_count += 1;
            }
        }
        // A filler that is itself two digits (69) counts as two digits here.
        let short_fillers = BOT_NAME_FILLER_NUMBERS
            .iter()
            .filter(|n| two_digits.contains(*n))
            .count() as f64;
        let fillers = BOT_NAME_FILLER_NUMBERS.len() as f64;
        near(share(year_count), BOT_NAME_YEAR_SHARE, "years");
        near(
            share(filler_count),
            BOT_NAME_FILLER_NUMBER_SHARE * (fillers - short_fillers) / fillers,
            "fillers",
        );
        near(
            share(short_count),
            1.0 - BOT_NAME_YEAR_SHARE - BOT_NAME_FILLER_NUMBER_SHARE * (fillers - short_fillers) / fillers,
            "two digits",
        );

        let mut chinese_years = 0;
        for _ in 0..DRAWS {
            let n: i64 = number(&mut rng, Register::ChineseHan).parse().expect("a number");
            if years.contains(&n) {
                chinese_years += 1;
            } else {
                assert!(BOT_NAME_CHINESE_NUMBERS.contains(&n), "{n}");
            }
        }
        near(share(chinese_years), BOT_NAME_CHINESE_YEAR_SHARE, "Chinese years");
    }

    /// R1454: a bare name is one word of its register's list, and a birth year is one of the years.
    #[test]
    fn r1454_a_bare_name_is_a_word_of_its_list_and_a_year_is_a_birth_year() {
        let nouns = entries(NOUNS);
        let mut rng = Rng::new("r1454-render", 0);
        for _ in 0..50 {
            let name = render(&mut rng, Register::Plain, BotNameShape::Bare).expect("a name");
            assert!(nouns.contains(&name.as_str()), "{name}");
            assert!((BOT_NAME_BIRTH_YEAR_MIN..=BOT_NAME_BIRTH_YEAR_MAX).contains(&year(&mut rng)));
        }
    }

    /// R1454: `never.txt`'s words, those refused anywhere in a name and those refused as a whole token
    /// (written `=word`), each in its own list.
    #[test]
    fn r1454_the_never_list_splits_into_words_refused_anywhere_and_as_a_token() {
        let (anywhere, token) = never_lists();
        assert!(!anywhere.is_empty() && !token.is_empty());
        let lines = entries(NEVER);
        assert_eq!(anywhere.len() + token.len(), lines.len());
        for line in lines {
            match line.strip_prefix('=') {
                Some(word) => assert!(token.contains(&word), "{word}"),
                None => assert!(anywhere.contains(&line), "{line}"),
            }
        }
    }

    /// R1454: a drawn username is the name the host's `accepts` gives back for it, and when it refuses
    /// every draw, BOT_NAME_DRAW_ATTEMPTS of them, the bot keeps its `Player#n`.
    #[test]
    fn r1454_a_drawn_username_is_what_the_host_accepts_and_none_after_its_attempts() {
        for n in 0..20 {
            let mut rng = Rng::new(&format!("r1454-username-{n}"), 0);
            let mut asked: Vec<String> = Vec::new();
            let got = draw_username(&mut rng, &mut |name| {
                asked.push(name.to_string());
                Some(format!("{name}!"))
            });
            assert_eq!(asked.len(), 1, "{asked:?}");
            assert!(!asked[0].is_empty());
            assert_eq!(got, Some(format!("{}!", asked[0])));

            let mut refused = 0;
            let none = draw_username(&mut rng, &mut |_| {
                refused += 1;
                None
            });
            assert_eq!(none, None);
            assert!((1..=BOT_NAME_DRAW_ATTEMPTS).contains(&refused), "{refused}");
        }
    }
}
