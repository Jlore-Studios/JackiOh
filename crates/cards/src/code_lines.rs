//! R1846000: a card script's code lines, counted alike by `build.rs` (which `#[path]`-includes this
//! file, so the BUSTED and complicated lists it compiles in come from here) and by the tools, which
//! count a version's lists for `patches ship` and a change's for `classify` (R2173000). Pure string
//! work, so the cards crate may hold it (CLAUDE.md rule 4).

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
            let raw_start = (ch == 'r' || (ch == 'b' && next == Some('r')))
                && !(i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_'));
            if raw_start {
                let after_r = if ch == 'b' { i + 2 } else { i + 1 };
                let hashes = chars[after_r.min(chars.len())..]
                    .iter()
                    .take_while(|c| **c == '#')
                    .count();
                if chars.get(after_r + hashes) == Some(&'"') {
                    code.extend(&chars[i..=after_r + hashes]);
                    quote = Some(hashes + 1);
                    i = after_r + hashes + 1;
                    continue;
                }
            }
            if ch == '\'' {
                // A character literal (`'x'`, `'\n'`) is one token; a lifetime is a quote and a name.
                let length = if next == Some('\\') {
                    chars[i + 2..]
                        .iter()
                        .position(|c| *c == '\'')
                        .map_or(1, |at| at + 3)
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
