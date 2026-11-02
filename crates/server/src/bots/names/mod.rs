//! A new ladder bot's username, drawn as people pick theirs (R1454): a register by its
//! share (`BOT_NAME_SHARE_*`, `BOT_NAME_LUIS_SHARE`), then shapes by the register's weights
//! (`BOT_NAME_SHAPES_*`), filled from the word lists beside this module. A name the shared
//! filter or the bots' own `never.txt` refuses is redrawn, up to `BOT_NAME_DRAW_ATTEMPTS` times,
//! before the bot keeps its `Player#n`.
//!
//! A politics word is never drawn beside an edgy or crude one: each register draws only from
//! its own lists, and the politics register's shapes never meet another register's. No shape
//! writes a `Real`, `Official` or `The_Real` prefix, so no bot poses as a person (R1454).

use jackioh_engine::Rng;

use crate::bots::pool::int_in;
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
use crate::username::{matches_lists, normalize_username};

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

/// R1454: matches `name` against `never.txt` tokens (R1433).
fn never(name: &str) -> bool {
    let mut anywhere: Vec<&str> = Vec::new();
    let mut token: Vec<&str> = Vec::new();
    for word in entries(NEVER) {
        if let Some(stripped) = word.strip_prefix('=') {
            token.push(stripped);
        } else {
            anywhere.push(word);
        }
    }
    matches_lists(name, &anywhere, &token)
}

/// R1454: draws a normalized username passing `never.txt`, or `None` to keep `Player#n`.
pub fn draw_username(rng: &mut Rng) -> Option<String> {
    let register = weighted(rng, &REGISTERS)?;
    for _ in 0..BOT_NAME_DRAW_ATTEMPTS {
        let shape = weighted(rng, register.shapes())?;
        let Some(name) = render(rng, register, shape) else {
            continue;
        };
        match normalize_username(&name) {
            Ok(base) if !never(&base) => return Some(base),
            _ => {}
        }
    }
    None
}
