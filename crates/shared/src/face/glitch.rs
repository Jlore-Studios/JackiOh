//! Glitch (issue #170, SPEC §7): the blank token whose every word is corrupted. Presentation only
//! (CLAUDE.md rule 7): a card the viewer may read whose id is `GLITCH_DEF_ID` wears these words instead of
//! the catalog's, and a face-down card stays a back, since its id is the hidden sentinel.
//!
//! The corruption is deterministic (a fixed seed and a small LCG, Numerical Recipes' constants), so every
//! client prints the same jumble. A string's seed folds its own characters into `GLITCH_SEED`, so the name
//! and the text jumble differently.

use jackioh_engine::wire::CardDef;

use crate::config::{GLITCH_MARK_IN, GLITCH_ODDS_SCALE, GLITCH_SEED, GLITCH_SWAP_IN};

/// The words a Glitch would have, before corruption.
const NAME: &str = "Glitch";
const TYPE: &str = "Spell";
const TEXT: &str = "When played, the match is no longer the match. Nothing here is what it was.";
const COST: &str = "0?";

/// The glyphs a corrupted character becomes: block, box and symbol characters, never a brace (B3.4's `{key}`).
const GLYPHS: &str = "\u{2588}\u{2593}\u{2592}\u{2591}\u{259a}\u{259e}\u{2599}\u{259f}\u{25e2}\u{25e3}\u{25e4}\u{25e5}\u{bf}\u{a1}\u{a7}\u{a4}\u{2206}\u{2211}\u{2260}\u{2248}#%&@*?!/\\|<>~^\u{d8}\u{c6}\u{de}\u{df}\u{f0}\u{b5}\u{b6}";

/// Combining marks that stack on a kept character (the "zalgo" look), one at most per character.
const MARKS: &str =
    "\u{336}\u{334}\u{338}\u{300}\u{301}\u{315}\u{31b}\u{340}\u{341}\u{358}\u{321}\u{322}\u{327}\u{328}";

/// The LCG: state' = (state × MULTIPLIER + INCREMENT) mod 2^32.
const LCG_MULTIPLIER: u32 = 1_664_525;
const LCG_INCREMENT: u32 = 1_013_904_223;
/// A character code is folded into the seed by this odd factor (the FNV prime).
const FOLD_PRIME: u32 = 16_777_619;

/// `source` corrupted from `seed`: the same pair always gives the same string.
fn corrupt(source: &str, seed: u32, swap_in: u32) -> String {
    let mut state = seed;
    for c in source.chars() {
        state = (state ^ c as u32).wrapping_mul(FOLD_PRIME);
    }
    let mut next = |bound: u32| {
        state = state.wrapping_mul(LCG_MULTIPLIER).wrapping_add(LCG_INCREMENT);
        state % bound
    };
    let glyphs: Vec<char> = GLYPHS.chars().collect();
    let marks: Vec<char> = MARKS.chars().collect();
    let mut out = String::new();
    for c in source.chars() {
        let roll = next(GLITCH_ODDS_SCALE);
        if c == ' ' {
            out.push(if roll < swap_in {
                glyphs[next(glyphs.len() as u32) as usize]
            } else {
                c
            });
        } else if roll < swap_in {
            out.push(glyphs[next(glyphs.len() as u32) as usize]);
        } else if roll < swap_in + GLITCH_MARK_IN {
            out.push(c);
            out.push(marks[next(marks.len() as u32) as usize]);
        } else {
            out.push(c);
        }
    }
    out
}

/// Everything a Glitch prints, corrupted once.
pub struct GlitchWords {
    pub name: String,
    pub type_: String,
    pub text: String,
    /// The gem never shows a readable number: every character of it is a glyph.
    pub cost: String,
}

pub fn glitch_words() -> GlitchWords {
    GlitchWords {
        name: corrupt(NAME, GLITCH_SEED, GLITCH_SWAP_IN),
        type_: corrupt(TYPE, GLITCH_SEED, GLITCH_SWAP_IN),
        text: corrupt(TEXT, GLITCH_SEED, GLITCH_SWAP_IN),
        cost: corrupt(COST, GLITCH_SEED, GLITCH_ODDS_SCALE),
    }
}

