// `/login`'s sign-up form and the age screen and terms box on it (R757000, R757001). The form asks;
// the server decides and records at the first sign-in (`auth/ConsentPrompt.tsx`). What is asserted
// here is that nothing is chosen for the player, nothing names an age, and the provider is never
// called for an answer under the minimum, for a missing answer or for an unticked box.

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AUTH_PASSWORD_MIN_LENGTH, BIRTH_YEAR_EARLIEST, MIN_ACCOUNT_AGE_YEARS, TERMS_VERSION } from "@jackioh/server-config";
import { heldSignUpConsent, signUpRefusedHere, utcToday } from "../auth/consent.ts";
import { CONSENT_MESSAGES } from "../auth/ConsentFields.tsx";
import { consentTestid, loginTestid } from "../auth/testids.ts";
import LoginRoute from "./login.tsx";

const URL_ = "https://project.supabase.co";
const KEY = "sb_publishable_test";
const EMAIL = "player@example.com";
const PASSWORD = "x".repeat(AUTH_PASSWORD_MIN_LENGTH);

type Call = { url: URL; body: unknown };

/** The provider, answering every sign-up; each request it sees is recorded. */
function provider(): Call[] {
  const calls: Call[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown, init?: RequestInit) => {
      const url = new URL(String(input));
      calls.push({ url, body: typeof init?.body === "string" ? JSON.parse(init.body) : undefined });
      const text = JSON.stringify({ id: "user-1" });
      return Promise.resolve({
        ok: true,
        status: 200,
        statusText: "200",
        headers: new Headers({ "content-type": "application/json" }),
        json: () => Promise.resolve(JSON.parse(text)),
        text: () => Promise.resolve(text),
      } as unknown as Response);
    }),
  );
  return calls;
}

function toSignUp(): void {
  fireEvent.click(screen.getByTestId(loginTestid.mode));
  expect(screen.getByTestId(loginTestid.form)).toHaveAttribute("data-mode", "signUp");
}

function setField(testId: string, value: string): void {
  fireEvent.change(screen.getByTestId(testId), { target: { value } });
}

/** A valid address and password, and the month and year given (`""` leaves one unchosen). */
function fillIn(month: string, year: string): void {
  setField(loginTestid.email, EMAIL);
  setField(loginTestid.password, PASSWORD);
  setField(consentTestid.birthMonth, month);
  setField(consentTestid.birthYear, year);
}

function submit(): void {
  fireEvent.submit(screen.getByTestId(loginTestid.form));
}

/** A year whose January leaves the player one year over the minimum, and one whose leaves them one under. */
const OLD_ENOUGH = String(utcToday().year - MIN_ACCOUNT_AGE_YEARS - 1);
const TOO_YOUNG = String(utcToday().year - MIN_ACCOUNT_AGE_YEARS + 1);

beforeEach(() => {
  vi.stubEnv("VITE_SUPABASE_URL", URL_);
  vi.stubEnv("VITE_SUPABASE_PUBLISHABLE_KEY", KEY);
  vi.stubEnv("VITE_AUTH_OAUTH_PROVIDERS", "google");
  window.localStorage.clear();
  window.sessionStorage.clear();
  window.history.replaceState(null, "", "/login");
});

afterEach(() => {
  cleanup();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
  window.localStorage.clear();
});

