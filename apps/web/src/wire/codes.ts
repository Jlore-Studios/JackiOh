/**
 * Client and server code parsing must agree (SURFACE §10.4; SPEC §11; R191, R104).
 *
 * Format comes from server config (CLAUDE.md rule 9). Refuse oversized input; normalize NFKC,
 * uppercase one code point at a time, remove separators, and stop at the first invalid character.
 * Never map or drop characters: R104 omits look-alikes, and dropping one changes the code.
 */

/** Code shape from server config (CLAUDE.md rule 9). */
export type CodeFormat = {
  readonly alphabet: string;
  readonly length: number;
  readonly groupSize: number;
  readonly separator: string;
  readonly maxInputLength: number;
};

export type CodeInputProblem =
  /** R104: ASCII alphanumerics outside the alphabet. */
  | { readonly kind: "excluded"; readonly character: string }
  | { readonly kind: "foreign"; readonly character: string }
  | { readonly kind: "tooLong" };

export type CodeInputReading = {
  readonly characters: string;
  readonly formatted: string;
  readonly complete: boolean;
  readonly problem: CodeInputProblem | null;
};

/** Code separators, including Unicode dashes and invisible characters introduced by mail or chat. */
const SEPARATOR_CLASS =
  "[\\s\\-\\u2010-\\u2015\\u2212\\uFE58\\uFE63\\uFF0D\\u00AD\\u200B-\\u200D\\u2060\\uFEFF]";
const SEPARATOR = new RegExp(`^${SEPARATOR_CLASS}$`, "u");

const ASCII_LETTER_OR_DIGIT = /^[0-9A-Z]$/u;
const ASCII_LETTERS_AND_DIGITS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

export function isCodeSeparator(character: string): boolean {
  return SEPARATOR.test(character);
}

/** NFKC and one-code-point uppercase; multi-character results stay foreign. */
function upperCodePoints(raw: string): string[] {
  const out: string[] = [];
  for (const character of raw.normalize("NFKC")) {
    const upper = character.toUpperCase();
    out.push(Array.from(upper).length === 1 ? upper : character);
  }
  return out;
}

export function normalizeCodeText(raw: string): string {
  return upperCodePoints(raw)
    .filter((character) => !isCodeSeparator(character))
    .join("");
}

export function formatCodeCharacters(characters: string, format: CodeFormat): string {
  const size = Math.max(1, format.groupSize);
  const groups: string[] = [];
  for (let start = 0; start < characters.length; start += size) {
    groups.push(characters.slice(start, start + size));
  }
  return groups.join(format.separator);
}

function readingOf(
  characters: string,
  problem: CodeInputProblem | null,
  format: CodeFormat,
): CodeInputReading {
  return {
    characters,
    formatted: formatCodeCharacters(characters, format),
    complete: problem === null && characters.length === format.length,
    problem,
  };
}

/** Refuses oversized input before parsing and stops at the first invalid character without guessing. */
export function readCodeInput(raw: string, format: CodeFormat): CodeInputReading {
  if (raw.length > format.maxInputLength) return readingOf("", { kind: "tooLong" }, format);

  let characters = "";
  for (const character of normalizeCodeText(raw)) {
    if (format.alphabet.includes(character)) {
      if (characters.length >= format.length) {
        return readingOf(characters, { kind: "tooLong" }, format);
      }
      characters += character;
      continue;
    }
    const problem: CodeInputProblem = ASCII_LETTER_OR_DIGIT.test(character)
      ? { kind: "excluded", character }
      : { kind: "foreign", character };
    return readingOf(characters, problem, format);
  }
  return readingOf(characters, null, format);
}

export function canonicalCode(raw: string, format: CodeFormat): string | null {
  const reading = readCodeInput(raw, format);
  return reading.complete ? reading.characters : null;
}

export function formattedCaret(characterCount: number, format: CodeFormat): number {
  const n = characterCount;
  return n + (n > 0 ? Math.floor((n - 1) / Math.max(1, format.groupSize)) : 0);
}

/** "sentence" means only the first letter is upper-case. */
type LetterCase = "upper" | "lower" | "sentence" | "mixed" | "none";

type Word = { readonly characters: string; readonly letterCase: LetterCase };

/** Same-case words adjacent through separators only. */
type Chain = { readonly words: Word[]; readonly joins: string[] };

function letterCaseOf(original: string): LetterCase {
  let upper = false;
  let lower = false;
  let sentence = true;
  let letters = 0;
  for (const character of original) {
    const isUpper = character !== character.toLowerCase();
    const isLower = !isUpper && character !== character.toUpperCase();
    if (!isUpper && !isLower) continue;
    if (isUpper) upper = true;
    else lower = true;
    if (letters === 0 ? !isUpper : isUpper) sentence = false;
    letters += 1;
  }
  if (upper && lower) return sentence ? "sentence" : "mixed";
  if (upper) return "upper";
  if (lower) return "lower";
  return "none";
}

