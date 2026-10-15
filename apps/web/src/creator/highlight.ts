// The code view's syntax highlighting (#920): a card-kit script cut into tokens the view colours.
// It reads Luau's lexical shape only (comments, strings, numbers, keywords, names) and the kit's own
// marks (`c:verb`, `n.key`, `Card`), never what a script means: the Luau module's check judges it.

export type TokenKind = "comment" | "string" | "number" | "keyword" | "kit" | "param" | "name" | "punct" | "space";

export type Token = { kind: TokenKind; text: string };

const KEYWORDS = new Set([
  "and",
  "break",
  "continue",
  "do",
  "else",
  "elseif",
  "end",
  "false",
  "for",
  "function",
  "if",
  "in",
  "local",
  "nil",
  "not",
  "or",
  "repeat",
  "return",
  "then",
  "true",
  "until",
  "while",
]);

/** The patterns, tried in order at each position; the first to match there wins. */
const RULES: readonly (readonly [TokenKind, RegExp])[] = [
  ["comment", /--\[(=*)\[[\s\S]*?(?:\]\1\]|$)/y],
  ["comment", /--[^\n]*/y],
  ["string", /\[(=*)\[[\s\S]*?(?:\]\1\]|$)/y],
  ["string", /"(?:[^"\\\n]|\\.)*"?/y],
  ["string", /'(?:[^'\\\n]|\\.)*'?/y],
  ["string", /`(?:[^`\\]|\\.)*`?/y],
  ["number", /0[xX][0-9a-fA-F_]+|0[bB][01_]+|(?:\d[\d_]*(?:\.[\d_]*)?|\.\d[\d_]*)(?:[eE][+-]?\d+)?/y],
  ["kit", /:[A-Za-z_]\w*/y],
  ["param", /\bn\.[A-Za-z_]\w*/y],
  ["name", /[A-Za-z_]\w*/y],
  ["space", /\s+/y],
];

/** The script as tokens, whose texts join back to it exactly. */
export function tokenize(source: string): Token[] {
  const tokens: Token[] = [];
  let at = 0;
  const push = (kind: TokenKind, text: string): void => {
    const last = tokens.at(-1);
    if (last !== undefined && last.kind === kind && (kind === "punct" || kind === "space")) last.text += text;
    else tokens.push({ kind, text });
  };
  while (at < source.length) {
    let matched = false;
    for (const [kind, pattern] of RULES) {
      pattern.lastIndex = at;
      const found = pattern.exec(source);
      if (found === null || found[0].length === 0) continue;
      const text = found[0];
      if (kind === "kit") {
        // `c:damage`: the colon is punctuation and the verb the kit's; a `::` type cast is neither.
        push("punct", ":");
        push("kit", text.slice(1));
      } else if (kind === "name") {
        push(KEYWORDS.has(text) ? "keyword" : text === "Card" ? "kit" : "name", text);
      } else {
        push(kind, text);
      }
      at += text.length;
      matched = true;
      break;
    }
    if (!matched) {
      push("punct", source.charAt(at));
      at += 1;
    }
  }
  return tokens;
}

/** The tokens line by line (a token across a line break is cut there), for a view that numbers its lines. */
export function tokenLines(source: string): Token[][] {
  const lines: Token[][] = [[]];
  for (const token of tokenize(source)) {
    const parts = token.text.split("\n");
    parts.forEach((part, index) => {
      if (index > 0) lines.push([]);
      if (part.length > 0) lines.at(-1)?.push({ kind: token.kind, text: part });
    });
  }
  return lines;
}
