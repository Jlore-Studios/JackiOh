//! The lint (#442 L6, L7): what a script may not write, refused before it compiles. The sandbox
//! takes some things away (L5), and the walk refuses a fraction when a hook hands one over (L6); the
//! lint refuses, in the source, what would make a script's answer hang on something else: an order
//! that is the hash's (`pairs`, `next`, `table.foreach`, a generic `for` over anything but
//! `ipairs(…)`, and `ipairs` anywhere else, so no name bound over it can hand a `for` a table, L7),
//! sorting (`table.sort`, L7), the operators and `math` functions that make fractions (L6), and the
//! names the sandbox removed (L5). The sandbox removes the library functions among them too, so a key
//! the lint cannot read finds nothing.
//!
//! A pure function over the source: a small lexer that knows Luau's comments, strings (interpolated
//! ones included) and operators, then a pass over the tokens. A name after `.`, `:` or `::` is a
//! field, a method or a type, never one of the names it refuses.

use std::fmt;

use crate::sandbox::{ABSENT_LIBRARIES, REMOVED_GLOBALS};

/// The `math` functions a script may call: each takes integers to an integer.
pub const MATH_ALLOWED: [&str; 7] = ["floor", "ceil", "max", "min", "abs", "clamp", "sign"];

/// One thing the lint refuses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rule {
    Pairs,
    Next,
    TableSort,
    TableForeach,
    GenericFor,
    /// `ipairs` anywhere but right after a generic `for`'s `in`.
    Ipairs,
    Slash,
    Caret,
    /// A `math` function outside `MATH_ALLOWED`, or `""` for `math` itself used bare.
    Math(String),
    RemovedGlobal(String),
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rule::Pairs => f.write_str("`pairs` is refused: its order is the hash's (L7); use `ipairs`"),
            Rule::Next => f.write_str("`next` is refused: its order is the hash's (L7)"),
            Rule::TableSort => f.write_str("`table.sort` is refused: it is not a stable sort (L7)"),
            Rule::TableForeach => {
                f.write_str("`table.foreach` is refused: its order is the hash's (L7); use `ipairs`")
            }
            Rule::GenericFor => {
                f.write_str("a generic `for` is refused unless it runs over `ipairs(…)` (L7)")
            }
            Rule::Ipairs => f.write_str(
                "`ipairs` is refused but as `for … in ipairs(…)`: a name bound over it could hand the `for` a table, whose order is the hash's (L7)",
            ),
            Rule::Slash => f.write_str("`/` is refused: it makes fractions (L6); use `J.div`"),
            Rule::Caret => f.write_str("`^` is refused: it makes fractions (L6)"),
            Rule::Math(name) if name.is_empty() => f.write_str(
                "`math` is refused but as `math.floor`, `ceil`, `max`, `min`, `abs`, `clamp` or `sign` (L6)",
            ),
            Rule::Math(name) => write!(
                f,
                "`math.{name}` is refused: only `floor`, `ceil`, `max`, `min`, `abs`, `clamp` and `sign` are allowed (L6)"
            ),
            Rule::RemovedGlobal(name) => write!(f, "`{name}` is not in the sandbox (L5)"),
        }
    }
}

/// One refusal: where it is and which rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub file: String,
    pub line: u32,
    pub rule: Rule,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.file, self.line, self.rule)
    }
}

/// Every finding in `source`, in source order. `file` names the source in each.
pub fn lint(file: &str, source: &str) -> Vec<Finding> {
    let tokens = lex(source);
    let mut findings = Vec::new();
    for (at, &(token, line)) in tokens.iter().enumerate() {
        let rule = match token {
            Token::Op("/" | "/=") => Some(Rule::Slash),
            Token::Op("^" | "^=") => Some(Rule::Caret),
            Token::Name(name) if !follows_access(&tokens, at) => {
                name_rule(name, at.checked_sub(1).map(|k| tokens[k].0), &tokens[at + 1..])
            }
            _ => None,
        };
        if let Some(rule) = rule {
            findings.push(Finding {
                file: file.to_string(),
                line,
                rule,
            });
        }
    }
    findings
}

