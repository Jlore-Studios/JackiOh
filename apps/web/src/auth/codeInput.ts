// The invite code's words while a code is typed. R191: client and server read a code through one
// shared `readCodeInput` (`@jackioh/shared`), so the field and the server never disagree about a
// code; this file only turns a reading into sentences. The format is config (CLAUDE.md rule 9).
// R104 leaves out both halves of each look-alike pair (0 and O, 1 and I): a typed `0` is refused by name.

import { excludedCharacters, type CodeFormat, type CodeInputProblem, type CodeInputReading } from "@jackioh/shared";

import { INVITE_CODE_FORMAT } from "@jackioh/server-config";
import { t } from "../i18n/i18n.ts";

export { INVITE_CODE_FORMAT } from "@jackioh/server-config";

/** §9.4's groups: `length / groupSize`, four for an invite code. */
export const INVITE_CODE_GROUPS = groupCount(INVITE_CODE_FORMAT);

/** `XXXX-XXXX-XXXX-XXXX`, built from the format so no literal spells it. */
export const INVITE_CODE_PLACEHOLDER = placeholderFor(INVITE_CODE_FORMAT);

/** How many groups a code of this format is drawn as. */
export function groupCount(format: CodeFormat): number {
  return Math.ceil(format.length / format.groupSize);
}

/** One `X` per character, grouped and separated the way a formatted code is. */
export function placeholderFor(format: CodeFormat): string {
  const groups: string[] = [];
  for (let at = 0; at < format.length; at += format.groupSize) {
    groups.push("X".repeat(Math.min(format.groupSize, format.length - at)));
  }
  return groups.join(format.separator);
}

/** "a, b, c or d" (or "… and d"): a list as a sentence names it. */
function listInWords(items: readonly string[], last: "or" | "and" = "or"): string {
  if (items.length === 0) return "";
  if (items.length === 1) return items[0] ?? "";
  return `${items.slice(0, -1).join(t(", "))} ${last === "or" ? t("or") : t("and")} ${items[items.length - 1] ?? ""}`;
}

/** A character a reader cannot see (a control, format, separator or combining mark) as U+XXXX. */
function visibleCharacter(character: string): string {
  if (/^[\p{C}\p{Z}\p{M}]+$/u.test(character)) {
    const point = character.codePointAt(0) ?? 0;
    return `U+${point.toString(16).toUpperCase().padStart(4, "0")}`;
  }
  return character;
}

/** The alphabet's characters matching `pattern`, lowest and highest: "2" and "9" for R104's digits. */
function rangeOf(format: CodeFormat, pattern: RegExp): { lo: string; hi: string; missing: string[] } | null {
  const inAlphabet = [...format.alphabet].filter((character) => pattern.test(character)).sort();
  const lo = inAlphabet[0];
  const hi = inAlphabet[inAlphabet.length - 1];
  if (lo === undefined || hi === undefined) return null;
  // What the range leaves out between its ends: R104's I and O among the letters.
  const missing: string[] = [];
  for (let point = lo.charCodeAt(0); point <= hi.charCodeAt(0); point += 1) {
    const character = String.fromCharCode(point);
    if (!format.alphabet.includes(character)) missing.push(character);
  }
  return { lo, hi, missing };
}

/** "the letters A–Z except I and O, and the digits 2–9", built from the alphabet. */
function alphabetInWords(format: CodeFormat): string {
  const parts: string[] = [];
  const letters = rangeOf(format, /^[A-Z]$/);
  if (letters !== null) {
    const { lo, hi } = letters;
    parts.push(
      letters.missing.length === 0
        ? t("the letters {lo}–{hi}", { lo, hi })
        : t("the letters {lo}–{hi} except {missing}", { lo, hi, missing: listInWords(letters.missing, "and") }),
    );
  }
  const digits = rangeOf(format, /^[0-9]$/);
  if (digits !== null) {
    const { lo, hi } = digits;
    parts.push(
      digits.missing.length === 0
        ? t("the digits {lo}–{hi}", { lo, hi })
        : t("the digits {lo}–{hi} except {missing}", { lo, hi, missing: listInWords(digits.missing, "and") }),
    );
  }
  return parts.join(t(", and "));
}

