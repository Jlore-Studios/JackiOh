//! R1846000: a card script's code lines, counted alike by `build.rs` (which `#[path]`-includes this
//! file, so the BUSTED and complicated lists it compiles in come from here) and by the tools, which
//! count a version's lists for `patches ship` and a change's for `classify` (R2173000). #3116: and
//! whether a script owes an extra turn, which `build.rs` lists for the AI's hand prices (R846).
//! R3882000: and whether a script reads the win rates of record, which a refresh of them claims. Pure
//! string work, so the cards crate may hold it (CLAUDE.md rule 4).

/// R2173000: what a script's code spells when it reads the BUSTED or complicated lists: a catalog
/// query's `busted` or `complicated` field (in JSON or as a Rust field) or the engine's readers.
const LIST_READS: [&str; 6] = [
    "\"busted\"",
    "\"complicated\"",
    "busted:",
    "complicated:",
    "is_busted(",
    "is_complicated(",
];

/// R2173000: whether a card script reads the BUSTED or complicated lists (R1846000): its code, above
/// its tests and without its comments, spells one of `LIST_READS`. Stinkyscape #109 xxFreshJack and
/// #111 Shockwave447 do.
pub fn reads_card_lists(source: &str) -> bool {
    script_code(source).any(|code| LIST_READS.iter().any(|read| code.contains(read)))
}

/// R3882000: what a script's code spells when it reads the win rates of record (R3538000): a catalog
/// query's `tier` (R3538001), a scope's `winRateBelow` (R3538007), or the engine's readers.
const RECORD_READS: [&str; 5] = [
    "\"tier\"",
    "\"winRateBelow\"",
    "tier_of_record(",
    "win_rate_of_record(",
    "lower_win_rate(",
];

/// R3882000: whether a card script reads the win rates of record: its code, above its tests and
/// without its comments, spells one of `RECORD_READS`. A refresh of the record changes how these
/// cards play, so its fragment claims them (`cargo jackioh winrates --refresh`).
pub fn reads_win_rate_record(source: &str) -> bool {
    script_code(source).any(|code| RECORD_READS.iter().any(|read| code.contains(read)))
}

/// #3116, R846: the engine's verb that owes a player an extra turn (`effects::take_extra_turn`), the
/// only one that does.
const EXTRA_TURN_VERB: &str = "take_extra_turn";

/// #3116, R846: whether a card script owes an extra turn: its code, above its tests and without its
/// comments, names `take_extra_turn` as a whole word (a call, or the `use` bringing it in), in any
/// hook: a cry (M #19.1 Temporal Rift, S #4.3 Event Horizon) or a Trap's resumed reveal step (S #4.4
/// Last Word). A turn owed through another file's helper or another card's cast is not seen.
pub fn owes_extra_turn(source: &str) -> bool {
    script_code(source).any(|code| names_word(&code, EXTRA_TURN_VERB))
}

/// Whether `code` holds `word` with no identifier character on either side.
fn names_word(code: &str, word: &str) -> bool {
    code.match_indices(word).any(|(at, _)| {
        !code[..at].chars().next_back().is_some_and(is_ident_char)
            && !code[at + word.len()..].chars().next().is_some_and(is_ident_char)
    })
}

/// A character a Rust identifier may hold.
fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The lines of a script's code: above its `#[cfg(test)]` (its tests are not its code), without
/// comments, each trimmed, blank ones left out.
fn script_code(source: &str) -> impl Iterator<Item = String> {
    without_comments(source)
        .into_iter()
        .map(|code| code.trim().to_string())
        .take_while(|code| !code.starts_with("#[cfg(test)]"))
        .filter(|code| !code.is_empty())
}

/// R1846000: a card script's code lines: the lines above its `#[cfg(test)]` (its tests are not its
/// code) that hold code once its comments are taken out (`without_comments`), so neither a comment
/// nor a blank line counts and every other line does, a `use` declaration's included.
pub fn code_lines(source: &str) -> usize {
    script_code(source).count()
}

/// Each line of `source` with its comments taken out (`crates/tools`' `count_loc` scanner): `//` to
/// the line's end and nested `/* */` blocks, while a string (plain or raw) or a character literal is
/// read as such, so a `//` inside one is code.
pub fn without_comments(source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut block_depth = 0usize;
    // `Some(0)` inside a plain string, `Some(n + 1)` inside a raw string of `n` hashes.
    let mut quote: Option<usize> = None;
    for line in source.split('\n') {
        let chars: Vec<char> = line.chars().collect();
        let mut code = String::new();
        let mut i = 0;
        while i < chars.len() {
            let ch = chars[i];
            let next = chars.get(i + 1).copied();
            if block_depth > 0 {
                if ch == '*' && next == Some('/') {
                    block_depth -= 1;
                    i += 2;
                } else if ch == '/' && next == Some('*') {
                    block_depth += 1;
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            match quote {
                Some(0) => {
                    code.push(ch);
                    if ch == '\\' {
                        if let Some(next) = next {
                            code.push(next);
                        }
                        i += 2;
                        continue;
                    }
                    if ch == '"' {
                        quote = None;
                    }
                    i += 1;
                    continue;
                }
                Some(raw) => {
                    let hashes = raw - 1;
                    code.push(ch);
                    let closes = ch == '"'
                        && chars.len() > i + hashes
                        && chars[i + 1..=i + hashes].iter().all(|c| *c == '#');
                    if closes {
                        code.extend(std::iter::repeat_n('#', hashes));
                        quote = None;
                        i += 1 + hashes;
                    } else {
                        i += 1;
                    }
                    continue;
                }
                None => {}
            }
            if ch == '/' && next == Some('/') {
                break;
            }
            if ch == '/' && next == Some('*') {
                block_depth += 1;
                i += 2;
                continue;
            }
            if ch == '"' {
                quote = Some(0);
                code.push(ch);
                i += 1;
                continue;
            }
            // A raw string, `r"…"` or `r#"…"#`; a `br` or `cr` one's `b` or `c` is read as code
            // first. Since Rust 2021 a name run into a quote or a `#` is reserved, so in code that
            // compiles no `r` ending a name meets one (#3836).
            if ch == 'r' {
                let hashes = chars[i + 1..].iter().take_while(|c| **c == '#').count();
                if chars.get(i + 1 + hashes) == Some(&'"') {
                    code.extend(&chars[i..=i + 1 + hashes]);
                    quote = Some(hashes + 1);
                    i += 2 + hashes;
                    continue;
                }
            }
            if ch == '\'' {
                // A character literal (`'x'`, `'\n'`) is one token; a lifetime is a quote and a name.
                // An escaped one closes after its escaped character, which may be a quote (`'\''`).
                let length = if next == Some('\\') {
                    chars
                        .get(i + 3..)
                        .and_then(|rest| rest.iter().position(|c| *c == '\''))
                        .map_or(1, |at| at + 4)
                } else if chars.get(i + 2) == Some(&'\'') {
                    3
                } else {
                    1
                };
                code.extend(&chars[i..(i + length).min(chars.len())]);
                i += length;
                continue;
            }
            code.push(ch);
            i += 1;
        }
        out.push(code);
    }
    out
}
