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

/// R1454: the plain words of a `QuietOtter` username.
const ADJECTIVES: &str = include_str!("adjectives.txt");
/// R1454: the animals, food and objects of a `QuietOtter` username.
const NOUNS: &str = include_str!("nouns.txt");
/// R1454: nonsense words and keyboard mashes.
const NONSENSE: &str = include_str!("nonsense.txt");
/// R1454: gamer filler.
const GAMER: &str = include_str!("gamer.txt");
/// R1454: edgy words.
const EDGY: &str = include_str!("edgy.txt");
/// R1454: crude and meme-bait words.
const CRUDE: &str = include_str!("crude.txt");
/// R1454: anime characters and vocabulary.
const ANIME: &str = include_str!("anime.txt");
/// R1454: game and pop-culture characters and memes.
const REFS: &str = include_str!("refs.txt");
/// R1454: politics and public-life memes, nicknames and slogans.
const POLITICS: &str = include_str!("politics.txt");
/// R1454: given names, weighted toward Spanish-speaking names (never Luis).
const GIVEN: &str = include_str!("given.txt");
/// R1454: Han-character nicknames and gamer slang.
const CHINESE_HAN: &str = include_str!("chinese_han.txt");
/// R1454: pinyin surnames.
const CHINESE_SURNAMES: &str = include_str!("chinese_surnames.txt");
/// R1454: pinyin given names and slang.
const CHINESE_PINYIN: &str = include_str!("chinese_pinyin.txt");
/// R1454: the bots' own exclusion list, on top of the shared filter.
const NEVER: &str = include_str!("never.txt");
/// R1454: the most common given name on the ladder, drawn by `BOT_NAME_LUIS_SHARE`.
const LUIS: &str = "Luis";

/// R1454: the onsets of a nonsense syllable (no `n`, `j`, `g` or `k`: slur soundalikes).
const ONSETS: &[&str] = &[
    "b", "bl", "br", "d", "dr", "f", "fl", "gl", "gr", "kr", "l", "m", "p", "pl", "qu", "r", "s", "sk",
    "skr", "sn", "sp", "t", "tr", "v", "w", "z",
];
/// R1454: the vowels of a nonsense syllable.
const VOWELS: &[&str] = &["a", "e", "i", "o", "u", "oo", "ee", "ai", "ou"];
/// R1454: the codas of a nonsense draw, two of them empty.
const CODAS: &[&str] = &[
    "", "", "rp", "nk", "x", "ng", "b", "dle", "mp", "zz", "rb", "rf", "m",
];
/// R1454: the initials of a given name's `{name}{initial}` shape.
const INITIALS: &[&str] = &[
    "A", "B", "C", "D", "E", "F", "G", "H", "J", "L", "M", "N", "P", "R", "S", "T", "V", "Z",
];
/// R1454: the endings of a `Doubled` shape past the word's own last letter.
const ENDINGS: &[&str] = &["ii", "zz", "_"];
/// R1454: the affixes of a `Prefix` shape.
const PREFIXES: &[&str] = &["The", "El", "Its", "Mr", "Lil"];
/// R1454: the affixes of a `Suffix` shape.
const SUFFIXES: &[&str] = &["TV", "YT", "Gaming", "Plays", "HD", "Live"];
/// R1454: the fan affixes of a `FanSuffix` shape.
const FAN_SUFFIXES: &[&str] = &["Main", "Fan", "Stan", "Kun", "4Life", "Enjoyer"];
/// R1454: the fan affixes of a `FanPrefix` shape.
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

/// R1454: the registers a named bot draws from, each with its share of named bots. Luis's entry
/// is its share of all bots (`BOT_NAME_LUIS_SHARE`) over the share of bots that draw a name.
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

/// R1454: one list's words: trimmed lines, blanks and `#` lines skipped.
fn entries(list: &'static str) -> Vec<&'static str> {
    list.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// R1454: one weighted draw from `table`: `draw` holds a point of the weights' total, and the
/// first item the point falls in wins; the last wins when the point falls past every weight.
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

/// R1454: the word with its first character uppercased.
fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// R1454: a birth year in `BOT_NAME_BIRTH_YEAR_MIN..=BOT_NAME_BIRTH_YEAR_MAX`.
fn year(rng: &mut Rng) -> i64 {
    int_in(rng, BOT_NAME_BIRTH_YEAR_MIN, BOT_NAME_BIRTH_YEAR_MAX)
}

/// R1454: a drawn number: a birth year, a filler number, or a two-digit number (the
/// Chinese-internet set for the Chinese registers).
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

/// R1454: the word in leetspeak (`a→4`, `e→3`, `o→0`, `i→1`), either case.
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

/// R1454: the word lowercased with its last vowel (else its last character) stretched by a drawn
/// number of extra repeats.
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

/// R1454: a given name's diminutive: a trailing `a` reads `ita`, a stripped `o` reads `ito`,
/// and anything else takes `ito`.
fn diminutive(word: &str) -> String {
    if let Some(stripped) = word.strip_suffix('a').or_else(|| word.strip_suffix('A')) {
        format!("{stripped}ita")
    } else if let Some(stripped) = word.strip_suffix('o').or_else(|| word.strip_suffix('O')) {
        format!("{stripped}ito")
    } else {
        format!("{word}ito")
    }
}

/// R1454: nonsense syllables: a drawn number of onset-vowel pairs, then one coda.
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

/// R1454: one name in `shape`, drawn from `register`'s lists: always a `lead` word and a `word`,
/// then the shape's rendering. None when a draw the shape needs finds nothing.
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

/// R1454: whether the bots' own `never.txt` refuses `name`: the `=`-prefixed entries as whole
/// tokens, the rest anywhere, both matched as R1433 matches.
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

/// R1454: a new bot's username: a register by its share, then up to `BOT_NAME_DRAW_ATTEMPTS`
/// names in its shapes, each shape drawn again, until one passes `normalize_username` and
/// `never.txt`. Its stored form, or none, so the bot keeps its `Player#n`.
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
