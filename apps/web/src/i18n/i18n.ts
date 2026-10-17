// The client's reader language (#711, R711000): which of LOCALES the screens are written in, and the
// lookup every user-visible sentence goes through. Presentation only (CLAUDE.md rule 7): no rule reads
// it, and nothing about it reaches the engine, the wire or the server.
//
// The English stays inline where it is written, wrapped in `t()`: `t("Your turn")` is the chosen
// language's entry for that exact English string, with `{name}` parameters filled, and the English
// itself when the language has no entry, so English needs no file and a sentence nobody has
// translated yet reads as it always did. The engine's and the server's English (prompt titles,
// refusals, validator issues) is looked up where it is displayed, by the same call; a dictionary key
// that holds `{name}` tokens also matches a sentence whose values are already filled in, since those
// arrive complete from Rust ("Choose 2 cards" finds "Choose {n} cards").
// That fallback is for the engine's and the server's sentences: every literal the client itself
// wraps in `t()` has an entry in `zh-CN.json` (an identity entry when it reads as written, as
// "English" has), which `i18n.test.ts` checks (R711000).
//
// A language is one dictionary file (`<tag>.json`, English to its words; a value is a string, or
// `{ one, other }` and the other CLDR plural categories, chosen by `Intl.PluralRules` over the
// `count` parameter), its card tables beside the catalog (`cards/chinese.ts`, CARD_TABLES) and one
// line of LOCALES. A dictionary loads on demand (a dynamic `import()`), so the English bundle does
// not carry any of them.
//
// The choice is the device's: `localStorage["jackioh.locale"]` inside try/catch, applied live (the
// store notifies, `useLocale()` re-renders) and set on `<html lang>`. A first visit takes the best
// match of `navigator.languages`, else English; the browser's pick is not written down, so only a
// click on the language button fixes it. Never call `t()` at module scope: it would freeze the
// language the module loaded in.

import { useSyncExternalStore } from "react";

/** A plural entry: `other` always, and the CLDR categories the language needs (`one`, `few`, …). */
export type PluralEntry = Partial<Record<Intl.LDMLPluralRule, string>> & { other: string };

/** One dictionary value: the words, or the words by plural category. */
export type Entry = string | PluralEntry;

/** English sentence (exact) to its entry. */
export type Dictionary = Readonly<Record<string, Entry>>;

export type Params = Readonly<Record<string, string | number>>;

type LocaleSpec = {
  /** The language's own name for itself. */
  name: string;
  /** What the language button shows. */
  short: string;
  /** Loads the dictionary on demand; English needs none. */
  load: () => Promise<Dictionary>;
  /** How a calendar day (a patch's date) reads; absent: as the ISO date it is. */
  day?: Intl.DateTimeFormatOptions;
};

/** Every language the client reads, in the order the button cycles them. A new one is a line here. */
export const LOCALES = {
  en: { name: "English", short: "EN", load: () => Promise.resolve({}) },
  "zh-CN": {
    name: "简体中文",
    short: "中文",
    load: async () => (await import("./zh-CN.json", { with: { type: "json" } })).default,
    day: { year: "numeric", month: "long", day: "numeric" },
  },
} as const satisfies Record<string, LocaleSpec>;

export type Locale = keyof typeof LOCALES;

export const DEFAULT_LOCALE: Locale = "en";

export const LOCALE_STORAGE_KEY = "jackioh.locale";

const LOCALE_TAGS = Object.keys(LOCALES) as Locale[];

/** Total: anything that is not a language of LOCALES is null. */
export function parseLocale(raw: unknown): Locale | null {
  return LOCALE_TAGS.find((tag) => tag === raw) ?? null;
}

/** The best of LOCALES for a browser's language list: an exact tag, else the same primary language. */
export function bestLocale(languages: readonly string[]): Locale {
  for (const language of languages) {
    const lower = language.toLowerCase();
    const exact = LOCALE_TAGS.find((tag) => tag.toLowerCase() === lower);
    if (exact !== undefined) return exact;
    const primary = lower.split("-")[0];
    const near = LOCALE_TAGS.find((tag) => tag.toLowerCase().split("-")[0] === primary);
    if (near !== undefined) return near;
  }
  return DEFAULT_LOCALE;
}

type Pattern = { key: string; names: readonly string[]; regex: RegExp; literal: number };

type State = {
  locale: Locale;
  dictionary: Dictionary;
  /** Patterns of the dictionary's keys that hold `{name}` tokens, built on the first miss. */
  patterns: readonly Pattern[] | null;
  /** English sentence to its finished text, for sentences looked up with no parameters. */
  seen: Map<string, string>;
};

