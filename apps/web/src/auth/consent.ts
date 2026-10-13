// The age screen and the terms box on this device (R757000, R757001; issue #757).
//
// UX, not a rule (CLAUDE.md rule 7). The server decides whether an account owes the age check or
// agreement to the terms (`ageCheckOwed`, `termsOwed` on `/api/auth/me`), counts the age itself and
// keeps only that the check passed (`POST /api/account/consent`, `crates/server/src/api/consent.rs`).
// What the device keeps, each in `localStorage` inside try/catch like every other store:
//
// - THE REFUSAL. An answer under the minimum age refuses sign-up on this device, and the device
//   remembers it, so going back and choosing another year does not work (`signUpRefusedHere`). It
//   names no age, and it is not undone.
// - THE HELD ANSWER. Sign-up asks for the month and year before the account exists, and the account
//   cannot be told until it is signed in, after its email is confirmed. So the answer and the terms
//   version waits here for the address it was given with, as long as this device remembers that
//   address (`AUTH_PENDING_ADDRESS_TTL_SECONDS`), and the consent screen posts it at the first
//   sign-in and forgets it once the server has answered (`heldSignUpConsent`, `forgetSignUpConsent`).
//   One past its time is dropped as the app starts (`dropStaleSignUpConsent`). Another device asks
//   again.

import {
  AUTH_PENDING_ADDRESS_TTL_SECONDS,
  BIRTH_YEAR_EARLIEST,
  MIN_ACCOUNT_AGE_YEARS,
} from "@jackioh/server-config";
import { addressKey } from "./cooldown.ts";

/** R757000: set once an answer under the minimum age refused sign-up on this device. */
export const SIGNUP_REFUSED_KEY = "jackioh.signup.refused";
/** R757000, R757001: the sign-up form's answer, waiting for the account's first sign-in. */
export const SIGNUP_CONSENT_KEY = "jackioh.signup.consent";

/** Unit conversion, not configuration. */
const MS_PER_SECOND = 1000;
const MONTHS_PER_YEAR = 12;

/** A calendar month: `month` is 1–12. */
export type YearMonth = { year: number; month: number };

/** The sign-up form's answer, held for the account's first sign-in. */
export type SignUpConsent = {
  email: string;
  birthYear: number;
  birthMonth: number;
  termsVersion: string;
};

/**
 * R757000: whole years from a birth month to `today`, counting the birth month itself as not yet
 * reached: with no day, the birthday may still be to come. The server's `age_in_years`
 * (`crates/server/src/api/consent.rs`) is the same sum, and it is the one that decides.
 */
export function ageInYears(birthYear: number, birthMonth: number, today: YearMonth): number {
  return today.year - birthYear - (today.month <= birthMonth ? 1 : 0);
}

/** The calendar month now, in UTC, as the server counts it. */
export function utcToday(now: number = Date.now()): YearMonth {
  const at = new Date(now);
  return { year: at.getUTCFullYear(), month: at.getUTCMonth() + 1 };
}

/** R757000: the answer is at least the minimum age, as the server will count it. */
export function oldEnough(birthYear: number, birthMonth: number, today: YearMonth = utcToday()): boolean {
  return ageInYears(birthYear, birthMonth, today) >= MIN_ACCOUNT_AGE_YEARS;
}

/** The years the age screen offers, newest first, down to `BIRTH_YEAR_EARLIEST`. */
export function birthYears(todayYear: number): number[] {
  const years: number[] = [];
  for (let year = todayYear; year >= BIRTH_YEAR_EARLIEST; year -= 1) years.push(year);
  return years;
}

/** The months of the year, 1–12. */
export function birthMonths(): number[] {
  return Array.from({ length: MONTHS_PER_YEAR }, (_, index) => index + 1);
}

function read(key: string): string | null {
  if (typeof window === "undefined") return null;
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function write(key: string, value: string): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // Blocked storage: the server still checks the age at the first sign-in.
  }
}

function remove(key: string): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.removeItem(key);
  } catch {
    // Storage is unavailable.
  }
}

/** R757000: this device refused a sign-up under the minimum age. */
export function signUpRefusedHere(): boolean {
  return read(SIGNUP_REFUSED_KEY) !== null;
}

/** R757000: remember on this device that sign-up was refused. Holds no age and no date. */
export function refuseSignUpHere(): void {
  write(SIGNUP_REFUSED_KEY, "1");
  // The answer that was being held, if any, goes with it.
  remove(SIGNUP_CONSENT_KEY);
}

/** R757000, R757001: hold the sign-up form's answer until the account first signs in. */
export function holdSignUpConsent(consent: SignUpConsent): void {
  write(SIGNUP_CONSENT_KEY, JSON.stringify({ ...consent, at: Date.now() }));
}

/** R757000, R757001: drop the held answer, once the server has answered it. */
export function forgetSignUpConsent(): void {
  remove(SIGNUP_CONSENT_KEY);
}

/** The held answer as stored, if it is whole and within its time. */
function freshHeld(): SignUpConsent | null {
  const raw = read(SIGNUP_CONSENT_KEY);
  if (raw === null) return null;
  let value: unknown;
  try {
    value = JSON.parse(raw);
  } catch {
    return null;
  }
  if (typeof value !== "object" || value === null) return null;
  const held = value as Partial<Record<keyof SignUpConsent | "at", unknown>>;
  const age = typeof held.at === "number" ? Date.now() - held.at : Number.NaN;
  if (
    !(age >= 0 && age <= AUTH_PENDING_ADDRESS_TTL_SECONDS * MS_PER_SECOND) ||
    typeof held.email !== "string" ||
    typeof held.birthYear !== "number" ||
    typeof held.birthMonth !== "number" ||
    typeof held.termsVersion !== "string"
  ) {
    return null;
  }
  return { email: held.email, birthYear: held.birthYear, birthMonth: held.birthMonth, termsVersion: held.termsVersion };
}

/** R757000: a held answer past its time, or unreadable, is dropped. Called as the app starts. */
export function dropStaleSignUpConsent(): void {
  if (read(SIGNUP_CONSENT_KEY) !== null && freshHeld() === null) remove(SIGNUP_CONSENT_KEY);
}

/**
 * R757000, R757001: the held answer for `email`, or null when none is held, it is for another
 * address, or its time is up. An answer past its time is dropped; one within it is only read, so
 * a render may call this.
 */
export function heldSignUpConsent(email: string | null): SignUpConsent | null {
  dropStaleSignUpConsent();
  const held = freshHeld();
  if (held === null || email === null) return null;
  // Another address's answer stays for its own sign-in.
  return addressKey(email) === addressKey(held.email) ? held : null;
}