/// Whether the name at `at` follows `.`, `:` or `::`.
fn follows_access(tokens: &[(Token<'_>, u32)], at: usize) -> bool {
    at > 0 && matches!(tokens[at - 1].0, Token::Op("." | ":" | "::"))
}

/// The rule a free name breaks, if any; `before` is the token before it and `rest` every token
/// after it.
fn name_rule(name: &str, before: Option<Token<'_>>, rest: &[(Token<'_>, u32)]) -> Option<Rule> {
    let next = |k: usize| rest.get(k).map(|(token, _)| *token);
    match name {
        "pairs" => Some(Rule::Pairs),
        "next" => Some(Rule::Next),
        "ipairs" if before != Some(Token::Name("in")) => Some(Rule::Ipairs),
        "table" if next(0) == Some(Token::Op(".")) && next(1) == Some(Token::Name("sort")) => {
            Some(Rule::TableSort)
        }
        "table" if next(0) == Some(Token::Op(".")) && next(1) == Some(Token::Name("foreach")) => {
            Some(Rule::TableForeach)
        }
        "math" => match (next(0), next(1)) {
            (Some(Token::Op(".")), Some(Token::Name(member))) if MATH_ALLOWED.contains(&member) => None,
            (Some(Token::Op(".")), Some(Token::Name(member))) => Some(Rule::Math(member.to_string())),
            _ => Some(Rule::Math(String::new())),
        },
        "for" if !for_is_numeric_or_ipairs(rest) => Some(Rule::GenericFor),
        _ if REMOVED_GLOBALS.contains(&name) || ABSENT_LIBRARIES.contains(&name) => {
            Some(Rule::RemovedGlobal(name.to_string()))
        }
        _ => None,
    }
}