describe("R757000 R757001 the sign-up form", () => {
  it("R757000 sign-up asks month and year, nothing chosen, no age named", () => {
    provider();
    render(<LoginRoute />);
    expect(screen.queryByTestId(consentTestid.birthYear), "sign-in asks no age").toBeNull();
    toSignUp();

    const month = screen.getByTestId<HTMLSelectElement>(consentTestid.birthMonth);
    const year = screen.getByTestId<HTMLSelectElement>(consentTestid.birthYear);
    expect(screen.getByRole("group", { name: "Date of birth" })).toContainElement(month);
    expect(screen.getByLabelText("Month")).toBe(month);
    expect(screen.getByLabelText("Year")).toBe(year);
    for (const select of [month, year]) {
      expect(select.value, "nothing is chosen for the player").toBe("");
      expect(select.options[0]?.value).toBe("");
      expect([...select.options].filter((option) => option.defaultSelected)).toEqual([]);
    }
    expect(month.options).toHaveLength(13);
    expect(month.options[1]?.textContent).toBe("January");
    expect(year.options[1]?.value).toBe(String(utcToday().year));
    expect(year.options[year.options.length - 1]?.value).toBe(String(BIRTH_YEAR_EARLIEST));

    // Nothing on the form names an age or says who may not join; the year options aside, no number.
    const form = screen.getByTestId(loginTestid.form).cloneNode(true) as HTMLElement;
    for (const option of form.querySelectorAll("option")) option.remove();
    expect(form.textContent).not.toMatch(/\d/);
    expect(form.textContent).not.toMatch(/\bage\b|under|young|old|child|parent/i);
  });

  it("R757000 a missing month or year is said at its field, and the provider is not called", () => {
    const calls = provider();
    render(<LoginRoute />);
    toSignUp();
    fillIn("", OLD_ENOUGH);
    fireEvent.click(screen.getByTestId(consentTestid.agree));
    submit();

    expect(calls).toHaveLength(0);
    expect(screen.getByTestId(consentTestid.birthError)).toHaveTextContent(CONSENT_MESSAGES.birthMissing);
    expect(document.activeElement).toBe(screen.getByTestId(consentTestid.birthMonth));
  });

  it("R757001 no box, no provider call", () => {
    const calls = provider();
    render(<LoginRoute />);
    toSignUp();
    fillIn("1", OLD_ENOUGH);
    const box = screen.getByTestId(consentTestid.agree);
    expect(box).not.toBeChecked();
    submit();

    expect(calls).toHaveLength(0);
    expect(screen.getByTestId(consentTestid.agreeError)).toHaveTextContent(CONSENT_MESSAGES.agreeMissing);
    expect(box).toHaveAttribute("aria-invalid", "true");
    expect(document.activeElement).toBe(box);
  });

  it("R757000 under the minimum: no provider call, refusal survives a remount", async () => {
    const calls = provider();
    render(<LoginRoute />);
    toSignUp();
    fillIn("1", TOO_YOUNG);
    fireEvent.click(screen.getByTestId(consentTestid.agree));
    submit();

    expect(calls, "no account is asked for").toHaveLength(0);
    const refused = screen.getByTestId(consentTestid.refused);
    expect(refused).toHaveTextContent("We can't create an account for you.");
    expect(refused).not.toHaveTextContent(/\d|\bage\b|young|old/i);
    expect(within(refused).getByRole("link", { name: "Go to Practice" })).toHaveAttribute("href", "/practice");
    expect(screen.queryByTestId(loginTestid.form), "the form is not offered again").toBeNull();
    expect(screen.queryByTestId(loginTestid.oauth), "nor another way to sign up").toBeNull();
    expect(signUpRefusedHere()).toBe(true);
    expect(heldSignUpConsent(EMAIL), "nothing is held for the server").toBeNull();

    // Going back and trying again on this device finds the refusal.
    cleanup();
    render(<LoginRoute />);
    fireEvent.click(screen.getByTestId(loginTestid.mode));
    expect(screen.getByTestId(consentTestid.refused)).toBeInTheDocument();
    expect(screen.queryByTestId(loginTestid.form)).toBeNull();
    expect(screen.queryByTestId(consentTestid.birthYear)).toBeNull();
    // Signing in to an account that already exists still works.
    fireEvent.click(screen.getByTestId(loginTestid.mode));
    await waitFor(() => {
      expect(screen.getByTestId(loginTestid.form)).toHaveAttribute("data-mode", "signIn");
    });
  });

  it("R757000 a passing sign-up holds the answer", async () => {
    const calls = provider();
    render(<LoginRoute />);
    toSignUp();
    fillIn("1", OLD_ENOUGH);
    fireEvent.click(screen.getByTestId(consentTestid.agree));
    submit();

    await waitFor(() => {
      expect(calls.filter((call) => call.url.pathname === "/auth/v1/signup")).toHaveLength(1);
    });
    // Only the address and the password go to the provider; the answer waits for the first sign-in.
    const body = calls.find((call) => call.url.pathname === "/auth/v1/signup")?.body as Record<string, unknown>;
    expect(Object.keys(body)).not.toContain("birthYear");
    expect(JSON.stringify(body)).not.toContain(OLD_ENOUGH);
    expect(heldSignUpConsent(EMAIL)).toEqual({
      email: EMAIL,
      birthYear: Number(OLD_ENOUGH),
      birthMonth: 1,
      termsVersion: TERMS_VERSION,
    });
    expect(signUpRefusedHere()).toBe(false);
  });
});
