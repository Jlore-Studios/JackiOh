// What a Radiant face prints that its base face does not (SPEC §10.10, R277).
//
// The catalog carries each Radiant face's whole text (`radiant.text`, §8's Radiant cell written out);
// the face prints it and marks the words and numbers the base text lacks (#44 True Strike marks its 9
// alone). The mark is a word-level LCS diff of the two public catalog texts, so it is the same wherever
// the face is drawn, and presentation only (CLAUDE.md rule 7). Case is ignored (#17's "Bounce" becoming
// "bounce" marks nothing). The separators `, ; : . ( )` are tokens of their own, and a marked range
// never starts or ends on one, so the underline reads as one phrase.
//
// A Chinese face (ME-CN, R1302, MD-B13) is compared character by character: Chinese puts no space
// between words, and its punctuation `，；：。（）、` is a separator. The same diff (`wordDiff`) marks
// what a patch changed (R388, patches/diff.ts), striking through the words the older text loses.

/** A stretch of a printed text, by UTF-16 offsets: `start` inclusive, `end` exclusive. */
export type TextRange = { readonly start: number; readonly end: number };

/**
 * Two texts' word diff (`wordDiff`): the stretches the newer text adds, by its own offsets, and the
 * stretches the older one loses, by the older one's offsets.
 */
export type WordDiff = { readonly added: TextRange[]; readonly removed: TextRange[] };

type Token = { text: string; start: number; end: number; word: boolean };

const SEPARATORS = ",;:.()，；：。（）、";
/**
 * R1302: the CJK ranges whose every character is a token of its own: CJK punctuation, Extension A,
 * the Unified Ideographs, and the full-width forms.
 */
const CJK = "\\u3000-\\u303F\\u3400-\\u4DBF\\u4E00-\\u9FFF\\uFF00-\\uFFEF";
/** A separator alone, a CJK character alone, or a run of anything that is none of those nor a space. */
const TOKEN = new RegExp(`[${SEPARATORS}]|[${CJK}]|[^\\s${SEPARATORS}${CJK}]+`, "g");
/** R102: a fused definition's text is its ingredients' texts, one per line. */
const LINE_BREAK = "\n";

function tokensOf(text: string, offset: number): Token[] {
  const tokens: Token[] = [];
  for (const match of text.matchAll(TOKEN)) {
    const start = offset + (match.index ?? 0);
    tokens.push({ text: match[0], start, end: start + match[0].length, word: !SEPARATORS.includes(match[0]) });
  }
  return tokens;
}

/**
 * Which tokens of each text are in one longest common subsequence of the two, case aside: `after`
 * for `after`'s tokens (R277's marks are the rest of them) and `before` for `before`'s (R388's
 * struck-through words are the rest of those). One alignment, so the two sides always agree.
 */
function commonTokens(before: readonly Token[], after: readonly Token[]): { before: boolean[]; after: boolean[] } {
  const a = before.map((token) => token.text.toLowerCase());
  const b = after.map((token) => token.text.toLowerCase());
  const rows = a.length + 1;
  const cols = b.length + 1;
  // lcs[i][j]: the LCS length of a[i..] and b[j..], filled from the end.
  const lcs: number[][] = Array.from({ length: rows }, () => new Array<number>(cols).fill(0));
  for (let i = a.length - 1; i >= 0; i -= 1) {
    const row = lcs[i] ?? [];
    const next = lcs[i + 1] ?? [];
    for (let j = b.length - 1; j >= 0; j -= 1) {
      row[j] = a[i] === b[j] ? (next[j + 1] ?? 0) + 1 : Math.max(next[j] ?? 0, row[j + 1] ?? 0);
    }
  }
  const keptBefore = new Array<boolean>(a.length).fill(false);
  const keptAfter = new Array<boolean>(b.length).fill(false);
  let i = 0;
  let j = 0;
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      keptBefore[i] = true;
      keptAfter[j] = true;
      i += 1;
      j += 1;
    } else if ((lcs[i + 1]?.[j] ?? 0) >= (lcs[i]?.[j + 1] ?? 0)) {
      i += 1;
    } else {
      j += 1;
    }
  }
  return { before: keptBefore, after: keptAfter };
}

/**
 * The stretches of `tokens` the alignment did not keep: from the first unkept word to the last of a
 * run, over the spaces and separators between them, never starting or ending on a separator.
 */
function stretches(tokens: readonly Token[], kept: readonly boolean[]): TextRange[] {
  const ranges: TextRange[] = [];
  let open: { start: number; end: number } | null = null;
  for (let at = 0; at < tokens.length; at += 1) {
    const token = tokens[at];
    if (token === undefined || !token.word) continue;
    // A separator, changed or kept, waits to see what follows it; a kept word closes the stretch.
    if (kept[at] === true) {
      if (open !== null) ranges.push(open);
      open = null;
    } else if (open === null) {
      open = { start: token.start, end: token.end };
    } else {
      open.end = token.end;
    }
  }
  if (open !== null) ranges.push(open);
  return ranges;
}

/** One line of each text against the other: what `after` adds and what `before` loses. */
function lineDiff(before: string, after: string, beforeOffset: number, afterOffset: number): WordDiff {
  const beforeTokens = tokensOf(before, beforeOffset);
  const afterTokens = tokensOf(after, afterOffset);
  const kept = commonTokens(beforeTokens, afterTokens);
  return { added: stretches(afterTokens, kept.after), removed: stretches(beforeTokens, kept.before) };
}

/**
 * A word-level diff of two texts (WordDiff), by R277's rules, each side's stretches in order and never
 * overlapping. When both texts have the same number of lines, and more than one, each line is diffed
 * against the line of the same place (a fused text's ingredients, R102).
 */
export function wordDiff(before: string, after: string): WordDiff {
  const beforeLines = before.split(LINE_BREAK);
  const afterLines = after.split(LINE_BREAK);
  if (afterLines.length > 1 && afterLines.length === beforeLines.length) {
    const added: TextRange[] = [];
    const removed: TextRange[] = [];
    let beforeOffset = 0;
    let afterOffset = 0;
    afterLines.forEach((line, at) => {
      const beforeLine = beforeLines[at] ?? "";
      const diff = lineDiff(beforeLine, line, beforeOffset, afterOffset);
      added.push(...diff.added);
      removed.push(...diff.removed);
      beforeOffset += beforeLine.length + LINE_BREAK.length;
      afterOffset += line.length + LINE_BREAK.length;
    });
    return { added, removed };
  }
  return lineDiff(before, after, 0, 0);
}

/** R277: the stretches of `radiant` that `base` does not have. */
export function radiantMarks(base: string, radiant: string): TextRange[] {
  return wordDiff(base, radiant).added;
}

/** The marked words themselves, for tests and accessible summaries. */
export function markedText(text: string, marks: readonly TextRange[]): string[] {
  return marks.map((range) => text.slice(range.start, range.end));
}
