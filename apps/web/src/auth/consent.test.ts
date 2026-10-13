// `auth/consent.ts`: the age screen's sum and what this device keeps for it (R757000, R757001). The
// server counts the age again and decides (`crates/server/src/api/consent.rs`); what is asserted
// here is that the client counts it the same way, keeps no age in its refusal, and hands a held
// answer only to the address it was given with, within its time.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AUTH_PENDING_ADDRESS_TTL_SECONDS, BIRTH_YEAR_EARLIEST, MIN_ACCOUNT_AGE_YEARS, TERMS_VERSION } from "@jackioh/server-config";
import {
  SIGNUP_CONSENT_KEY,
  SIGNUP_REFUSED_KEY,
  ageInYears,
  birthYears,
  dropStaleSignUpConsent,
  forgetSignUpConsent,
  heldSignUpConsent,
  holdSignUpConsent,
  oldEnough,
  refuseSignUpHere,
  signUpRefusedHere,
  utcToday,
} from "./consent.ts";

/** Unit conversion, not configuration. */
const MS_PER_SECOND = 1000;

const HELD = { email: "Player@Example.com", birthYear: 2000, birthMonth: 4, termsVersion: TERMS_VERSION } as const;

beforeEach(() => {
  window.localStorage.clear();
});

afterEach(() => {
  vi.useRealTimers();
  window.localStorage.clear();
});

describe("R757000 the age screen's sum", () => {
  it("R757000 ageInYears counts the birth month as not yet reached", () => {
    // The server's own cases (consent.rs): born in October 2013 is 12 in October 2026, 13 in November.
    expect(ageInYears(2013, 10, { year: 2026, month: 10 })).toBe(12);
    expect(ageInYears(2013, 10, { year: 2026, month: 11 })).toBe(13);
    expect(ageInYears(2013, 10, { year: 2027, month: 1 })).toBe(13);
    expect(ageInYears(2013, 12, { year: 2026, month: 12 })).toBe(12);
    expect(ageInYears(2013, 1, { year: 2026, month: 2 })).toBe(13);
  });

  it("R757000 oldEnough is the minimum, counted in UTC", () => {
    const today = { year: 2026, month: 10 };
    expect(oldEnough(today.year - MIN_ACCOUNT_AGE_YEARS, today.month - 1, today)).toBe(true);
    expect(oldEnough(today.year - MIN_ACCOUNT_AGE_YEARS, today.month, today)).toBe(false);
    // 2025-10-01T00:00:00Z, and the millisecond before it.
    expect(utcToday(1_759_276_800_000)).toEqual({ year: 2025, month: 10 });
    expect(utcToday(1_759_276_800_000 - 1)).toEqual({ year: 2025, month: 9 });
  });

  it("R757000 the years offered run from this year down to the earliest", () => {
    const years = birthYears(2026);
    expect(years[0]).toBe(2026);
    expect(years.at(-1)).toBe(BIRTH_YEAR_EARLIEST);
    expect(years).toHaveLength(2026 - BIRTH_YEAR_EARLIEST + 1);
  });
});

describe("R757000 what the device keeps", () => {
  it("R757000 a refusal is remembered, holds no age, and drops a held answer", () => {
    holdSignUpConsent(HELD);
    expect(signUpRefusedHere()).toBe(false);
    refuseSignUpHere();
    expect(signUpRefusedHere()).toBe(true);
    expect(window.localStorage.getItem(SIGNUP_REFUSED_KEY)).toBe("1");
    expect(window.localStorage.getItem(SIGNUP_CONSENT_KEY)).toBeNull();
  });

  it("R757000 a held answer is taken once, by its address", () => {
    holdSignUpConsent(HELD);
    expect(heldSignUpConsent("someone@else.com"), "another address's answer is not handed out").toBeNull();
    expect(heldSignUpConsent(null)).toBeNull();
    // The provider compares addresses trimmed and case-insensitively, and so does this.
    expect(heldSignUpConsent(" player@example.com ")).toEqual(HELD);
    expect(heldSignUpConsent("player@example.com"), "reading it changes nothing").toEqual(HELD);
    forgetSignUpConsent();
    expect(heldSignUpConsent("player@example.com")).toBeNull();
  });

  it("R757000 a held answer lasts as long as the device remembers the sign-up's address", () => {
    vi.useFakeTimers({ now: 1_800_000_000_000 });
    holdSignUpConsent(HELD);
    vi.advanceTimersByTime(AUTH_PENDING_ADDRESS_TTL_SECONDS * MS_PER_SECOND);
    dropStaleSignUpConsent();
    expect(heldSignUpConsent(HELD.email), "still within its time").toEqual(HELD);
    vi.advanceTimersByTime(1);
    dropStaleSignUpConsent();
    expect(window.localStorage.getItem(SIGNUP_CONSENT_KEY), "dropped once its time is up").toBeNull();
    expect(heldSignUpConsent(HELD.email)).toBeNull();
  });

  it("R757000 an unreadable held answer is dropped and hands out nothing", () => {
    window.localStorage.setItem(SIGNUP_CONSENT_KEY, "{not json");
    expect(heldSignUpConsent(HELD.email)).toBeNull();
    expect(window.localStorage.getItem(SIGNUP_CONSENT_KEY)).toBeNull();
    window.localStorage.setItem(SIGNUP_CONSENT_KEY, JSON.stringify({ ...HELD, birthYear: "2000", at: Date.now() }));
    expect(heldSignUpConsent(HELD.email)).toBeNull();
  });
});
