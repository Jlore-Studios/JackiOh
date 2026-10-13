// `auth/ConsentPrompt.tsx` at the gate (`main.tsx`): the consent screen an account meets while it owes
// the age check or agreement to the terms in force (R757000, R757001), asserted through the real
// modules: a session in `localStorage`, `/api/auth/me` and `/api/account/consent` over a stubbed
// `fetch`. The server decides what is owed and counts the age (CLAUDE.md rule 7); what is asserted
// here is where the screen stands, what it sends, and what the device keeps when the server refuses.

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { MIN_ACCOUNT_AGE_YEARS, TERMS_VERSION } from "@jackioh/server-config";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import { holdSignUpConsent, heldSignUpConsent, refuseSignUpHere, signUpRefusedHere, utcToday } from "./consent.ts";
import { CONSENT_TERMS_CHANGED } from "./ConsentPrompt.tsx";
import { consentTestid, inviteTestid, shellTestid, usernameTestid } from "./testids.ts";

const { App, Gated, resetUsernamePromptForTests } = await import("../main.tsx");

type Status = "pending" | "active";
type Owed = { ageCheckOwed: boolean; termsOwed: boolean };
type Me = { status: Status; owed: Owed; extra?: Record<string, unknown> };
type Call = { path: string; method: string; body: unknown };

const EMAIL = "player@example.test";
const BOTH: Owed = { ageCheckOwed: true, termsOwed: true };
const TERMS_ONLY: Owed = { ageCheckOwed: false, termsOwed: true };
const NOTHING: Owed = { ageCheckOwed: false, termsOwed: false };

/** A lazily-imported route chunk plus a stubbed round trip can outrun the 1 s default. */
const SLOW = { timeout: 5_000 } as const;

function jsonResponse(status: number, body: unknown): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    text: () => Promise.resolve(JSON.stringify(body)),
  } as unknown as Response;
}

/**
 * The server: `/api/auth/me` answers `me()` (read on every call, so a test can move the account
 * on), and `/api/account/consent` answers `consent`, after which the account owes nothing.
 */
function serveAs(initial: Me, consent: { status: number; body: unknown } = { status: 200, body: NOTHING }): {
  calls: Call[];
  set: (next: Me) => void;
} {
  let me = initial;
  const calls: Call[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown, init?: RequestInit) => {
      const path = new URL(String(input), "http://localhost").pathname;
      const method = (init?.method ?? "GET").toUpperCase();
      calls.push({ path, method, body: typeof init?.body === "string" ? JSON.parse(init.body) : undefined });
      if (path === "/api/auth/me") {
        return Promise.resolve(
          jsonResponse(200, {
            profile: { id: "profile-1", status: me.status },
            needsInviteCode: me.status === "pending",
            emailVerified: true,
            currentMatchId: null,
            email: EMAIL,
            ...me.owed,
            ...me.extra,
          }),
        );
      }
      if (path === "/api/account/consent") {
        if (consent.status === 200) me = { ...me, owed: NOTHING };
        return Promise.resolve(jsonResponse(consent.status, consent.body));
      }
      if (path === "/api/codes/status") {
        return Promise.resolve(jsonResponse(200, { redemptionEnabled: true, retryAfterMs: 0 }));
      }
      return Promise.resolve(jsonResponse(404, { error: { code: "not_found", message: `no stub for ${path}` } }));
    }),
  );
  return {
    calls,
    set: (next) => {
      me = next;
    },
  };
}

function consentPosts(calls: readonly Call[]): Call[] {
  return calls.filter((call) => call.path === "/api/account/consent" && call.method === "POST");
}

function signedIn(): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "e2e-token" }));
}

function at(path: string): void {
  window.history.replaceState(null, "", path);
}

/** A year whose January leaves the player one year over the minimum. */
const OLD_ENOUGH = utcToday().year - MIN_ACCOUNT_AGE_YEARS - 1;

beforeEach(() => {
  window.localStorage.clear();
  resetUsernamePromptForTests();
  at("/");
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
});