/// Whether the `for` whose tokens follow is numeric (`for i = …`) or runs over `ipairs(…)` alone
/// (`for i, x in ipairs(list) do`).
fn for_is_numeric_or_ipairs(rest: &[(Token<'_>, u32)]) -> bool {
    let Some(at) = rest
        .iter()
        .position(|(token, _)| matches!(token, Token::Op("=") | Token::Name("in")))
    else {
        return true;
    };
    if rest[at].0 == Token::Op("=") {
        return true;
    }
    let iterator = &rest[at + 1..];
    if !matches!(iterator, [(Token::Name("ipairs"), _), (Token::Op("("), _), ..]) {
        return false;
    }
    let mut depth = 0;
    for (k, (token, _)) in iterator.iter().enumerate().skip(1) {
        match token {
            Token::Op("(") => depth += 1,
            Token::Op(")") => {
                depth -= 1;
                if depth == 0 {
                    return matches!(iterator.get(k + 1), Some((Token::Name("do"), _)));
                }
            }
            _ => {}
        }
    }
    false
}

/// A token: a name (keywords included), an operator or punctuation, or a literal the lint never reads
/// (a number or a string).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Token<'s> {
    Name(&'s str),
    Op(&'static str),
    Literal,
}

/// Luau's operators and punctuation but the braces (which `run` reads itself), longest first, so
/// `//=` is read before `//`, `/=` and `/`, and `::` before `:`.
const OPS: [&str; 39] = [
    "...", "..=", "//=", "==", "~=", "<=", ">=", "->", "::", "+=", "-=", "*=", "/=", "%=", "^=", "..", "//",
    "+", "-", "*", "/", "%", "^", "#", "&", "|", "<", ">", "=", "(", ")", "[", "]", ";", ":", ",", ".", "?",
    "@",
];

/// The tokens of `source`, each with its 1-based line.
fn lex(source: &str) -> Vec<(Token<'_>, u32)> {
    let mut lexer = Lexer {
        source,
        bytes: source.as_bytes(),
        at: 0,
        line: 1,
        tokens: Vec::new(),
        interpolations: Vec::new(),
    };
    lexer.run();
    lexer.tokens
}

struct Lexer<'s> {
    source: &'s str,
    bytes: &'s [u8],
    at: usize,
    line: u32,
    tokens: Vec<(Token<'s>, u32)>,
    /// One entry per interpolated string whose `{…}` is being read: the depth of the braces opened
    /// inside it, so the `}` that closes the expression is told from one that closes a table.
    interpolations: Vec<u32>,
}

impl<'s> Lexer<'s> {
    fn peek(&self, k: usize) -> Option<u8> {
        self.bytes.get(self.at + k).copied()
    }

    fn push(&mut self, token: Token<'s>, line: u32) {
        self.tokens.push((token, line));
    }

    /// One byte forward, counting a newline.
    fn bump(&mut self) {
        if self.peek(0) == Some(b'\n') {
            self.line += 1;
        }
        self.at += 1;
    }

    fn run(&mut self) {
        while let Some(byte) = self.peek(0) {
            let line = self.line;
            match byte {
                b'-' if self.peek(1) == Some(b'-') => self.comment(),
                b'"' | b'\'' => {
                    self.quoted(byte);
                    self.push(Token::Literal, line);
                }
                b'[' if self.long_bracket().is_some() => {
                    self.long_string();
                    self.push(Token::Literal, line);
                }
                b'`' => {
                    self.bump();
                    self.push(Token::Literal, line);
                    self.interpolated();
                }
                b'0'..=b'9' => {
                    self.number();
                    self.push(Token::Literal, line);
                }
                b'.' if self.peek(1).is_some_and(|b| b.is_ascii_digit()) => {
                    self.number();
                    self.push(Token::Literal, line);
                }
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                    let start = self.at;
                    while self
                        .peek(0)
                        .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
                    {
                        self.at += 1;
                    }
                    self.push(Token::Name(&self.source[start..self.at]), line);
                }
                b'{' => {
                    if let Some(depth) = self.interpolations.last_mut() {
                        *depth += 1;
                    }
                    self.at += 1;
                    self.push(Token::Op("{"), line);
                }
                b'}' => {
                    self.at += 1;
                    match self.interpolations.last_mut() {
                        Some(0) => {
                            self.interpolations.pop();
                            self.interpolated();
                        }
                        Some(depth) => {
                            *depth -= 1;
                            self.push(Token::Op("}"), line);
                        }
                        None => self.push(Token::Op("}"), line),
                    }
                }
                _ => match OPS
                    .iter()
                    .find(|op| self.bytes[self.at..].starts_with(op.as_bytes()))
                {
                    Some(op) => {
                        self.at += op.len();
                        self.push(Token::Op(op), line);
                    }
                    None => self.bump(),
                },
            }
        }
    }

    /// `--` to the end of the line, or a `--[==[ … ]==]` block.
    fn comment(&mut self) {
        self.at += 2;
        if self.peek(0) == Some(b'[') && self.long_bracket().is_some() {
            self.long_string();
            return;
        }
        while self.peek(0).is_some_and(|b| b != b'\n') {
            self.at += 1;
        }
    }

    /// A `'…'` or `"…"` string, escapes included.
    fn quoted(&mut self, quote: u8) {
        self.at += 1;
        while let Some(byte) = self.peek(0) {
            self.bump();
            if byte == b'\\' {
                self.bump();
            } else if byte == quote {
                return;
            }
        }
    }

    /// At a `[`: the level of the long bracket that opens here (`[[` is 0, `[==[` is 2), if one does.
    fn long_bracket(&self) -> Option<usize> {
        let level = self.bytes[self.at + 1..]
            .iter()
            .take_while(|b| **b == b'=')
            .count();
        (self.peek(1 + level) == Some(b'[')).then_some(level)
    }

    /// A `[==[ … ]==]` string or comment body, from its opening bracket to its closing one.
    fn long_string(&mut self) {
        let level = self.long_bracket().unwrap_or(0);
        self.at += level + 2;
        let close: Vec<u8> = std::iter::once(b']')
            .chain(std::iter::repeat_n(b'=', level))
            .chain(std::iter::once(b']'))
            .collect();
        while self.peek(0).is_some() {
            if self.bytes[self.at..].starts_with(&close) {
                self.at += close.len();
                return;
            }
            self.bump();
        }
    }

    /// The text of an interpolated string, up to its closing `` ` `` or the `{` of an expression,
    /// which is read as code until its own `}`.
    fn interpolated(&mut self) {
        while let Some(byte) = self.peek(0) {
            self.bump();
            match byte {
                b'\\' => self.bump(),
                b'`' => return,
                b'{' => {
                    self.interpolations.push(0);
                    return;
                }
                _ => {}
            }
        }
    }

    /// A number, stopping before a `..` that follows it (`1..2` is `1`, `..`, `2`).
    fn number(&mut self) {
        let hex = self.peek(0) == Some(b'0') && matches!(self.peek(1), Some(b'x' | b'X'));
        while let Some(byte) = self.peek(0) {
            if byte.is_ascii_alphanumeric() || byte == b'_' {
                self.at += 1;
                if !hex && matches!(byte, b'e' | b'E') && matches!(self.peek(0), Some(b'+' | b'-')) {
                    self.at += 1;
                }
            } else if byte == b'.' && self.peek(1) != Some(b'.') {
                self.at += 1;
            } else {
                return;
            }
        }
    }
}