function stateFor(locale: Locale, dictionary: Dictionary): State {
  return { locale, dictionary, patterns: null, seen: new Map() };
}

let state: State = stateFor(DEFAULT_LOCALE, {});
let latest = 0;
const listeners = new Set<() => void>();

const TOKEN = /\{(\w+)\}/g;

/** The fewest words of its own a `{name}` key needs to match a finished sentence. */
const MIN_PATTERN_WORDS = 2;

function escapeRegex(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function patternsOf(dictionary: Dictionary): Pattern[] {
  const patterns: Pattern[] = [];
  for (const key of Object.keys(dictionary)) {
    const names: string[] = [];
    let source = "";
    let last = 0;
    let enclosed = true;
    for (const match of key.matchAll(TOKEN)) {
      const end = match.index + match[0].length;
      // A token in parentheses ("({cost}) Cost") holds only what stands inside them, never words.
      const inParentheses = key[match.index - 1] === "(" && key[end] === ")";
      enclosed &&= inParentheses;
      source += escapeRegex(key.slice(last, match.index)) + (inParentheses ? "([^()]+?)" : "(.+?)");
      names.push(match[1] ?? "");
      last = end;
    }
    // A key whose own words are fewer than two ("{n} cards") would take any sentence that ends the
    // same way and leave it half translated, so it only ever answers its own calls, unless each of
    // its tokens is held in parentheses ("({cost}) Cost" takes "(3) Cost" and nothing longer).
    const own = (key.replace(TOKEN, " ").match(/\p{L}+/gu) ?? []).length;
    if (names.length > 0 && (own >= MIN_PATTERN_WORDS || (enclosed && own > 0))) {
      const regex = new RegExp(`^${source}${escapeRegex(key.slice(last))}$`, "s");
      patterns.push({ key, names, regex, literal: key.replace(TOKEN, "").length });
    }
  }
  // The most specific key first: "In the {mode} queue. …" is tried before a shorter key that would
  // also match the same sentence.
  return patterns.sort((a, b) => b.literal - a.literal);
}

function fill(text: string, params: Params): string {
  return text.replace(TOKEN, (token, name: string) => (Object.hasOwn(params, name) ? String(params[name]) : token));
}

/** An entry's words before filling: a plural entry's category for `params.count` in `locale`. */
export function words(entry: Entry, locale: Locale, params: Params | undefined): string {
  if (typeof entry === "string") return entry;
  return entry[new Intl.PluralRules(locale).select(Number(params?.["count"]))] ?? entry.other;
}

/**
 * The sentence a complete English sentence reads as when a `{name}` key matches it. Each value is
 * looked up in turn, since the engine nests its sentences ("{label} (next turn)", "Next {type} costs
 * {amount} less"); a value is shorter than the sentence it came from, so the lookup ends.
 */
function matched(english: string): string | null {
  state.patterns ??= patternsOf(state.dictionary);
  for (const pattern of state.patterns) {
    const found = pattern.regex.exec(english);
    if (found === null) continue;
    const values: Record<string, string> = {};
    pattern.names.forEach((name, index) => {
      values[name] = t(found[index + 1] ?? "");
    });
    const entry = state.dictionary[pattern.key];
    return entry === undefined ? null : fill(words(entry, state.locale, values), values);
  }
  return null;
}

/**
 * The chosen language's words for this exact English sentence, `{name}`s filled from `params`; the
 * English, filled the same way, when the language has none. Without `params` a key with `{name}`
 * tokens may also match a finished sentence (the engine's and the server's).
 */
export function t(english: string, params?: Params): string {
  const { dictionary } = state;
  if (Object.hasOwn(dictionary, english)) {
    const entry = dictionary[english];
    if (entry !== undefined) {
      const text = words(entry, state.locale, params);
      return params === undefined ? text : fill(text, params);
    }
  }
  if (params !== undefined) return fill(english, params);
  if (state.locale === DEFAULT_LOCALE) return english;
  const seen = state.seen.get(english);
  if (seen !== undefined) return seen;
  const found = matched(english) ?? english;
  state.seen.set(english, found);
  return found;
}

export function getLocale(): Locale {
  return state.locale;
}

/** Whether the reader reads Simplified Chinese: a card table is drawn for them (cards/chinese.ts). */
export function isZhCn(): boolean {
  return state.locale === "zh-CN";
}

function notify(): void {
  for (const listener of [...listeners]) {
    try {
      listener();
    } catch {
      // A throwing subscriber must not stop the others from hearing the change.
    }
  }
}

export function subscribeLocale(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** The reader's language; the component re-renders when it changes. */
export function useLocale(): Locale {
  return useSyncExternalStore(subscribeLocale, getLocale, getLocale);
}

function readStored(): Locale | null {
  try {
    return parseLocale(window.localStorage.getItem(LOCALE_STORAGE_KEY));
  } catch {
    return null;
  }
}

function persist(locale: Locale): void {
  try {
    window.localStorage.setItem(LOCALE_STORAGE_KEY, locale);
  } catch {
    // Storage is unavailable or full: the choice still holds for this page.
  }
}

async function apply(locale: Locale, remember: boolean): Promise<Locale> {
  const mine = ++latest;
  let dictionary: Dictionary;
  try {
    dictionary = await LOCALES[locale].load();
  } catch {
    // The dictionary's chunk did not arrive (offline): the language stays as it was.
    return state.locale;
  }
  // A later choice made while this one loaded has the last word.
  if (mine !== latest) return state.locale;
  state = stateFor(locale, dictionary);
  if (remember) persist(locale);
  try {
    document.documentElement.lang = locale;
  } catch {
    // No document (a worker): nothing to set.
  }
  notify();
  return locale;
}

/** Switches the language live, loading its dictionary first, and keeps the choice on this device. */
export function setLocale(locale: Locale): Promise<Locale> {
  return apply(locale, true);
}

/**
 * Before the first render: the device's choice, else the best match of `navigator.languages`, else
 * English. Resolves once the dictionary is in, so the first paint is already in the language.
 */
export function initLocale(): Promise<Locale> {
  let languages: readonly string[] = [];
  try {
    languages = navigator.languages.length > 0 ? navigator.languages : [navigator.language];
  } catch {
    // No navigator: English.
  }
  return apply(readStored() ?? bestLocale(languages), false);
}

/** Forgets this device's choice, back to what a first visit picks: the settings panel's reset. */
export function resetLocale(): Promise<Locale> {
  try {
    window.localStorage.removeItem(LOCALE_STORAGE_KEY);
  } catch {
    // Storage is unavailable: nothing was kept.
  }
  return initLocale();
}

/** The language after `locale` in LOCALES, which the button switches to. */
export function nextLocale(locale: Locale): Locale {
  return LOCALE_TAGS[(LOCALE_TAGS.indexOf(locale) + 1) % LOCALE_TAGS.length] ?? DEFAULT_LOCALE;
}

/**
 * The language tag numbers and moments are written in: the reader's, or for English the browser's own,
 * as the bare `toLocaleString()` these helpers replace wrote them, so English reads as it always did.
 */
function formatTag(): string | undefined {
  return state.locale === DEFAULT_LOCALE ? undefined : state.locale;
}

/** A number in the reader's language (replaces the bare `toLocaleString()`). */
export function formatNumber(value: number, options?: Intl.NumberFormatOptions): string {
  return new Intl.NumberFormat(formatTag(), options).format(value);
}

/** A span of seconds as the clock reads it: "5s", "7200s", "5秒"; never grouped, as `${n}s` never was. */
export function formatSeconds(seconds: number): string {
  return new Intl.NumberFormat(state.locale, { style: "unit", unit: "second", unitDisplay: "narrow", useGrouping: false }).format(seconds);
}

/** A moment (epoch ms, or a Date) as the reader's date and time (replaces the bare `toLocaleString()`). */
export function formatDateTime(at: Date | number, options?: Intl.DateTimeFormatOptions): string {
  return new Date(at).toLocaleString(formatTag(), options);
}

/** A calendar day given as ISO `YYYY-MM-DD` (a patch's date): the ISO date itself in a language with no `day` format. */
export function formatDay(iso: string): string {
  const spec: LocaleSpec = LOCALES[state.locale];
  const at = new Date(`${iso}T00:00:00Z`);
  if (spec.day === undefined || Number.isNaN(at.getTime())) return iso;
  return new Intl.DateTimeFormat(state.locale, { ...spec.day, timeZone: "UTC" }).format(at);
}

/** Back to English with no listeners and no choice kept. Tests only. */
export function resetLocaleForTests(): void {
  latest++;
  state = stateFor(DEFAULT_LOCALE, {});
  listeners.clear();
  try {
    window.localStorage.removeItem(LOCALE_STORAGE_KEY);
    document.documentElement.lang = DEFAULT_LOCALE;
  } catch {
    // Nothing to reset.
  }
}