describe("R757000 the age screen at the gate", () => {
  it("R757000 pending meets the consent screen before the code screen", async () => {
    signedIn();
    const server = serveAs({ status: "pending", owed: BOTH });
    at("/invite");
    render(<App />);

    const prompt = await screen.findByTestId(consentTestid.prompt, undefined, SLOW);
    expect(screen.queryByTestId(inviteTestid.input), "no code screen yet").toBeNull();
    expect(screen.queryByTestId(consentTestid.changed), "a first agreement, not a changed one").toBeNull();
    expect(within(prompt).getByTestId<HTMLSelectElement>(consentTestid.birthYear).value).toBe("");
    expect(within(prompt).getByTestId(consentTestid.agree)).not.toBeChecked();
    expect(within(prompt).getByTestId(shellTestid.signOut), "a way out").toBeInTheDocument();

    // Nothing is sent until both are answered.
    fireEvent.click(screen.getByTestId(consentTestid.submit));
    expect(screen.getByTestId(consentTestid.birthError)).toBeInTheDocument();
    expect(consentPosts(server.calls)).toHaveLength(0);

    fireEvent.change(screen.getByTestId(consentTestid.birthMonth), { target: { value: "1" } });
    fireEvent.change(screen.getByTestId(consentTestid.birthYear), { target: { value: String(OLD_ENOUGH) } });
    fireEvent.click(screen.getByTestId(consentTestid.agree));
    fireEvent.click(screen.getByTestId(consentTestid.submit));

    await waitFor(() => {
      expect(consentPosts(server.calls)).toHaveLength(1);
    });
    expect(consentPosts(server.calls)[0]?.body).toEqual({
      termsVersion: TERMS_VERSION,
      birthYear: OLD_ENOUGH,
      birthMonth: 1,
    });
    expect(await screen.findByTestId(inviteTestid.input, undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId(consentTestid.prompt)).toBeNull();
  });

  it("R757000 a held answer posts without asking", async () => {
    signedIn();
    holdSignUpConsent({ email: EMAIL, birthYear: OLD_ENOUGH, birthMonth: 6, termsVersion: TERMS_VERSION });
    const server = serveAs({ status: "pending", owed: BOTH });
    at("/invite");
    render(<App />);

    expect(await screen.findByTestId(inviteTestid.input, undefined, SLOW)).toBeInTheDocument();
    expect(consentPosts(server.calls)).toHaveLength(1);
    expect(consentPosts(server.calls)[0]?.body).toEqual({
      termsVersion: TERMS_VERSION,
      birthYear: OLD_ENOUGH,
      birthMonth: 6,
    });
    expect(heldSignUpConsent(EMAIL), "the held answer is gone once the server has it").toBeNull();
  });

  it("R757000 a held answer for another address is not sent", async () => {
    signedIn();
    holdSignUpConsent({ email: "other@example.test", birthYear: OLD_ENOUGH, birthMonth: 6, termsVersion: TERMS_VERSION });
    const server = serveAs({ status: "pending", owed: BOTH });
    at("/invite");
    render(<App />);

    expect(await screen.findByTestId(consentTestid.prompt, undefined, SLOW)).toHaveAttribute("data-outcome", "asking");
    expect(consentPosts(server.calls)).toHaveLength(0);
    expect(screen.getByTestId<HTMLSelectElement>(consentTestid.birthYear).value).toBe("");
  });

  it("R757000 under_age signs out and locks", async () => {
    signedIn();
    const server = serveAs(
      { status: "pending", owed: BOTH },
      { status: 403, body: { error: { code: "under_age", message: "We can't create an account for you." } } },
    );
    at("/invite");
    render(<App />);

    await screen.findByTestId(consentTestid.prompt, undefined, SLOW);
    fireEvent.change(screen.getByTestId(consentTestid.birthMonth), { target: { value: "1" } });
    fireEvent.change(screen.getByTestId(consentTestid.birthYear), { target: { value: String(OLD_ENOUGH) } });
    fireEvent.click(screen.getByTestId(consentTestid.agree));
    fireEvent.click(screen.getByTestId(consentTestid.submit));

    const refused = await screen.findByTestId(consentTestid.refused, undefined, SLOW);
    expect(refused).toHaveTextContent("We can't create an account for you.");
    expect(refused).not.toHaveTextContent(/\d|\bage\b|young|old/i);
    expect(consentPosts(server.calls)).toHaveLength(1);
    expect(window.localStorage.getItem(E2E_SESSION_STORAGE_KEY), "the deleted account's session is gone").toBeNull();
    expect(signUpRefusedHere()).toBe(true);
    expect(screen.queryByTestId(consentTestid.submit)).toBeNull();
  });

  it("R757000 a device that refused a sign-up before is not asked again", async () => {
    signedIn();
    refuseSignUpHere();
    const server = serveAs({ status: "pending", owed: BOTH });
    at("/invite");
    render(<App />);

    const prompt = await screen.findByTestId(consentTestid.prompt, undefined, SLOW);
    expect(within(prompt).getByTestId(consentTestid.refused)).toBeInTheDocument();
    expect(within(prompt).queryByTestId(consentTestid.birthYear)).toBeNull();
    expect(within(prompt).getByTestId(shellTestid.signOut), "it can still sign out").toBeInTheDocument();
    expect(consentPosts(server.calls)).toHaveLength(0);
  });
});

describe("R757001 the terms at the gate", () => {
  it("R757001 asked on /decks, not /account or mid-match", async () => {
    signedIn();
    const server = serveAs({ status: "active", owed: TERMS_ONLY });
    at("/decks");
    render(<App />);

    const prompt = await screen.findByTestId(consentTestid.prompt, undefined, SLOW);
    expect(within(prompt).getByTestId(consentTestid.changed)).toHaveTextContent(
      "The Terms or the Privacy Policy changed since you last agreed to them.",
    );
    expect(within(prompt).queryByTestId(consentTestid.birthYear), "an active account is not asked its age").toBeNull();
    fireEvent.click(screen.getByTestId(consentTestid.agree));
    fireEvent.click(screen.getByTestId(consentTestid.submit));
    await waitFor(() => {
      expect(consentPosts(server.calls)).toHaveLength(1);
    });
    expect(consentPosts(server.calls)[0]?.body).toEqual({ termsVersion: TERMS_VERSION });
    await waitFor(() => {
      expect(screen.queryByTestId(consentTestid.prompt)).toBeNull();
    }, SLOW);
    cleanup();
    resetUsernamePromptForTests();

    // The account page opens all the same, so a player who will not agree can delete or sign out.
    server.set({ status: "active", owed: TERMS_ONLY });
    at("/account");
    render(<App />);
    await waitFor(() => {
      expect(screen.queryByTestId(shellTestid.loading)).toBeNull();
    }, SLOW);
    expect(screen.queryByTestId(consentTestid.prompt)).toBeNull();
    cleanup();

    // Nor in front of a match or a Conquest series the account is in.
    for (const playing of [{ currentMatchId: "m-1" }, { currentSeriesId: "s-1" }]) {
      server.set({ status: "active", owed: TERMS_ONLY, extra: playing });
      render(<Gated>{() => <p>the board</p>}</Gated>);
      expect(await screen.findByText("the board", undefined, SLOW)).toBeInTheDocument();
      expect(screen.queryByTestId(consentTestid.prompt)).toBeNull();
      cleanup();
    }
  });

  it("R757001 no fields, nothing owed", async () => {
    signedIn();
    serveAs({ status: "active", owed: NOTHING });
    render(<Gated>{() => <p>the gate opened</p>}</Gated>);
    expect(await screen.findByText("the gate opened", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId(consentTestid.prompt)).toBeNull();
  });

  it("R757001 a server that sends neither flag owes nothing", async () => {
    signedIn();
    vi.stubGlobal(
      "fetch",
      vi.fn(() =>
        Promise.resolve(
          jsonResponse(200, {
            profile: { id: "profile-1", status: "active" },
            needsInviteCode: false,
            emailVerified: true,
            currentMatchId: null,
            email: EMAIL,
          }),
        ),
      ),
    );
    render(<Gated>{() => <p>the gate opened</p>}</Gated>);
    expect(await screen.findByText("the gate opened", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId(consentTestid.prompt)).toBeNull();
  });

  it("R757001 comes before the username prompt", async () => {
    signedIn();
    serveAs({
      status: "active",
      owed: TERMS_ONLY,
      extra: { username: { name: "Player#7", nextChangeAt: null, promptOwed: true } },
    });
    render(<Gated>{() => <p>the gate opened</p>}</Gated>);
    expect(await screen.findByTestId(consentTestid.prompt, undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId(usernameTestid.prompt)).toBeNull();
    fireEvent.click(screen.getByTestId(consentTestid.agree));
    fireEvent.click(screen.getByTestId(consentTestid.submit));
    expect(await screen.findByTestId(usernameTestid.prompt, undefined, SLOW)).toBeInTheDocument();
  });

  it("R757001 a re-read that finds the terms owed once the screen is up leaves the screen in place", async () => {
    signedIn();
    const server = serveAs({ status: "active", owed: NOTHING });
    render(<Gated>{({ me }) => <p>{me.termsOwed === true ? "the gate read again" : "the gate opened"}</p>}</Gated>);
    expect(await screen.findByText("the gate opened", undefined, SLOW)).toBeInTheDocument();

    // The terms changed while the player was here: the screen takes the new read and stays.
    server.set({ status: "active", owed: TERMS_ONLY });
    const { announceAccountChange } = await import("../net/gate.ts");
    announceAccountChange();
    expect(await screen.findByText("the gate read again", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId(consentTestid.prompt)).toBeNull();
  });

  it("R757001 terms newer than the page's are a reload, not a retry", async () => {
    signedIn();
    serveAs(
      { status: "active", owed: TERMS_ONLY },
      { status: 409, body: { error: { code: "update_required", message: CONSENT_TERMS_CHANGED } } },
    );
    render(<Gated>{() => <p>the gate opened</p>}</Gated>);
    await screen.findByTestId(consentTestid.prompt, undefined, SLOW);
    fireEvent.click(screen.getByTestId(consentTestid.agree));
    fireEvent.click(screen.getByTestId(consentTestid.submit));
    expect(await screen.findByTestId(consentTestid.error, undefined, SLOW)).toHaveTextContent(CONSENT_TERMS_CHANGED);
    expect(screen.queryByText("the gate opened")).toBeNull();
  });
});