/** A letter outside ASCII: most likely typed on a keyboard set to another script. */
function isNonLatinLetter(character: string): boolean {
  return /^\p{L}$/u.test(character) && !/^[A-Za-z]$/.test(character);
}

/** The sentence `code-field-hint` shows for a refused keystroke or paste. */
export function codeProblemMessage(problem: CodeInputProblem, format: CodeFormat = INVITE_CODE_FORMAT): string {
  switch (problem.kind) {
    case "excluded": {
      const never = listInWords(excludedCharacters(format));
      return t("Invite codes never use {never}, so “{character}” can’t be part of one. Check that character again.", {
        never,
        character: visibleCharacter(problem.character),
      });
    }
    case "foreign": {
      // Not "capital letters": a capital Ф is one, and a player on a Russian layout typed it.
      const keyboard = isNonLatinLetter(problem.character)
        ? ` ${t("Check that your keyboard is set to Latin letters.")}`
        : "";
      const refused = t("“{character}” isn’t used in invite codes. They use {alphabet}.", {
        character: visibleCharacter(problem.character),
        alphabet: alphabetInWords(format),
      });
      return `${refused}${keyboard}`;
    }
    case "tooLong":
      return t("That’s longer than an invite code, which has {count} characters.", { count: String(format.length) });
  }
}

/**
 * The refused paste, quoted after the hint so the player can see what they pasted: "You pasted
 * “ABCD-EFGH-JKMN-PQRST”." Trimmed, and cut at `maxInputLength` characters, the most the reading
 * would ever look at.
 */
export function pastedText(text: string, format: CodeFormat = INVITE_CODE_FORMAT): string {
  const trimmed = text.trim();
  const shown = Array.from(trimmed);
  const quoted = shown.length > format.maxInputLength ? `${shown.slice(0, format.maxInputLength).join("")}…` : trimmed;
  return t("You pasted “{text}”.", { text: quoted });
}

/** `code-field-progress`: "12 of 16 characters". */
export function codeProgressText(reading: CodeInputReading, format: CodeFormat = INVITE_CODE_FORMAT): string {
  return t("{count} of {total} characters", { count: String(reading.characters.length), total: String(format.length) });
}

/** Unit conversions, not configuration. */
const MS_PER_MINUTE = 60_000;
const MINUTES_PER_HOUR = 60;

/**
 * A stated wait in words: whole minutes rounded up under an hour ("about a minute", "about 12
 * minutes"), and hours from an hour on ("about an hour", "about 2 hours"), never "about 60 minutes".
 */
export function waitInWords(retryAfterMs: number): string {
  const minutes = Math.ceil(Math.max(0, retryAfterMs) / MS_PER_MINUTE);
  if (minutes <= 1) return t("about a minute");
  if (minutes < MINUTES_PER_HOUR) return t("about {count} minutes", { count: String(minutes) });
  const hours = Math.round(minutes / MINUTES_PER_HOUR);
  return hours <= 1 ? t("about an hour") : t("about {count} hours", { count: String(hours) });
}

/**
 * `invite-attempts`: how many redemptions `GET /api/codes/status` says this account has left, and,
 * with none left, how long until one comes back (R192) when the server said.
 */
export function attemptsText(remaining: number, retryAfterMs: number | null = null): string {
  if (remaining <= 0) {
    return retryAfterMs === null || retryAfterMs <= 0
      ? t("No tries left for now. Try again later.")
      : t("No tries left for now. You can try again in {wait}.", { wait: waitInWords(retryAfterMs) });
  }
  if (remaining === 1) return t("1 try left this hour");
  return t("{count} tries left this hour", { count: String(remaining) });
}