/** Strict reading requires equal case; loose reading treats sentence case as lower for phone pastes. */
function sameCase(a: LetterCase, b: LetterCase, loose: boolean): boolean {
  const fold = (letterCase: LetterCase): LetterCase => (loose && letterCase === "sentence" ? "lower" : letterCase);
  const x = fold(a);
  const y = fold(b);
  return x === "none" || y === "none" || x === y;
}

function chainsIn(text: string, loose: boolean): Chain[] {
  const chains: Chain[] = [];
  let words: Word[] = [];
  let joins: string[] = [];
  let original = "";
  let characters = "";
  /** Cast permits `endWord` to update this value. */
  let join = null as string | null;

  const endWord = (): void => {
    if (characters.length === 0) return;
    const word: Word = { characters, letterCase: letterCaseOf(original) };
    const last = words[words.length - 1];
    if (last !== undefined && join !== null && sameCase(last.letterCase, word.letterCase, loose)) {
      joins.push(join);
      words.push(word);
    } else {
      if (words.length > 0) chains.push({ words, joins });
      words = [word];
      joins = [];
    }
    original = "";
    characters = "";
    join = "";
  };

  for (const character of text.normalize("NFKC")) {
    const upper = character.toUpperCase();
    const read = Array.from(upper).length === 1 ? upper : character;
    if (ASCII_LETTER_OR_DIGIT.test(read)) {
      original += character;
      characters += read;
      continue;
    }
    endWord();
    if (isCodeSeparator(character)) {
      if (join !== null) join += character;
    } else {
      join = null;
    }
  }
  endWord();
  if (words.length > 0) chains.push({ words, joins });
  return chains;
}

/** A possible code fragment is one character, a partial group, or whole alphabet groups. */
function couldBelongToCode(word: Word, format: CodeFormat): boolean {
  const length = word.characters.length;
  if (length <= 1) return true;
  for (const character of word.characters) {
    if (!format.alphabet.includes(character)) return false;
  }
  const size = Math.max(1, format.groupSize);
  return length <= size || length % size === 0;
}

/**
 * A whole chain may be one code; embedded candidates need uniform joins and unambiguous neighbours.
 * A joined possible fragment makes the chain too long (R145), never a truncated code.
 */
function codesInChain(chain: Chain, format: CodeFormat): string[] {
  let text = "";
  chain.words.forEach((word, index) => {
    text += (index === 0 ? "" : (chain.joins[index - 1] ?? "")) + word.characters;
  });
  const whole = canonicalCode(text, format);
  if (whole !== null) return [whole];

  const size = Math.max(1, format.groupSize);
  const found: string[] = [];
  const count = chain.words.length;
  for (let start = 0; start < count; start += 1) {
    const internal = chain.joins[start];
    let characters = "";
    for (let end = start; end < count; end += 1) {
      if (end > start) {
        if (chain.joins[end - 1] !== internal || characters.length % size !== 0) break;
      }
      characters += chain.words[end]?.characters ?? "";
      if (characters.length < format.length) continue;
      if (characters.length === format.length && end > start) {
        const leftWord = chain.words[start - 1];
        const rightWord = chain.words[end + 1];
        const leftBlocks =
          leftWord !== undefined && chain.joins[start - 1] === internal && couldBelongToCode(leftWord, format);
        const rightBlocks =
          rightWord !== undefined && chain.joins[end] === internal && couldBelongToCode(rightWord, format);
        const code = !leftBlocks && !rightBlocks ? canonicalCode(characters, format) : null;
        if (code !== null) found.push(code);
      }
      break;
    }
  }
  return found;
}

function codesInText(text: string, format: CodeFormat, loose: boolean): Set<string> {
  const found = new Set<string>();
  for (const chain of chainsIn(text, loose)) {
    for (const code of codesInChain(chain, format)) {
      found.add(code);
      if (found.size > 1) return found;
    }
  }
  return found;
}

/** Finds exactly one code; strict chains win over the sentence-case fallback. */
export function findCodeInText(text: string, format: CodeFormat): string | null {
  const direct = canonicalCode(text, format);
  if (direct !== null) return direct;

  let found = codesInText(text, format, false);
  if (found.size === 0) found = codesInText(text, format, true);
  if (found.size !== 1) return null;
  const [only] = [...found];
  return only ?? null;
}

/** R104: sorted ASCII alphanumerics excluded by the format. */
export function excludedCharacters(format: CodeFormat): readonly string[] {
  return Array.from(ASCII_LETTERS_AND_DIGITS)
    .filter((character) => !format.alphabet.includes(character))
    .sort();
}
