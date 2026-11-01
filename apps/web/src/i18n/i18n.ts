// The client's reader language (#711, R711000): which of LOCALES the screens are written in, and the
// lookup every user-visible sentence goes through. Presentation only (CLAUDE.md rule 7): no rule reads
// it, and nothing about it reaches the engine, the wire or the server.
//
// The English stays inline, wrapped in `t()`, and reads as itself where the language has no entry.
// The engine's and the server's English arrives complete from Rust, so a key holding `{name}` tokens
// also matches a filled-in sentence; `i18n.test.ts` checks each literal in `t()` has a `zh-CN.json` entry.
// A language is one `<tag>.json`, its card tables (`cards/chinese.ts`) and one line of LOCALES; each
// dictionary loads on demand, so the English bundle carries none. Only the language button writes
// the choice down, never the browser's pick. The button stays until the device settles (R1916000).
// Never call `t()` at module scope: it would freeze the language the module loaded in.

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

/** Every language the client reads, in the order the button cycles them. */
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

/** The boots since this device's last language change (R1916000), beside the language itself. */
export const LOCALE_VISITS_STORAGE_KEY = "jackioh.locale.settledVisits";

/** Boots with no language change after which the device has settled and the language button leaves (R1916000). */
export const LANGUAGE_SETTLED_VISITS = 3;

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
/** Whether this boot found the device settled (R1916000); a language change clears it. */
let settled = false;

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
      const inParentheses = key[match.index - 1] === "(" && key[end] === ")";
      enclosed &&= inParentheses;
      source += escapeRegex(key.slice(last, match.index)) + (inParentheses ? "([^()]+?)" : "(.+?)");
      names.push(match[1] ?? "");
      last = end;
    }
    // A key with fewer than two words of its own ("{n} cards") would half-translate any sentence that
    // ends the same way, so it answers only its own calls, unless each of its tokens is in parentheses.
    const own = (key.replace(TOKEN, " ").match(/\p{L}+/gu) ?? []).length;
    if (names.length > 0 && (own >= MIN_PATTERN_WORDS || (enclosed && own > 0))) {
      const regex = new RegExp(`^${source}${escapeRegex(key.slice(last))}$`, "s");
      patterns.push({ key, names, regex, literal: key.replace(TOKEN, "").length });
    }
  }
  // The most specific key first, before a shorter key that would also match the same sentence.
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
 * When a `{name}` key matches a complete English sentence, each value is looked up in turn, as the
 * engine nests its sentences; a value is shorter than the sentence it came from, so the lookup ends.
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
 * tokens may also match a finished sentence.
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

/** A stored count: a whole number of boots, anything else none. */
function parseVisits(raw: string | null): number {
  const visits = Number(raw);
  return Number.isSafeInteger(visits) && visits > 0 ? visits : 0;
}

function setSettled(next: boolean): void {
  if (settled === next) return;
  settled = next;
  notify();
}

/**
 * Once per boot, before the first render (main.tsx): one more boot on this device's language. The
 * device has settled once the count it wrote reaches LANGUAGE_SETTLED_VISITS; with storage
 * unavailable it never settles, so the button is never lost (R1916000).
 */
export function countLocaleVisit(): void {
  let visits: number;
  try {
    visits = parseVisits(window.localStorage.getItem(LOCALE_VISITS_STORAGE_KEY)) + 1;
    window.localStorage.setItem(LOCALE_VISITS_STORAGE_KEY, String(visits));
  } catch {
    setSettled(false);
    return;
  }
  setSettled(visits >= LANGUAGE_SETTLED_VISITS);
}

/** A language change: the count starts again and the button is back (R1916000). */
function restartLocaleVisits(): void {
  try {
    window.localStorage.setItem(LOCALE_VISITS_STORAGE_KEY, "0");
  } catch {
    // Storage is unavailable: nothing was counted.
  }
  setSettled(false);
}

/** Whether this device has settled on its language (R1916000): the language button then shows nowhere. */
export function isLocaleSettled(): boolean {
  return settled;
}

/** isLocaleSettled(); the component re-renders when it changes. */
export function useLocaleSettled(): boolean {
  return useSyncExternalStore(subscribeLocale, isLocaleSettled, isLocaleSettled);
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
  if (remember) {
    persist(locale);
    restartLocaleVisits();
  }
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

/** Forgets this device's choice, back to what a first visit picks, and starts the count again (R1916000): the settings panel's reset. */
export function resetLocale(): Promise<Locale> {
  try {
    window.localStorage.removeItem(LOCALE_STORAGE_KEY);
  } catch {
    // Storage is unavailable: nothing was kept.
  }
  restartLocaleVisits();
  return initLocale();
}

/** The language after `locale` in LOCALES, which the button switches to. */
export function nextLocale(locale: Locale): Locale {
  return LOCALE_TAGS[(LOCALE_TAGS.indexOf(locale) + 1) % LOCALE_TAGS.length] ?? DEFAULT_LOCALE;
}

/** The language tag numbers and moments are written in: the reader's, or for English the browser's own. */
function formatTag(): string | undefined {
  return state.locale === DEFAULT_LOCALE ? undefined : state.locale;
}

/** A number in the reader's language. */
export function formatNumber(value: number, options?: Intl.NumberFormatOptions): string {
  return new Intl.NumberFormat(formatTag(), options).format(value);
}

/** A span of seconds as the clock reads it, never grouped. */
export function formatSeconds(seconds: number): string {
  return new Intl.NumberFormat(state.locale, { style: "unit", unit: "second", unitDisplay: "narrow", useGrouping: false }).format(seconds);
}

/** A moment (epoch ms, or a Date) as the reader's date and time. */
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
  settled = false;
  try {
    window.localStorage.removeItem(LOCALE_STORAGE_KEY);
    window.localStorage.removeItem(LOCALE_VISITS_STORAGE_KEY);
    document.documentElement.lang = DEFAULT_LOCALE;
  } catch {
    // Nothing to reset.
  }
}