/// Glitch's definition with its name and both faces' text corrupted.
pub fn glitch_def(def: &CardDef) -> CardDef {
    let words = glitch_words();
    let mut out = def.clone();
    out.name = words.name;
    out.base.text = words.text.clone();
    out.radiant.text = words.text;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_words_always_jumble_the_same_way_and_never_show_a_readable_cost() {
        let once = glitch_words();
        let again = glitch_words();
        assert_eq!(once.name, again.name);
        assert_ne!(once.text, TEXT);
        assert!(once.cost.chars().all(|c| GLYPHS.contains(c)), "{}", once.cost);
    }

    /// Issue #170: every client prints the same jumble, so it is pinned, each of Glitch's words folded into
    /// `GLITCH_SEED` and rolled through the LCG, the cost's every character a glyph.
    #[test]
    fn glitchs_words_jumble_exactly_so() {
        let words = glitch_words();
        assert_eq!(words.name, "\u{d8}l\u{322}i\u{334}\u{2592}ch\u{321}");
        assert_eq!(words.type_, "S\u{341}\u{25e5}\u{2211}^l");
        assert_eq!(
            words.text,
            "\u{2599}\u{c6}en p\u{315}\\a\u{300}\u{b5}e\u{328}%,\u{328} >h\u{259f}\u{bf}m\u{322}a\u{322}~ch\u{301} \
             i\u{358}s n\u{2206}!lo\u{300}#g?r\u{31b} t\u{300}h~\u{a7}\u{f0}a\u{2248}^h\u{334}.\u{334}\u{a1}\u{259a}o\
             \u{b5}\u{25e2}\u{de}^<%\u{2206}!r\u{327}e\u{338}\u{a7}\u{259f}\u{25e5} w\u{358}\u{f0}a\u{259a} #\u{b5} \
             ^as\u{340}~"
        );
        assert_eq!(words.cost, "\u{2248}\u{de}");
    }

    /// A character becomes a glyph on a roll below `GLITCH_SWAP_IN` (of `GLITCH_ODDS_SCALE`) and gains a mark on
    /// one below `GLITCH_SWAP_IN + GLITCH_MARK_IN`, never on a roll at either; a space becomes a glyph or stays.
    #[test]
    fn a_roll_at_an_odds_edge_falls_past_it() {
        // A one-character source's first roll: the character folded into the seed, then one step of the LCG.
        let roll = |source: char, seed: u32| {
            (seed ^ source as u32)
                .wrapping_mul(FOLD_PRIME)
                .wrapping_mul(LCG_MULTIPLIER)
                .wrapping_add(LCG_INCREMENT)
                % GLITCH_ODDS_SCALE
        };
        let rolling =
            |source: char, at: u32| (0_u32..).find(|seed| roll(source, *seed) == at).expect("a seed");
        let kept_and_marked = |out: &str| out.starts_with('a') && out.chars().count() == 2;
        let mark_in = GLITCH_SWAP_IN + GLITCH_MARK_IN;
        assert_eq!(corrupt(" ", rolling(' ', GLITCH_SWAP_IN), GLITCH_SWAP_IN), " ");
        assert_ne!(
            corrupt(" ", rolling(' ', GLITCH_SWAP_IN - 1), GLITCH_SWAP_IN),
            " "
        );
        let at_swap = corrupt("a", rolling('a', GLITCH_SWAP_IN), GLITCH_SWAP_IN);
        assert!(kept_and_marked(&at_swap), "{at_swap}");
        assert_eq!(corrupt("a", rolling('a', mark_in), GLITCH_SWAP_IN), "a");
        let below_mark = corrupt("a", rolling('a', mark_in - 1), GLITCH_SWAP_IN);
        assert!(kept_and_marked(&below_mark), "{below_mark}");
    }
}
